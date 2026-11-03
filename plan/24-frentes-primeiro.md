# Todas as frentes primeiro, depois os versos

**Estado:** Em andamento

## Objetivo

Pedido do usuário em 2026-11-03: imprimir primeiro só as frentes de vários lotes, e deixar os versos para depois. Hoje cada lote exige fazer frente e verso antes de passar ao próximo.

## Comportamento

- Nas opções da aba, ao lado de "Folhas por lote", uma escolha: **Lote a lote** (como hoje, padrão) ou **Frentes primeiro**. Só aparece para o fluxo manual.
- **Primeira fase:** o PlotS envia a frente do lote 1 e, a cada lote pronto, diz para tirar o maço e guardá-lo separado, com o número do lote e as páginas, e colocar folhas em branco. O próximo lote começa num clique, sem gesto de recolocação no meio.
- **Quando todas as frentes acabam:** o cartão diz quantos maços foram feitos e oferece "Imprimir versos" quando a pessoa quiser. Ele pode ficar esperando sem prazo.
- **Segunda fase:** lote por lote, o cartão mostra qual maço pegar, pelo número do lote e das páginas, com a frase e a figura do gesto. Ao fim, as frases de junção dizem como empilhar os maços.
- **Reimprimir** refaz só aquele lote: na primeira fase, a frente; na segunda, avisa que refazer exige a frente de novo.
- **Fechar o PlotS durante a espera:** a confirmação de fechamento já existente diz que os versos ainda não foram impressos. Nesta primeira entrega, o estado fica só enquanto o programa está aberto.
- Vale com intervalo, ajustar ao papel, folha de controle, compensação de driver, espelho e divisão. O automático não usa: já não há espera entre as faces.

## Plano de implementação

- [ ] Máquina de estados com as duas fases
- [ ] Opção na aba e confirmação dizendo o que vai acontecer
- [ ] Cartões das duas fases
- [ ] Simuladas e PlotSim conferem o maço final

## Pronto quando

Nas simuladas, com 3 lotes, todas as frentes saem sem gesto, os versos saem quando a pessoa pede, e os maços juntos formam o documento.

## Depende de

`13-fluxo-e-recolocacao`, `15-multiplas-impressoras`, `22-opcoes-de-envio-e-historico`, `23-plotsim`.
