# Linux e macOS

**Estado:** Não iniciado · OK 0 · PARTIAL 0 · MISSING 1 · BUG 0 · N/A 0

## Objetivo

Levar a impressão para CUPS depois do Windows.

## Plano de implementação

- [ ] Backend IPP ou `lp` implementando o mesmo `trait Backend`
- [ ] Status do job com `Get-Job-Attributes`

## Pronto quando

Mesmo fluxo em uma fila CUPS.

## Depende de

`08-impressao-windows`. Fica para depois, conforme as decisões tomadas.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A043 | Linux e macOS com IPP/CUPS ou `lp` | MISSING |

## Especificação

| Peça | Escolha | Por quê |
| --- | --- | --- |
| Linux/macOS | `ipp` (IPP no CUPS) ou `lp` | Listar filas, enviar PDF, ler status do job |
