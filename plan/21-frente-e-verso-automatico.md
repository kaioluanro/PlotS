# Frente e verso automático

**Estado:** Concluído no código (`0a8f451`); falta a conferência em impressora com duas faces.

## Objetivo

Aceitar impressoras que imprimem as duas faces sozinhas, como a referência Epson L4360, sem gesto e sem espera entre as passadas.

## O que a referência diz

Consulta ao manual do usuário da L4360 (cpd64038, p. 50, 53, 54 e 101). Na referência, o automático só serve para papel comum em A4 ou Carta, e os outros tamanhos comuns só têm o fluxo manual. Papel fotográfico e envelope não saem em duas faces em nenhum fluxo, e papel furado fica fora do automático. A virada pode ser pela borda longa ou pela borda curta. **Consequência:** a mesma máquina precisa dos dois fluxos, conforme o papel. O produto usa nome genérico, sem marca.

## Comportamento proposto

- A lateral oferece um terceiro modelo, "Frente e verso automático", só quando o driver declara ter duas faces (`DeviceCapabilitiesW` com `DC_DUPLEX`). O driver pode declarar sem ter a unidade, então a pessoa é quem escolhe; o PlotS não liga sozinho.
- O modelo guarda também o gesto manual da máquina, para os papéis que o automático não aceita.
- **A4 e Carta vão pelo automático.** Outros papéis, inclusive em documento misto, seguem o fluxo manual, e a confirmação diz antes de enviar. A regra é conservadora: outra máquina pode aceitar mais, mas o manual funciona em todas.
- O envio pede duas faces com virada pela borda longa e confere se o driver aceitou. Se recusar, nada sai, e o cartão diz para escolher um modelo manual.
- Ordem: 1, 2, 3… na ordem do documento, sem inverter, sem girar e sem folha em branco no fim.
- O cartão não mostra gesto nem figura. Ao terminar, diz: "Se a primeira página não estiver em cima, vire o maço."
- Sem páginas de teste: nada foi medido em papel. Uma terceira impressora simulada imita esse modelo.
- Espelho, divisão e intervalo de páginas continuam valendo. Em aviso de papel, "Reimprimir lote" refaz o lote inteiro.

## Decisões

- **Lotes:** o lote automático segue sozinho, sem clique. "Folhas por lote" não se aplica a ele; só com automáticas, o rodapé mostra só as páginas.
- **Borda curta:** fica para depois. Página deitada gira para caber na folha em pé e vira pela borda longa, lendo como calendário. Conferir em papel.
- **Pergunta ao driver:** em thread própria, 1,5 s no total por leitura, resposta guardada por 60 s, sem perguntar a fila offline. Resposta tardia só aparece na próxima leitura.
- **Pendente, entra junto do lote de opções:** seletor do gesto sem `disabled`, centro da seta, contagem de lotes em documento misto, linhas com impressoras de modelos diferentes, palavra "lote" na recusa, altura da lista em 640x420.
- **Não verificado:** envio real em duas faces e `DC_DUPLEX` verdadeiro, porque a única fila aqui não declara duas faces; driver lento de verdade; recusa real no meio; ordem de saída, borda longa e página deitada em papel.

## Pronto quando

Na simulada, um documento A4 sai 1|2, 3|4… sem nenhum clique no meio, e um documento misto usa os dois fluxos. Confirmação em papel fica para quando houver impressora.

## Depende de

`15-multiplas-impressoras`, `13-fluxo-e-recolocacao`, `19-modelos-de-impressora`, `20-preferencias-do-driver`.
