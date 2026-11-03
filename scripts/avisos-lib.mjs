// Funções do gerador de THIRD-PARTY-NOTICES. Só a biblioteca padrão do Node.
// Tudo o que entra no arquivo vem de arquivos de licença reais; quando um pacote não tem nenhum,
// a geração falha com o nome dele, em vez de inventar um texto.

import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const PADRAO_LICENCA = /^(licen[cs]e|copying|unlicense|notice|copyright)([-_. ].*)?$/i;
const LIMITE_ARQUIVO = 2 * 1024 * 1024;
const NOMES_PASTA_LICENCAS = new Set(["licenses", "licences", "license", "licence"]);

/** Arquivos que nunca entram: a fonte que o PlotS não distribui. */
export function arquivoExcluido(nome) {
  return /liberation/i.test(nome);
}

export function ehArquivoDeLicenca(nome) {
  return PADRAO_LICENCA.test(nome) && !arquivoExcluido(nome);
}

/** Lê o texto como UTF-8; se não for válido, como Latin-1 (alguns avisos antigos usam ©). */
export function lerTexto(caminho) {
  const tamanho = statSync(caminho).size;
  if (tamanho > LIMITE_ARQUIVO) {
    throw new Error(`arquivo de licença grande demais: ${caminho}`);
  }
  const bytes = readFileSync(caminho);
  let texto;
  try {
    texto = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    texto = bytes.toString("latin1");
  }
  return texto.replace(/^\uFEFF/, "").replace(/\r\n?/g, "\n").replace(/[ \t]+$/gm, "").trimEnd();
}

/** Arquivos de licença da pasta de um pacote: os da raiz e os de uma subpasta `licenses`. */
export function arquivosDeLicenca(pasta) {
  const achados = [];
  let itens;
  try {
    itens = readdirSync(pasta, { withFileTypes: true });
  } catch {
    return achados;
  }
  for (const item of itens) {
    if (item.isFile() && ehArquivoDeLicenca(item.name)) {
      achados.push(join(pasta, item.name));
    } else if (item.isDirectory() && NOMES_PASTA_LICENCAS.has(item.name.toLowerCase())) {
      for (const filho of readdirSync(join(pasta, item.name), { withFileTypes: true })) {
        if (filho.isFile() && !arquivoExcluido(filho.name)) {
          achados.push(join(pasta, item.name, filho.name));
        }
      }
    }
  }
  return achados.sort();
}

/**
 * Crates de terceiros que entram no programa. A lista vem de `cargo tree` só com dependências
 * normais, sem crates de macro nem de compilação, linhas no formato `nome vVERSÃO`; os
 * metadados dão a pasta de cada crate. Membros do workspace ficam de fora.
 */
export function cratesDeProducao(metadados, textoDaArvore) {
  const membros = new Set(metadados.workspace_members);
  const desejados = new Set();
  for (const linha of textoDaArvore.split("\n")) {
    const achado = /^(\S+) v(\S+)/.exec(linha.trim());
    if (achado) desejados.add(`${achado[1]} ${achado[2]}`);
  }
  const conhecidos = new Set(metadados.packages.map((pacote) => `${pacote.name} ${pacote.version}`));
  const achados = metadados.packages.filter(
    (pacote) => desejados.has(`${pacote.name} ${pacote.version}`) && !membros.has(pacote.id),
  );
  const sem = [...desejados].filter((nome) => !conhecidos.has(nome));
  if (sem.length > 0) {
    throw new Error(`crates da árvore sem metadados: ${sem.join(", ")}`);
  }
  return achados.sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`));
}

/** Pacotes npm de produção do lock: sem os de desenvolvimento e sem os opcionais. */
export function pacotesNpmDeProducao(lock) {
  return Object.entries(lock.packages ?? {})
    .filter(([caminho, dados]) => caminho !== "" && !dados.dev && !dados.optional)
    .map(([caminho, dados]) => ({
      caminho,
      nome: caminho.slice(caminho.lastIndexOf("node_modules/") + "node_modules/".length),
      versao: dados.version,
      licenca: dados.license ?? "",
    }))
    .sort((a, b) => a.nome.localeCompare(b.nome));
}

/** Arquivos de licença do `pdfjs-dist` que valem para o que o PlotS distribui, e só eles. */
export const LICENCAS_PDFJS = [
  "LICENSE",
  "cmaps/LICENSE",
  "standard_fonts/LICENSE_FOXIT",
  "wasm/LICENSE_JBIG2",
  "wasm/LICENSE_PDFJS_JBIG2",
  "wasm/LICENSE_OPENJPEG",
  "wasm/LICENSE_PDFJS_OPENJPEG",
  "wasm/LICENSE_PDFJS_QCMS",
  "wasm/LICENSE_QCMS",
];

function item(titulo, arquivos, extra = "") {
  return { titulo, arquivos, extra };
}

function lerArquivos(caminhos) {
  return caminhos.map((caminho) => ({ caminho, texto: lerTexto(caminho) }));
}

/**
 * Licença guardada em `pastaExternas` para um pacote cujo arquivo publicado não traz texto
 * algum. O índice `origens.json` diz de que endereço cada texto foi copiado. Sem entrada no
 * índice, não há texto: o pacote continua na lista de faltantes.
 */
export function licencaExterna(pastaExternas, chave, titulo) {
  if (!pastaExternas) return null;
  const indice = lerIndice(pastaExternas);
  const entrada = indice?.pacotes?.[chave];
  if (!entrada) return null;
  const caminho = conferirArquivo(pastaExternas, entrada);
  const nota = entrada.nota ?? "o pacote publicado não traz arquivo de licença";
  return {
    caminho,
    extra: `${titulo}: ${nota}; o texto abaixo vem de ${entrada.origem}`,
  };
}

function lerIndice(pastaExternas) {
  const indice = join(pastaExternas, "origens.json");
  return existsSync(indice) ? JSON.parse(readFileSync(indice, "utf8")) : null;
}

/** Confere o sha256 do arquivo guardado contra o índice; texto alterado ou trocado falha. */
function conferirArquivo(pastaExternas, entrada) {
  const caminho = join(pastaExternas, entrada.arquivo);
  if (!existsSync(caminho)) throw new Error(`arquivo de licença externo ausente: ${entrada.arquivo}`);
  // O checkout pode trocar a quebra de linha; o hash do índice é o do arquivo original, então
  // vale o conteúdo como está, só com LF ou só com CRLF.
  const bruto = readFileSync(caminho).toString("latin1");
  const lf = bruto.replace(/\r\n/g, "\n");
  const variantes = [bruto, lf, lf.replace(/\n/g, "\r\n")];
  const hashes = variantes.map((v) => createHash("sha256").update(Buffer.from(v, "latin1")).digest("hex"));
  if (!hashes.includes(entrada.sha256)) {
    throw new Error(`sha256 de ${entrada.arquivo} difere do índice (${hashes[0]} em vez de ${entrada.sha256})`);
  }
  return caminho;
}

/**
 * Componentes que acompanham o programa ou o instalador sem ser crate nem pacote npm: cada um
 * com origem fixada e hash conferido, descritos em `origens.json`.
 */
export function itensComponentes(pastaExternas) {
  const indice = pastaExternas ? lerIndice(pastaExternas) : null;
  if (!indice?.componentes) return { itens: [], faltando: ["componentes de origens.json"] };
  const itens = indice.componentes.map((componente) => {
    const caminhos = componente.arquivos.map((arquivo) => conferirArquivo(pastaExternas, arquivo));
    return item(componente.titulo, lerArquivos(caminhos), `${componente.titulo}: origem ${componente.origem}`);
  });
  return { itens, faltando: [] };
}

/**
 * O build da interface coloca no JavaScript do aplicativo o auxiliar de pré-carregamento do Vite
 * (o evento `vite:preloadError` e o carregamento de módulos). O `vite` é dependência de
 * desenvolvimento, então só a parte "Vite core license" do `LICENSE.md` dele entra, e não as
 * licenças das dependências que o próprio Vite embute em si.
 */
export function itemVite(pastaUi) {
  const caminho = join(pastaUi, "node_modules", "vite", "LICENSE.md");
  if (!existsSync(caminho)) return { itens: [], faltando: ["pacote npm vite (LICENSE.md)"] };
  const texto = lerTexto(caminho);
  const fim = texto.indexOf("# Licenses of bundled dependencies");
  if (fim <= 0) return { itens: [], faltando: ["pacote npm vite: LICENSE.md sem a seção própria do Vite"] };
  const proprio = texto.slice(0, fim).trimEnd();
  const versao = JSON.parse(readFileSync(join(pastaUi, "node_modules", "vite", "package.json"), "utf8")).version;
  const titulo = `vite ${versao} (MIT; auxiliar de pré-carregamento incluído no JavaScript do aplicativo)`;
  return { itens: [item(titulo, [{ caminho, texto: proprio }])], faltando: [] };
}

/**
 * Licenças que ficam em subpastas do crate e que a busca na raiz não acha. Cada uma cobre uma
 * parte do código do crate que entra no programa. O `rfd` tem uma licença aninhada em
 * `src/backend/xdg_desktop_portal/window_identifier/`, mas esse código só compila no Linux e
 * por isso não entra no programa para Windows.
 */
export const LICENCAS_ANINHADAS = {
  "regex-syntax": ["src/unicode_tables/LICENSE-UNICODE"],
  "tracing-core": ["src/spin/LICENSE"],
};

export function itensRust(metadados, textoDaArvore, pastaExternas) {
  const itens = [];
  const faltando = [];
  for (const pacote of cratesDeProducao(metadados, textoDaArvore)) {
    const pasta = pacote.manifest_path.replace(/[\\/]Cargo\.toml$/, "");
    let caminhos = arquivosDeLicenca(pasta);
    if (pacote.license_file) {
      const declarado = join(pasta, pacote.license_file);
      if (!caminhos.includes(declarado) && existsSync(declarado)) caminhos.push(declarado);
    }
    const nome = `${pacote.name} ${pacote.version}`;
    const licenca = pacote.license ? ` (${pacote.license})` : "";
    let extra = "";
    if (caminhos.length === 0) {
      const externa = licencaExterna(pastaExternas, `crate/${pacote.name}@${pacote.version}`, nome);
      if (!externa) {
        faltando.push(`crate ${nome}`);
        continue;
      }
      caminhos = [externa.caminho];
      extra = externa.extra;
    }
    const aninhadas = LICENCAS_ANINHADAS[pacote.name] ?? [];
    for (const relativo of aninhadas) {
      const caminho = join(pasta, relativo);
      if (!existsSync(caminho)) {
        faltando.push(`crate ${nome}: licença aninhada ${relativo}`);
      } else if (!caminhos.includes(caminho)) {
        caminhos.push(caminho);
      }
    }
    if (/MPL/i.test(pacote.license ?? "") && pacote.repository) {
      extra = `${nome}: código-fonte em ${pacote.repository.replace(/\.git$/, "")}`;
    }
    itens.push(item(`${nome}${licenca}`, lerArquivos(caminhos), extra));
  }
  return { itens, faltando };
}

export function itensNpm(lock, pastaUi, pastaExternas) {
  const itens = [];
  const faltando = [];
  for (const pacote of pacotesNpmDeProducao(lock)) {
    const pasta = join(pastaUi, pacote.caminho);
    const nome = `${pacote.nome} ${pacote.versao}`;
    let caminhos;
    let extra = "";
    if (pacote.nome === "pdfjs-dist") {
      caminhos = LICENCAS_PDFJS.map((relativo) => join(pasta, relativo));
      const ausentes = caminhos.filter((c) => !existsSync(c));
      if (ausentes.length > 0) {
        faltando.push(`pacote npm ${nome}: faltam ${ausentes.length} arquivos de licença`);
        continue;
      }
    } else {
      caminhos = arquivosDeLicenca(pasta);
    }
    if (caminhos.length === 0) {
      const externa = licencaExterna(pastaExternas, `npm/${pacote.nome}@${pacote.versao}`, nome);
      if (!externa) {
        faltando.push(`pacote npm ${nome}`);
        continue;
      }
      caminhos = [externa.caminho];
      extra = externa.extra;
    }
    const licenca = pacote.licenca ? ` (${pacote.licenca})` : "";
    itens.push(item(`${nome}${licenca}`, lerArquivos(caminhos), extra));
  }
  return { itens, faltando };
}

/** PDFium: o `LICENSE` do pacote e cada arquivo de `licenses/`, inteiros. */
export function itensPdfium(pastaPdfium) {
  const raiz = join(pastaPdfium, "LICENSE");
  const pastaLicencas = join(pastaPdfium, "licenses");
  if (!existsSync(raiz) || !existsSync(pastaLicencas)) {
    return { itens: [], faltando: [`pacote do PDFium em ${pastaPdfium} (LICENSE e licenses/)`] };
  }
  const arquivos = [raiz, ...readdirSync(pastaLicencas).sort().map((nome) => join(pastaLicencas, nome))];
  return {
    itens: arquivos.map((caminho) => {
      const relativo = caminho.slice(pastaPdfium.length + 1).replace(/\\/g, "/");
      return item(`PDFium (pacote pdfium-win-x64): ${relativo}`, lerArquivos([caminho]));
    }),
    faltando: [],
  };
}

const CABECALHO = `PlotS: avisos de software de terceiros
======================================

O PlotS é distribuído sob a licença MIT (arquivo LICENSE). Este arquivo reúne os avisos e as
licenças dos componentes de terceiros que fazem parte do programa e do instalador. Cada texto
abaixo foi copiado inteiro do arquivo de licença do próprio componente ou da origem indicada na
entrada. Os componentes continuam sob as suas licenças, qualquer que seja a licença do PlotS.

Os avisos de direitos autorais da biblioteca padrão do Rust, que está dentro do programa,
acompanham o programa no arquivo RUST-STD-NOTICES.html, copiado do toolchain que compilou o PlotS.

`;

const CREDITOS = `Créditos exigidos
-----------------

* Partes deste programa são baseadas no trabalho do Independent JPEG Group
  ("this software is based in part on the work of the Independent JPEG Group").
* Portions of this software are copyright © 2026 The FreeType Project (www.freetype.org).
  All rights reserved.
* A fonte Inter, embutida no programa, é distribuída sob a SIL Open Font License 1.1, cujo texto
  integral está abaixo, em @fontsource/inter. Ela não é vendida separadamente.
* PDFium é o nome de um projeto de terceiros, citado aqui só para identificar o componente; o
  PlotS não tem vínculo com os autores do PDFium nem com o Google.
* O crate option-ext é licenciado sob a MPL-2.0 e é usado sem alterações. O texto da licença está
  abaixo, e o código-fonte está em https://github.com/soc/option-ext.

Componentes da pdfium.dll sem texto próprio no pacote do PDFium
---------------------------------------------------------------

* libc++ e libc++abi (LLVM) estão na pdfium.dll; os textos completos estão abaixo.
* A pdfium.dll contém as fontes ChromeSansMM e ChromeSerifMM, publicadas pelo PDFium nos arquivos
  core/fxge/fontdata/chromefontdata/FoxitSansMM.cpp e FoxitSerifMM.cpp, sob a licença BSD de
  licenses/pdfium.txt. Os dados dessas fontes contêm a nota "Copyright (c) 1990-1994 Adobe
  Systems Incorporated. All Rights Reserved", mantida sem alteração.

Instalador (NSIS)
-----------------

O instalador é gerado com o NSIS 3.13, cujo arquivo COPYING está abaixo, inteiro.

Nos termos da Common Public License 1.0, seção 3(b), quanto ao módulo de compressão LZMA do NSIS, que está sob a CPL 1.0:

  (i) em nome de todos os contribuidores, ficam afastadas todas as garantias e condições, expressas e implícitas, incluindo as de titularidade e não violação, e as garantias ou condições implícitas de comerciabilidade e de adequação a uma finalidade específica;
  (ii) em nome de todos os contribuidores, fica excluída toda responsabilidade por danos, incluindo danos diretos, indiretos, especiais, incidentais e consequenciais, como lucros cessantes;
  (iii) qualquer disposição deste aviso ou da licença do PlotS que difira da CPL 1.0 é oferecida somente por quem distribui este instalador, e não por qualquer outra parte. A exceção especial do módulo LZMA, no COPYING abaixo, é concedida pelos autores nela nomeados;
  (iv) o código-fonte do NSIS 3.13, com o módulo LZMA, está em https://prdownloads.sourceforge.net/nsis/nsis-3.13-src.tar.bz2?download (versões atuais em https://nsis.sourceforge.io/Download).

In English, under section 3(b) of the Common Public License 1.0, for the NSIS LZMA compression module: (i) on behalf of all Contributors, all warranties and conditions, express and implied, are disclaimed, including warranties or conditions of title and non-infringement, and implied warranties or conditions of merchantability and fitness for a particular purpose; (ii) on behalf of all Contributors, all liability for damages is excluded, including direct, indirect, special, incidental and consequential damages, such as lost profits; (iii) any provisions in this notice or in the PlotS licence that differ from the CPL 1.0 are offered by the distributor of this installer alone and not by any other party; (iv) the source code of NSIS 3.13, including the LZMA module, is available at the address above.

O plugin nsis_tauri_utils 0.5.3, do projeto Tauri, também vai no instalador; suas licenças MIT
e Apache-2.0 estão abaixo.

Se o WebView2 Runtime faltar, o instalador o baixa da Microsoft; ele segue os termos da Microsoft.

`;

/** Monta o texto final. Textos idênticos aparecem uma vez, com a lista de quem os usa. */
export function montarAvisos(grupos) {
  const porTexto = new Map();
  for (const { rotulo, itens } of grupos) {
    for (const entrada of itens) {
      for (const arquivo of entrada.arquivos) {
        const chave = createHash("sha256").update(arquivo.texto).digest("hex");
        if (!porTexto.has(chave)) {
          porTexto.set(chave, { rotulo, donos: [], texto: arquivo.texto, extras: new Set() });
        }
        const alvo = porTexto.get(chave);
        if (!alvo.donos.includes(entrada.titulo)) alvo.donos.push(entrada.titulo);
        if (entrada.extra) alvo.extras.add(entrada.extra);
      }
    }
  }
  const secoes = [];
  for (const alvo of porTexto.values()) {
    const donos = alvo.donos.map((dono) => `  * ${dono}`).join("\n");
    const extras = [...alvo.extras].map((extra) => `\n${extra}`).join("");
    secoes.push(`${"=".repeat(78)}\n${alvo.rotulo}\n${donos}${extras}\n${"-".repeat(78)}\n${alvo.texto}\n`);
  }
  return `${CABECALHO}${CREDITOS}${secoes.join("\n")}`;
}

/** Texto completo a partir das três origens; lança erro com todos os nomes que não têm licença. */
export function gerarAvisos({ metadados, arvore, lock, pastaUi, pastaPdfium, pastaExternas }) {
  const rust = itensRust(metadados, arvore, pastaExternas);
  const npm = itensNpm(lock, pastaUi, pastaExternas);
  const pdfium = itensPdfium(pastaPdfium);
  const componentes = itensComponentes(pastaExternas);
  const vite = itemVite(pastaUi);
  const faltando = [
    ...rust.faltando,
    ...npm.faltando,
    ...pdfium.faltando,
    ...componentes.faltando,
    ...vite.faltando,
  ];
  if (faltando.length > 0) {
    throw new Error(`sem arquivo de licença:\n  ${faltando.join("\n  ")}`);
  }
  const texto = montarAvisos([
    { rotulo: "Rust (crates)", itens: rust.itens },
    { rotulo: "npm (interface)", itens: npm.itens },
    { rotulo: "PDFium", itens: pdfium.itens },
    { rotulo: "Outros componentes do programa e do instalador", itens: [...componentes.itens, ...vite.itens] },
  ]);
  return {
    texto,
    crates: rust.itens.length,
    pacotes: npm.itens.length,
    titulos: [...rust.itens, ...npm.itens].map((entrada) => entrada.titulo),
  };
}
