# Licenças de terceiros

Registro das dependências e dos ativos do PlotS. Atualize a cada lote que trouxer dependência ou ativo novo.

**Pendência que bloqueia qualquer distribuição:** gerar `THIRD-PARTY-NOTICES` a partir dos arquivos de licença de cada pacote e incluí-lo no instalador. Vale também para qualquer distribuição avulsa do `plots-cli`. O build da interface remove os avisos de React, Radix e lucide do JavaScript, então o arquivo precisa existir à parte. A licença da fonte Inter também vai nesse arquivo, porque a fonte fica embutida no app. Plano: `17-acabamento`.

## Decisões do usuário

- **`option-ext` 0.2.0 (MPL-2.0):** mantido sem alterações (2026-11-02). Entra no binário por `plots-perfis → directories → dirs-sys` e por `tauri → dirs → dirs-sys`. Ao distribuir: incluir o texto da MPL 2.0 e informar a origem do código-fonte, https://github.com/soc/option-ext.
- **Norma de estilo:** vale como instrução de estilo (2026-11-02), e os valores de cores e medidas foram usados. Não cobre logotipos, nomes de marca nem código de terceiros.

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

**npm** (35 pacotes de produção): OFL-1.1 em `@fontsource/inter` (fonte, ver Ativos); MIT em 30; ISC em `lucide-react`, cuja licença também cita ícones derivados do Feather, usados aqui (`minus`, `plus`, `square`, `x`), então o aviso leva as duas partes; 0BSD em `tslib`; `Apache-2.0 OR MIT` em `@tauri-apps/api` e `@tauri-apps/plugin-dialog`.

## Só na compilação

Não entram no binário; repetir a checagem do grafo ao montar o instalador.

- **Rust, MPL-2.0:** `cssparser` 0.37.0, `cssparser-macros` 0.7.1, `dtoa-short` 0.3.5 e `selectors` 0.38.0, usados pelas macros do Tauri.
- **npm, MPL-2.0:** `lightningcss` 1.33.0 e seus pacotes de plataforma, usados pelo Vite.
- **`r-efi`:** oferece MIT, Apache ou LGPL; optamos por MIT ou Apache. Fora do grafo Windows.

## A conferir antes de distribuir

- **`webview2-com-sys` 0.39.1:** declara MIT, mas traz binários da Microsoft (`WebView2Loader`). Ler os termos do SDK e do Runtime, confirmar o tipo de vínculo no release e registrar os avisos exigidos.
- **Caminhos locais no executável:** o build de debug contém o nome de usuário. Compilar o release com `--remap-path-prefix` e conferir com busca no arquivo final.
- **Nome e licença do projeto:** busca de anterioridade de "PlotS" e definição da licença própria.
- **Linux:** `gtk`, `webkit2gtk` e `soup3` estão no lock; conferir as licenças das bibliotecas do sistema ao empacotar (plano `18-linux-e-macos`).

## Ativos

| Ativo | Origem | Licença |
|---|---|---|
| Ícone e marca provisórios (`src-tauri/icons/icon.ico`, `src-tauri/icons/marca.svg`, marca em `ui/src/components/TitleBar.tsx`) | Desenho próprio, feito pelo agente Claude (Opus 5.5) para o projeto do usuário em 2026-11-02. `icon.ico` e `marca.svg` são gerados por `src-tauri/icons/gerar-icone.mjs`, só com a biblioteca padrão do Node; a marca da barra é desenhada no próprio componente. Sem base em marca de terceiros | Do projeto |
| Fonte Inter, pesos 400, 500 e 600, subconjuntos latin e latin-ext (seis woff2 embutidos no bundle, não versionados) | `@fontsource/inter` 5.3.0, sem alteração. Copyright 2016 The Inter Project Authors (https://github.com/rsms/inter). Sem Reserved Font Name declarado | SIL OFL 1.1. Ao distribuir: incluir o aviso de copyright e o texto integral da licença em `THIRD-PARTY-NOTICES`; não vender a fonte sozinha; a fonte permanece sob OFL, qualquer que seja a licença do PlotS |
