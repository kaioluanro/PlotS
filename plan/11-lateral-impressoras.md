# Barra lateral de impressoras

**Estado:** Em andamento · OK 0 · PARTIAL 1 · MISSING 2 · BUG 0 · N/A 0

## Objetivo

Escolher por documento quais impressoras recebem o arquivo e em que modo de papel.

## Plano de implementação

- [ ] Listar filas e impressoras simuladas, com caixa de seleção e seletor de modo
- [ ] Seleção por aba; aba nova começa sem nenhuma marcada
- [ ] Campo de lote com resumo e botão "Imprimir frente" com a contagem
- [ ] "Calibrar…" em impressora sem modo

## A reavaliar com o conteúdo real

Lateral de 240 px e altura mínima de 420 px, com a linha de impressora e "Imprimir frente" sempre visível; o que Ctrl+P faz; menu de contexto no campo de lote.

## Pronto quando

Trocar de aba troca a seleção, e o modo escolhido define `passada1_inversa`.

## Depende de

`06-perfis`, `09-janela-e-abas`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A014 | Marcar impressoras e tamanho do lote (padrão 10 folhas) | PARTIAL |
| PS-A021 | Barra lateral de impressoras com caixa de seleção e seletor de modo (Traseira, Gaveta, "Calibrar…") | MISSING |
| PS-A024 | Seleção de impressoras por documento (decidido no AGENTS) | MISSING |

## Especificação

**Barra lateral de impressoras**: lista as impressoras instaladas na máquina. A caixa de seleção define quem recebe o arquivo; à direita, na mesma linha, fica o modo de papel daquela impressora.

| Modo | Entrada do papel | Saída | Frente enviada |
| --- | --- | --- | --- |
| Traseira (vertical) | Atrás, em pé | Na frente, face impressa para cima | Da última para a primeira |
| Gaveta (toner) | Embaixo | Em cima, face impressa para baixo | Da primeira para a última |

O modo escolhido define `passada1_inversa` no perfil: face para cima inverte, face para baixo não. A calibração continua necessária só para o verso (`passada2_inversa` e `verso_girar_180`). Impressora ainda sem modo aparece com "Calibrar…" no lugar do seletor.

Abaixo da lista ficam o tamanho do lote e o botão "Imprimir frente", que mostra quantas impressoras vão receber. Durante o lote, a seção EM ANDAMENTO mostra um cartão por impressora com o estado, o lote atual e o botão "Imprimir verso".

**Decidido:** a seleção de impressoras é de cada documento. Uma aba nova começa sem nenhuma marcada; trocar de aba troca a lista marcada na lateral.
