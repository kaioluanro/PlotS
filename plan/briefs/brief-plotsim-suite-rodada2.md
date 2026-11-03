# plotsim-suite: ajustes finais antes de integrar

Responda em português brasileiro. Trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-aed5d1bec710ee50c`, commit `36b809a`, base `c12856d`. Não crie outro worktree. Build e saídas em `%LOCALAPPDATA%/plots-build/plotsim-suite`, com `CARGO_TARGET_DIR` ali, `CARGO_INCREMENTAL=0`, `-j 4`. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`. **Não mude o PlotS (`plots-*`, `src-tauri`, `ui/`) nem o modelo físico.** Regras comuns: `plan/briefs/coordenacao-paralela-2.md`. Limite de tempo em todo comando; não espere processo em segundo plano sem prazo.

## Contexto

A rodada `plotsim suite --rapida` deu 42 de 45. As 3 falhas são Jato Ofício, Laser Ofício e Laser misto de 7 páginas, todas com: "a fila não aplicou o papel de 216 × 330 mm e ficou em 210 × 297 mm; nada foi enviado". A fila usa o driver "Microsoft Print To PDF", cujo Ofício tem 215,9 × 355,6 mm, e não 216 × 330 mm. O PlotS recusou sem enviar nada. O que falta é a suíte dizer isso sem fingir que testou.

## Itens

1. **Estado "sem cobertura" na suíte** (`PlotSim/plotsim-suite/src/execucao.rs`, `main.rs`, `relatorio.rs`). Um cenário vira "sem cobertura" só se as duas condições valerem: o papel do cenário inclui Ofício, e o `plots` saiu com a recusa de papel não aplicado para 216 × 330 mm. Qualquer outro erro, ou essa recusa em cenário sem Ofício, continua falha. Não conta como aprovado. A linha final diz, por exemplo, "42 de 45 passaram, 3 sem cobertura, 0 falharam". O código de saída é 0 só sem falhas. O relatório HTML mostra o estado à parte, com o motivo: "o driver da fila não tem este papel; o PlotS recusou sem enviar nada". Testes dos dois lados da regra, com falha prévia em `saidas/fail-before.log`.
2. **Ligação do PDFium** (`PlotSim/plotsim-core/src/leitor.rs`, perto da linha 183). A biblioteca recusa uma segunda ligação no mesmo processo. Torne pública `plotsim_core::leitor::biblioteca() -> Result<&'static Pdfium, ErroLeitura>`, para a janela usar a mesma, com doc de uma linha dizendo isso.
3. **Página de proporção absurda** (`ler_marca_da_pagina`, em `leitor.rs`). `set_target_width` não limita a altura, e uma página de 1 × 1.000.000 pt aloca o bitmap antes de `LIMITE_PONTOS` valer. Recuse antes de desenhar, com erro claro. Teste de medidas hostis.
4. **Nome no teste:** troque a string `fila=HP real` por `fila=Impressora real`, sem mudar o que o teste confere.

## Final

CI completo em um único log `saidas/ci.log`: `cargo fmt --all --check`, `cargo test --workspace --locked -j 4`, em `ui/` `npm ci`, `npm test`, `npm run build`, 0 `__plotsE2e` em `ui/dist`, `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR="%LOCALAPPDATA%/plots-build/pdfium/chromium-8125/bin"`, terminando com `CI_EXIT <código>`. Depois do CI, recompile `plotsim` e `plots-cli` e rode só `plotsim suite --rapida --filtro Ofício --pasta <saidas/rel-oficio>`, em primeiro plano, com limite de 300 s. **A matriz inteira fica com o coordenador.** Autoverificação `sh E:/PlotS/.claude/team/selfcheck.sh c12856d PlotSim plots-cli plots-print scripts` não imprime nada. `git status` limpo com novo wip, apague `ui/node_modules` e `ui/dist`. Não deixe processo nem fila sua.

Relatório em `%LOCALAPPDATA%/plots-build/plotsim-suite/relatorio-rodada2.md`, curto, com a mesma mensagem na resposta final, mesmo que falte algo: item a item, testes com grupo de falha prévia, linha do CI, resultado do filtro e o que não foi verificado.
