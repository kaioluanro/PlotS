#!/usr/bin/env node
// Baixa a biblioteca de rasterização usada no envio à impressora, em versão fixa.
// Confere o tamanho e o sha256 do pacote antes de abri-lo e extrai só o que o programa usa.
// Só usa a biblioteca padrão do Node. Uso: node scripts/baixar-pdfium.mjs

import { createHash, randomUUID } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, normalize, sep } from "node:path";
import { gunzipSync } from "node:zlib";

const TAG = "chromium/8125";
const ARQUIVO = "pdfium-win-x64.tgz";
const URL_PACOTE = `https://github.com/bblanchon/pdfium-binaries/releases/download/${TAG}/${ARQUIVO}`;
const BYTES_ESPERADOS = 3_904_987;
const SHA256_ESPERADO = "9a964261760cff52570aabf038ab346d3a5034dd8b30d0d5fe8861235d356b8b";
const SHA256_DLL = "4b0dec92555d69437bef0df37be630181ab3e83a6823ca264ede355a4f2ef994";
const LIMITE_DESCOMPACTADO = 256 * 1024 * 1024;
const OPCOES_EXIGIDAS = [
  ["pdf_enable_v8", "false"],
  ["pdf_enable_xfa", "false"],
];

function falhar(mensagem) {
  console.error(`Erro: ${mensagem}`);
  process.exit(1);
}

function pastaDestino() {
  const base = process.env.LOCALAPPDATA;
  if (!base) {
    falhar("a variável LOCALAPPDATA não está definida");
  }
  return join(base, "plots-build", "pdfium", "chromium-8125");
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

/** O pacote fica só na memória: nada vai ao disco antes da conferência. */
async function baixar() {
  console.log(`Baixando ${URL_PACOTE}`);
  const resposta = await fetch(URL_PACOTE, { redirect: "follow" });
  if (!resposta.ok) {
    falhar(`o download respondeu ${resposta.status}`);
  }
  const anunciado = resposta.headers.get("content-length");
  if (anunciado !== null && Number(anunciado) !== BYTES_ESPERADOS) {
    falhar(`o servidor anunciou ${anunciado} bytes; esperado ${BYTES_ESPERADOS}`);
  }
  return Buffer.from(await resposta.arrayBuffer());
}

/** Lê um campo numérico octal do cabeçalho tar; devolve null se não for válido. */
function octal(bytes) {
  const campo = bytes.toString("latin1").replace(/\0.*$/s, "").trim();
  if (campo === "") {
    return 0;
  }
  return /^[0-7]+$/.test(campo) ? Number.parseInt(campo, 8) : null;
}

function texto(bytes) {
  return bytes.toString("utf8").replace(/\0.*$/s, "");
}

/** Devolve as entradas de arquivo comum do tar, sem criar nada no disco. */
function lerTar(tar) {
  const entradas = [];
  let posicao = 0;
  let nomeLongo = null;
  while (posicao + 512 <= tar.length) {
    const cabecalho = tar.subarray(posicao, posicao + 512);
    if (cabecalho.every((b) => b === 0)) {
      break;
    }
    const tamanho = octal(cabecalho.subarray(124, 136));
    if (tamanho === null) {
      falhar("cabeçalho inválido no pacote");
    }
    const inicio = posicao + 512;
    const fim = inicio + tamanho;
    if (fim > tar.length) {
      falhar("entrada além do fim do pacote");
    }
    const tipo = String.fromCharCode(cabecalho[156]);
    const corpo = tar.subarray(inicio, fim);
    if (tipo === "L") {
      nomeLongo = texto(corpo);
    } else {
      const ustar = cabecalho.subarray(257, 262).toString("latin1") === "ustar";
      const prefixo = ustar ? texto(cabecalho.subarray(345, 500)) : "";
      const curto = texto(cabecalho.subarray(0, 100));
      const nome = nomeLongo ?? (prefixo ? `${prefixo}/${curto}` : curto);
      nomeLongo = null;
      if (tipo === "0" || tipo === "\0") {
        entradas.push({ nome: nome.replace(/^(\.\/)+/, ""), corpo });
      }
    }
    posicao = inicio + Math.ceil(tamanho / 512) * 512;
  }
  return entradas;
}

/** Só estes caminhos saem do pacote; qualquer outro é ignorado. */
function permitido(nome) {
  return (
    nome === "bin/pdfium.dll" ||
    nome === "LICENSE" ||
    nome === "args.gn" ||
    /^licenses\/[^/\\]+$/.test(nome)
  );
}

/** Recusa caminhos que sairiam da pasta de destino. */
function caminhoSeguro(pasta, nome) {
  const relativo = normalize(nome);
  if (relativo.split(sep).includes("..") || /^[a-zA-Z]:/.test(relativo) || relativo.startsWith(sep)) {
    falhar(`caminho recusado no pacote: ${nome}`);
  }
  return join(pasta, relativo);
}

function conferirOpcoes(argsGn) {
  for (const [opcao, valor] of OPCOES_EXIGIDAS) {
    const padrao = new RegExp(`^\\s*${opcao}\\s*=\\s*${valor}\\s*$`, "m");
    if (!padrao.test(argsGn)) {
      falhar(`args.gn não traz ${opcao} = ${valor}; o pacote foi recusado`);
    }
  }
}

const destino = pastaDestino();
const dll = join(destino, "bin", "pdfium.dll");
const argsExtraido = join(destino, "args.gn");
if (existsSync(dll) && existsSync(argsExtraido)) {
  conferirOpcoes(readFileSync(argsExtraido, "utf8"));
  const existente = sha256(readFileSync(dll));
  if (existente !== SHA256_DLL) {
    falhar(`a DLL em ${destino} tem sha256 ${existente}; esperado ${SHA256_DLL}. Apague a pasta e rode de novo.`);
  }
  console.log(`Já existe em ${destino}`);
  console.log(`sha256 da DLL: ${existente}`);
  process.exit(0);
}

const pacote = await baixar();
if (pacote.length !== BYTES_ESPERADOS) {
  falhar(`o pacote tem ${pacote.length} bytes; esperado ${BYTES_ESPERADOS}. Nada foi gravado.`);
}
const obtido = sha256(pacote);
if (obtido !== SHA256_ESPERADO) {
  falhar(`sha256 divergente: ${obtido}; esperado ${SHA256_ESPERADO}. Nada foi gravado.`);
}
console.log(`Pacote conferido: ${pacote.length} bytes, sha256 ${obtido}`);

const tar = gunzipSync(pacote, { maxOutputLength: LIMITE_DESCOMPACTADO });
const escolhidas = lerTar(tar).filter((entrada) => permitido(entrada.nome));
const argsGn = escolhidas.find((entrada) => entrada.nome === "args.gn");
if (!argsGn) {
  falhar("o pacote não traz args.gn");
}
conferirOpcoes(argsGn.corpo.toString("utf8"));
for (const exigido of ["bin/pdfium.dll", "LICENSE"]) {
  if (!escolhidas.some((entrada) => entrada.nome === exigido)) {
    falhar(`o pacote não traz ${exigido}`);
  }
}
if (!escolhidas.some((entrada) => entrada.nome.startsWith("licenses/"))) {
  falhar("o pacote não traz a pasta licenses");
}

// Grava numa pasta nova ao lado e só então a coloca no lugar: nunca fica extração pela metade.
const provisoria = `${destino}.${randomUUID()}`;
for (const entrada of escolhidas) {
  const caminho = caminhoSeguro(provisoria, entrada.nome);
  mkdirSync(dirname(caminho), { recursive: true });
  writeFileSync(caminho, entrada.corpo, { flag: "wx" });
}
rmSync(destino, { recursive: true, force: true });
mkdirSync(dirname(destino), { recursive: true });
renameSync(provisoria, destino);

console.log(`Extraído em ${destino}: ${escolhidas.length} arquivos`);
console.log(`sha256 da DLL: ${sha256(readFileSync(dll))}`);
