# Modelos de impressora

**Estado:** Em andamento: impressoras virtuais e seletor prontos; falta a confirmação em papel. Impressoras virtuais (166adee); falta levar os modelos à calibração e à escolha de modo.

## Objetivo

Oferecer duas configurações prontas, uma para cada tipo de máquina do usuário, sem exigir calibração do zero. O produto usa nomes genéricos, sem marca. Os modelos abaixo são só referência: as máquinas do usuário são parecidas com eles, não exatamente eles.

## O usuário não tem as impressoras à mão

Os testes físicos ficam para quando houver impressora. Até lá, duas impressoras virtuais imitam os modelos abaixo. Elas seguem a documentação e o que o usuário informou, sem medição em papel.

## Os dois modelos

| | Jato de tinta (traseira) | Laser (gaveta) |
|---|---|---|
| Referência | HP Smart Tank 581 | Brother HL-1200 |
| Fonte | Usuário, na Oficina do Maço; manual da HP não lido | Manual da série (HL-1210W), p. 33, e usuário |
| Sai com a face impressa para | cima | baixo |
| A folha seguinte cai | em cima da anterior | em cima da anterior |
| A entrada puxa | a folha de cima | a folha de cima |
| Imprime na face voltada para | cima | cima |
| Gesto | vira pela borda de cima, como bloco de notas | **não vira**: recoloca como saiu |
| Frente | da última para a primeira (6, 4, 2) | da última para a primeira (6, 4, 2) |
| Verso | da última para a primeira (5, 3, 1), **girado 180°** | da primeira para a última (1, 3, 5), sem giro |
| Face impressa na entrada | para baixo | para baixo |
| Para ler o maço final | como saiu | vira o maço |

## De onde vem o gesto da laser

Conforme a p. 33 do manual da série HL-1210W, nesta laser o maço volta à bandeja sem ser virado. Por isso, ela imprime na face voltada para cima. Isso substitui a virada informada na Oficina do Maço, que não fechava: virar deixaria a face impressa para cima, e o verso cairia sobre ela.

## Exceção à especificação

`11-lateral-impressoras` diz que face para baixo não inverte a frente. Isso vale para uma impressão de uma face. No frente e verso desta laser, sem virar o maço, a frente precisa ser invertida para o maço final sair 1|2, 3|4, 5|6. Sem inverter, o maço sairia na ordem contrária quando virado para ler. Neste modelo, `passada1_inversa` é verdadeiro.

## Plano de implementação

- [x] Registrar o comportamento dos dois modelos
- [x] Levar os dois modelos ao simulador, em `Mecanica` e nos perfis embutidos, com a virada pela borda de cima e o gesto sem virar, e testes contra o simulador (166adee)
- [x] Ajustar `plots simular` e `plots imprimir --simular`; os nomes `Simulada — Traseira` e `Simulada — Gaveta` ficam
- [x] Escolher o modelo na barra lateral e por `plots perfil` (826b726), com `calibrar` partindo do perfil salvo
- [x] O perfil salvo guarda o modelo, e a barra lateral deixou de deduzi-lo de `passada1_inversa` (826b726)
- [x] Perfil salvo expressa a virada pela borda de cima (826b726)
- [ ] Confirmar com um lote de 2 ou 3 folhas, quando houver impressora

## Modelo físico

Folha certa exige as duas faces em pé, no referencial da folha lida como página de livro. A virada pela borda de cima é virar mais girar 180° no plano. A cobertura exaustiva é de 16 verificadas e 16 puladas por perfil.

## Pronto quando

Cada impressora real entrega o maço 1|2, 3|4… com um dos dois modelos, sem folha de cabeça para baixo.

## Depende de

`01-nucleo-ordenacao`, `03-simulacao`, `04-calibracao`, `06-perfis`, `11-lateral-impressoras`.
