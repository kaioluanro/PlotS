# Licenças de terceiros

Registro das dependências e dos ativos do PlotS. Atualize a cada lote que trouxer dependência ou ativo novo.

**Pendência que bloqueia qualquer distribuição:** gerar `THIRD-PARTY-NOTICES` a partir dos arquivos de licença de cada pacote e incluí-lo no instalador. Vale também para qualquer distribuição avulsa do `plots-cli`. O build da interface remove os avisos de React, Radix e lucide do JavaScript, então o arquivo precisa existir à parte. A licença da fonte Inter também vai nesse arquivo, porque a fonte fica embutida no app. Plano: `17-acabamento`.

## Decisões do usuário

- **`option-ext` 0.2.0 (MPL-2.0):** mantido sem alterações (2026-11-02). Entra no binário por `plots-perfis → directories → dirs-sys` e por `tauri → dirs → dirs-sys`. Ao distribuir: incluir o texto da MPL 2.0 e informar a origem do código-fonte, https://github.com/soc/option-ext.
- **Norma de estilo:** vale como instrução de estilo (2026-11-02), e os valores de cores e medidas foram usados. Não cobre logotipos, nomes de marca nem código de terceiros.

- **Distribuição:** o PlotS será só público e gratuito no GitHub, sem venda (2026-11-02). Por isso, o risco residual de patentes de imagem fica aceito e registrado, sem consulta a advogado.

## Entra no binário

Levantamento de 2026-11-02, a partir das licenças declaradas nos metadados. Em 2026-11-03, `scripts/gerar-avisos.mjs` contou 191 crates de terceiros no grafo de produção e 36 pacotes npm e gerou `THIRD-PARTY-NOTICES` dos arquivos reais; a tabela abaixo é a do levantamento.

**Rust** (133 crates de terceiros no grafo Windows, sem macros):

| Licença declarada | Crates |
|---|---|
| MIT OR Apache-2.0 | 88 |
| MIT | 16 |
| Unicode-3.0 | 15 |
| Unlicense OR MIT | 5 |
| BSD-3-Clause | 2 |
| Apache-2.0 (`tao`; conferir se há `NOTICE`) | 1 |
| MIT OR Apache-2.0 OR Zlib | 1 |
| CC0-1.0 OR MIT-0 OR Apache-2.0 | 1 |
| BSD-3-Clause OR MIT | 1 |
| BSD-3-Clause AND MIT | 1 |
| Apache-2.0 AND MIT | 1 |
| MPL-2.0 (`option-ext`) | 1 |

**Dependência direta nova do `plots-windows`** (2026-11-02): `windows` 0.62.2, versão exata, licença MIT OR Apache-2.0, com as features `Win32_Foundation`, `Win32_Graphics_Gdi`, `Win32_Graphics_Printing` e `Win32_Security`. Já estava no grafo pelo Tauri; agora também entra pelo `plots-windows`, só no alvo Windows.

**Dependências novas da abertura de imagens** (2026-11-07): `image` 0.25.10 (MIT OR Apache-2.0), `tiff` 0.11.3 (MIT) e `flate2` 1.1.10 (MIT OR Apache-2.0). Entram no binário por `plots-pdf`; em `src-tauri` são dev-dependencies e não somam obrigação. O grafo de produção ganha entradas transitivas como `image-webp`, `gif`, `color_quant`, `byteorder-lite`, `moxcms`/`pxfm` (BSD-3-Clause OR Apache-2.0), `zune-core`/`zune-jpeg`, `fax`, `half`, `crunchy`, `zerocopy`, `quick-error`, `weezl`, `png`; a lista completa está no relatório do lote A. Nenhum copyleft forte. Regenerar `THIRD-PARTY-NOTICES` no lote D; até lá `scripts/avisos.test.mjs` fica vermelho e o empacotador recusa zip com avisos velhos.

**Dependências do motor de tinta** (2026-11-07): a crate `plots-info` usa `plots-core`, `serde` e `thiserror`, todos já no grafo; o `src-tauri` ganha `plots-info` (MIT, nosso) e `toml = "=1.1.6"`, que já vinha de `plots-perfis` e de `tauri-utils`, então não há pacote de terceiro novo no binário. No lote D, conferir a cadeia `toml`/`toml_datetime`/`toml_parser`/`toml_writer`/`serde_spanned`/`winnow` nos avisos regenerados.

- **Patentes de imagem:** a decisão de 2026-11-02 sobre risco residual vale também para `image`, `tiff`, `zune-jpeg` e `image-webp`, com as features sem AVIF/HEIF/EXR. WebP tem concessão royalty-free; JPEG vem de implementação independente; LZW/CCITT estão com patentes expiradas.
- **MuPDF 1.28.5 (AGPL-3.0 ou posterior, `mutool.exe`):** decisão do usuário em 2026-11-07 de incluir o binário separado. Antes de distribuir: (a) disponibilizar o `mupdf-1.28.5-source.tar.gz` exato **fora do zip, na mesma pasta da entrega**, sem custo, com SHA-256 (§6(d)), além da URL oficial; (b) incluir o texto integral da AGPL e as licenças de todos os componentes do `thirdparty/` que entram no `mutool.exe` — brotli, cmark-gfm, curl, extract, freeglut, freetype, gumbo, harfbuzz e ms-use, jbig2dec, lcms2, leptonica, libjpeg, mujs, openjpeg, tesseract, zint (BSD-3 da biblioteca), zlib e zxing-cpp; (c) declarar a separação AGPL/MIT e a isenção de marca da Artifex; (d) manter o `mutool.exe` sem patch, em subpasta própria, com `AGPL-3.0.txt` e `MUPDF-SOURCE.txt` ao lado, registrando copyright, “ou posterior”, URL, data e SHA-256 do zip, do exe e da fonte; (e) se houver verificação de integridade, que seja aviso e nunca recusa de binário modificado; (f) README/release notes com o tamanho (~44 MB) e a natureza AGPL; (g) parecer jurídico antes de publicar. O PlotS não linka MuPDF; o processo é separado. O `mutool.exe` oficial exige o runtime Microsoft Visual C++ 2015-2022 (x64); sem ele, o app cai no modo simples com aviso.
- **Cache de imagem:** o derivado fica em `%LOCALAPPDATA%\plots\cache\imagens` por até 30 dias; documentar no README. A chave FNV-1a de 64 bits deve ganhar um sufixo de SHA-256 para evitar colisão entre imagens; corrigir antes do lote D.

**npm** (36 pacotes de produção): OFL-1.1 em `@fontsource/inter` (fonte, ver Ativos); MIT em 30; ISC em `lucide-react`, cuja licença também cita ícones derivados do Feather, usados aqui (`arrow-down`, `arrow-up`, `check`, `chevron-down`, `chevron-right`, `circle-alert` (antes `alert-circle`), `minus`, `plus`, `square`, `trash-2`, `triangle-alert` (antes `alert-triangle`), `x`), então o aviso leva as duas partes; 0BSD em `tslib`; `Apache-2.0 OR MIT` em `@tauri-apps/api` e `@tauri-apps/plugin-dialog`; Apache-2.0 em `pdfjs-dist`. O lock traz ainda 12 pacotes opcionais `@napi-rs/canvas*` (MIT), dependências opcionais do `pdfjs-dist`, sem arquivos no bundle.

**`pdfjs-dist` 6.5.438** (Apache-2.0), versão exata; o pacote não traz `NOTICE`. `pdf.worker.min` e `pdf.renderer.min` entram sem alteração. O build remove o cabeçalho de licença do trecho principal, então o aviso vai em `THIRD-PARTY-NOTICES`. Entram só estes dados do pacote: `cmaps/*.bcmap`, `standard_fonts/Foxit*.pfb`, `wasm/jbig2.wasm` e `wasm/openjpeg.wasm`. **Ficam fora do bundle:** `standard_fonts/LiberationSans-*.ttf` (versão 1.07.4, GPLv2 com exceções), `wasm/qcms_bg.wasm`, `iccs/` e `quickjs-eval`. Os testes de `ui/src/visualizador/recursos.test.ts` barram a Liberation e o `qcms_bg`. Com `useSystemFonts` ligado, as fontes base-14 não embutidas no PDF usam a fonte instalada no sistema do usuário; nada dela é copiado nem distribuído.

**Dados de métricas no worker:** `pdf.worker.min` contém nomes `LiberationSans-*` e tabelas de largura e de mapeamento de glifos (`liberationsans_widths.js`), publicadas pela Mozilla sob Apache-2.0. As larguras da versão Regular coincidem com o `hmtx` da fonte 1.07.4 (681 de 681). Não há glifos nem contornos. Pelos trechos lidos, só o caminho XFA as usa, e o XFA fica desligado. Tratadas como dados funcionais de métricas, sem obrigação de GPL identificada; é risco residual registrado, sem parecer de advogado (revisão de 2026-11-03).

**`pdfium-render` 0.9.5** (MIT OR Apache-2.0), versão exata. **`pdfium.dll`**, de `bblanchon/pdfium-binaries`, tag `chromium/8125`, `pdfium-win-x64.tgz` sem V8, 3 904 987 bytes, sha256 `9a964261760cff52570aabf038ab346d3a5034dd8b30d0d5fe8861235d356b8b`, sha256 da DLL `4b0dec92555d69437bef0df37be630181ab3e83a6823ca264ede355a4f2ef994`, distribuída sem alteração. Scripts sob MIT; PDFium sob BSD-3 e Apache-2.0; terceiros conforme `licenses/`, lidos em 2026-11-02. O usuário decidiu seguir com o pacote sem revisão jurídica adicional (2026-11-02). A DLL não entra no Git; é obtida por `scripts/baixar-pdfium.mjs`.

## PDFium (`pdfium.dll`)

O pacote `pdfium-win-x64.tgz`, tag `chromium/8125`, fica em `%LOCALAPPDATA%/plots-build/pdfium/chromium-8125/`, obtido por `scripts/baixar-pdfium.mjs`. Os textos abaixo foram lidos em 2026-11-02. Ao gerar `THIRD-PARTY-NOTICES`, copie cada arquivo **inteiro**, sem resumir.

| Arquivo do pacote | Componente | Licença | O que exige |
|---|---|---|---|
| `LICENSE` | Scripts de `bblanchon/pdfium-binaries` | MIT | Manter o aviso |
| `licenses/pdfium.txt` | PDFium | BSD-3 e Apache-2.0 | Manter o aviso; não usar o nome do Google para promover o PlotS |
| `licenses/abseil.txt` | Abseil | Apache-2.0 | Manter o aviso |
| `licenses/agg23.txt` | Anti-Grain Geometry 2.3 | Permissiva própria | Manter o aviso |
| `licenses/dragonbox-Apache2-LLVM.txt` | Dragonbox | Apache-2.0 com exceções LLVM | Manter o aviso |
| `licenses/dragonbox-Boost.txt` | Dragonbox | Boost 1.0 | Manter o aviso |
| `licenses/fast_float.txt` | fast_float | MIT | Manter o aviso |
| `licenses/freetype.txt` | FreeType | FTL | **Crédito à FreeType na documentação** |
| `licenses/harfbuzz.txt` | HarfBuzz | "Old MIT" | Manter o aviso |
| `licenses/highway.txt` | Highway | Apache-2.0 | Manter o aviso |
| `licenses/icu.txt` | ICU e dados | Unicode-3.0 e outras (ver abaixo) | Manter o aviso |
| `licenses/lcms.txt` | Little CMS 2.19 | MIT | Manter o aviso |
| `licenses/libjpeg_turbo.ijg` | libjpeg | IJG | **Dizer na documentação: "based in part on the work of the Independent JPEG Group"** |
| `licenses/libjpeg_turbo.md` | libjpeg-turbo | BSD-3, IJG e zlib | Manter o aviso |
| `licenses/libopenjpeg.txt` | OpenJPEG | BSD-2, com ressalva de patentes de terceiros | Manter o aviso |
| `licenses/libpng.txt` | libpng | PNG Reference Library v2 e v1 | Manter o aviso |
| `licenses/llvm-libc.txt` | LLVM libc | Apache-2.0 com exceções LLVM e NCSA antiga | Manter o aviso |
| `licenses/simdutf.txt` | simdutf | MIT | Manter o aviso |
| `licenses/zlib.txt` | zlib | zlib | Manter o aviso |

**`icu.txt` por dentro:** Unicode-3.0 e a licença antiga do ICU; dicionários cjdict (BSD, Libtabe, IPADIC e ICOT), Lao e Burmese (BSD); Time Zone Database (domínio público); double-conversion (BSD-3); nlohmann/json e sorttable.js (MIT); install-sh (X11). Cita também GPL com exceção do Autoconf em `aclocal.m4` (pkg.m4) e `config.guess`, marcados como "only for ICU4C". Parecem arquivos só de compilação, sem marcadores deles na DLL, o que indica mas não prova que não entram no binário. O texto vai inteiro nos avisos.

**Na DLL, sem texto próprio no pacote; incluir à parte nos avisos:**
- **libc++:** confirmada por marcadores na DLL. Licença esperada: Apache-2.0 com exceções LLVM; conferir o texto na fonte. libc++abi e os builtins do compilador são prováveis.
- **Fontes embutidas `ChromeSansMM` e `ChromeSerifMM`:** cabeçalho com "Copyright (c) 1990-1994 Adobe Systems Incorporated. All Rights Reserved". O pacote não traz licença específica; podem estar cobertas pelo texto do PDFium, o que não foi confirmado. Confirmar a origem na fonte do PDFium antes de distribuir.

**CRT da Microsoft:** a DLL só importa `KERNEL32`, `ADVAPI32`, `GDI32` e `USER32`, então parece não exigir outro redistribuível. O CRT provavelmente é estático.

**Decisão do usuário (2026-11-02):** seguir com o pacote, sem revisão jurídica adicional. As patentes de imagem ficam como risco residual aceito, conforme "Decisões do usuário"; reavaliar se o PlotS passar a ser vendido.

Crates ligadas ao `pdfium-render` no grafo Windows: `itertools`, `maybe-owned` e `utf16string` (MIT OR Apache-2.0); `piston-float` e `vecmath` (MIT); `libloading` (ISC).

## Só na compilação

Não entram no binário; repetir a checagem do grafo ao montar o instalador.

- **Rust, MPL-2.0:** `cssparser` 0.37.0, `cssparser-macros` 0.7.1, `dtoa-short` 0.3.5 e `selectors` 0.38.0, usados pelas macros do Tauri.
- **npm, MPL-2.0:** `lightningcss` 1.33.0 e seus pacotes de plataforma, usados pelo Vite.
- **`r-efi`:** oferece MIT, Apache ou LGPL; optamos por MIT ou Apache. Fora do grafo Windows.

## A conferir antes de distribuir

- **Avisos, feitos em `1a1f6d7`:** SDK do WebView2 1.0.3800.47 com `LICENSE.txt` e `NOTICE.txt`; libc++ e libc++abi com origem fixada; NSIS 3.13 com `COPYING` e a declaração da CPL 1.0, seção 3(b); `nsis_tauri_utils` 0.5.3 (MIT e Apache-2.0); `RUST-STD-NOTICES.html`, copiado do toolchain; seção própria da licença do Vite, pelo auxiliar de pré-carregamento no bundle; licenças aninhadas de `regex-syntax` e `tracing-core`, com `rfd` fora por ser só Linux. Cinco pacotes sem arquivo de licença têm o texto fixado por commit e hash em `scripts/licencas-externas/origens.json`.
- **Risco residual das fontes do PDFium:** a `pdfium.dll` contém ChromeSansMM e ChromeSerifMM, publicadas pelo PDFium sob BSD, com a nota de 1990-1994 da Adobe mantida nos dados. O BSD do PDFium não prova a cadeia de direitos entre Adobe, Foxit e Google. O uso do pacote é decisão do usuário; o assunto volta se o PlotS passar a ser vendido.
- **Ainda não lidos:** termos do WebView2 Runtime, que o instalador baixa da Microsoft se faltar; contrato do Visual Studio da máquina de build, pelo CRT estático.
- **Executável:** sem caminhos da máquina, conferido por busca de bytes; "Kaio Luan" fica no recurso de versão, por escolha. Os briefs antigos em `plan/briefs/` trazem caminhos locais: decidir antes de abrir o repositório.
- **Nome do projeto:** sem busca de anterioridade de "PlotS", por decisão do usuário em 2026-11-03. A licença do PlotS é MIT (`LICENSE`), por decisão do usuário em 2026-11-02; as dependências continuam sob as próprias licenças.
- **Linux:** `gtk`, `webkit2gtk` e `soup3` estão no lock; conferir as licenças das bibliotecas do sistema ao empacotar (plano `18-linux-e-macos`).

## Ativos

| Ativo | Origem | Licença |
|---|---|---|
| Imagem da tela de abertura e ícone (`imgs/`, e os gerados em `ui/assets/splash.png`, `ui/assets/marca.png`, `src-tauri/icons/icon.ico`) | Entregues pelo usuário em 2026-11-03, com o nome dele na imagem. Origem e direitos sob a responsabilidade dele; não houve revisão jurídica do desenho | Do projeto |
| Figura do gesto de recolocação (`ui/src/components/FiguraGesto.tsx`, `FiguraGesto.module.css`) | Desenho próprio, feito em código pelo agente Claude (Opus 5.5) para o projeto do usuário em 2026-11-03, só com primitivas SVG e os tokens da interface. Impressora genérica, sem marca nem modelo, sem base em ícone de terceiros nem em ilustração de manual | Do projeto |
| Figuras das respostas da calibração (`ui/src/components/CalibracaoFigura.tsx`, `CalibracaoFigura.module.css`) | Desenho próprio, feito em código pelo agente Claude (Opus 5.5) para o projeto do usuário em 2026-11-03, só com primitivas SVG e os tokens da interface. Impressora genérica, sem marca nem modelo, na mesma composição da figura de `FiguraGesto.tsx`; folhas, números, setas e selo de certo ou errado são formas funcionais. Sem base em ícone de terceiros nem em ilustração de manual | Do projeto |
| PlotSim (`PlotSim/`) | Ferramenta de teste do projeto, sob a licença MIT da raiz. Não é distribuída com o PlotS e fica fora de `THIRD-PARTY-NOTICES`. Se um dia for distribuída, precisa de aviso próprio (Inter OFL, lucide, PDFium, `option-ext` MPL) | Do projeto |
| Dados do `pdfjs-dist` 6.5.438 embutidos no bundle, não versionados, sem alteração: 168 `cmaps/*.bcmap`; 10 `standard_fonts/Foxit*.pfb`; `wasm/jbig2.wasm`; `wasm/openjpeg.wasm` | Pacote npm `pdfjs-dist` | CMaps: BSD-3 da Adobe (`cmaps/LICENSE`). Foxit: BSD-3 dos PDFium Authors (`standard_fonts/LICENSE_FOXIT`). JBIG2: BSD-3 dos PDFium Authors e Apache-2.0 da Mozilla (`wasm/LICENSE_JBIG2`, `wasm/LICENSE_PDFJS_JBIG2`). OpenJPEG: BSD-2 com ressalva de patentes de terceiros e BSD da Mozilla (`wasm/LICENSE_OPENJPEG`, `wasm/LICENSE_PDFJS_OPENJPEG`). Patentes de imagem: ver "Decisões do usuário". Ao distribuir: copiar cada arquivo inteiro em `THIRD-PARTY-NOTICES` |
| Fonte Inter, pesos 400, 500 e 600, subconjuntos latin e latin-ext (seis woff2 embutidos no bundle, não versionados) | `@fontsource/inter` 5.3.0, sem alteração. Copyright 2016 The Inter Project Authors (https://github.com/rsms/inter). Sem Reserved Font Name declarado | SIL OFL 1.1. Ao distribuir: incluir o aviso de copyright e o texto integral da licença em `THIRD-PARTY-NOTICES`; não vender a fonte sozinha; a fonte permanece sob OFL, qualquer que seja a licença do PlotS |
