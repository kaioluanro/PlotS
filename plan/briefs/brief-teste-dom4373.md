# teste de impressão com documento real

Pedido do usuário em 2026-11-04: usar o PlotS e o PlotSim como um usuário, com o PDF real
`C:\Users\Kaio Luan\Downloads\DOM4373-04112026-ASSINADO.pdf` (4,05 MB, assinado, sem a marca
visual do PlotSim). Nada vai para impressora real.

Leia `E:\PlotS\AGENTS.md`, `.claude/team.json`, `.claude/skills/craft-team/SKILL.md` e
`.claude/agents/plots-debugger.md` antes de agir. Siga o método do papel: reproduzir, observar,
não corrigir nada sem ordem. Este teste é de uso e diagnóstico; **não altere código**.

**Graphify:** antes do primeiro grep ou leitura de código, `graphify query "<pergunta>"
--graph E:/PlotS/graphify-out/graph.json`.

## Ambiente

- PlotSim: `C:\Users\Kaio Luan\AppData\Local\plots-build\plotsim-alpha2\PlotSim\plotsim-app.exe`
  (recém-compilado, com `pdfium.dll` ao lado).
- PlotS: `%LOCALAPPDATA%\plots-build\teste-usuario\target\release\plots-app.exe` (alpha.2).
- Portas CDP: 9360 (PlotS) e 9361 (PlotSim), com
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<porta>` e
  `WEBVIEW2_USER_DATA_FOLDER` separado.
- Dados isolados: `PLOTS_PERFIS`, `PLOTS_HISTORICO`, `PLOTSIM_CONFIG` e pastas próprias em
  `%LOCALAPPDATA%\plots-build\teste-usuario\dados\`. `PLOTS_PDFIUM_DIR` aponta para
  `%LOCALAPPDATA%\plots-build\pdfium\chromium-8125\bin`.
- Não toque nas filas `PlotSim - IMP1` e `PlotSim - IMP2` nem em processos do usuário.
- Filas novas só com o prefixo `PlotSim — Uso `, criadas e removidas pela janela do PlotSim.

## Privacidade do documento

O PDF é um documento assinado do usuário. Ele será guardado pelo PlotSim em
`%APPDATA%\plotsim\historico` (ou no `PLOTSIM_CONFIG` isolado) e na pasta da porta em
`C:\ProgramData\PlotSim\<fila>`. Ao final:
- use "Limpar histórico" na janela do PlotSim e confirme que o PDF saiu do histórico;
- remova a fila pela janela e confirme que a pasta da porta sumiu;
- apague o perfil/histórico isolado do teste e a pasta de dados;
- **não copie o PDF para o repositório** nem para nenhuma pasta de build além da que o teste
  exigir; não o abra em nenhum outro programa.

## Roteiro (tudo por clique/tecla real via CDP)

1. Abra o PlotSim e crie "Uso DOM" no modo jato de tinta, pela janela, com "Nova impressora
   virtual".
2. Abra o PlotS com o PDF. O diálogo nativo "Abrir" não é acionável por CDP; abra o executável
   com o caminho do PDF como argumento e diga isso no relatório. Todo o resto é por clique.
3. Confira o que o PlotS mostra: número de páginas, folhas, papel. Anote o que a janela diz.
4. Marque a fila, escolha o modelo jato, deixe Frente e clique em imprimir frentes; confirme na
   tela de confirmação.
5. Observe o PlotSim: as folhas da passada 1 devem chegar. Como o PDF não tem a marca visual,
   espere "Sem marca legível" em cada folha, sem palpite de veredito. Anote quantas folhas
   chegaram e se as páginas batem com o PlotS.
6. Marque no PlotSim o gesto de recolocar que o cartão/confirmação do PlotS manda. Depois, no
   PlotS, escolha Verso e mande imprimir, passando pela confirmação.
7. Confira o resultado no PlotSim: as folhas devem receber o verso e continuar "Sem marca
   legível"; o rodapé não pode dar "Certa" nem "Errada". Anote a contagem.
8. Repita uma vez com 2 cópias, frente e verso, se o documento tiver mais de uma página, e diga
   o que o PlotSim mostra (marcas "Folha N" contínuas, contagem).
9. Teste um caso de uso comum: cancelar um envio curto pelo painel de avisos e conferir o
   estado final. Se não der tempo de cancelar, diga.

## Capturas e provas

Capturas só por CDP, prefixo `dom-`, em
`%LOCALAPPDATA%\plots-build\teste-usuario\capturas\`: PlotS na confirmação da frente, frentes
concluídas, confirmação do verso, verso concluído; PlotSim com a passada 1, com o gesto e com a
passada 2; o painel de avisos. Leia cada imagem e diga o que viu.

Não capture a tela do Windows, não use SendKeys, não use gancho de verificação (`__plotsE2e`).
Nada de comando de terminal para imprimir ou configurar: terminal só para abrir os programas e
observar.

## Final

Relatório: passo a passo do que apareceu, páginas e folhas em cada etapa, defasagens entre
PlotS e PlotSim, qualquer defeito com passos para reproduzir e gravidade, capturas lidas e não
lidas, e a limpeza do documento real (histórico, porta, fila, dados). Se algo impedir o teste,
pare e relate o ponto exato.
