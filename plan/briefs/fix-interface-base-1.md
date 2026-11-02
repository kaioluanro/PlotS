# interface-base, correção 1

Responda em português brasileiro. Trabalhe no mesmo worktree: `E:\PlotS\.claude\worktrees\agent-a6db429760e6413f3`. Atenção: o coordenador fez rebase do seu branch sobre `master`; o commit agora é `18d6afa` sobre `cc9d4f8`, e o workspace inclui `plots-perfis`. Faça commits wip; nunca dê push nem squash.

As regras de ambiente, processos, identificadores do plano e relatório do brief `E:\PlotS\plan\briefs\brief-interface-base.md` continuam valendo. Use `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/interface"`. Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh cc9d4f8 src-tauri ui`.

## A. Comportamento
1. **Fechar a janela** (botão × e Alt+F4): com alguma aba em impressão, interceptar e abrir o mesmo diálogo, com título "Fechar janela com impressão em andamento?", foco inicial em "Manter janela aberta" e o nome do documento, ou a contagem se houver mais de um.
2. **Consequência no diálogo.** Acrescente nos dois casos a frase: "O que já foi enviado continua na fila, mas o PlotS deixa de acompanhar este documento e o verso não poderá ser impresso por aqui." / "Anything already sent stays in the queue, but PlotS stops tracking this document and the back side can no longer be printed from here." Em inglês, troque o texto atual por "{nome} is still printing."
3. **Excesso de abas alcançável:** máscara de fade de 24 px na borda com conteúdo oculto, roda vertical do mouse rolando a lista na horizontal e "+" sempre visível.
4. **Erros em lote.** Em `abrirCaminhos`, não apague o erro de um caminho anterior quando um seguinte abre. Extraia a lógica para função pura com teste de regressão: uma falha mostra a mensagem dela; várias mostram "{n} arquivos não abriram" / "{n} files did not open".
5. **Arrasto.** Trate enter/over/leave: contorno interno de 2 px em `--accent` na área do documento e o texto "Solte para abrir" / "Drop to open". Limite o lote a 32 caminhos, como a linha de comando, e avise quando houver corte, nos dois caminhos: "Só os primeiros 32 arquivos foram abertos." / "Only the first 32 files were opened."
6. **Recarga do WebView2.** Bloqueie Ctrl+R, Ctrl+Shift+R, F5, Ctrl+P e o menu de contexto padrão: recarregar perde as abas. Ignore argumentos de linha de comando que começam com `--`.
7. **Foco itinerante nas abas:** ativa com `tabindex=0`, demais com `-1`, setas para navegar; × focável por Tab só na aba ativa. Fechar inativa por teclado é seta + Ctrl+W.

## B. Visual
8. **Barra da aba ativa** com 16×2 px na base da aba (`left: 8px; bottom: 1px`), em `--accent`.
9. **Documento aberto:** abaixo do nome, em 11 px e `--fg-muted`: "Esta versão ainda não mostra o documento." / "This version does not show the document yet."
10. **Textos da lateral:** "Ainda não disponível" / "Esta versão não lista impressoras." e "Not available yet" / "This version does not list printers."
11. **EmptyState discreto na lateral:** círculo de 38 px com borda `--border-strong`, ícone de 18 px em `--fg-faint`, título 12/500 em `--fg-muted`, texto 11 px. Na área principal, 16 px entre texto e botão.
12. **Fonte Inter embutida** em woff2 via `@font-face`, só os pesos usados, a partir do pacote `@fontsource/inter` com versão exata. Confirme a licença OFL-1.1 e informe a versão, os arquivos usados e o tamanho em disco.
13. **Cores forçadas:** aba ativa com `background: Highlight; color: HighlightText`, e hover visível.
14. **Marca provisória:** duas folhas com a de trás preenchida, mantendo 17×17 px, borda de 1 px e raio 5. Espelhe o ícone de restaurar com `scaleX(-1)` para coincidir com o glifo nativo.
15. **Janela mínima:** descubra por que as capturas têm 624×411 e garanta área útil de 640×420.
16. **Ajustes finos:** `padding-left: 12px` na barra; token `--status-offline-hover` (escuro `#f87171`, claro `#b91c1c`) no hover do perigo, sem borda de acento; anel de foco na aba inteira e revelação do × via `.aba:has(:focus-visible)` no lugar de `:focus-within`; hover do × com `background: var(--border-strong)` e `color: var(--fg)`.
17. **Tamanho dos ícones.** O ícone de 18 px no estado vazio e o × de 12 px em caixa de 24×24 foram aprovados pela autoridade de interface. Nos demais, mantenha 12 a 14 px. Registre essas exceções e a interpretação de que o tema segue o sistema, com escuro como reserva, em comentário curto no `theme.css`.

## C. Jurídico
18. **Identificador:** em `src-tauri/tauri.conf.json`, use `"identifier": "io.github.kaioluanro.plots"`.
19. **Origem do ícone:** versione o SVG-fonte em `src-tauri/icons/marca.svg` e um gerador pequeno e sem dependência nova em `src-tauri/icons/`, que produza o `.ico` a partir das medidas. Regenere o `.ico` com o desenho novo do item 14. Informe no relatório autor, data e ferramenta, para o coordenador registrar. O desenho é próprio e não usa marca de terceiros.

## Testes
Testes novos ou ajustados para 4, 5, 6 (argumentos com `--`), 7, consequência nos dois idiomas e fechamento da janela (lógica de decisão em função pura). Registre falha prévia onde houver base.

## Capturas e teste de uso
Refaça **todas** as capturas, porque a fonte muda as larguras, e acrescente: 12+ abas em 1280 px e no mínimo; arrasto sobre a janela; hover do ×; foco em "+", lateral, controles da janela e × da faixa de erro; diálogo e faixa de erro com nome de mais de 60 caracteres, e no mínimo; diálogo e foco em cores forçadas; movimento reduzido; larguras 1024 e 1440; escala 125% e 150% (por `Emulation.setDeviceMetricsOverride`, dizendo que é emulação). Exercite fechar a janela com impressão em andamento, com a porta de teste, e confirme com `grep` que `__plotsE2e` não está em `ui/dist` no build normal.

Redimensionar pelas bordas, sombra, cantos e Snap Layouts ficam para verificação manual do usuário: liste os passos no relatório, curtos.

## Relatório
Cada item com o que mudou e seu teste, linha do CI (`cargo fmt --all --check`, `cargo test --workspace --locked`, `npm test`, `npm run build`), autoverificação vazia, contraste de cada par novo nos dois temas, cada captura com o que mostra, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo.
