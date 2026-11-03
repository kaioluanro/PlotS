#!/usr/bin/env node
// Gera THIRD-PARTY-NOTICES na raiz do repositório a partir dos arquivos de licença reais dos
// crates e pacotes npm de produção e do pacote do PDFium. Só usa a biblioteca padrão do Node
// e o cargo. Uso: node scripts/gerar-avisos.mjs
// A pasta do PDFium vem de PLOTS_PDFIUM_DIR (ou da pasta que baixar-pdfium.mjs usa).

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gerarAvisos } from "./avisos-lib.mjs";

const ALVO = "x86_64-pc-windows-msvc";
export const RAIZ = resolve(dirname(fileURLToPath(import.meta.url)), "..");

export function pastaDoPdfium() {
  if (process.env.PLOTS_PDFIUM_DIR) {
    // Vale a raiz do pacote ou a sua pasta `bin`, como a procura da DLL aceita.
    const pasta = resolve(process.env.PLOTS_PDFIUM_DIR);
    return basename(pasta).toLowerCase() === "bin" && !existsSync(join(pasta, "LICENSE")) ? dirname(pasta) : pasta;
  }
  if (!process.env.LOCALAPPDATA) {
    throw new Error("defina PLOTS_PDFIUM_DIR com a pasta do pacote do PDFium");
  }
  return join(process.env.LOCALAPPDATA, "plots-build", "pdfium", "chromium-8125");
}

export function avisosDoRepositorio() {
  const saida = execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--locked", "--offline", "--filter-platform", ALVO],
    { cwd: RAIZ, maxBuffer: 256 * 1024 * 1024, encoding: "utf8" },
  );
  const arvore = execFileSync(
    "cargo",
    ["tree", "--workspace", "--edges", "no-build,no-dev,no-proc-macro", "--target", ALVO, "--prefix", "none", "--format", "{p}", "--offline", "--locked"],
    { cwd: RAIZ, maxBuffer: 64 * 1024 * 1024, encoding: "utf8" },
  );
  const pastaUi = join(RAIZ, "ui");
  return gerarAvisos({
    metadados: JSON.parse(saida),
    arvore,
    lock: JSON.parse(readFileSync(join(pastaUi, "package-lock.json"), "utf8")),
    pastaUi,
    pastaPdfium: pastaDoPdfium(),
    pastaExternas: join(RAIZ, "scripts", "licencas-externas"),
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const { texto, crates, pacotes } = avisosDoRepositorio();
    writeFileSync(join(RAIZ, "THIRD-PARTY-NOTICES"), texto);
    console.log(`THIRD-PARTY-NOTICES gerado: ${crates} crates, ${pacotes} pacotes npm.`);
  } catch (erro) {
    console.error(`Erro: ${erro.message}`);
    process.exit(1);
  }
}
