# Tamanhos de papel

**Estado:** Em andamento · OK 6 · PARTIAL 1 · MISSING 1 · BUG 0 · N/A 0

## Objetivo

Reconhecer o papel de cada página e evitar folha com frente e verso incompatíveis.

## Plano de implementação

- [x] Leitura do tamanho, reconhecimento com tolerância de 2 mm e lotes por formato
- [x] Recusa de tamanhos diferentes e aviso de orientação mista
- [ ] Mapear formatos para `DMPAPER_*` e `DMPAPER_USER`
- [ ] Aviso de troca de papel no cartão
- [ ] Opção "Ajustar ao papel"

## Pronto quando

Documento com tamanhos misturados imprime em lotes separados, com aviso de troca.

## Depende de

`08-impressao-windows`, `13-fluxo-e-recolocacao`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A053 | Ler o tamanho de cada página do PDF | OK |
| PS-A054 | Reconhecer A4, A5, Carta, Ofício, Legal, com tolerância de 2 mm | OK |
| PS-A055 | Formato "Outro" com `DMPAPER_USER` | PARTIAL (126ac26) |
| PS-A056 | Documento com tamanhos misturados dividido em lotes por tamanho | OK |
| PS-A057 | Cartão avisa a troca de papel | OK |
| PS-A058 | Frente e verso de tamanhos diferentes são recusados antes de imprimir | OK |
| PS-A059 | Frente e verso da mesma folha com mesma orientação (retrato e paisagem) | OK (882ba68) |
| PS-A060 | Opção "Ajustar ao papel" (imprimir Carta em A4) | MISSING |

## Especificação

#### Tamanhos de papel

O PlotS lê o tamanho de cada página do PDF e escolhe o papel correspondente no `DEVMODE`.

| Formato | Medida (mm) | Constante do Windows |
| --- | --- | --- |
| A4 | 210 × 297 | `DMPAPER_A4` |
| A5 | 148 × 210 | `DMPAPER_A5` |
| Carta | 216 × 279 | `DMPAPER_LETTER` |
| Ofício (Brasil) | 216 × 330 | `DMPAPER_FOLIO` |
| Legal (EUA) | 216 × 356 | `DMPAPER_LEGAL` |
| Outro | da página | `DMPAPER_USER` com largura e altura, se o driver aceitar |

Regras:

- Tolerância de 2 mm para reconhecer o formato.
- Documento com tamanhos misturados é dividido em lotes por tamanho, e o cartão avisa a troca de papel.
- Frente e verso da mesma folha nunca ficam com tamanhos diferentes; se o PDF tiver isso, o PlotS avisa antes de imprimir.
- Opção "Ajustar ao papel" para imprimir um formato em outro (ex.: Carta em A4).
