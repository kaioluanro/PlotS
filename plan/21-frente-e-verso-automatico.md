# Frente e verso automático

**Estado:** Em pausa, com trabalho parcial em worktree, sem CI nem revisão (ver `STATUS-MANUAIS.md`).

## Objetivo

Aceitar impressoras que imprimem as duas faces sozinhas, como a referência Epson L4360, sem gesto e sem espera entre as passadas.

## O que a referência diz

Manual do usuário da L4360, p. 53 e 54:

| Papel | Automático | Manual |
|---|---|---|
| Normal, cópia: Carta ou A4 | até 100 folhas | até 30 folhas |
| Normal, cópia: Executivo, Meia carta, A6 | não | até 30 folhas |
| Normal, cópia: Ofício e parecidos | não | 1 folha |
| Fotográfico, envelope | não imprime nas duas faces | não imprime nas duas faces |

Papel pré-perfurado também não aceita o automático (p. 50). Opções de encadernação: borda longa ou borda curta (p. 101). **Consequência:** a mesma máquina precisa dos dois fluxos, conforme o papel. O produto usa nome genérico, sem marca.

## Comportamento proposto

- A lateral oferece um terceiro modelo, "Frente e verso automático", só quando o driver declara ter duas faces (`DeviceCapabilitiesW` com `DC_DUPLEX`). O driver pode declarar sem ter a unidade, então a pessoa é quem escolhe; o PlotS não liga sozinho.
- O modelo guarda também o gesto manual da máquina, para os papéis que o automático não aceita.
- **A4 e Carta vão pelo automático.** Outros papéis, inclusive em documento misto, seguem o fluxo manual, e a confirmação diz antes de enviar. A regra é conservadora: outra máquina pode aceitar mais, mas o manual funciona em todas.
- O envio pede duas faces com virada pela borda longa e confere se o driver aceitou. Se recusar, nada sai, e o cartão diz para escolher um modelo manual.
- Ordem: 1, 2, 3… na ordem do documento, sem inverter, sem girar e sem folha em branco no fim.
- O cartão não mostra gesto nem figura. Ao terminar, diz: "Se a primeira página não estiver em cima, vire o maço."
- Sem páginas de teste: nada foi medido em papel. Uma terceira impressora simulada imita esse modelo.
- Espelho, divisão e intervalo de páginas continuam valendo. Em aviso de papel, "Reimprimir lote" refaz o lote inteiro.

## A decidir no brief

- Como juntar isso à máquina de estados, sem a espera entre as passadas.
- Se "Folhas por lote" faz sentido nesse modelo, sem gesto para limitar o maço.
- Se a borda curta entra agora ou fica para depois.

## Pronto quando

Na simulada, um documento A4 sai 1|2, 3|4… sem nenhum clique no meio, e um documento misto usa os dois fluxos. Confirmação em papel fica para quando houver impressora.

## Depende de

`15-multiplas-impressoras`, `13-fluxo-e-recolocacao`, `19-modelos-de-impressora`, `20-preferencias-do-driver`.
