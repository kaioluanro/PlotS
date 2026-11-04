// Testes do gerador de avisos. Uso: node --test scripts/
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { execFileSync } from "node:child_process";
import { ARGUMENTOS_DA_ARVORE, avisosDoRepositorio, RAIZ } from "./gerar-avisos.mjs";
import {
  arquivoExcluido,
  cratesDeProducao,
  gerarAvisos,
  itensComponentes,
  itensRust,
  LICENCAS_ANINHADAS,
  licencaExterna,
  pacotesNpmDeProducao,
} from "./avisos-lib.mjs";

test("licença aninhada de crate listado entra no aviso, e a que falta é apontada", () => {
  const raiz = pastaTemporaria();
  try {
    const pasta = join(raiz, "regex-syntax");
    mkdirSync(join(pasta, "src", "unicode_tables"), { recursive: true });
    writeFileSync(join(pasta, "LICENSE-MIT"), "texto da raiz\n");
    const metadados = metadadosFalsos([{ nome: "regex-syntax", pasta }]);
    const arvore = "regex-syntax v1.0.0\n";
    assert.deepEqual(LICENCAS_ANINHADAS["regex-syntax"], ["src/unicode_tables/LICENSE-UNICODE"]);
    assert.deepEqual(LICENCAS_ANINHADAS["tracing-core"], ["src/spin/LICENSE"]);
    assert.equal(LICENCAS_ANINHADAS.rfd, undefined);
    const sem = itensRust(metadados, arvore, undefined);
    assert.deepEqual(sem.faltando, ["crate regex-syntax 1.0.0: licença aninhada src/unicode_tables/LICENSE-UNICODE"]);
    writeFileSync(join(pasta, "src", "unicode_tables", "LICENSE-UNICODE"), "texto aninhado\n");
    const com = itensRust(metadados, arvore, undefined);
    assert.deepEqual(com.faltando, []);
    assert.deepEqual(com.itens[0].arquivos.map((a) => a.texto).sort(), ["texto aninhado", "texto da raiz"]);
  } finally {
    rmSync(raiz, { recursive: true, force: true });
  }
});

test("texto externo com hash diferente do índice faz a geração falhar", () => {
  const raiz = pastaTemporaria();
  try {
    writeFileSync(join(raiz, "x.txt"), "licença alterada\n");
    const indice = {
      pacotes: { "crate/x@1.0.0": { arquivo: "x.txt", sha256: "0".repeat(64), origem: "https://exemplo/x" } },
      componentes: [
        { titulo: "comp", origem: "https://exemplo/c", arquivos: [{ arquivo: "x.txt", sha256: "1".repeat(64) }] },
      ],
    };
    writeFileSync(join(raiz, "origens.json"), JSON.stringify(indice));
    assert.throws(() => licencaExterna(raiz, "crate/x@1.0.0", "x 1.0.0"), /sha256 de x\.txt difere/);
    assert.throws(() => itensComponentes(raiz), /sha256 de x\.txt difere/);
    assert.equal(licencaExterna(raiz, "crate/y@1.0.0", "y 1.0.0"), null);
  } finally {
    rmSync(raiz, { recursive: true, force: true });
  }
});

function pastaTemporaria() {
  return mkdtempSync(join(tmpdir(), "plots-avisos-"));
}

/** Metadados mínimos com os crates informados, cada um com a sua pasta. */
function metadadosFalsos(crates) {
  return {
    workspace_members: [],
    packages: crates.map(({ nome, pasta }) => ({
      id: `registry+x#${nome}@1.0.0`,
      name: nome,
      version: "1.0.0",
      license: "MIT",
      manifest_path: join(pasta, "Cargo.toml"),
      targets: [{ kind: ["lib"] }],
    })),
  };
}

test("crate de produção sem arquivo de licença faz a geração falhar com o nome dele", () => {
  const raiz = pastaTemporaria();
  try {
    const comLicenca = join(raiz, "com");
    const semLicenca = join(raiz, "sem");
    mkdirSync(comLicenca);
    mkdirSync(semLicenca);
    writeFileSync(join(comLicenca, "LICENSE"), "texto real\n");
    const metadados = metadadosFalsos([
      { nome: "crate-com", pasta: comLicenca },
      { nome: "crate-ficticio-sem-licenca", pasta: semLicenca },
    ]);
    const arvore = "crate-com v1.0.0\ncrate-ficticio-sem-licenca v1.0.0\n";
    const { itens, faltando } = itensRust(metadados, arvore, undefined);
    assert.equal(itens.length, 1);
    assert.deepEqual(faltando, ["crate crate-ficticio-sem-licenca 1.0.0"]);
    assert.throws(
      () =>
        gerarAvisos({
          metadados,
          arvore,
          lock: { packages: {} },
          pastaUi: raiz,
          pastaPdfium: join(raiz, "pdfium-que-nao-existe"),
        }),
      /crate-ficticio-sem-licenca/,
    );
  } finally {
    rmSync(raiz, { recursive: true, force: true });
  }
});

test("só entra o que a árvore de produção lista, sem os membros do workspace", () => {
  const metadados = {
    workspace_members: ["path+x#meu@0.1.0"],
    packages: [
      { id: "path+x#meu@0.1.0", name: "meu", version: "0.1.0", targets: [] },
      { id: "r#a@1.0.0", name: "a", version: "1.0.0", targets: [] },
      { id: "r#so-compilacao@1.0.0", name: "so-compilacao", version: "1.0.0", targets: [] },
    ],
  };
  const nomes = cratesDeProducao(metadados, "meu v0.1.0 (/x)\na v1.0.0\n").map((c) => c.name);
  assert.deepEqual(nomes, ["a"]);
  assert.throws(() => cratesDeProducao(metadados, "fantasma v9.9.9\n"), /fantasma/);
});

test("pacote npm opcional ou de desenvolvimento fica de fora da lista de produção", () => {
  const lock = {
    packages: {
      "": {},
      "node_modules/a": { version: "1.0.0" },
      "node_modules/b": { version: "1.0.0", dev: true },
      "node_modules/c": { version: "1.0.0", optional: true },
    },
  };
  assert.deepEqual(
    pacotesNpmDeProducao(lock).map((p) => p.nome),
    ["a"],
  );
});

test("o arquivo da fonte que não é distribuída nunca é aceito", () => {
  assert.equal(arquivoExcluido("LICENSE_LIBERATION"), true);
  assert.equal(arquivoExcluido("LICENSE_FOXIT"), false);
});

test("a lista gerada cobre todo crate e pacote de produção e deixa a fonte de fora", () => {
  const gerado = avisosDoRepositorio();
  const texto = gerado.texto;
  assert.ok(gerado.crates > 100, "poucos crates na lista");
  assert.equal(gerado.pacotes, 36);
  for (const titulo of gerado.titulos) {
    assert.ok(texto.includes(`* ${titulo}`), `faltou ${titulo}`);
  }
  const lock = JSON.parse(readFileSync(join(RAIZ, "ui", "package-lock.json"), "utf8"));
  for (const pacote of pacotesNpmDeProducao(lock)) {
    assert.ok(texto.includes(`* ${pacote.nome} ${pacote.versao}`), `faltou ${pacote.nome}`);
  }
  assert.ok(texto.includes("option-ext 0.2.0 (MPL-2.0)"));
  assert.ok(texto.includes("https://github.com/soc/option-ext"));
  assert.ok(texto.includes("Feather"));
  assert.ok(texto.includes("Inter Project Authors"));
  assert.ok(texto.includes("pdfium-win-x64): licenses/freetype.txt"));
  assert.ok(texto.includes("Microsoft Edge WebView2 SDK 1.0.3800.47 (WebView2LoaderStatic.lib"));
  assert.ok(texto.includes("libc++ (LLVM, na pdfium.dll)"));
  assert.ok(texto.includes("libc++abi (LLVM, na pdfium.dll)"));
  assert.ok(texto.includes("NSIS 3.13"));
  assert.ok(texto.includes("COMMON PUBLIC LICENSE VERSION 1.0"));
  assert.ok(texto.includes("https://nsis.sourceforge.io/Download"));
  // Só o trecho do aviso próprio: o COPYING do NSIS, mais abaixo, cita os autores do LZMA.
  const iniAviso = texto.indexOf("Instalador (NSIS)\n");
  const fimAviso = texto.indexOf("O plugin nsis_tauri_utils 0.5.3, do projeto Tauri");
  assert.ok(iniAviso > 0 && fimAviso > iniAviso);
  const aviso = texto.slice(iniAviso, fimAviso);
  assert.ok(aviso.includes("on behalf of all Contributors"));
  assert.ok(aviso.includes("lost profits"));
  assert.ok(aviso.includes("nsis-3.13-src.tar.bz2"));
  assert.ok(!aviso.includes("Igor Pavlov e Amir Szekely"));
  assert.ok(texto.includes("Igor Pavlov and Amir Szekely"));
  assert.ok(texto.includes("nsis_tauri_utils 0.5.3"));
  assert.ok(texto.includes("copyright © 2026 The FreeType Project"));
  assert.ok(texto.includes("FoxitSansMM.cpp"));
  assert.ok(texto.includes("RUST-STD-NOTICES.html"));
  assert.ok(texto.includes("Cada texto\nabaixo foi copiado inteiro do arquivo de licença do próprio componente ou da origem"));
  assert.ok(texto.includes("alloc-stdlib 0.2.4: o pacote alloc-stdlib 0.2.4 não traz arquivo"));
  assert.ok(texto.includes("Copyright (c) 2019-present, VoidZero Inc. and Vite contributors"));
  assert.ok(!texto.includes("Licenses of bundled dependencies"));
  assert.ok(!/LICENSE_LIBERATION/i.test(texto));
  assert.ok(!texto.includes("LIBERATION FONT SOFTWARE"));
  assert.ok(!texto.includes("LICENSE AGREEMENT AND LIMITED PRODUCT WARRANTY"));
});

test("o simulador de testes e as dependências dele ficam fora da árvore do aviso", () => {
  const arvore = execFileSync("cargo", ARGUMENTOS_DA_ARVORE, {
    cwd: RAIZ,
    maxBuffer: 64 * 1024 * 1024,
    encoding: "utf8",
  });
  const nomes = new Set(arvore.split(/\r?\n/).map((linha) => linha.split(" ")[0]));
  assert.ok(nomes.has("plots-core"), "a árvore veio sem o programa");
  for (const fora of ["plotsim-core", "plotsim-physics", "plotsim-suite", "plotsim-app", "notify", "notify-types"]) {
    assert.ok(!nomes.has(fora), `${fora} apareceu na árvore`);
  }
});

test("o arquivo versionado é igual ao que o gerador produz agora", () => {
  const versionado = readFileSync(join(RAIZ, "THIRD-PARTY-NOTICES"), "utf8").split("\r\n").join("\n");
  assert.equal(versionado, avisosDoRepositorio().texto);
});
