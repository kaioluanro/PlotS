# CLI de impressão

**Estado:** Em andamento · OK 2 · PARTIAL 1 · MISSING 2 · BUG 0 · N/A 0

## Objetivo

Resolver o problema principal pelo terminal, antes de qualquer interface.

## Plano de implementação

- [x] Mensagens e ajuda fiéis ao que existe
- [x] Simulação correta nas duas impressoras virtuais
- [ ] `plots imprimir doc.pdf --impressora X --lote 10`, primeiro com o backend simulado
- [ ] Mesmo comando com o backend Windows
- [ ] Revisar a frase da ajuda que diz que nada vai à impressora quando `imprimir` existir
- [ ] 3 lotes seguidos sem erro em impressora real

## Pronto quando

Simulação correta nas duas impressoras virtuais; com impressora real, 3 lotes seguidos sem erro.

## Depende de

`02-geracao-pdf`, `06-perfis`, `08-impressao-windows`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A074 | `plots imprimir doc.pdf --impressora X --lote 10` | MISSING |
| PS-A077 | Critério: simulação correta nas duas impressoras virtuais | PARTIAL |
| PS-A078 | Critério: 3 lotes seguidos sem erro numa impressora real | MISSING |
| PS-A090 | Mensagens do CLI coerentes com as funções atuais | OK (882ba68) |
| PS-A091 | Texto de uso lista todos os comandos e aparece só no erro de uso | OK (882ba68) |

## Especificação

| Fase | Entrega | Pronto quando |
| --- | --- | --- |
| 3. Impressão por CLI | `plots imprimir doc.pdf --impressora X --lote 10` e `plots calibrar X`, com backend CUPS ou Windows | Simulação correta nas duas impressoras virtuais; com impressora real (quando houver), 3 lotes seguidos sem erro |
