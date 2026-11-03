# opcoes-de-envio: últimos ajustes de interface

Responda em português brasileiro. Trabalhe na cópia `E:\PlotS\.deps-fetch\retomada-opcoes`, branch `retomada-opcoes`, sobre `a7937e1`, já rebaseada no automático. Não crie outra cópia. Regras em `plan/briefs/brief-retomada-depois-do-codex.md`, em `E:\PlotS`. Relatório anterior: `%LOCALAPPDATA%/plots-build/opcoes-de-envio/relatorio-correcoes.md`. Build, saídas, cenas e capturas na mesma pasta. `ui/node_modules` foi apagado: rode `npm ci`. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.

As revisões aprovaram. Faça, nesta ordem:

1. **Nota da folha de controle pela rota**, com `passaPelaRotaManual` em `notasDasOpcoes`: tudo manual, "Com folha de controle" / "With control sheet"; misto, "Folha de controle só nos trechos manuais." / "Control sheet on manual sections only."; só automático, sem nota, e aviso na lateral com a opção ligada: "Neste envio não sai marca: o frente e verso automático não usa." / "No mark on this print: automatic duplex doesn't use it."
2. **Frases de velocidade com três impressoras ou mais:** troque o final: espelho "Cada uma imprime o documento inteiro." / "Each prints the whole document."; divisão "Os lotes seguem em rodízio." / "Batches are shared in turn."
3. **Dica da folha de controle:** fixa, uma frase: "Escreve o lote e a folha no canto do verso, em letra pequena." / "Writes the batch and sheet in the corner of the back, in small print." "Pode ficar sobre o que houver nesse canto." só com a opção ligada. Com `data-com-cartao`, esconda as dicas visualmente e mantenha o `aria-describedby`.
4. **Avisos de rota na divisão:** uma nota por impressora, com todas as faixas: "Em Simulada — Automática, págs. 3–4, 7–8: …".
5. **Menores:** "Marcas:" compacta, "Lote 1: folhas 4, 3, 2, 1"; ícone de alerta em `historico.guardar.falhaLeitura`.
6. Recapture `r03`, `r05`, `r04` e `11b`, mais um envio só automático com a folha de controle ligada e um com `inversao.compensadaEm`. Leia cada uma.

Testes de unidade com grupo de falha prévia. CI completo em um único log, autoverificação com base `94caa79` (falsos positivos conhecidos: "this batch" na recusa e `automatica-a5-em-a4`), novo wip, `git status` limpo, apague `ui/node_modules` e `ui/dist`. Resposta final curta: item a item, linha do CI, capturas e o que não foi verificado. Limite de tempo em todo comando de cena.
