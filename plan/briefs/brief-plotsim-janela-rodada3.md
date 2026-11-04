# plotsim-janela: correções das revisões

Responda em português brasileiro. Worktree existente `E:\PlotS\.claude\worktrees\agent-ae5845800b5cc2708`, sem criar outro. Build e saídas em `%LOCALAPPDATA%/plots-build/plotsim-janela`, com `CARGO_TARGET_DIR` ali, `CARGO_INCREMENTAL=0`, `-j 4`; porta CDP 9350. Rode `npm ci` em `ui/` e em `PlotSim/app/ui/`. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`. Especificação: `PlotSim/AGENTS.md`, `plan/23-plotsim.md`. Limite de tempo em todo comando; não espere processo em segundo plano sem prazo. Só crie ou remova filas `PlotSim — Cena *`, com porta em `C:\ProgramData\PlotSim\`.

**Primeiro, `git rebase master`.** O `master` tem a calibração, sem relação com este lote. Conflito previsto: nenhum ou só `Cargo.toml`.

Os textos são definitivos. **Faça todos os itens, um por um, e abra o aplicativo real.** Só `PlotSim/app/` e o item 2 fora dele.

## Obrigatório

1. **Veredito honesto (o principal):** hoje `Bandeja::conferir()` (`bandeja.rs`, perto da linha 508) testa como saiu e virando o maço e fica com a forma de menos motivos, e `canal.rs` joga fora a `leitura` de `Mensagem::Fim`. Assim, o PlotS pode mandar "pegue como saiu" quando era preciso virar, e a janela diz "Certa". Corrija:
   - Guarde por fila a `leitura` do `Fim` e passe à bandeja. `ComoSaiu` e `Virando` conferem só nessa forma. `SemInstrucao` usa a regra de `PlotSim/plotsim-suite/src/bancada.rs`, perto da linha 190.
   - A escolha pela forma de menos motivos fica só no uso manual, sem canal, e a tela diz: "Sem instrução do PlotS: melhor forma de pegar." / "No instruction from PlotS: best way to hold the stack."
   - Teste com falha prévia: gesto que só fecha virando o maço com `Fim(ComoSaiu)` dá "Errada".
   - Por `Fim` chegar depois da passada 2, o veredito é refeito quando ele chega, e a tela atualiza.

2. **Verso em branco não tem orientação** (`PlotSim/plotsim-physics/src/conferencia.rs`, perto da linha 307): em folha com verso em branco, não acuse "verso de cabeça para baixo". Decisão minha: não há nada visível para estar de cabeça para baixo. A folha continua acusável por qualquer outro motivo. Teste com falha prévia, e todos os testes do `plotsim-physics` seguem passando. Relate qualquer teste antigo que dependia disso.

3. **Limpar histórico** (`estado.rs`, `limpar_historico`) também apaga o `saida.pdf` da fila. Se ele estiver preso pelo spooler, a tela avisa, em vez de se calar. `FilaTela` ganha o caminho da porta, e `centro.pastaDosPdfs` passa a: "PDFs recebidos ficam em {historico}. O último trabalho também fica em {porta}, que outras contas deste computador podem ler." / "Received PDFs stay in {historico}. The last job also stays in {porta}, which other accounts on this computer can read." Confira que esse arquivo some na tela e no disco, em um teste com falha prévia.

4. **Tirar o ✓ de seleção:** saem `Folha.tsx`, perto da linha 153, e `.marcaSelecao`; `aria-pressed` fica. Em `Folha.module.css`, `.selecionada` ganha `box-shadow: inset 0 0 0 1px var(--accent)`. Em `App.tsx`, o botão diz a folha, com a chave nova `rodape.virar.folha`: "Virar folha {n}" / "Flip sheet {n}". Sem seleção, fica `rodape.virar`.

5. **Desenho a caminho:** em `Folha.tsx`, um único indicador por papel, enquanto qualquer camada visível estiver pendente, inclusive com a frente pronta e só o verso faltando. Aparece após 150 ms, para não piscar. Centralizado no papel, `LoaderCircle` de 12 px e texto de 11 px em `--papel-texto`, fundo `--papel`, padding 2px 6px, radius 2px. `folha.desenhando`: "Desenhando…" / "Drawing…". `aria-busy` no botão da folha, sem região viva. Página em branco não mostra indicador.

6. **Falha do verso:** mostre-a (hoje `falhou && !camada.atras`, perto da linha 85, esconde), com o estilo de `.semDesenho`. `folha.semDesenhoVerso`: "Verso sem desenho" / "Back not drawn". `folha.semDesenho` fica para a frente.

7. **Canal fechado:** `main.rs`, perto da linha 225, guarda o resultado de `canal::iniciar` no `Estado`, e a tela o consulta. Em `App.tsx`, `CaixaDeErro` no topo de `.rolagem`, em qualquer fila e estado, sem botão de fechar; com o canal aberto, nada. `centro.canalFechado`: "O canal com o PlotS não abriu. O gesto do modo de teste não chega aqui; marque-o à mão em Recolocar. Se houver outro PlotSim aberto, feche-o e abra este de novo." / "The channel to PlotS did not open. The test-mode gesture will not arrive here; mark it by hand in Reload. If another PlotSim is open, close it and open this one again." Verifique de verdade: abra duas janelas, ou segure o pipe com a suíte, e registre o que acontece.

8. **Textos:**
   - `centro.pastaDosPdfs`, se não usar o item 3, e `centro.semEnvio`: "PDFs recebidos ficam em" e "O PlotSim não envia nada pela rede." / "Received PDFs stay in" e "PlotSim sends nothing over the network."
   - `aviso.pdf_grande`: "O PDF recebido passa do limite aceito e não entrou no histórico." / "The received PDF is over the accepted limit and did not go into the history."
   - `erro.windows_recusou`: "O Windows recusou o pedido. Atualize a lista para ver como a fila ficou." / "Windows refused the request. Refresh the list to see how the queue stands."
   - `erro.sem_permissao`: tire "Nada foi alterado." / "Nothing was changed."
   - Diálogo de criar: "Ela grava PDF e guarda uma cópia de cada trabalho; nenhum papel é usado." / "It writes a PDF and keeps a copy of each job; no paper is used."
   - Perto do resumo: "Conferido pelo modelo do PlotSim; não substitui o teste em papel." / "Checked by the PlotSim model; it does not replace the paper test."

## Também

9. `recolocar` e `limpar_bandeja` em `main.rs` passam a `async` com `fora_da_tela`.
10. Ao remover a fila ou limpar o histórico, pare o ouvinte (`join` com prazo) antes de apagar, para não gravar PDF depois da limpeza.
11. Em `scripts/avisos.test.mjs`, inclua `plotsim-app` na lista de excluídos.
12. Rodapé: `CircleCheck` ou `CircleX` de 14 px ao lado de "n de m". Em `.folha` errada, borda de 1 px `--status-offline`, sem competir com a seleção.

## Uso real e capturas

Repita pelo spooler real, nos dois modelos: `plots imprimir --teste` com PDF da suíte, 3 de 3 corretas. Provoque o item 1 de verdade, se der, ou registre por que só há teste. Capturas por CDP em `saidas/shots`, prefixo `r3-`, lendo cada uma, em claro e escuro, pt e en, incluindo 640×420, das telas dos itens 1 a 7: desenhando, verso sem desenho, canal fechado, seleção certa e errada, foco de teclado em folha selecionada, errada em cores forçadas. Regrave também, com o documento atual, o que está em `shots/antigas/`: recebendo, sem marca, par à frente, gesto errado na entrada, hover da folha, foco no rodapé, gesto vazio e recusado, PDF ilegível, sem permissão, nome recusado, erro de fila, sem modo, fora do Windows, limpar histórico, remover.

## Final

CI em um único log `saidas/ci.log`: `cargo fmt --all --check`, `cargo test --workspace --locked -j 4`, `cargo test -p plotsim-app -- --ignored`, `ui/` e `PlotSim/app/ui/` com `npm ci`, `npm test`, `npm run build`, 0 `__plotsE2e` e 0 `__plotsimE2e`, `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR="%LOCALAPPDATA%/plots-build/pdfium/chromium-8125/bin"`, terminando com `CI_EXIT <código>`. Autoverificação com a base do rebase. `git status` limpo com novo wip, apague `node_modules` e `dist` das duas interfaces. Remova suas filas. **Não rode a matriz inteira da suíte:** fica comigo.

Resposta final, curta, mesmo que falte algo: item a item, testes com grupo de falha prévia, linha do CI, capturas lidas e não lidas, com caminho, e o que não foi verificado.
