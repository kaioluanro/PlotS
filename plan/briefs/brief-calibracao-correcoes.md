# calibracao-na-janela: correções das revisões e rebase

Responda em português brasileiro. Trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-a3ae2fc018eebc568`, commit `3e8a0b6`. Não crie outro worktree. Leia `plan/briefs/brief-calibracao-na-janela.md` e as regras comuns de `plan/briefs/coordenacao-paralela-2.md`. O agente anterior foi encerrado sem relatório: leia o código. Build, saídas e cenas em `%LOCALAPPDATA%/plots-build/calibracao-na-janela`, porta CDP 9349. `ui/node_modules` foi apagado: rode `npm ci`. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`. Limite de tempo em todo comando de cena; não fique esperando processo em segundo plano sem prazo.

**Primeiro, rebase sobre o `master` atual.** Ele tem a laser com verso girado e a curva em U no simulador, e as filas do PlotSim. Conflitos previstos: nenhum de texto em `formato.rs` e `i18n.ts`; os textos e as rodadas das cenas de simulada mudam. Outros agentes: suíte e janela do PlotSim, imagem e ícone.

## Obrigatório

1. **Ordem das perguntas** (`perguntasDaRodada`, `ui/src/calibracao.ts`): a pergunta do maço vem antes de mandar pegar folhas, porque hoje a pessoa já desfez o maço. Manual: lado, leitura, [primeira], par, setas. Automática: leitura, par, setas. O Rust não muda. Atualize os testes.
2. `calibracao.pergunta.lado`: "Sem mudar a ordem das folhas, olhe os dois lados do maço. Onde saiu a segunda impressão?" / "Without changing the order of the sheets, look at both sides of the stack. Where did the second print land?"
3. `calibracao.pergunta.leitura`: "Ainda sem mudar a ordem, olhe o número de cima. Depois vire o maço inteiro e olhe de novo. Quando o 1 aparece por cima?" / "Still without changing the order, look at the top number. Then flip the whole stack and look again. When is the 1 on top?" Figuras e rótulos ficam.
4. **Perfil calibrado usado no envio:** hoje nenhum teste falha sem isso. Teste `vira_para_ler("fila:Sala", ...)` em `src-tauri/src/comandos_trabalho.rs` com `le_virando` verdadeiro e falso. Extraia a escolha de `recolocar` e `vira_para_ler` de `preparar_fila` (`src-tauri/src/trabalhos.rs`) para uma função pura, e teste com e sem calibração. Registre a falha prévia.
5. **Folga de rodadas:** 44% dos casos usam as 3, sem margem para um engano da pessoa. Passe `RODADAS_MAXIMAS` a **5**, em `plots-core/src/calibracao.rs` e `ui/src/calibracao.ts`, com o papel gasto dito antes de imprimir. O teste de convergência continua exigindo no máximo 3 rodadas com respostas certas, e ganha um caso: uma resposta errada em uma rodada ainda converge dentro de 5.

## Também, de uma vez

6. **Frases que não afirmam como fato** (jurídico):
   - `calibracao.certo.titulo`: "Pelas suas respostas, as folhas saíram certas." / "From your answers, the sheets came out right."
   - `calibracao.conferida.certa`: "Pelas suas respostas, as duas faces viram pela borda longa e o maço fica em ordem." / "From your answers, both sides flip over the long edge and the stack is in order."
   - `calibracao.conferida.borda`: "Pelas setas, as faces não parecem virar pela borda longa. Confira as preferências da impressora, ou escolha jato de tinta ou laser." / "Going by the arrows, the sides do not seem to flip over the long edge. Check the printer preferences, or choose inkjet or laser."
   - `calibracao.mudou.driverCompensa`: "Pelo que você viu, o driver parece inverter a ordem: o PlotS passa a compensar." / "From what you saw, the driver seems to reverse the order: PlotS now compensates."
   - `calibracao.mudou.driverNaoCompensa`: "Pelo que você viu, o driver não parece inverter a ordem: o PlotS deixa de compensar." / "From what you saw, the driver does not seem to reverse the order: PlotS stops compensating."
7. **Recusa da fila que pede arquivo:** sem "Tentar de novo", com "Fechar" como principal, e "Esta fila não serve para calibrar. Escolha uma impressora de papel." / "This queue cannot be calibrated. Choose a paper printer."
8. **Pergunta da primeira folha:** "Na segunda impressão, qual folha saiu primeiro: a que tem o 1 ou a que tem o 3?" / "In the second print, which sheet came out first: the one with the 1 or the one with the 3?", com "A do 1", "A do 3" e "Não reparei".
9. **Tela "certo":** abre com "Daqui em diante, ao imprimir o verso:" / "From now on, when printing the back:", para não parecer mandar agir agora.
10. **`CalibracaoDialog.tsx` (perto das linhas 157 e 167):** separe "ocupada" de falha de envio e de falha no perfil inicial; hoje toda falha diz que a impressora já tem um trabalho.
11. **Frase das setas:** "Olhe a seta. Vire a folha com o 1 pela borda longa, como página de livro, e olhe a seta do outro lado. Para onde as duas apontam?" / "Look at the arrow. Flip the sheet with the 1 over the long edge, like a book page, and look at the arrow on the other side. Where do the two point?" Inglês de "não encaixou": "could not be calibrated in {n} rounds".
12. **Foco e leitor de tela:** rótulo no corpo e anúncio de passo por `aria-live`. **Esc e Desistir** com confirmação na tela "certo", depois de folhas gastas.
13. **Trocar a compensação do driver em fila calibrada:** a linha avisa que a calibração pode deixar de valer, sem apagá-la.
14. **Teste do componente** com `gravarCalibracao` simulado: desistir não grava.

## Final

Rode as cenas de novo, com a laser nova: simuladas com perfil errado de propósito até dar certo. Recapture leitura, par, setas, primeira folha, recolocar e setas no perfil laser, recusa da fila, nos dois temas, em inglês, na janela mínima e em cores forçadas. Leia cada imagem. CI completo em um único log, incluindo `ui/` e `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR`, autoverificação com a base do rebase, `git status` limpo com novo wip, apague `ui/node_modules` e `ui/dist`. Relatório em `%LOCALAPPDATA%/plots-build/calibracao-na-janela/relatorio-correcoes.md`, curto, com a mesma mensagem na resposta final: item a item, testes com grupo de falha prévia, linha do CI, tabela de convergência, capturas com caminho e o que não foi verificado.
