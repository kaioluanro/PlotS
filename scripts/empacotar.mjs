#!/usr/bin/env node
// Gera o instalador do PlotS (NSIS, por usuário, sem pedir administrador).
// Só usa a biblioteca padrão do Node e a CLI do Tauri da pasta ui.
// Uso: node scripts/empacotar.mjs
//
// Variáveis:
//   PLOTS_PDFIUM_DIR  pasta do pacote do PDFium (com bin/pdfium.dll); obrigatória
//   PLOTS_SAIDA       pasta de trabalho e de saída; padrão %LOCALAPPDATA%/plots-build/instalador
//
// A DLL não fica no repositório: é copiada daqui para src-tauri/gen/preparo (ignorada pelo Git, apagada ao fim) e entra no
// instalador ao lado do programa, onde a procura da biblioteca a encontra. O pacote só é ativado
// nesta chamada, para o `cargo test` não depender da DLL nem do NSIS.

import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { avisosDoRepositorio } from "./gerar-avisos.mjs";

const RAIZ = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function falhar(mensagem) {
  console.error(`Erro: ${mensagem}`);
  process.exit(1);
}

const pastaPdfium = process.env.PLOTS_PDFIUM_DIR;
if (!pastaPdfium) falhar("defina PLOTS_PDFIUM_DIR com a pasta do pacote do PDFium");
const dll = [join(pastaPdfium, "pdfium.dll"), join(pastaPdfium, "bin", "pdfium.dll")].find(existsSync);
if (!dll) falhar(`pdfium.dll não encontrada em ${pastaPdfium}`);

const saida = resolve(
  process.env.PLOTS_SAIDA ??
    join(process.env.LOCALAPPDATA ?? falhar("defina PLOTS_SAIDA ou LOCALAPPDATA"), "plots-build", "instalador"),
);
const preparo = join(RAIZ, "src-tauri", "gen", "preparo");
const destino = join(saida, "saidas");

// Tudo o que pode faltar é conferido antes de criar qualquer pasta de preparo.
const cli = join(RAIZ, "ui", "node_modules", "@tauri-apps", "cli", "tauri.js");
if (!existsSync(cli)) falhar("rode `npm ci` em ui/ antes: a CLI do Tauri não está instalada");

// Os avisos versionados têm de ser iguais aos que o gerador produz agora.
if (!existsSync(join(RAIZ, "THIRD-PARTY-NOTICES"))) falhar("THIRD-PARTY-NOTICES não existe na raiz do repositório");
if (!existsSync(join(RAIZ, "LICENSE"))) falhar("LICENSE não existe na raiz do repositório");
let avisosAtuais;
try {
  avisosAtuais = avisosDoRepositorio().texto;
} catch (erro) {
  falhar(`não foi possível gerar os avisos: ${erro.message}`);
}
const avisosVersionados = readFileSync(join(RAIZ, "THIRD-PARTY-NOTICES"), "utf8").split("\r\n").join("\n");
if (avisosVersionados !== avisosAtuais) {
  falhar("THIRD-PARTY-NOTICES está desatualizado: rode `node scripts/gerar-avisos.mjs` e versione o arquivo");
}

// Avisos de direitos autorais da biblioteca padrão do Rust: sempre os do toolchain do build.
const sysroot = spawnSync("rustc", ["--print", "sysroot"], { encoding: "utf8" });
if (sysroot.status !== 0 || !sysroot.stdout.trim()) falhar("`rustc --print sysroot` falhou");
const avisosRust = join(sysroot.stdout.trim(), "share", "doc", "rust", "COPYRIGHT-library.html");
if (!existsSync(avisosRust)) falhar(`o toolchain não traz ${avisosRust}`);

rmSync(preparo, { recursive: true, force: true });
mkdirSync(preparo, { recursive: true });
mkdirSync(destino, { recursive: true });

for (const arquivo of ["THIRD-PARTY-NOTICES", "LICENSE"]) {
  copyFileSync(join(RAIZ, arquivo), join(preparo, arquivo));
}
copyFileSync(avisosRust, join(preparo, "RUST-STD-NOTICES.html"));
copyFileSync(dll, join(preparo, "pdfium.dll"));

const barra = (caminho) => caminho.replace(/\\/g, "/");
const configuracao = {
  bundle: {
    active: true,
    targets: ["nsis"],
    resources: {
      ["gen/preparo/pdfium.dll"]: "pdfium.dll",
      ["gen/preparo/THIRD-PARTY-NOTICES"]: "THIRD-PARTY-NOTICES",
      ["gen/preparo/LICENSE"]: "LICENSE",
      ["gen/preparo/RUST-STD-NOTICES.html"]: "RUST-STD-NOTICES.html",
    },
  },
};
const arquivoConfiguracao = join(preparo, "release.conf.json");
writeFileSync(arquivoConfiguracao, JSON.stringify(configuracao, null, 2));

// Caminhos da máquina de build não podem ir para o executável: o diretório do repositório, o
// do cargo e o do rustup viram nomes neutros. As opções vão codificadas porque os caminhos
// podem ter espaço.
const casa = process.env.USERPROFILE ?? process.env.HOME ?? "";
const cargoHome = process.env.CARGO_HOME ?? join(casa, ".cargo");
const rustupHome = process.env.RUSTUP_HOME ?? join(casa, ".rustup");
const prefixos = [[RAIZ, "plots"], [cargoHome, "cargo"], [rustupHome, "rustup"]];
const variantes = prefixos.flatMap(([caminho, nome]) => [
  [caminho, nome],
  [barra(caminho), nome],
]);
const flags = variantes.map(([caminho, nome]) => `--remap-path-prefix=${caminho}=${nome}`);

const alvoCompilacao = join(saida, "alvo");
const ambiente = {
  ...process.env,
  CARGO_TARGET_DIR: alvoCompilacao,
  CARGO_INCREMENTAL: "0",
  CARGO_BUILD_JOBS: "4",
  CARGO_ENCODED_RUSTFLAGS: flags.join("\x1f"),
};
delete ambiente.RUSTFLAGS;

const resultado = spawnSync(
  process.execPath,
  // A versao final sai sem as impressoras simuladas: a compilacao padrao do programa as mantem.
  [cli, "build", "--config", arquivoConfiguracao, "--bundles", "nsis", "--", "--locked", "-j", "4", "--no-default-features"],
  { cwd: join(RAIZ, "src-tauri"), env: ambiente, stdio: "inherit" },
);
rmSync(preparo, { recursive: true, force: true });
if (resultado.status !== 0) falhar(`o tauri build terminou com código ${resultado.status ?? "desconhecido"}`);

const pastaNsis = join(alvoCompilacao, "release", "bundle", "nsis");
const instaladores = existsSync(pastaNsis) ? readdirSync(pastaNsis).filter((nome) => nome.endsWith(".exe")) : [];
if (instaladores.length === 0) falhar(`nenhum instalador em ${pastaNsis}`);
for (const nome of instaladores) {
  copyFileSync(join(pastaNsis, nome), join(destino, nome));
  console.log(`Instalador: ${join(destino, nome)}`);
}
