# Modelos de impressora

**Estado:** Em andamento: jato de tinta informado pelo usuário; laser aguarda uma confirmação.

## Objetivo

Oferecer duas configurações prontas, uma para cada tipo de máquina do usuário, sem exigir calibração do zero. O produto usa nomes genéricos, sem marca. Os modelos abaixo são só referência: as máquinas do usuário são parecidas com eles, não exatamente eles.

## Dados informados pelo usuário (Oficina do Maço, 2026-11-02)

| | Jato de tinta (traseira) | Laser (gaveta) |
|---|---|---|
| Referência | HP Smart Tank 581 | Brother HL-1200 |
| Sai com a face impressa para | cima | baixo |
| A folha seguinte cai | em cima da anterior | em cima da anterior |
| A entrada puxa | a folha de cima | a folha de cima |
| Imprime na face voltada para | cima | **cima, a confirmar** |
| Gesto | vira pela borda de cima, como bloco de notas | vira pela lateral, como página de livro |
| Gira ou reordena o maço | não | não |

## Resultado esperado

- **Jato de tinta:** frente da última para a primeira (6, 4, 2); verso da última para a primeira (5, 3, 1); **verso girado 180°**; maço na entrada com a face impressa para baixo.
- **Laser:** com os dados acima, nenhuma ordem resolve. Virar deixa a face impressa para cima, e a máquina imprimiria por cima dela.

## A confirmar na laser

Manuais de modelos Brother parecidos mandam recolocar com a face a imprimir para baixo, o que indica "imprime na face voltada para baixo". O manual do HL-1200 não foi lido. Com essa resposta, o resultado seria: frente da primeira para a última, verso da primeira para a última, sem giro, maço com a face impressa para cima. O teste do usuário decide: marcar um X a lápis na face de cima de uma folha, colocar na gaveta e imprimir uma página.

## Plano de implementação

- [x] Registrar o comportamento informado
- [ ] Confirmar a face impressa da laser
- [ ] Levar os dois modelos ao simulador, em `Mecanica` e nos perfis embutidos, com a virada pela borda de cima no modelo físico, e testes contra o simulador
- [ ] Usar os dois modelos como ponto de partida ao calibrar uma fila real, e na escolha de modo da barra lateral
- [ ] Confirmar com um lote de 2 ou 3 folhas em cada impressora real

## Pronto quando

Cada impressora real entrega o maço 1|2, 3|4… com um dos dois modelos, sem folha de cabeça para baixo.

## Depende de

`01-nucleo-ordenacao`, `03-simulacao`, `04-calibracao`, `06-perfis`, `11-lateral-impressoras`.
