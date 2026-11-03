# instalador: correções da revisão jurídica

Worktree `E:\PlotS\.claude\worktrees\agent-a70e3cb360a896251`, sobre o wip `b9ad57b`. Mesmas regras de `coordenacao-paralela-2.md`. A revisão de código aprovou. A jurídica rejeitou, com os itens abaixo, nesta ordem. **Não mude a escolha de dependências nem a licença do projeto.** `plan/` é do coordenador: relate o que precisa mudar lá.

1. **SDK do WebView2 1.0.3800.47:** copie sem alterar `LICENSE.txt` (sha256 começa com `0af8f1b8`) e `NOTICE.txt` do nupkg `https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/1.0.3800.47/microsoft.web.webview2.1.0.3800.47.nupkg` para `scripts/licencas-externas/`. Crie a entrada "Microsoft Edge WebView2 SDK 1.0.3800.47 (WebView2LoaderStatic.lib, ligado estaticamente por webview2-com-sys)", citando essa origem.
2. **libc++ e libc++abi:** inclua inteiros, com origem fixada, os `LICENSE.TXT` das revisões do ramo `chromium/8125` do PDFium:
   - `https://chromium.googlesource.com/external/github.com/llvm/llvm-project/libcxx/+/97b436da4c33663581d394f4ee0a5977fc38c2f4/LICENSE.TXT`
   - `https://chromium.googlesource.com/external/github.com/llvm/llvm-project/libcxxabi/+/09351f6ec00c2b65f2d03585ae6da07098b024dc/LICENSE.TXT`

   Troque a frase por: "libc++ e libc++abi (LLVM) estão na pdfium.dll; os textos completos estão abaixo."
3. **Fontes:** em `avisos-lib.mjs`, troque a frase por: "A pdfium.dll contém as fontes ChromeSansMM e ChromeSerifMM, publicadas pelo PDFium nos arquivos core/fxge/fontdata/chromefontdata/FoxitSansMM.cpp e FoxitSerifMM.cpp, sob a licença BSD de licenses/pdfium.txt. Os dados dessas fontes contêm a nota 'Copyright (c) 1990-1994 Adobe Systems Incorporated. All Rights Reserved', mantida sem alteração."
4. **NSIS (alternativa A):** inclua o `COPYING` do NSIS 3.13 inteiro e frases que cubram a cláusula 3(b), itens i a iv, da CPL 1.0: o módulo LZMA está sob a CPL 1.0; sem garantias; condições oferecidas só por seus autores; código-fonte em `https://nsis.sourceforge.io/Download`. Inclua `LICENSE_MIT` e `LICENSE_APACHE-2.0` de `tauri-apps/nsis-tauri-utils`, conferindo versão e origem exatas do plugin usado pelo `tauri-bundler` desta versão.
5. **Biblioteca padrão do Rust:** em `empacotar.mjs`, copie `share/doc/rust/COPYRIGHT-library.html` de `rustc --print sysroot` para os recursos como `RUST-STD-NOTICES.html`, sempre o do toolchain do build, e cite-o no cabeçalho de `THIRD-PARTY-NOTICES`. Sem o arquivo, o script falha.
6. **Origens fixadas:** em `origens.json`, troque `master` e `main` por SHAs, com `sha256` e `versao`, e faça o gerador validar o hash:
   - `webview2-com` e `webview2-com-sys` 0.39.1: `edc2caf886175ccaebe86078c9cfe1ae2a187328`;
   - `alloc-stdlib` 0.2.4: `ae42d22078b98549e987d2f03d12df7b984fde47`;
   - `alloc-stdlib` 0.3.0: `0a81fd6928ea3b33c8cd484aa4575d50ffb98012`;
   - `react-remove-scroll-bar` 2.3.8: `7301c160fda44cb8cf2b9fdfde61efad35736196`, com a nota "o pacote 2.3.8 declara MIT e não traz arquivo; texto adicionado depois pelo autor, no commit 7301c16".

   Em seção compartilhada, a nota "não traz arquivo" nomeia os pacotes a que se aplica (hoje `THIRD-PARTY-NOTICES:632` soa como se valesse para `brotli` e `alloc-no-stdlib`). No cabeçalho: "Cada texto abaixo foi copiado inteiro do arquivo de licença do próprio componente ou da origem indicada na entrada."
7. **README, sem executar o instalador nesta máquina:** a instalação não foi testada, então não afirme como fato. Diga, em uma frase, "O instalador foi gerado, mas ainda não foi testado em uma instalação real", e redija "não pede administrador", "Abrir com" e "não apaga os perfis" como o que ele foi configurado para fazer. Acrescente: "Se o WebView2 Runtime faltar, o instalador o baixa da Microsoft; ele segue os termos da Microsoft."
8. **Licenças aninhadas:** lista explícita `LICENCAS_ANINHADAS`, com `regex-syntax` (`src/unicode_tables/LICENSE-UNICODE`) e `tracing-core` (`src/spin/LICENSE`), e um teste. Comentário explicando por que `rfd` fica fora (Linux).
9. **FreeType:** use "Portions of this software are copyright © 2026 The FreeType Project (www.freetype.org). All rights reserved."
10. **Avisos desatualizados:** `empacotar.mjs` regera os avisos e falha se houver diferença, antes do build. Mova a checagem da CLI do Tauri para antes de criar `src-tauri/gen/preparo`.
11. **Metadado:** em `tauri.conf.json`, "impressao" vira "impressão".
12. **Bundle:** confira se o `ui/dist` final leva helpers de `vite` ou `@vitejs/plugin-react`; se houver trecho relevante, acrescente o MIT correspondente, e relate o que achou.
13. Regere os avisos, gere o instalador de novo e rode o CI completo depois da última edição, inclusive os testes de `scripts/`, em um único log real, se o sistema permitir. `git status` limpo, com novo wip. Apague `ui/node_modules` e `ui/dist`.

Relatório novo em `plots-build/instalador/relatorio-correcoes.md`, com a mesma mensagem na resposta final: item a item, novo caminho do instalador, linha do CI, o que não foi verificado e as linhas que `plan/licencas-terceiros.md` precisa receber (contagem de crates, seção "A conferir", risco residual das fontes).
