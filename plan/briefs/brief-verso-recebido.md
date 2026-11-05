# PlotSim: dizer quando o verso chegou e quando falta o gesto

Pedido do usuário em 2026-11-04. Ele clicou em "Virar folha" e viu a pilha em branco com
"Aguardando verso", concluindo que o verso não tinha chegado. O verso estava no histórico.
A causa é de interface: "Virar folha" só muda a visualização, e o estado não distingue
"o verso ainda não chegou" de "o verso chegou e falta escolher o gesto em Recolocar".

Leia `E:\PlotS\AGENTS.md`, `.claude/team.json`, `.claude/skills/craft-team/SKILL.md` e
`.claude/agents/plots-implementer.md` antes de agir; siga o método do papel. Não faça push nem
squash. Trabalhe no worktree isolado em que você foi iniciado.

**Graphify:** antes do primeiro grep ou leitura de código, `graphify query "<pergunta>"
--graph E:/PlotS/graphify-out/graph.json`. Ao terminar, `graphify update .` no worktree.

Base: `master` `e6c1883`. Build em `%LOCALAPPDATA%\plots-build\verso-recebido\target`, com
`CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, `-j 4`; CDP 9372 (PlotSim). Rode `npm ci` em
`PlotSim/app/ui/`. `PLOTS_PDFIUM_DIR=%LOCALAPPDATA%\plots-build\pdfium\chromium-8125\bin`.

**Capturas:** só por CDP, prefixo `vr-`, em `%LOCALAPPDATA%\plots-build\verso-recebido\capturas`.
Nunca capture a tela do Windows nem use SendKeys. Filas só `PlotSim — Uso *`, pela janela,
removidas ao fim. Não toque em `IMP1`, `IMP2` nem em processos do usuário.

## Comportamento

1. **Estado novo na lista de impressoras: "Verso recebido".**
   - No backend, quando a bandeja está em `AguardandoGesto` e há passada pendente
     (`pendentes` não vazio), o estado da fila é `verso_recebido`.
   - Quando está em `AguardandoGesto` sem passada pendente, continua `aguardando_verso`.
   - Quando está em `NaEntrada` (gesto já aplicado) e ainda não imprimiu, continua
     `aguardando_verso`.
   - Aviso e "Recebendo" mantêm a precedência que já existe.
   - pt-BR: "Verso recebido"; en: "Back side received".
2. **Dica no centro, acima da pilha, quando `verso_recebido`:**
   - pt-BR: "O verso já chegou. Escolha o gesto em Recolocar para imprimir o outro lado."
   - en: "The back side has arrived. Choose the gesture in Reload to print the other side."
   - Quando `aguardando_verso` sem verso recebido, a dica (se houver) diz que falta o verso:
     pt-BR "Aguardando o verso chegar do PlotS." / en "Waiting for the back side from PlotS."
     Não invente firula: uma linha, sem caixa nova.
3. **"Virar folha" vira "Ver o outro lado"**, pt-BR, e "See the other side", en.
   - `rodape.virar` e `rodape.virar.folha` mudam de texto; o comportamento não muda.
   - `title`/`aria-description`: pt-BR "Só muda o que aparece na tela. O gesto é aplicado em
     Recolocar." / en "Only changes what the screen shows. The gesture is applied in Reload."
   - O botão continua desabilitado/habilitado pelas mesmas regras de hoje.
4. **A confirmação do gesto** continua igual. Não mexa no modelo físico.

## Testes

- Rust: teste de `estado_da_bandeja` cobrindo os três casos (sem pendente, com pendente, na
  entrada), com falha prévia registrada.
- UI: teste de `tela.ts` para a escolha do texto/estado e da dica; teste de `i18n` com as
  chaves nos dois idiomas.
- Sem mudança de física; a suíte do PlotSim não precisa rodar, mas o `cargo test --workspace`
  sim.

## Uso real e capturas

Pela janela, com `teste-5-paginas.pdf` (tem marca) numa fila `PlotSim — Uso VR`:
1. frente enviada; captura `vr-1-aguardando` — lista em "Aguardando verso", dica de espera.
2. verso enviado pelo PlotS, sem clicar em Recolocar; captura `vr-2-verso-recebido` — lista em
   "Verso recebido" e a dica do gesto.
3. "Ver o outro lado" clicado; captura `vr-3-outro-lado` — a folha mostra o lado em branco, e a
   lista continua "Verso recebido" (o botão não aplica gesto).
4. "Recolocar" com o gesto; captura `vr-4-recolocado` — verso impresso e veredito "Certa".
Leia cada imagem. Se o resultado não for esse, pare e relate; não amenize.

## Final

- CI em um log único `saidas/ci.log`: `cargo fmt --all --check`, `cargo test --workspace
  --locked -j 4`, em `PlotSim/app/ui/` `npm ci`, `npm test`, `npm run build`, 0 ocorrências de
  `__plotsimE2e` no `dist`, terminando em `CI_EXIT <código>`.
- Wip commit com árvore limpa; `node_modules` e `dist` apagados. Filas removidas e PIDs seus
  encerrados.
- Não mexa em `plan/`.
- Resposta final: item a item, teste com falha prévia (caminho do log), capturas lidas, linha do
  CI, e o que não foi verificado.
