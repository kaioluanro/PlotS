# Licenças de terceiros

Registro das dependências e dos ativos do PlotS. Atualize a cada lote que trouxer dependência ou ativo novo.

**Pendência que bloqueia qualquer distribuição:** gerar `THIRD-PARTY-NOTICES` a partir dos arquivos de licença de cada pacote e incluí-lo no instalador. Vale também para qualquer distribuição avulsa do `plots-cli`. O build da interface remove os avisos de React, Radix e lucide do JavaScript, então o arquivo precisa existir à parte. A licença da fonte Inter também vai nesse arquivo, porque a fonte fica embutida no app. Plano: `17-acabamento`.

## Decisões do usuário

- **`option-ext` 0.2.0 (MPL-2.0):** mantido sem alterações (2026-11-02). Entra no binário por `plots-perfis → directories → dirs-sys` e por `tauri → dirs → dirs-sys`. Ao distribuir: incluir o texto da MPL 2.0 e informar a origem do código-fonte, https://github.com/soc/option-ext.
- **Norma de estilo:** vale como instrução de estilo (2026-11-02), e os valores de cores e medidas foram usados. Não cobre logotipos, nomes de marca nem código de terceiros.

- **Distribuição:** o PlotS será só público e gratuito no GitHub, sem venda (2026-11-02). Por isso, o risco residual de patentes de imagem fica aceito e registrado, sem consulta a advogado.

## Entra no binário

Levantamento de 2026-11-02, a partir das licenças declaradas nos metadados; o texto de cada licença será conferido ao gerar os avisos.

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

**npm** (35 pacotes de produção): OFL-1.1 em `@fontsource/inter` (fonte, ver Ativos); MIT em 30; ISC em `lucide-react`, cuja licença também cita ícones derivados do Feather, usados aqui (`minus`, `plus`, `square`, `x`), então o aviso leva as duas partes; 0BSD em `tslib`; `Apache-2.0 OR MIT` em `@tauri-apps/api` e `@tauri-apps/plugin-dialog`.

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

## Só na compilação

Não entram no binário; repetir a checagem do grafo ao montar o instalador.

- **Rust, MPL-2.0:** `cssparser` 0.37.0, `cssparser-macros` 0.7.1, `dtoa-short` 0.3.5 e `selectors` 0.38.0, usados pelas macros do Tauri.
- **npm, MPL-2.0:** `lightningcss` 1.33.0 e seus pacotes de plataforma, usados pelo Vite.
- **`r-efi`:** oferece MIT, Apache ou LGPL; optamos por MIT ou Apache. Fora do grafo Windows.

## A conferir antes de distribuir

- **`webview2-com-sys` 0.39.1:** declara MIT, mas traz binários da Microsoft (`WebView2Loader`). Ler os termos do SDK e do Runtime, confirmar o tipo de vínculo no release e registrar os avisos exigidos.
- **PDFium:** copiar para os avisos os textos de `licenses/`, inteiros, e os de `libc++` e das fontes embutidas, que o pacote não traz; crédito da FreeType e do IJG; confirmação de que a DLL não exige outro redistribuível e de eventuais `NOTICE` da Apache; cópia do `.tgz` guardada fora do Git; uso de "PDFium" só nos avisos, sem sugerir vínculo.
- **Caminhos locais no executável:** o build de debug contém o nome de usuário. Compilar o release com `--remap-path-prefix` e conferir com busca no arquivo final.
- **Nome do projeto:** busca de anterioridade de "PlotS". A licença do PlotS é MIT (`LICENSE`), por decisão do usuário em 2026-11-02; as dependências continuam sob as próprias licenças.
- **Linux:** `gtk`, `webkit2gtk` e `soup3` estão no lock; conferir as licenças das bibliotecas do sistema ao empacotar (plano `18-linux-e-macos`).

## Ativos

| Ativo | Origem | Licença |
|---|---|---|
| Ícone e marca provisórios (`src-tauri/icons/icon.ico`, `src-tauri/icons/marca.svg`, marca em `ui/src/components/TitleBar.tsx`) | Desenho próprio, feito pelo agente Claude (Opus 5.5) para o projeto do usuário em 2026-11-02. `icon.ico` e `marca.svg` são gerados por `src-tauri/icons/gerar-icone.mjs`, só com a biblioteca padrão do Node; a marca da barra é desenhada no próprio componente. Sem base em marca de terceiros | Do projeto |
| Fonte Inter, pesos 400, 500 e 600, subconjuntos latin e latin-ext (seis woff2 embutidos no bundle, não versionados) | `@fontsource/inter` 5.3.0, sem alteração. Copyright 2016 The Inter Project Authors (https://github.com/rsms/inter). Sem Reserved Font Name declarado | SIL OFL 1.1. Ao distribuir: incluir o aviso de copyright e o texto integral da licença em `THIRD-PARTY-NOTICES`; não vender a fonte sozinha; a fonte permanece sob OFL, qualquer que seja a licença do PlotS |
