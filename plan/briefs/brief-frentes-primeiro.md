# frentes-primeiro: imprimir todas as frentes e deixar os versos para depois

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. O comportamento está em `E:\PlotS\plan\24-impressao-sem-lote.md`. Leia também `plan/13-fluxo-e-recolocacao.md`, `plan/15-multiplas-impressoras.md`, `plan/22-opcoes-de-envio-e-historico.md`, as regras comuns de `plan/briefs/coordenacao-paralela-2.md` e a norma `E:\PlotS\plan\estilo-ui-ux.md`, com a skill `ui-design-craft`. Build e saídas em `%LOCALAPPDATA%/plots-build/frentes-primeiro`, porta CDP 9352. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`.

**Outros agentes ao mesmo tempo:** calibração (`src-tauri/src/calibracao.rs`, `CalibracaoDialog`, `trabalhos.rs` em `preparar_fila`), suíte do PlotSim (`Instrucao` em `plots-print/src/fluxo.rs`, `plots-cli`), janela do PlotSim, imagens. Entram antes de você, e um rebase virá depois. Ponha a escolha em componente novo, com uma linha em `Sidebar.tsx`, e mexa no mínimo em `trabalhos.rs`, em `fluxo.rs` e em `job.rs`.

## Pontos de cuidado

1. **Máquina de estados:** a fase de frentes não pode pedir gesto nem liberar verso; a fase de versos parte de um lote cuja frente já foi confirmada, nunca de um lote sem frente. Nenhuma frente é reenviada sem pedido.
2. **A confirmação de um verso diz qual maço entra.** É o maior risco: pôr o maço errado estraga o lote. Em cada lote: número do lote e páginas, na frase e no título do cartão. Use "Lote 2 (págs. 11–20)", com as faixas do backend.
3. **Papéis mistos:** cada lote pede o seu papel; a troca é dita antes, nas duas fases.
4. **Compatível** com intervalo, ajustar ao papel, folha de controle, compensação de driver, espelho, divisão e histórico (o tempo só conta passadas). "Reimprimir" e aviso de papel nas duas fases; o aviso de falta de papel no verso continua proibindo folhas em branco. Automático não oferece a escolha; lote automático em documento misto segue como já é, e a confirmação explica.
5. **Fechamento:** a confirmação de fechar a janela e de fechar a aba diz que há versos por imprimir. Sem persistir em disco nesta entrega.
6. **Com um lote só**, a escolha não muda nada: esconda.

## Testes

Rust: fases com 3 lotes; verso não sai antes da fase; reimprimir nas duas fases; cancelar; erro no meio; divisão com duas impressoras, cada uma na sua fase; papéis mistos; total ímpar. Propriedade: toda página sai exatamente uma vez. Conferência do maço final com `plotsim-physics` em testes do `PlotSim/plotsim-physics/tests/`, com o gesto do PlotS, nos dois modelos. Vitest: redutor, frases, cartões. App real por CDP: simuladas com 3 lotes, sem clique de gesto na primeira fase, espera, segunda fase até o fim; dois temas, inglês, janela mínima, cores forçadas por emulação. Leia cada imagem. Fila "Microsoft Print to PDF": conferir os arquivos de um envio em duas fases.

## Final

CI completo em um único log, com `ui/` e `avisos.test.mjs`, autoverificação, `git status` limpo com wip, apague `ui/node_modules` e `ui/dist`. Relatório em `%LOCALAPPDATA%/plots-build/frentes-primeiro/relatorio.md`, com a mesma mensagem na resposta final: item a item, testes com grupo de falha prévia, capturas com caminho, contraste, textos novos e o que não foi verificado. Limite de tempo em todo comando de cena; não espere processo em segundo plano sem prazo.
