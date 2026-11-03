# opcoes-de-envio: correções das revisões, depois rebase

Responda em português brasileiro. Trabalhe na cópia `E:\PlotS\.deps-fetch\retomada-opcoes`, branch `retomada-opcoes`, sobre `cf4809f`. Não crie outra cópia. Regras em `plan/briefs/brief-retomada-depois-do-codex.md`, em `E:\PlotS`. Relatório anterior: `%LOCALAPPDATA%/plots-build/opcoes-de-envio/relatorio.md`. Build, saídas e cenas na mesma pasta. `ui/node_modules` foi apagado: rode `npm ci`. Antes do primeiro grep ou leitura de código, rode `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. A identidade `noreply` já está configurada: confirme antes de commitar. O automático está na cópia `E:\PlotS\.deps-fetch\retomada`, em correção: pode ler, não mexa.

**Duas etapas.** Faça a etapa 1 agora e entregue. A etapa 2 só começa quando o coordenador avisar que o automático entrou no `master`.

## Etapa 1: correções

**Decisões do usuário:**

1. **Frase do aviso de velocidade.** Sai "deixe mais papel nela", que contradiz o rodízio.
   - divisão: "Pelo histórico deste PC, {rapida} imprime cerca de {fator}x mais rápido que {lenta} e termina antes. As duas recebem lotes em rodízio." / "Going by this PC's history, {rapida} prints about {fator}x faster than {lenta} and finishes first. Both get batches in turn."
   - espelho: frase equivalente, sem falar em rodízio.
   - rótulo: "Velocidade medida pelo PlotS" / "Speed measured by PlotS".
2. **"Guardar histórico", ligada por padrão.** Controle dentro do diálogo do histórico, gravado junto das preferências do PlotS, com arquivo antigo valendo como ligada. Desligada, nenhum envio novo é registrado, a lista existente continua até ser apagada, e o diálogo diz isso em uma frase. Com ela desligada, sem novas medidas; o aviso de velocidade usa só o que já existe. Trabalho em andamento segue a escolha feita quando começou. Testes: ligada, desligada, arquivo antigo, falha ao gravar a escolha.

**Jurídico (obrigatório):**

3. `historico.local`: "O PlotS guarda esta lista no seu perfil do Windows e não a envia pela internet." / "PlotS keeps this list in your Windows profile and does not send it over the internet."
4. `historico.apagarDialogo.texto`: "A lista inteira será apagada. Os documentos continuam como estão." / "The whole list will be deleted. Documents stay as they are."
5. `inversao.titulo`: "Ligue se este driver imprime a última página primeiro. O PlotS envia cada passada na ordem contrária para compensar." / "Turn on if this driver prints the last page first. PlotS sends each pass in the opposite order to make up for it."
6. **`README.md`, em "Seus dados"**, depois do primeiro parágrafo: "O histórico de impressão fica em `%APPDATA%\plots\historico.toml`, também em texto sem criptografia, e guarda até 500 envios feitos pela janela: o nome do arquivo impresso (sem a pasta), a data, as impressoras, as páginas, o trecho, os lotes, como o envio terminou, e o tempo e as folhas de cada impressora. O nome de um arquivo ou de uma impressora pode revelar algo pessoal ou de trabalho, e quem usa a mesma conta do Windows consegue ler essa lista. O PlotS não envia o histórico pela internet. Para não registrar novos envios, desligue **Guardar histórico** no próprio histórico. Para apagá-lo, abra o histórico na barra de título e use **Apagar histórico**. Um envio ainda em andamento entra na lista quando terminar." Troque: "Para apagar tudo, feche o PlotS e apague a pasta `plots`. Na próxima abertura o programa começa sem perfis, sem ajustes e sem histórico."; no parágrafo do Roaming, "...junto com o nome das impressoras, os ajustes do driver gravados no arquivo e o histórico, com os nomes dos arquivos impressos."; "O desinstalador foi configurado para não apagar os perfis, os ajustes nem o histórico."
7. **Menores:** ao apagar, remova também `.historico.toml.*.tmp` da mesma pasta. `PLOTS_HISTORICO`: apagar só se o arquivo tiver esse nome ou carregar como histórico. `controle.titulo` avisa: "Pode ficar sobre o que houver nesse canto." / "It may overlap whatever is in that corner."

**Interface (obrigatório):**

8. `--focus-ring` vira `--accent` em `.caixa` e `.campoAjuste`.
9. Dica fixa sob a opção do driver, só na linha marcada, 11 px em `--fg-muted`: "Ligue se a última página sair primeiro." / "Turn on if the last page prints first.", com `aria-describedby`. O mesmo tratamento em "Folha de controle".
10. `inversao.aviso`: "Se o driver imprimir a última página primeiro, o maço sai embaralhado. Desligue isso nas preferências dele ou marque "O driver inverte a ordem" na linha da impressora." / "If the driver prints the last page first, the stack comes out shuffled. Turn that off in its preferences or check "The driver reverses the order" on the printer's row."
11. Histórico: uma linha por impressora, "Nome · 1 s de impressão", sem repetir o nome; só o nome para quem não tem tempo.
12. Nota na confirmação com a compensação ligada: "Ordem compensada: este driver inverte." / "Order compensated: this driver reverses.", citando as filas se houver várias.
13. Falha ao gravar a opção: ícone de alerta em `--status-offline-text` e "Não foi possível gravar esta opção. Tente de novo." / inglês. Refaça `en-light-24` e corrija a rolagem se a mensagem não aparecer.
14. Com "Folha de controle" ligada, a confirmação diz "Com folha de controle" / "With control sheet", porque na janela curta com cartão a opção fica oculta. Conserte `.campoAjuste` conforme a norma, se couber sem estourar a janela mínima.

**Código (opcional, se for pequeno):** teste direto de `folhas_certas_na_ordem`; página em branco com a mesma orientação da que acompanha.

Termine a etapa 1 com testes da parte alterada e `git status` limpo com wip. **Não rode o CI completo nem refaça capturas agora: o rebase muda tudo.** Devolva: item a item, testes com grupo de falha prévia, textos novos.

## Etapa 2: rebase, quando o coordenador avisar

**Pendências do automático, faça junto, pequenas:** seletor do gesto sem `disabled`, como o do modelo (`aria-busy`, `title` e indicador quando demorar, `onChange` ignorado na gravação), conferindo no teclado que o foco fica. `.setaSeletor` com `top: 9px` e `.indicadorGravacao` com `top: 8px`. Rodapé sem contagem de lotes em documento com mais de um papel. Com impressoras de modelos diferentes: "Em {nomes}, {paginas}: frente e verso automático, sem gesto." e "Em {nomes}, {paginas}: frente e verso manual, com gesto." (en: "On {nomes}, {paginas}: automatic duplex, no handling." e "On {nomes}, {paginas}: manual duplex, with handling."), e no espelho "Em Copa: frente e verso manual, com gesto." (en: "On Copa: manual duplex, with handling."). Recusa no meio: "O driver recusou duas faces. Este lote não saiu; os anteriores saíram. Escolha um modelo manual." (en: "The driver rejected two-sided printing. This batch did not print; the earlier ones did. Choose a manual model."). Até 560 px sem cartão, `.camposDoRodape` com altura limitada e rolagem própria, para a lista manter ao menos 140 px, com o botão sempre visível.

Rebase sobre o `master` novo, com o automático. Lista de conflitos prováveis, da revisão: `LoteDoFluxo` (`marcas`, em construtores e `plots-cli/src/enviar.rs`), `EntradaPerfil` (`driver_inverte`, cerca de 15 literais), `Impressora`, `Pedido`, `SimGuiado::folhas_conferidas`, `lotes_do_plano`, `preparar_simulada` com `DriverDeTeste`, `planejar_pedido`, `i18n.ts`, `cartoes.ts`, `Sidebar.tsx`, `impressoras.ts`, `App.tsx`.

Regras depois do rebase:

1. `marcar_lotes` e `compensar_driver` pulam todo lote `duas_faces`, e a fila em modo automático não usa compensação. **O `match` de `usa_opcoes_do_manual` não protege nada, porque o automático não é um novo `ModeloImpressora`:** troque por condição real, corrija comentário e relatório. Teste com lote de duas faces: faces iguais e `marcas` vazio.
2. `pode_inverter` falso em fila automática, salvo se este envio tiver papel na rota manual, quando a opção aparece na linha. `inversao.aviso` não aparece para destino só automático.
3. **Ajustar ao papel decide a rota pelo papel de destino**, não pelo original: Carta ajustada a A4 vai pelo automático, e A5 ajustado a A4 também. A confirmação diz o que vale. A marca nunca cria folha em branco no automático.
4. Unir papel, escala e rota em uma linha na caixa da confirmação, sem repetir "Papel". Ordem fixa dos avisos: rota, driver, velocidade, instruções gerais. Botões sempre visíveis.
5. Velocidade compara só medidas da mesma rota, sem misturar automático e manual da mesma fila.
6. Cenas nos dois temas e idiomas: 10, 11, 11b, 15, confirmação com todos os avisos em 640x420, lista do histórico com fila real, `pt-light-03`, opções junto com a simulada automática, simulada manual e divisão. Leia cada imagem.
7. CI completo em um único log, autoverificação com a base do rebase, novo wip, `git status` limpo, apague `ui/node_modules` e `ui/dist`. Relatório em `%LOCALAPPDATA%/plots-build/opcoes-de-envio/relatorio-correcoes.md`, curto, com a mesma mensagem na resposta final. Limite de tempo em todo comando de cena.
