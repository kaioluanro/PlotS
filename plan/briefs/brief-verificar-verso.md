# verificar "Virar folha" e "Recolocar" no PlotSim

Pedido do usuário em 2026-11-04, depois de ver a pilha em branco e o PlotSim dizendo
"Aguardando verso": verificar se é defeito ou só a diferença entre **Virar folha** (girar a
visualização) e **Recolocar** (aplicar o gesto e imprimir o verso). Ele confirmou que usou
"Virar folha".

Leia `E:\PlotS\AGENTS.md`, `.claude/skills/craft-team/SKILL.md` e
`.claude/agents/plots-debugger.md` antes de agir. **Não altere código.** Este é um teste de
observação; a correção, se necessária, vem depois.

**Graphify:** antes do primeiro grep ou leitura de código, `graphify query "<pergunta>"
--graph E:/PlotS/graphify-out/graph.json`.

## Ambiente

- PlotSim alpha.2: `%LOCALAPPDATA%\plots-build\plotsim-alpha2\PlotSim\plotsim-app.exe`.
- PlotS alpha.2: `%LOCALAPPDATA%\plots-build\teste-usuario\PlotS.exe`.
- Portas CDP 9370 (PlotS) e 9371 (PlotSim); `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` e
  `WEBVIEW2_USER_DATA_FOLDER` próprios.
- Dados isolados: `%LOCALAPPDATA%\plots-build\verif-verso\dados\` via `PLOTS_PERFIS`,
  `PLOTS_HISTORICO`, `PLOTSIM_CONFIG`; `PLOTS_PDFIUM_DIR` compartilhado.
- PDF: `%LOCALAPPDATA%\plots-build\plotsim-alpha2\documentos\teste-5-paginas.pdf` (tem a marca
  visual). Não use o PDF assinado do usuário.
- Fila nova: `PlotSim — Uso V`, jato, criada e removida pela janela do PlotSim. Não toque em
  `IMP1`, `IMP2` nem em nada do usuário.
- Capturas em `%LOCALAPPDATA%\plots-build\verif-verso\capturas\`, prefixo `vv-`.

## Roteiro

1. Abra o PlotSim isolado, crie `Uso V` (jato). Abra o PlotS isolado com o PDF.
2. Marque a fila, modelo jato, Frente, imprimir e confirmar. Espere as 5 folhas no PlotSim.
3. **Reproduza o que o usuário viu:** clique em "Virar folha". Capture: a pilha mostra o lado
   em branco, o estado continua "Aguardando verso" e o rodapé não mudou. Registre os textos
   exatos da tela antes e depois.
4. Mande o verso pelo rodapé do PlotS e confirme. **Sem clicar em Recolocar ainda**, espere o
   verso chegar (confira o histórico do PlotSim e o `saida.pdf` da porta; anote tamanho e hora).
   Capture de novo: a tela ainda diz "Aguardando verso" e a pilha continua mostrando o lado em
   branco?
5. Agora clique em "Recolocar" e escolha o gesto que o cartão/confirmação do PlotS mandar.
   Capture: o verso é impresso e as folhas ficam "Certa" (PDF com marca), com o veredito.
6. Diga, com evidência:
   - o verso chegou antes do Recolocar? (arquivo no histórico e hora)
   - a tela distingue "verso ainda não chegou" de "verso chegou, falta escolher o gesto"?
   - "Virar folha" muda alguma coisa no estado além da visualização?
   - depois do Recolocar, o verso sai e confere?
7. Proponha, sem implementar, o texto ou estado que evitaria a confusão (por exemplo, estado
   "Verso recebido: escolha o gesto em Recolocar" e um rótulo que diga que "Virar folha" só
   olha o outro lado).

## Final

- Remova a fila `Uso V` pela janela e apague os dados do teste; confirme que só restam as filas
  do usuário.
- Feche os PIDs que você abriu (só os seus). Nada de captura da tela do Windows ou SendKeys.
- Relatório: passo a passo com os textos exatos, capturas lidas, resposta às perguntas acima e
  a proposta de texto. Se encontrar um defeito de verdade, diga o passo exato para reproduzir.
