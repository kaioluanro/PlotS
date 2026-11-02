// Gera marca.svg e icon.ico a partir das medidas abaixo, so com a biblioteca padrao do Node.
// Uso: node src-tauri/icons/gerar-icone.mjs
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { crc32, deflateSync } from "node:zlib";

const pasta = dirname(fileURLToPath(import.meta.url));

// Desenho proprio, em uma grade de 64: azulejo escuro, folha de tras cheia e folha da frente clara.
const GRADE = 64;
const azulejo = { x: 0, y: 0, l: 64, a: 64, r: 12, cor: "#101114" };
const folhaTras = { x: 11, y: 21, l: 28, a: 32, r: 2, cor: "#8b8b95" };
const recorte = { x: 22, y: 9, l: 34, a: 38, r: 4, cor: "#101114" };
const folhaFrente = { x: 25, y: 12, l: 28, a: 32, r: 2, cor: "#f3f4f6" };
const camadas = [azulejo, folhaTras, recorte, folhaFrente];
const LADOS = [16, 32, 48, 256];

function svg() {
  const retangulos = camadas
    .map((c) => `  <rect x="${c.x}" y="${c.y}" width="${c.l}" height="${c.a}" rx="${c.r}" fill="${c.cor}"/>`)
    .join("\n");
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${GRADE} ${GRADE}">\n${retangulos}\n</svg>\n`;
}

function cobre(c, x, y) {
  if (x < c.x || y < c.y || x > c.x + c.l || y > c.y + c.a) return false;
  const px = Math.min(Math.max(x, c.x + c.r), c.x + c.l - c.r);
  const py = Math.min(Math.max(y, c.y + c.r), c.y + c.a - c.r);
  return (x - px) ** 2 + (y - py) ** 2 <= c.r ** 2;
}

function rgb(cor) {
  return [1, 3, 5].map((i) => Number.parseInt(cor.slice(i, i + 2), 16));
}

/** Cor do ponto; `null` fora do azulejo. */
function amostra(x, y) {
  let cor = null;
  for (const c of camadas) if (cobre(c, x, y)) cor = c.cor;
  return cor && rgb(cor);
}

function imagem(lado) {
  const AMOSTRAS = 4;
  const linhas = Buffer.alloc((lado * 4 + 1) * lado);
  let i = 0;
  for (let py = 0; py < lado; py++) {
    linhas[i++] = 0;
    for (let px = 0; px < lado; px++) {
      const soma = [0, 0, 0];
      let cheios = 0;
      for (let sy = 0; sy < AMOSTRAS; sy++) {
        for (let sx = 0; sx < AMOSTRAS; sx++) {
          const cor = amostra(
            ((px + (sx + 0.5) / AMOSTRAS) / lado) * GRADE,
            ((py + (sy + 0.5) / AMOSTRAS) / lado) * GRADE,
          );
          if (!cor) continue;
          cheios++;
          cor.forEach((v, canal) => (soma[canal] += v));
        }
      }
      for (const v of soma) linhas[i++] = cheios ? Math.round(v / cheios) : 0;
      linhas[i++] = Math.round((cheios * 255) / AMOSTRAS ** 2);
    }
  }
  return linhas;
}

function png(lado) {
  const bloco = (tipo, dados) => {
    const nome = Buffer.from(tipo, "latin1");
    const tamanho = Buffer.alloc(4);
    tamanho.writeUInt32BE(dados.length);
    const verificacao = Buffer.alloc(4);
    verificacao.writeUInt32BE(crc32(Buffer.concat([nome, dados])));
    return Buffer.concat([tamanho, nome, dados, verificacao]);
  };
  const cabecalho = Buffer.alloc(13);
  cabecalho.writeUInt32BE(lado, 0);
  cabecalho.writeUInt32BE(lado, 4);
  cabecalho.set([8, 6, 0, 0, 0], 8);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    bloco("IHDR", cabecalho),
    bloco("IDAT", deflateSync(imagem(lado), { level: 9 })),
    bloco("IEND", Buffer.alloc(0)),
  ]);
}

function ico() {
  const imagens = LADOS.map(png);
  const topo = Buffer.alloc(6);
  topo.writeUInt16LE(1, 2);
  topo.writeUInt16LE(LADOS.length, 4);
  let deslocamento = 6 + 16 * LADOS.length;
  const entradas = LADOS.map((lado, n) => {
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

writeFileSync(join(pasta, "marca.svg"), svg());
writeFileSync(join(pasta, "icon.ico"), ico());
