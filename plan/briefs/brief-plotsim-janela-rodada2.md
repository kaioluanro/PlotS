# plotsim-janela: leitor de marca, canal e rebase

Responda em português brasileiro. Trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-ae5845800b5cc2708`, commit `b1a6541`, base `c12856d`. Não crie outro worktree. Build e saídas em `%LOCALAPPDATA%/plots-build/plotsim-janela`, com `CARGO_TARGET_DIR` ali, `CARGO_INCREMENTAL=0`, `-j 4`; porta CDP 9350. Rode `npm ci` em `ui/` e em `PlotSim/app/ui/`. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`. Regras comuns: `plan/briefs/coordenacao-paralela-2.md`. Especificação: `PlotSim/AGENTS.md`, `plan/23-plotsim.md`. Limite de tempo em todo comando; não espere processo em segundo plano sem prazo. Só crie ou remova filas com o prefixo `PlotSim — ` e prefixo `Cena` no nome, com porta em `C:\ProgramData\PlotSim\`.

**Primeiro, rebase sobre o `master`.** Ele tem a suíte: `plotsim_core::leitor::biblioteca()` pública, `pub use ...Pdfium`, `plotsim_core::marca`, `plotsim_core::canal`. Só `PlotSim/app/`, `Cargo.toml` e `Cargo.lock` são seus; não mude o resto. Se precisar mudar o núcleo, relate.

## Itens

1. **Uma ligação só do PDFium:** apague a ligação própria de `PlotSim/app/src/desenho.rs` e a cópia de `inspecionar`. Use `plotsim_core::leitor::biblioteca()` e a leitura do núcleo. Mantenha o teste real `le_e_desenha_o_mesmo_pdf_no_mesmo_processo`.
2. **Leitor de marca em `marca.rs` da janela:** use `ler_marcas` do núcleo. Folha com marca mostra número e giro e dá veredito; folha sem marca continua "Sem marca legível", sem palpite. Mantenha o texto como segundo caminho só se ele já for necessário para PDFs de texto, e diga qual fonte foi usada. Folha em branco do total ímpar deve aparecer como branca, não como sem marca.
3. **Canal:** ligue `plotsim_core::canal` a `Estado::aplicar_gesto`, para o gesto publicado por `plots imprimir --teste` chegar à janela sem escolha manual, e com o total do documento, se a mensagem trouxer. Sem cliente, nada trava. O modo manual segue funcionando. Se a suíte e a janela disputarem o pipe, relate o que acontece e não travar é o requisito.
4. **Total do documento:** se vier do canal, a falta no fim do documento passa a ser acusada.

## Uso real

Por CDP: fila real criada pela janela, `plots-cli imprimir --teste` real com um PDF com marca (gere pela suíte), nos dois modelos. Veja "Certa" pelo caminho real do spooler, sem PDF entregue à mão: passada 1, gesto vindo do canal, passada 2, 3 de 3 corretas. Depois, um gesto errado escolhido à mão e a folha acusada. Recapture, nos dois temas, em inglês, na janela mínima e em cores forçadas: folhas certas e erradas pelo caminho real, fila em modo teste, e as telas que mudaram. Leia cada imagem. Regrave o que estava antigo: `01` a `16` e as `en-`.

## Final

CI em um único log `saidas/ci.log`: `cargo fmt --all --check`, `cargo test --workspace --locked -j 4`, `cargo test -p plotsim-app -- --ignored`, `ui/` e `PlotSim/app/ui/` com `npm ci`, `npm test`, `npm run build`, 0 `__plotsE2e` e 0 `__plotsimE2e`, `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR="%LOCALAPPDATA%/plots-build/pdfium/chromium-8125/bin"`, terminando com `CI_EXIT <código>`. Autoverificação com a base do rebase. `git status` limpo com novo wip, apague `ui/node_modules`, `ui/dist`, `PlotSim/app/ui/node_modules` e `PlotSim/app/ui/dist`. Não deixe processo nem fila sua.

Resposta final, mesmo que falte algo: item a item, testes com grupo de falha prévia, linha do CI, capturas com caminho e o que não foi verificado.
