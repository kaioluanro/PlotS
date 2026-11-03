# visualizador: rodada final

Responda em português brasileiro. Worktree: `E:\PlotS\.claude\worktrees\agent-a31b1831d9fb8e0eb`, branch `worktree-agent-a31b1831d9fb8e0eb`, último wip `fe46218`, base `c50e6d1`. **Trabalhe só nele.** Nunca dê push nem squash. Não edite `plan/`. Regras comuns em `coordenacao-paralela.md`, nesta pasta. Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.

**Ambiente:** `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/visualizador"`, `CARGO_INCREMENTAL=0`, `-j 4`. `npm ci` em `ui/`. Capturas em `.../plots-build/visualizador/saidas/depois/tela`, com PDFs de teste gerados por você. Só `ci.log` e `fail-before.log` ficam em `target/`. Capturas por CDP, porta e pasta do WebView2 suas, encerre só seus PIDs, não envie nada a nenhuma fila, não grave no `%APPDATA%` real. Python sempre com `-I`; nunca use `python3 -` por entrada padrão (um processo travou assim). Se o sistema negar algo, pare e relate.

**Primeiro, rebase sobre `master`** (`git rebase master`). Os outros três lotes já entraram. Conflitos previstos, todos de adição vizinha: `src-tauri/src/main.rs` (registro do comando, ao lado de `interface_montou` e dos comandos de trabalho) e `ui/src/i18n.ts` (blocos de chaves). Mantenha os dois lados. Se aparecer conflito de outro tipo, relate.

## Faça só isto
1. **Cores forçadas, ícone que some** (`Visualizador.module.css`, bloco `forced-colors`): `.botao::before { background: transparent !important; }`.
2. **Cores forçadas, foco invisível:** `.botao[aria-pressed='true']:focus-visible { outline-color: HighlightText; outline-offset: -5px; }`.
3. **Erro de página corta a instrução** (`.textoErroDaPagina`): `white-space: normal; overflow-wrap: anywhere; text-align: left; min-width: 0`, sem `overflow` nem `text-overflow`.
4. **Giratório piscando** (`Visualizador.tsx:582-585`): leve o `LoaderCircle` para dentro de `TextoDeCarga`, para ícone e texto surgirem juntos depois de 200 ms. O contêiner `role="status"` continua montado. O teste atual de `mostrarTextoDeCarga` só confirma a constante: troque por um que prove o atraso.
5. **Zoom que salta ao voltar à aba em documento misto:** guarde `tamanhos` em `EstadoDaAba`, se forem as cerca de 3 linhas previstas. Se ficar maior, deixe e relate.
6. **Teste de integração:** em `ui/src/tabs-impressoras.test.ts`, aplique os eventos `cartao` e `alternar_impressora` e afirme `aba.documento === antes`: o `WeakMap` do estado por aba depende disso.
7. **Confirme com o `plots-app` real, compilado depois do rebase:**
   - um JPX e um JBIG2 (MMR, e aritmético se conseguir gerar) ainda aparecem. Se o aritmético não puder ser gerado, diga;
   - a tela em inglês: outro agente trocou o idioma com o argumento `en`, em `C:/Users/Kaio Luan/AppData/Local/plots-build/cenas/cdp.mjs`. Veja como ele fez e faça uma captura `01` em inglês;
   - com o lote de impressão presente: trocar de aba com um cartão de simulada em andamento volta à mesma página e zoom.

## Capturas
Refaça `22` nos dois temas, com o ponteiro sobre "Ajustar" e depois com o foco de teclado nele. Erro de página (`18`), se forçar a falha for barato. Leia cada uma em tamanho real.

## Relatório
Cada item, testes e falha prévia real, CI (`cargo fmt --all --check`, `cargo test --workspace --locked -j 4`, em `ui/` `npm test` e `npm run build`, em `target/ci.log`, terminando em `CI_EXIT <código>`, depois da última edição), 0 ocorrências de `__plotsE2e` em `ui/dist`, autoverificação `sh E:/PlotS/.claude/team/selfcheck.sh master src-tauri ui`, capturas lidas e o que não verificou. Apague `ui/node_modules` ao fim. Termine com `git status` limpo, em um novo commit wip. Entregue o relatório como sua resposta final, e não comece mais nada.
