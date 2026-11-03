# Opções de envio e histórico

**Estado:** Em andamento, em revisão

## Objetivo

Dar as opções finais de impressão: conferir o maço, imprimir em outro papel, lidar com driver que inverte a ordem e ver o que já foi impresso.

## Comportamento

- **Folha de controle:** opção por aba, desligada por padrão. Um número discreto no canto do verso, para conferir se o maço entrou certo.
- **Ajustar ao papel:** opção por aba, desligada por padrão. Imprime o documento no papel escolhido, por exemplo Carta em A4, em escala proporcional e centralizada, sem cortar. A confirmação diz a escala.
- **Driver que inverte a ordem:** o PlotS não consegue detectar isso em todo driver, porque a opção costuma ser privada de cada fabricante. Por isso há duas partes: um aviso em texto antes do envio manual e uma opção no perfil da impressora, "O driver inverte a ordem", com que o PlotS compensa a ordem enviada para o maço sair certo mesmo assim.
- **Histórico:** lista local do que foi impresso: documento, data, impressoras, páginas, resultado e tempo de cada impressora. Fica só no computador, com limite de tamanho, e pode ser apagado inteiro.
- **Tempo de impressão:** conta só o tempo em que a impressora trabalhou, sem a espera da pessoa na recolocação. A velocidade de cada fila sai do histórico, com pelo menos 10 folhas medidas.
- **Aviso de velocidade:** com duas ou mais impressoras marcadas e dados de todas, se a diferença passar de 25%, a confirmação avisa qual termina antes. É só aviso: os lotes continuam repartidos em rodízio, então as duas gastam quase o mesmo papel. O conselho de deixar mais papel volta com a divisão por velocidade (`15-multiplas-impressoras`). Decisão do usuário em 2026-11-03.
- **Guardar histórico:** opção ligada por padrão. Desligada, nada novo é registrado, e a lista existente fica até ser apagada.

## Plano de implementação

- [ ] Folha de controle
- [ ] Ajustar ao papel
- [ ] Aviso e compensação de driver que inverte a ordem
- [ ] Histórico

## Pronto quando

Nas simuladas, cada opção muda o que sai como descrito, e ligada ou desligada não altera a ordem das folhas.

## Depende de

`02-geracao-pdf`, `05-tamanhos-de-papel`, `08-impressao-windows`, `13-fluxo-e-recolocacao`, `17-acabamento`.
