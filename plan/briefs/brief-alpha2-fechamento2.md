# alpha.2: segunda rodada de fechamento

Responda em português brasileiro. Leia `E:\PlotS\AGENTS.md`, `.claude/team.json`,
`.claude/skills/craft-team/SKILL.md` e `.claude/agents/plots-debugger.md` antes de agir; siga o
método do papel. Worktree existente `E:\PlotS\.claude\worktrees\agent-a690fcada973242e0`, HEAD
`1fd312a`, base do lote `861ab57`. Não crie worktree. Build em
`%LOCALAPPDATA%\plots-build\uso-real\target`, com `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`,
`-j 4`; CDP 9354/9355. `npm ci` nas duas UIs. Scripts anteriores em
`%LOCALAPPDATA%\plots-build\uso-real\` (`cap6/`, `saidas/`, `*.mjs`). Não mexa em `plan/`.

**Graphify obrigatório:** antes do primeiro grep ou leitura de código, `graphify query
"<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Ao terminar, `graphify update .`.

**Regras de uso:** clique/tecla reais via CDP, sem comando de terminal para imprimir ou
configurar, sem gancho de verificação. Dados isolados `PLOTS_PERFIS`, `PLOTS_HISTORICO`,
`PLOTSIM_CONFIG`. Filas só `PlotSim — Uso *`, criadas e removidas pela janela do PlotSim. Não
toque em `IMP1`, `IMP2` nem nos processos do usuário. Nenhuma impressora real. **Capturas só
por CDP: nunca capture a tela do Windows nem use SendKeys.** Permissão negada: relate, sem
contornar. Encerre só PIDs que você abriu.

Os dois revisores leram o delta `d85d143..1fd312a`. O jurídico aprovou.

## A. Código (obrigatório)

1. **`null` é valor legítimo do plano, não ausência.** Em `ui/src/face-do-envio.ts:186`,
   `frente?.trecho ?? trechoDoEnvio(...)` troca o trabalho inteiro pelo trecho da aba quando o
   enviado era o documento inteiro (`trecho: null`); o mesmo em `ui/src/App.tsx:520,522` para
   `trecho` e `papel`. Use checagem explícita, como já existe em `App.tsx:526`:
   `frente === null ? trechoDoEnvio(lerIntervaloDaAba(aba)) : frente.trecho` e
   `plano === null ? trecho : plano.trecho` / `plano === null ? papel : plano.papel`. Faça o
   mesmo com `copias`: nada cai para a aba quando o cartão em espera trouxer o número; se não
   trouxer, use 1, não a aba.
2. **Teste do caso nulo + falha prévia.** Em `ui/src/verso-plano.test.ts`, caso com
   `frenteEnviada {trecho: null, papel: null}` e a aba alterada para páginas 9–16, A5 e 2
   cópias: o plano e o diálogo continuam com o documento inteiro, o papel do envio e as cópias
   do trabalho; não pode aparecer "9 a 16". Registre em
   `%LOCALAPPDATA%\plots-build\uso-real\saidas\falha-a1b.log` a execução desse teste contra
   `1fd312a` (falhando) e depois passando.

## B. Interface (obrigatório)

3. **640×420: o título do aviso não pode passar por baixo do ×.** Em
   `ui/src/components/AvisosDoTopo.module.css`, ou permita quebra em duas linhas no `.titulo`
   (`flex-wrap: wrap`), ou deixe o `.estado` ceder (`flex: 0 1 auto; min-width: 0;
   overflow: hidden; text-overflow: ellipsis; white-space: nowrap`), mantendo 8 px livres antes
   do `.fechar` (28×28, `flex-shrink: 0`). Critério: com documento longo + `PlotSim — Uso L` +
   "Pronto para o próximo trecho", em 640×420, `titulo.scrollWidth <= titulo.clientWidth` e
   nenhum glifo sob o ×. Meça por CDP e salve em `saidas/`.
4. **Largura da faixa medida, não lida no CSS.** `min-width: max-content` anula o
   `max-width: 60%` quando o nome da fila é longo. Ajuste (por exemplo `min-width: 0` com
   `width: max-content` + `max-width: 60%`). Salve em `saidas/` a medição `fila.scrollWidth <=
   fila.clientWidth` e `documento.clientWidth >= 48`, em 1280 e 640, para `Uso L`, `Uso J` e um
   nome de fila com mais de 60% da faixa. Recapture o 640 com esse nome longo, mostrando a fila
   com reticências e o × livre.
5. **Captura estável do "Mais opções" em 640.** Substitua `r6-640x420-mais-salvar.png`, que
   pegou um quadro de rolagem instável, por um quadro assentado com "Salvar seleção como
   grupo…", "Ajustar ao papel" e "Folha de controle" e o botão "Continuar frentes em 2" visível.
   Se os três não couberem juntos, faça a rolagem parar no ponto em que os três apareçam e diga
   a altura disponível; não basta a sequência de três quadros.

## C. Miúdos adotados

6. `ui/src/i18n.ts:1348` e o teste: "Nothing was cancelled." → "Nothing was canceled.",
   alinhado ao restante do inglês americano.
7. `README.md:27`: sem repetir "registro" — "Quando um trabalho sai da fila, o PlotS procura no
   registro de impressão do Windows, quando consegue lê-lo, se houve falha naquela fila."
8. Comentário de `.documento` em `AvisosDoTopo.module.css`: o fator 40 não existe mais.
9. Apague `%LOCALAPPDATA%\plots-build\uso-real\saidas\ci.sh`, resto de outro worktree; o script
   deste lote é o `ci6.sh`.

## D. Fechamento

- `cargo fmt`, wip commit com árvore limpa.
- CI em log único `saidas/ci.log`, com os mesmos passos do lote, terminando em `CI_SUCCESS:
  cargo test --workspace` e `CI_EXIT 0` (inclua `cargo clippy -p plots-app
  --no-default-features --all-targets -- -D warnings` e `cargo test -p plots-app
  --no-default-features`).
- Não precisa repetir a suíte rápida: não há mudança em Rust.
- Apague `ui/node_modules`, `ui/dist`, `PlotSim/app/ui/node_modules`,
  `PlotSim/app/ui/dist`. Remova filas `PlotSim — Uso *` que você criar e encerre seus PIDs.
- Resposta final: item a item, causa, teste com falha prévia (caminho), medições salvas,
  capturas lidas e não lidas, linha do CI, filas/processos e o que não foi verificado.
