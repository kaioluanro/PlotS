// Gera, a partir das duas imagens de `imgs/`, tudo o que o programa usa delas, so com a biblioteca
// padrao do Node: a imagem da tela de abertura, o icone do programa (.ico) e a marca da barra.
// Uso: node src-tauri/icons/gerar-imagens.mjs
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { crc32, deflateSync, inflateSync } from "node:zlib";

const pastaDoScript = dirname(fileURLToPath(import.meta.url));
const raiz = join(pastaDoScript, "..", "..");
const FONTE_ABERTURA = join(raiz, "imgs", "PlotS_ Frente e Verso.png");
const FONTE_ICONE = join(raiz, "imgs", "Ícone_logo.png");
const SAIDA_ABERTURA = join(raiz, "ui", "assets", "splash.png");
const SAIDA_MARCA = join(raiz, "ui", "assets", "marca.png");
const SAIDA_ICO = join(pastaDoScript, "icon.ico");

const LARGURA_ABERTURA = 960;
const ALTURA_ABERTURA = 600;
const LADOS_ICO = [16, 24, 32, 48, 64, 128, 256];
const LADO_MARCA = 48;
const LIMITE_ABERTURA = 400 * 1024;

// ---------- PNG: leitura ----------

const LADO_MAXIMO = 8192;
const PIXELS_MAXIMOS = 64_000_000;

/** Le um PNG de 8 bits, sem entrelacamento, em tons de cinza, RGB ou RGBA; devolve RGBA. */
export function lerPng(arquivo) {
  return decodificarPng(readFileSync(arquivo), arquivo);
}

/** Decodifica os bytes de um PNG; recusa cabecalho ausente, medidas absurdas e blocos cortados. */
export function decodificarPng(dados, arquivo = "PNG") {
  if (dados.length < 33 || dados.readUInt32BE(0) !== 0x89504e47) {
    throw new Error(`${arquivo}: nao e PNG`);
  }
  let largura = 0;
  let altura = 0;
  let tipo = 0;
  let temCabecalho = false;
  const partes = [];
  for (let p = 8; p + 8 <= dados.length; ) {
    const tamanho = dados.readUInt32BE(p);
    if (p + 12 + tamanho > dados.length) throw new Error(`${arquivo}: bloco cortado`);
    const nome = dados.toString("latin1", p + 4, p + 8);
    const corpo = dados.subarray(p + 8, p + 8 + tamanho);
    if (nome === "IHDR") {
      if (corpo.length < 13) throw new Error(`${arquivo}: cabecalho incompleto`);
      temCabecalho = true;
      largura = corpo.readUInt32BE(0);
      altura = corpo.readUInt32BE(4);
      tipo = corpo[9];
      if (corpo[8] !== 8 || corpo[12] !== 0) {
        throw new Error(`${arquivo}: so PNG de 8 bits, sem entrelacamento`);
      }
    } else if (nome === "IDAT") partes.push(corpo);
    else if (nome === "IEND") break;
    p += 12 + tamanho;
  }
  if (!temCabecalho) throw new Error(`${arquivo}: sem cabecalho`);
  if (
    largura < 1 ||
    altura < 1 ||
    largura > LADO_MAXIMO ||
    altura > LADO_MAXIMO ||
    largura * altura > PIXELS_MAXIMOS
  ) {
    throw new Error(`${arquivo}: medidas ${largura}x${altura} fora do limite`);
  }
  const canais = { 0: 1, 2: 3, 4: 2, 6: 4 }[tipo];
  if (!canais) throw new Error(`${arquivo}: tipo de cor ${tipo} nao suportado`);
  const linha = largura * canais;
  const esperado = (linha + 1) * altura;
  const bruto = inflateSync(Buffer.concat(partes), { maxOutputLength: esperado });
  if (bruto.length < esperado) throw new Error(`${arquivo}: dados incompletos`);
  const atual = Buffer.alloc(linha);
  const anterior = Buffer.alloc(linha);
  const rgba = new Uint8Array(largura * altura * 4);
  for (let y = 0; y < altura; y++) {
    const filtro = bruto[y * (linha + 1)];
    const origem = bruto.subarray(y * (linha + 1) + 1, (y + 1) * (linha + 1));
    for (let i = 0; i < linha; i++) {
      const a = i >= canais ? atual[i - canais] : 0;
      const b = anterior[i];
      const c = i >= canais ? anterior[i - canais] : 0;
      let soma = origem[i];
      if (filtro === 1) soma += a;
      else if (filtro === 2) soma += b;
      else if (filtro === 3) soma += (a + b) >> 1;
      else if (filtro === 4) soma += paeth(a, b, c);
      else if (filtro !== 0) throw new Error(`${arquivo}: filtro ${filtro} invalido`);
      atual[i] = soma & 255;
    }
    for (let x = 0; x < largura; x++) {
      const o = (y * largura + x) * 4;
      const i = x * canais;
      if (tipo === 0 || tipo === 4) {
        rgba.set([atual[i], atual[i], atual[i], tipo === 4 ? atual[i + 1] : 255], o);
      } else {
        rgba.set([atual[i], atual[i + 1], atual[i + 2], tipo === 6 ? atual[i + 3] : 255], o);
      }
    }
    atual.copy(anterior);
  }
  return { largura, altura, rgba };
}

function paeth(a, b, c) {
  const pa = Math.abs(b - c);
  const pb = Math.abs(a - c);
  const pc = Math.abs(a + b - 2 * c);
  return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
}

// ---------- PNG: escrita ----------

function bloco(tipo, dados) {
  const nome = Buffer.from(tipo, "latin1");
  const tamanho = Buffer.alloc(4);
  tamanho.writeUInt32BE(dados.length);
  const verificacao = Buffer.alloc(4);
  verificacao.writeUInt32BE(crc32(Buffer.concat([nome, dados])));
  return Buffer.concat([tamanho, nome, dados, verificacao]);
}

/** Escolhe, linha a linha, o filtro de menor soma absoluta (a regra usual) e comprime. */
function comprimir(linhas, largura, altura, bytesPorPixel) {
  const tamanho = largura * bytesPorPixel;
  const saida = Buffer.alloc((tamanho + 1) * altura);
  const candidato = Buffer.alloc(tamanho);
  for (let y = 0; y < altura; y++) {
    let melhor = null;
    let menor = Infinity;
    for (let filtro = 0; filtro <= 4; filtro++) {
      let soma = 0;
      for (let i = 0; i < tamanho; i++) {
        const x = linhas[y * tamanho + i];
        const a = i >= bytesPorPixel ? linhas[y * tamanho + i - bytesPorPixel] : 0;
        const b = y > 0 ? linhas[(y - 1) * tamanho + i] : 0;
        const c = y > 0 && i >= bytesPorPixel ? linhas[(y - 1) * tamanho + i - bytesPorPixel] : 0;
        let previsto = 0;
        if (filtro === 1) previsto = a;
        else if (filtro === 2) previsto = b;
        else if (filtro === 3) previsto = (a + b) >> 1;
        else if (filtro === 4) previsto = paeth(a, b, c);
        const v = (x - previsto) & 255;
        candidato[i] = v;
        soma += v < 128 ? v : 256 - v;
      }
      if (soma < menor) {
        menor = soma;
        melhor = { filtro, linha: Buffer.from(candidato) };
      }
    }
    saida[y * (tamanho + 1)] = melhor.filtro;
    melhor.linha.copy(saida, y * (tamanho + 1) + 1);
  }
  return deflateSync(saida, { level: 9, memLevel: 9 });
}

const ASSINATURA = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

function cabecalho(largura, altura, tipoDeCor) {
  const dados = Buffer.alloc(13);
  dados.writeUInt32BE(largura, 0);
  dados.writeUInt32BE(altura, 4);
  dados.set([8, tipoDeCor, 0, 0, 0], 8);
  return bloco("IHDR", dados);
}

function pngRgba(largura, altura, rgba) {
  return Buffer.concat([
    ASSINATURA,
    cabecalho(largura, altura, 6),
    bloco("IDAT", comprimir(Buffer.from(rgba), largura, altura, 4)),
    bloco("IEND", Buffer.alloc(0)),
  ]);
}

function pngPaleta(largura, altura, indices, paleta) {
  return Buffer.concat([
    ASSINATURA,
    cabecalho(largura, altura, 3),
    bloco("PLTE", Buffer.from(paleta.flat())),
    bloco("IDAT", comprimir(Buffer.from(indices), largura, altura, 1)),
    bloco("IEND", Buffer.alloc(0)),
  ]);
}

// ---------- Reducao de tamanho (Lanczos, com alfa pre-multiplicado) ----------

function lanczos(x) {
  if (x === 0) return 1;
  if (Math.abs(x) >= 3) return 0;
  const px = Math.PI * x;
  return (3 * Math.sin(px) * Math.sin(px / 3)) / (px * px);
}

function pesos(origem, destino) {
  const escala = origem / destino;
  const alcance = Math.max(1, escala);
  return Array.from({ length: destino }, (_, d) => {
    const centro = (d + 0.5) * escala;
    const de = Math.max(0, Math.floor(centro - alcance * 3));
    const ate = Math.min(origem - 1, Math.ceil(centro + alcance * 3));
    const lista = [];
    let total = 0;
    for (let s = de; s <= ate; s++) {
      const peso = lanczos((s + 0.5 - centro) / alcance);
      lista.push(peso);
      total += peso;
    }
    return { de, lista: lista.map((p) => p / total) };
  });
}

function redimensionar({ largura, altura, rgba }, novaLargura, novaAltura) {
  const premultiplicado = new Float32Array(largura * altura * 4);
  for (let i = 0; i < largura * altura; i++) {
    const a = rgba[i * 4 + 3] / 255;
    for (let c = 0; c < 3; c++) premultiplicado[i * 4 + c] = rgba[i * 4 + c] * a;
    premultiplicado[i * 4 + 3] = rgba[i * 4 + 3];
  }
  const horizontal = pesos(largura, novaLargura);
  const vertical = pesos(altura, novaAltura);
  const meio = new Float32Array(novaLargura * altura * 4);
  for (let y = 0; y < altura; y++) {
    for (let x = 0; x < novaLargura; x++) {
      const { de, lista } = horizontal[x];
      for (let c = 0; c < 4; c++) {
        let soma = 0;
        for (let k = 0; k < lista.length; k++) {
          soma += lista[k] * premultiplicado[(y * largura + de + k) * 4 + c];
        }
        meio[(y * novaLargura + x) * 4 + c] = soma;
      }
    }
  }
  const saida = new Uint8Array(novaLargura * novaAltura * 4);
  for (let y = 0; y < novaAltura; y++) {
    const { de, lista } = vertical[y];
    for (let x = 0; x < novaLargura; x++) {
      const valor = [0, 0, 0, 0];
      for (let c = 0; c < 4; c++) {
        let soma = 0;
        for (let k = 0; k < lista.length; k++) {
          soma += lista[k] * meio[((de + k) * novaLargura + x) * 4 + c];
        }
        valor[c] = soma;
      }
      const alfa = Math.min(255, Math.max(0, valor[3]));
      const o = (y * novaLargura + x) * 4;
      for (let c = 0; c < 3; c++) {
        const v = alfa > 0 ? (valor[c] * 255) / alfa : 0;
        saida[o + c] = Math.min(255, Math.max(0, Math.round(v)));
      }
      saida[o + 3] = Math.round(alfa);
    }
  }
  return { largura: novaLargura, altura: novaAltura, rgba: saida };
}

// ---------- Paleta de 256 cores (corte pela mediana) ----------

function quantizar({ largura, altura, rgba }) {
  const chaveDe = (i) => (rgba[i * 4] << 16) | (rgba[i * 4 + 1] << 8) | rgba[i * 4 + 2];
  const contagem = new Map();
  for (let i = 0; i < largura * altura; i++) {
    const chave = chaveDe(i);
    contagem.set(chave, (contagem.get(chave) ?? 0) + 1);
  }
  const cores = [...contagem].map(([chave, n]) => ({
    c: [chave >> 16, (chave >> 8) & 255, chave & 255],
    n,
  }));
  const caixas = [cores];
  const amplitude = (caixa) => {
    const faixas = [0, 1, 2].map((k) => {
      let menor = 255;
      let maior = 0;
      for (const e of caixa) {
        if (e.c[k] < menor) menor = e.c[k];
        if (e.c[k] > maior) maior = e.c[k];
      }
      return maior - menor;
    });
    return { faixas, canal: faixas.indexOf(Math.max(...faixas)) };
  };
  while (caixas.length < 256) {
    let alvo = -1;
    let pior = 0;
    caixas.forEach((caixa, i) => {
      if (caixa.length < 2) return;
      const { faixas } = amplitude(caixa);
      const peso = Math.max(...faixas) * Math.sqrt(caixa.reduce((s, e) => s + e.n, 0));
      if (peso > pior) {
        pior = peso;
        alvo = i;
      }
    });
    if (alvo < 0) break;
    const caixa = caixas[alvo];
    const { canal } = amplitude(caixa);
    caixa.sort((a, b) => a.c[canal] - b.c[canal]);
    const total = caixa.reduce((s, e) => s + e.n, 0);
    let acumulado = 0;
    let corte = 1;
    for (let i = 0; i < caixa.length - 1; i++) {
      acumulado += caixa[i].n;
      corte = i + 1;
      if (acumulado >= total / 2) break;
    }
    caixas.splice(alvo, 1, caixa.slice(0, corte), caixa.slice(corte));
  }
  const paleta = caixas.map((caixa) => {
    const total = caixa.reduce((s, e) => s + e.n, 0);
    return [0, 1, 2].map((k) => Math.round(caixa.reduce((s, e) => s + e.c[k] * e.n, 0) / total));
  });
  // Cada cor distinta aponta para a entrada mais proxima da paleta.
  const destino = new Map();
  for (const chave of contagem.keys()) {
    const c = [chave >> 16, (chave >> 8) & 255, chave & 255];
    let melhor = 0;
    let menor = Infinity;
    paleta.forEach((p, i) => {
      const d = (p[0] - c[0]) ** 2 + (p[1] - c[1]) ** 2 + (p[2] - c[2]) ** 2;
      if (d < menor) {
        menor = d;
        melhor = i;
      }
    });
    destino.set(chave, melhor);
  }
  const indices = new Uint8Array(largura * altura);
  for (let i = 0; i < largura * altura; i++) indices[i] = destino.get(chaveDe(i));
  return { indices, paleta };
}

// ---------- Icone: cantos transparentes, sem aureola ----------

const LIMIAR_ESCURO = 40;
const BRANCO_DA_BORDA = 250;
const LARGURA_DA_FRANJA = 3;

/** O que esta fora do quadrado arredondado vira transparente; a borda branca fica inteira. */
function recortarCantos({ largura, altura, rgba }) {
  const luz = (i) => Math.max(rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2]);
  const fora = new Uint8Array(largura * altura);
  const pilha = [];
  const semear = (x, y) => {
    const i = y * largura + x;
    if (!fora[i] && luz(i) <= LIMIAR_ESCURO) {
      fora[i] = 1;
      pilha.push(i);
    }
  };
  for (let x = 0; x < largura; x++) {
    semear(x, 0);
    semear(x, altura - 1);
  }
  for (let y = 0; y < altura; y++) {
    semear(0, y);
    semear(largura - 1, y);
  }
  while (pilha.length) {
    const i = pilha.pop();
    const x = i % largura;
    const y = (i - x) / largura;
    if (x > 0) semear(x - 1, y);
    if (x < largura - 1) semear(x + 1, y);
    if (y > 0) semear(x, y - 1);
    if (y < altura - 1) semear(x, y + 1);
  }
  const saida = new Uint8Array(rgba);
  // A franja entre o fundo e a linha branca e um degrade de preto a branco: ali a luz vira
  // cobertura (alfa) e a cor fica branca, para nao sobrar contorno escuro.
  const perto = (x, y) => {
    for (let dy = -LARGURA_DA_FRANJA; dy <= LARGURA_DA_FRANJA; dy++) {
      for (let dx = -LARGURA_DA_FRANJA; dx <= LARGURA_DA_FRANJA; dx++) {
        const nx = x + dx;
        const ny = y + dy;
        if (nx >= 0 && ny >= 0 && nx < largura && ny < altura && fora[ny * largura + nx]) {
          return true;
        }
      }
    }
    return false;
  };
  for (let y = 0; y < altura; y++) {
    for (let x = 0; x < largura; x++) {
      const i = y * largura + x;
      if (fora[i]) {
        saida.set([0, 0, 0, 0], i * 4);
      } else if (perto(x, y)) {
        const alfa = Math.min(255, Math.round((luz(i) * 255) / BRANCO_DA_BORDA));
        saida.set([255, 255, 255, alfa], i * 4);
      }
    }
  }
  return { largura, altura, rgba: saida };
}

function ico(lados, quadrado) {
  const imagens = lados.map((lado) => {
    const { rgba } = redimensionar(quadrado, lado, lado);
    return pngRgba(lado, lado, rgba);
  });
  const topo = Buffer.alloc(6);
  topo.writeUInt16LE(1, 2);
  topo.writeUInt16LE(lados.length, 4);
  let deslocamento = 6 + 16 * lados.length;
  const entradas = lados.map((lado, n) => {
    const entrada = Buffer.alloc(16);
    entrada[0] = entrada[1] = lado === 256 ? 0 : lado;
    entrada.writeUInt16LE(1, 4);
    entrada.writeUInt16LE(32, 6);
    entrada.writeUInt32LE(imagens[n].length, 8);
    entrada.writeUInt32LE(deslocamento, 12);
    deslocamento += imagens[n].length;
    return entrada;
  });
  return Buffer.concat([topo, ...entradas, ...imagens]);
}

// ---------- Execucao ----------

function gerar() {
  mkdirSync(dirname(SAIDA_ABERTURA), { recursive: true });

  const abertura = redimensionar(lerPng(FONTE_ABERTURA), LARGURA_ABERTURA, ALTURA_ABERTURA);
  const { indices, paleta } = quantizar(abertura);
  const pngAbertura = pngPaleta(LARGURA_ABERTURA, ALTURA_ABERTURA, indices, paleta);
  if (pngAbertura.length > LIMITE_ABERTURA) {
    throw new Error(`imagem de abertura com ${pngAbertura.length} bytes, acima de ${LIMITE_ABERTURA}`);
  }
  writeFileSync(SAIDA_ABERTURA, pngAbertura);

  const icone = recortarCantos(lerPng(FONTE_ICONE));
  writeFileSync(SAIDA_ICO, ico(LADOS_ICO, icone));
  const marca = redimensionar(icone, LADO_MARCA, LADO_MARCA);
  writeFileSync(SAIDA_MARCA, pngRgba(LADO_MARCA, LADO_MARCA, marca.rgba));
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) gerar();
