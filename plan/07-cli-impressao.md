# CLI de impressão

**Estado:** Em andamento · OK 2 · PARTIAL 2 · MISSING 1 · BUG 0 · N/A 0

## Objetivo

Resolver o problema principal pelo terminal, antes de qualquer interface.

## Plano de implementação

- [x] Mensagens e ajuda fiéis ao que existe
- [x] Simulação correta nas duas impressoras virtuais
- [x] `plots imprimir doc.pdf --impressora X --lote 10 --simular`, com pausa para recolocar e avisos claros
- [x] Mesmo comando com o backend Windows, com confirmação antes de cada envio (126ac26)
- [x] Revisar `FRASE_NAO_ENVIA` e a recusa sem `--simular`
- [ ] 3 lotes seguidos sem erro em impressora real

## Pronto quando

Simulação correta nas duas impressoras virtuais; com impressora real, 3 lotes seguidos sem erro.

## Depende de

`02-geracao-pdf`, `06-perfis`, `08-impressao-windows`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A074 | `plots imprimir doc.pdf --impressora X --lote 10` | PARTIAL (126ac26) |
| PS-A077 | Critério: simulação correta nas duas impressoras virtuais | PARTIAL |
| PS-A078 | Critério: 3 lotes seguidos sem erro numa impressora real | MISSING |
| PS-A090 | Mensagens do CLI coerentes com as funções atuais | OK (882ba68) |
| PS-A091 | Texto de uso lista todos os comandos e aparece só no erro de uso | OK (882ba68) |

## Especificação

| Fase | Entrega | Pronto quando |
| --- | --- | --- |
| 3. Impressão por CLI | `plots imprimir doc.pdf --impressora X --lote 10` e `plots calibrar X`, com backend CUPS ou Windows | Simulação correta nas duas impressoras virtuais; com impressora real (quando houver), 3 lotes seguidos sem erro |
