# Impressora virtual

**Estado:** Não iniciado · OK 0 · PARTIAL 0 · MISSING 1 · BUG 0 · N/A 0

## Objetivo

Fila "PlotS" que recebe o documento de qualquer programa e o abre em uma aba.

## Plano de implementação

- [ ] Pesquisar o caminho no Windows (porta de impressora ou driver)
- [ ] Instalar e remover a fila
- [ ] Abrir o arquivo recebido em uma aba, com instância única

## Pronto quando

Imprimir pelo Firefox na fila "PlotS" abre o app com o documento.

## Depende de

`09-janela-e-abas`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A052 | Fila virtual "PlotS" que recebe o PDF de qualquer programa | MISSING |

## Especificação

**Impressora virtual (opcional, Fase 5)**: uma fila "PlotS" no Windows serve só de porta de entrada. Você imprime de qualquer programa (Word, Firefox) nessa fila e o arquivo abre numa aba do PlotS, que então faz todo o trabalho acima. Sem ela, basta abrir o PDF no PlotS.
| Fase | Entrega | Pronto quando |
| --- | --- | --- |
| 5. Várias impressoras e impressora virtual | Jobs paralelos, modos espelho/divisão, status do spooler; fila virtual "PlotS" que recebe o PDF de qualquer programa | Imprimir pelo Firefox na fila "PlotS" abre o app com o documento |
