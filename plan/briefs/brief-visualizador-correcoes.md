# visualizador: rodada de correções

Responda em português brasileiro. Worktree: `E:\PlotS\.claude\worktrees\agent-a31b1831d9fb8e0eb`, branch `worktree-agent-a31b1831d9fb8e0eb`, último wip `db14e25`, base `c50e6d1`. **Trabalhe só nele.** Nunca dê push nem squash. Leia antes `brief-visualizador.md` e `coordenacao-paralela.md`, nesta pasta, e o código em `ui/src/visualizador/`. Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; o grafo pode não cobrir a interface.

Compilação em `C:/Users/Kaio Luan/AppData/Local/plots-build/visualizador`, `CARGO_INCREMENTAL=0`, `-j 4`. Saídas e capturas em `.../plots-build/visualizador/saidas`. Só `ci.log` e `fail-before.log` ficam em `target/`. Rode `npm ci` em `ui/`. Abaixo de 500 MB livres em E:, pare e relate. Registre a falha prévia de cada teste novo com execução real.

## A. Fonte sob GPLv2 (bloqueia)
`recursos.ts:16-19` embute `LiberationSans-*.ttf` 1.07.4, do `pdfjs-dist`, sob GPLv2 sem "ou posterior". O conjunto com o pdf.js (Apache-2.0) é discutível, e a troca é barata. **Essa versão não pode entrar no bundle.** Tente nesta ordem:
1. **Fontes do sistema.** Tire a Liberation do glob e tente `useSystemFonts: true`, para Helvetica não embutida cair na Arial do Windows. Teste no app real com PDF de teste, gerado por você, com Helvetica não embutida: o texto aparece, com largura certa e sem erro de CSP? Este caminho não traz binário nem download, e a prévia não é o que vai à impressora.
2. **Se o caminho 1 falhar:** Liberation Sans 2.1.5 sob SIL OFL 1.1, as quatro TTF e o `LICENSE`, sem alteração, em `ui/src/visualizador/fontes/`. Pacote: `https://github.com/liberationfonts/liberation-fonts/files/7261482/liberation-fonts-ttf-2.1.5.tar.gz`, sha256 `7191c669bf38899f73a2094ed00f7b800553364f90e2637010a69c0e268f25d0`. Baixe em uma pasta vazia em `saidas`. Confira os hashes das cópias:
   - Regular `76d04c18ea243f426b7de1f3ad208e927008f961dc5945e5aad352d0dfde8ee8`
   - Bold `788abee4c806d660e8aee46689dd8540cd4bb98da03dcc9d171ce3efd99a9173`
   - Italic `e5bae5c4cde31f22142753855f4f8fb86da6ff39955ed3c0a11248b0d16948b0`
   - BoldItalic `698da70fc191cc5f33ad4d6d3fe830fe4624b898ea2e3169955928b7c491f1ee`
   - LICENSE `93fed46019c38bbe566b479d22148e2e8a1e85ada614accb0211c37b2c61c19b`
   Se o sistema negar o download, pare e relate, sem contornar.

Em qualquer caminho: confirme no `ui/dist` que a fonte 1.07.4 não está em nenhum asset ou chunk, e acrescente um teste que confira as chaves dos globs. Relate qual caminho ficou, porque atualizo o registro de licenças com isso.

## B. Código
3. **JPEG 2000, JBIG2 e CMap de CJK sob a CSP.** A CSP não tem `'wasm-unsafe-eval'`, e o WebView2 deve bloquear a compilação do wasm. O fallback em JS também falha, porque `wasmUrl` não é passado. Gere PDFs de teste com JPX e JBIG2 (PyMuPDF ou Pillow, `python -I`, se possível) e um com CMap de CJK, e abra no app real. **Se o wasm falhar:** você está autorizado a acrescentar só `'wasm-unsafe-eval'` ao `script-src` de `src-tauri/tauri.conf.json`, uma mudança mínima que integro depois com o lote da tela de abertura. Mostre o antes e o depois. Se não conseguir gerar a fixture, diga isso: não relate como testado.
4. **`abrirPdf` em `pdf.ts`:** cheque `aberto()` antes de criar o `PDFWorker` e de transferir os bytes, com teste.
5. **Tire `qcms_bg` do glob** em `recursos.ts`: é código morto sem `wasmUrl`. Relate a limitação de cores ICC e CMYK na prévia.
6. **Vazamento:** meça a memória privada depois de fechar a aba do PDF de 419 MB. Precisa voltar perto da base. Se não voltar, investigue.
7. **Remova a chave órfã** `documento.semVisualizacao` dos dois idiomas. Autorizo esta exceção à regra de `i18n.ts`.

## C. Interface (em ordem)
8. **Ajustar à largura:** use a maior largura medida do documento, sem `larguraDeReferencia`. Reavalie o zoom uma vez quando a medição terminar, mantendo a âncora, e arredonde para baixo. Sem barra horizontal nesse modo. Recapture `01`, `10`, `12`. Meça o que acontece com A4 misturado a uma página muito maior e relate: o custo aceito é a página menor aparecer pequena.
9. **Por aba:** guarde `ancora`, `ajustando` e `zoomPedido` por aba, descarte ao fechar a aba e restaure após reler. O PDF continua sendo solto ao trocar. Mostre "Abrindo o PDF…" só depois de 200 ms. `DocumentArea.tsx` é seu; `App.tsx` e `tabs.ts` não: guarde o estado no seu módulo, por id da aba. Nova captura: voltar à aba na mesma página.
10. **Miniaturas:** chame `mostrar(atual)` sempre que `atual` mudar, mesmo com foco na lista (`Miniaturas.tsx:127-129,144-147`). Setas na lista não mudam `atual`. Recapture `13`.
11. **Ações nos erros:**
    - `17`: botão secundário "Tentar de novo" / "Try again", que relê na mesma aba. Texto: "Não foi possível mostrar este PDF. Tente de novo; se continuar, salve uma nova cópia no programa de origem." / "Could not show this PDF. Try again; if it keeps failing, save a new copy from the original program."
    - `15`: botão secundário com o `documento.abrir` existente, ligado a `aoAbrir`.
    - `erro.grande`, acrescente: "Feche outros programas ou divida o PDF em partes menores e abra de novo." / "Close other programs or split the PDF into smaller parts and open it again."
    - `visualizador.pagina.erro`: "Não foi possível mostrar a página {n}. Salve uma nova cópia no programa de origem e abra de novo." / "Could not show page {n}. Save a new copy from the original program and open it again."
    - Texto do erro de página em `--fg-muted` 12 px, com `title`. Recapture `15`, `17`, `18`.
12. **Tecnologia assistiva:** `aria-label` conforme o estado em `Pagina.tsx` e `Miniaturas.tsx`. Na miniatura: "Ir para a página {n}: não foi possível mostrar" / "Go to page {n}: could not be shown".
13. **Limite da folha:** contorno de página e miniatura em `--border-control` (`Visualizador.module.css:143,259`). Recapture `08-*-light`, `13-*-light`.
14. **Miniatura:** `padding: 6px 6px 2px` e 8 px a mais na altura do item (`contas.ts:327`), sem colisão entre foco e moldura. Recapture `05`, `06`, `11`, `12`.
15. **Cores forçadas:**
    - `.botao[aria-pressed='true'] { background: Highlight; color: HighlightText }`
    - `.botao:hover:not([aria-disabled='true']) { border-color: Highlight }`
    - `.miniatura:hover { outline: 1px solid Highlight }`
    - `.divisor { background: CanvasText }`
    Nova captura `22-cores-forcadas-{dark,light}` por emulação no CDP.
16. **Barra:** tabindex móvel e setas na `toolbar`; `aria-label` fixo "Miniaturas" / "Thumbnails", com mostrar e ocultar no `title`; percentual com `role="status"`, só no zoom.
17. **Pressionado** com `--border-control` (`Visualizador.module.css:87-91`). Recapture `01`.
18. **Polimento:**
    - fundo com recuo de 2 px mantendo a área de 28×28;
    - `min-height` em `.barra`;
    - número da atual centrado, com `ChevronRight` de 12 px no lugar do `▸`;
    - ícone de miniaturas estável no carregamento;
    - no teclado, respeitar os limites do zoom: Ctrl+`-` no limite não desliga "ajustar".
    Recapture `02`, `05`, `16`, `08`.
19. **Movimento reduzido:** captura `23-movimento-reduzido-{dark,light}` no estado carregando, por `Emulation.setEmulatedMedia`. Informe a duração computada de `.girando`. **Não altere configurações do Windows.**

Fica para depois, eu registro no plano: Ctrl+roda, ir à página digitando o número e o token `--panel-hover` no tema claro.

## Regras
Mesmas do brief original: porta de depuração e pasta do WebView2 só suas; encerre só os seus PIDs, por número; capturas por CDP, com PDFs de teste gerados por você; PDFs reais em Downloads só para medir, em leitura; não envie nada a nenhuma fila; não grave no `%APPDATA%` real; não edite `plan/`. Meça o contraste de cada par novo com `contrast.py`. Leia as capturas no tamanho real.

## Relatório
Cada item, com testes e falha prévia, caminho escolhido em A, resultado de JPX, JBIG2 e CJK (antes e depois), memória após fechar a aba, CI (`target/ci.log` terminando em `CI_EXIT 0`, depois da última edição), autoverificação (`sh E:/PlotS/.claude/team/selfcheck.sh c50e6d1 src-tauri ui`), contraste, capturas e o que não verificou. Apague `ui/node_modules` ao fim. Termine com `git status` limpo, em um novo commit wip.
