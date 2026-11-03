# Todas as frentes primeiro, depois os versos

**Estado:** Em andamento

## Objetivo

Pedido do usuário em 2026-11-03: imprimir primeiro só as frentes de vários lotes, e deixar os versos para depois. Hoje cada lote exige fazer frente e verso antes de passar ao próximo.

## Comportamento

- Nas opções da aba, ao lado de "Folhas por lote", uma escolha: **Lote a lote** (como hoje, padrão) ou **Frentes primeiro**. Só aparece para o fluxo manual, e não com a divisão, que reparte lotes.
- **Lote a lote:** para a cada "Folhas por lote", porque o verso daquele lote precisa ser feito antes do próximo. O número existe por causa da bandeja, do manuseio e do prejuízo de um erro.
- **Frentes primeiro:** decisão do usuário em 2026-11-03: a impressão não para em 10. Todas as frentes saem em um trabalho contínuo, sem clique no meio, e "Folhas por lote" não se aplica. Tecnicamente, é um lote só por trecho de mesmo papel.
- **Depois das frentes:** o cartão espera sem prazo e oferece "Imprimir versos", que envia um trabalho só, com a frase e a figura do gesto.
- **Bandeja menor que o maço:** na frente, o aviso de papel da fila manda colocar mais folhas em branco. No verso, a pessoa coloca a parte de cima e completa com o resto quando a entrada esvazia, sem mudar a ordem.
- **Papéis mistos:** cada trecho de mesmo papel é um lote; a troca de papel para o envio e é dita antes.
- **Reimprimir** refaz o trecho inteiro, com confirmação dizendo quantas folhas.
- **Fechar o PlotS durante a espera:** a confirmação de fechamento diz que os versos ainda não foram impressos. Nesta primeira entrega, o estado fica só enquanto o programa está aberto.
- Vale com intervalo, ajustar ao papel, folha de controle, compensação de driver e espelho. O automático não usa: já não há espera entre as faces.

## Plano de implementação

- [ ] Máquina de estados com as duas fases
- [ ] Opção na aba e confirmação dizendo o que vai acontecer
- [ ] Cartões das duas fases
- [ ] Simuladas e PlotSim conferem o maço final

## Pronto quando

Nas simuladas, um documento de 24 páginas com 10 no campo de lote dá um único trabalho de frentes, sem clique, os versos saem quando a pessoa pede, e o maço final sai certo.

## Depende de

`13-fluxo-e-recolocacao`, `15-multiplas-impressoras`, `22-opcoes-de-envio-e-historico`, `23-plotsim`.
