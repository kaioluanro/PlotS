# Impressão sem lote

**Estado:** Concluído no código (`18bb5cc`), sem papel

## Objetivo

Decisão do usuário em 2026-11-03: remover "Folhas por lote" da janela e deixar imprimir livremente, porque parar a cada 10 folhas não faz sentido para ele. Substitui o item 3 do fluxo do plano `13`, que previa o tamanho do lote com padrão de 10 folhas.

## Comportamento

- **Um fluxo só na janela:** todas as frentes saem em um trabalho contínuo, sem clique no meio. O cartão espera sem prazo e oferece "Imprimir versos", que envia um trabalho só, com a frase e a figura do gesto.
- **Papéis mistos:** um trecho por papel; a troca de papel para o envio e é dita antes.
- **Todas as impressoras marcadas imprimem a mesma coisa.** A divisão sai da janela, por decisão do usuário em 2026-11-03; o núcleo e o PlotSim seguem testando a repartição.
- **Escolha do que imprimir, no rodapé: Frente ou Verso**, para todas as marcadas. Só frente hoje, e só verso quando quiser, mesmo depois de fechar o programa. A escolha nunca muda sozinha, por decisão do usuário: só ele a troca. O verso sempre pede confirmação, com o gesto de cada impressora, porque o maço errado estraga as folhas.
- **Campo "Quantidade", no rodapé, por decisão do usuário em 2026-11-03:** quantas cópias do documento, de 1 a 999, com padrão 1. Com 3, as frentes do documento saem 3 vezes, em sequência e sem clique, em um maço só, e o Verso imprime as 3 também, na ordem que o gesto do modelo exige. No Verso o campo também é editável, por decisão do usuário, e já vem com o número da Frente. Editar o Verso não muda a Frente; editar a Frente leva o novo número ao Verso. Se os dois forem diferentes, a confirmação do verso diz os dois números e quantas cópias o maço deve ter. Vale para todas as marcadas, e o número nunca muda ao trocar de Frente para Verso. Cada cópia fecha com a sua folha: em total ímpar, cada uma tem o verso em branco da última folha, e as páginas nunca se juntam entre cópias. O intervalo vale para cada cópia. O PlotS manda as cópias como páginas, sem usar as cópias do driver. A confirmação do verso diz o número, para evitar maço e quantidade diferentes, e o aviso de maço maior que a bandeja conta o total de folhas.
- **Automático:** sem essa escolha, porque imprime as duas faces de uma vez. A quantidade também vale.
- **Maço maior que a bandeja:** acima de 30 folhas, a confirmação avisa. Na frente, a impressora pede papel, e a pessoa coloca mais folhas em branco. No verso, coloca a parte de cima e completa com o resto quando a entrada esvazia, sem mudar a ordem.
- **O preço da decisão:** um atolamento afeta o trecho inteiro. "Reimprimir" refaz o trecho, com confirmação dizendo quantas folhas.
- **Fechar o PlotS durante a espera:** a confirmação diz que os versos ainda não foram impressos. Depois, é só abrir o documento, marcar as impressoras e escolher Verso. O PlotS não guarda estado: quem garante que o maço é o certo é a pessoa.
- **Documento mudou:** quando a aba já imprimiu frentes, o verso compara o arquivo com o que a frente leu, e recusa antes de enviar se ele mudou. Aba sem frente segue livre. Reimprimir na simulada não pede confirmação, como no plano `13`.
- **O CLI e o núcleo continuam com lotes:** o PlotSim e a suíte dependem disso.

## Plano de implementação

- [x] Tirar "Folhas por lote" da janela (`18bb5cc`)
- [x] Um trecho por papel no envio (`18bb5cc`)
- [x] Cartões e confirmação sem a palavra lote (`18bb5cc`)
- [x] Campo "Quantidade" no rodapé, com cópias em frente, verso e automático (`18bb5cc`)
- [x] Simuladas e PlotSim conferem o maço final (`18bb5cc`). O automático com cópias só tem teste de planejamento, porque o PlotSim não modela as duas faces automáticas
- [x] Verso em espera sempre pede confirmação, com o gesto, as páginas, o papel e as cópias do trabalho enfileirado, nunca os números da aba (`7b16a4f`)
- [x] Envio simultâneo não perde trabalho no driver que falha com dois trabalhos juntos: uma fila por vez nesse driver, e o registro do Windows transforma trabalho descartado em erro com Reimprimir (`7b16a4f`)
- [x] Papel misto mantém o lado do documento; o par de tamanhos diferentes se separa com o verso em branco, avisado na análise (`7b16a4f`)

## Pronto quando

Nas simuladas, um documento de 24 páginas dá um único trabalho de frentes, sem clique, os versos saem quando a pessoa pede, e o maço final sai certo.

## Depende de

`13-fluxo-e-recolocacao`, `15-multiplas-impressoras`, `22-opcoes-de-envio-e-historico`, `23-plotsim`.
