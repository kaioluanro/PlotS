#!/usr/bin/env node
// Define a mesma versão em todos os manifestos do projeto, ou confere que já são iguais.
// Uso: node scripts/definir-versao.mjs 1.0.0-alpha.2
//      node scripts/definir-versao.mjs --conferir
// Depois de definir, rode `cargo update --workspace --offline` para o Cargo.lock acompanhar.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const RAIZ = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const CRATES = [
  "plots-core", "plots-print", "plots-cli", "plots-pdf", "plots-perfis", "plots-windows", "src-tauri",
  "PlotSim/plotsim-physics", "PlotSim/plotsim-core", "PlotSim/plotsim-suite", "PlotSim/app",
].map((pasta) => `${pasta}/Cargo.toml`);
const JSONS = ["src-tauri/tauri.conf.json", "PlotSim/app/tauri.conf.json", "ui/package.json", "PlotSim/app/ui/package.json"];
const TRAVAS = ["ui/package-lock.json", "PlotSim/app/ui/package-lock.json"];

const VERSAO_DO_CRATE = /^version = "([^"]+)"/m;
// A primeira ocorrência é a do próprio projeto, tanto no manifesto quanto na trava.
const VERSAO_DO_JSON = /"version": "([^"]+)"/;
// Na trava do npm, a segunda fica em packages[""].
const VERSAO_NA_RAIZ_DA_TRAVA = /("packages": \{\s*"": \{\s*"name": "[^"]+",\s*"version": ")([^"]+)(")/;

const ler = (arquivo) => readFileSync(join(RAIZ, arquivo), "utf8");

function versoes() {
  const lista = [];
  for (const arquivo of CRATES) lista.push([arquivo, ler(arquivo).match(VERSAO_DO_CRATE)?.[1]]);
  for (const arquivo of [...JSONS, ...TRAVAS]) lista.push([arquivo, ler(arquivo).match(VERSAO_DO_JSON)?.[1]]);
  for (const arquivo of TRAVAS) lista.push([`${arquivo} (raiz)`, ler(arquivo).match(VERSAO_NA_RAIZ_DA_TRAVA)?.[2]]);
  return lista;
}

const pedido = process.argv[2];
if (pedido === "--conferir") {
  const lista = versoes();
  const unicas = new Set(lista.map(([, versao]) => versao));
  if (unicas.size !== 1 || unicas.has(undefined)) {
    for (const [arquivo, versao] of lista) console.error(`${versao ?? "sem versão"}  ${arquivo}`);
    process.exit(1);
  }
  console.log([...unicas][0]);
  process.exit(0);
}

if (!pedido || !/^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/.test(pedido)) {
  console.error("Uso: node scripts/definir-versao.mjs <versão> | --conferir");
  process.exit(2);
}

function gravar(arquivo, trocar) {
  const antes = ler(arquivo);
  const depois = trocar(antes);
  if (depois === antes) return;
  writeFileSync(join(RAIZ, arquivo), depois);
}

for (const arquivo of CRATES) gravar(arquivo, (texto) => texto.replace(VERSAO_DO_CRATE, `version = "${pedido}"`));
for (const arquivo of JSONS) gravar(arquivo, (texto) => texto.replace(VERSAO_DO_JSON, `"version": "${pedido}"`));
for (const arquivo of TRAVAS) {
  gravar(arquivo, (texto) =>
    texto.replace(VERSAO_DO_JSON, `"version": "${pedido}"`).replace(VERSAO_NA_RAIZ_DA_TRAVA, `$1${pedido}$3`),
  );
}
console.log(`Versão ${pedido} definida. Rode: cargo update --workspace --offline`);
