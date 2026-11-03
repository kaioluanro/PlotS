# frente-e-verso-automatico: impressoras que imprimem as duas faces sozinhas

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia primeiro `E:\PlotS\plan\21-frente-e-verso-automatico.md`, que define o comportamento, `plan/19-modelos-de-impressora.md`, `plan/briefs/coordenacao-paralela-2.md` (regras comuns) e, para o estilo dos cenários, `%LOCALAPPDATA%/plots-build/cenas/varias/`. Lote `frente-e-verso-automatico`, porta CDP 9346, build em `%LOCALAPPDATA%/plots-build/frente-e-verso-automatico`. Antes do primeiro grep ou leitura de código, rode `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Scripts em pasta sua: o scratchpad é compartilhado. Você é o único agente de implementação agora, então pode mexer em qualquer arquivo de código. Não mexa em `plan/`.

## Decisões já tomadas

- **Terceiro modelo**, "Frente e verso automático", ao lado de jato e laser. Só aparece para fila cujo driver declara duas faces (`DeviceCapabilitiesW` com `DC_DUPLEX`) e na nova impressora simulada. Nunca é escolhido sozinho. O perfil guarda também qual gesto manual usar nos papéis que o automático não aceita, com jato como padrão, e o seletor deixa escolher.
- **Por papel:** A4 e Carta vão pelo automático. Qualquer outro papel segue o fluxo manual daquela impressora. Em documento misto, os dois fluxos convivem, em ordem de documento, e a confirmação diz quais páginas vão por qual, antes de enviar. Tipo de papel do driver (fotográfico, envelope) não é detectável: a nota da confirmação pede papel normal.
- **Envio automático:** um trabalho com `dmDuplex = DMDUP_VERTICAL` (borda longa) e a ordem do documento, 1, 2, 3…, sem inverter, sem girar e sem folha em branco ao fim. Confira que o driver aceitou. Se recusar, nada sai, e o cartão manda escolher um modelo manual. O ajuste salvo do driver continua valendo como base, e o PlotS fixa duas faces nesse caso. **No fluxo manual, nada muda: uma face só.**
- **Borda curta:** fica para depois. Página deitada segue o que o envio já faz; documente o que acontece e teste com um PDF deitado na simulada.
- **"Folhas por lote":** um lote automático não precisa de gesto, então o documento sai em um trabalho por trecho de mesmo papel. O campo não se aplica a esse modelo, e a interface diz isso de forma discreta, sem esconder nada de quem também marcou outra impressora. Na divisão, um lote atribuído a ela sai como um trabalho automático, um depois do outro, sem clique.
- **Cartão:** sem figura, sem gesto e sem "Imprimir verso". Estado, lote, avisos da fila e "Reimprimir lote". Ao concluir: "Se a primeira página não estiver em cima, vire o maço." / "If the first page is not on top, flip the stack." Aviso de falta de papel diz para colocar folhas em branco, em qualquer momento, porque a entrada não tem maço impresso.
- **Impressora simulada:** "Simulada — Automática", que imita o modelo: confere folhas 1|2, 3|4… e recusa papel que não seja A4 ou Carta, como a referência. Espelho, divisão e intervalo funcionam com ela.
- **Marca:** o produto usa nome genérico, sem marca nem modelo.

## Acabamento pendente da engrenagem (faça junto, é pequeno)

1. Diálogo de envio com dois ou mais destinos nomeia só as filas com ajuste, sem truncar: "Ajuste do driver só no PlotS: {filas}. Papel e faces seguem o PlotS." / "PlotS-only driver settings: {filas}. Paper and sides follow PlotS." Com um destino, fica como está.
2. `preferencias.salvo.duasFaces`: "O driver está em duas faces, mas o PlotS imprime uma por vez." / "The driver is set to two sides, but PlotS prints one at a time." Essa frase não vale para o modelo automático, que imprime em duas faces: nesse modelo, não a mostre.
3. `preferencias.salvo.semFila`: "A impressora {nome} não está na lista. O ajuste dela segue salvo no PlotS." / "The printer {nome} is not in the list. Its settings remain saved in PlotS." Nesse bloco: X de 24×24, com dispensa pela sessão, e botão "Apagar ajuste" / "Delete settings", com o botão do diálogo mudando junto.

## Testes

- Rust: plano automático em ordem de documento; total ímpar sem folha em branco; documento misto com trechos e fluxos certos; intervalo; espelho e divisão; driver que recusa duas faces não envia nada; fluxo manual continua em uma face, inclusive com o modelo automático em papel A5; `DC_DUPLEX` em backend falso; perfil antigo sem o campo novo carrega.
- Com "Microsoft Print to PDF", que não tem duas faces: ela não oferece o modelo. Sem envio real em duas faces, relate como não verificado.
- Vitest: seletor, confirmação, cartão, redutor.
- App real por CDP: "Simulada — Automática" com A4, sem clique no meio, e com documento misto; junto de uma simulada manual em espelho e em divisão. Nos dois temas, em inglês e na janela mínima; cores forçadas por emulação. Leia cada imagem.

## Final

CI completo em um único log, autoverificação com a base em que começou, `git status` limpo com wip, apague `ui/node_modules` e `ui/dist`. Relatório em `%LOCALAPPDATA%/plots-build/frente-e-verso-automatico/relatorio.md`, com a mesma mensagem na resposta final: o que já existia, item a item, testes com grupo de falha prévia, linha do CI, capturas com caminho, textos novos, o que não foi verificado, caminho do worktree e nome do branch. Não espere processo em segundo plano sem limite de tempo.
