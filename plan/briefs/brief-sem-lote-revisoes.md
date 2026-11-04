# Impressão sem lote: correções das revisões

Responda em português brasileiro. Trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-acbf7ba0ce97216fc`, commit `d3a5c29`, base `3b6e31f`. Não crie outro worktree. Build e saídas em `%LOCALAPPDATA%/plots-build/frentes-primeiro`, com `CARGO_TARGET_DIR` ali, `CARGO_INCREMENTAL=0`, `-j 4`; porta CDP 9352. As cenas já prontas estão em `cena/` e `saidas/` nessa pasta. Rode `npm ci` em `ui/`. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`. Comportamento: `E:\PlotS\plan\24-impressao-sem-lote.md`. Regras comuns: `plan/briefs/coordenacao-paralela-2.md`. Norma visual: `E:\PlotS\plan\estilo-ui-ux.md`. Limite de tempo em todo comando; não espere processo em segundo plano sem prazo. Só simuladas e "Microsoft Print to PDF"; nada a impressoras reais. Testes nunca tocam `%APPDATA%`.

Os textos são definitivos, nos dois idiomas. Testes que fixam texto mudam junto, com a mesma garantia. Um relatório de qualquer forma é entregue na resposta final; não precisa gravar arquivo.

## A. Física e dados

1. **Marcas por posição, não por número de página** (`plots-print/src/opcoes.rs`, `marcas_do_lote`; `plots-pdf/src/lib.rs`, perto da linha 261). Hoje a marca é um mapa pelo número da página, e a última cópia sobrescreve as outras: com 24 páginas e 3 cópias, as três imprimem Folha 25 a 36. A marca continua "Folha N" / "Sheet N", contínua no maço. Teste com falha prévia: 24 páginas e 3 cópias dão marcas únicas de 1 a 36. Captura da simulada com folha de controle.
2. **Teste de cópias com perfil que inverte ou gira:** `passada1_inversa`, `passada2_inversa` e `verso_girar_180`, com N = 2 e 3, fixando que a inversão vale para o maço inteiro. Se reprovar, pare e relate.
3. **Mesmos bytes:** em `src-tauri/src/trabalhos.rs` (`verificar_impressao`, chamadas em `preparar_*`), calcule a impressão sobre os mesmos bytes que o `PdfOrigem` carrega, sem ler o arquivo duas vezes.
4. **Guardar a impressão só depois de aceito:** em `comandos_trabalho.rs`, perto da linha 265, a frente recusada (impressora ocupada) não pode sobrescrever a impressão da aba. Teste com falha prévia.
5. **Fechar a aba** remove a impressão guardada (`soltar_trabalhos`).

## B. Estilos e campo

6. **Regressão: restaure** `.campoLote`, `.campoInvalido`, `.loteLinha`, `.erroLote` e as regras de `forced-colors`, em `Sidebar.module.css` ou movidas para `BlocoIntervalo.module.css`: os campos de Páginas perderam o estilo (`BlocoIntervalo.tsx`).
7. **Quantidade em uma linha:** `CampoCopias.tsx` e `.module.css`. Rótulo à esquerda, campo à direita, reutilizando `.loteLinha` e `.campoLote`: 56×32 px, 13 px, números tabulares alinhados à direita, com o hover `--border-control-hover`. Foco: `outline: 2px solid var(--accent); outline-offset: 2px`. Em `forced-colors`: foco e borda de erro em `Highlight`, borda de erro de 2 px. Com `max-height: 560px`, esconda a dica.
8. **Entrada inválida:** o texto digitado fica, inclusive ao sair do campo; remova `aoSair`. O botão de envio fica desabilitado até corrigir: leve o estado inválido para fora do campo. Trocar de face limpa o erro. O erro **substitui** a dica, com `CircleAlert` de 12 px, 11 px, no padrão `.recusado`. Texto: "Use um número de 1 a 999." / "Use a number from 1 to 999." Teste: o botão desabilita, o envio nunca sai com número velho.

## C. Rodapé

9. **Botões, sem "(n)":** `imprimir.frente` "Imprimir frentes" / "Print fronts"; `imprimir.frenteContagem` "Imprimir frentes em {n}" / "Print fronts on {n}"; `cartao.simular.frente` "Simular frentes" / "Simulate fronts"; `cartao.simular.frenteContagem` "Simular frentes em {n}" / "Simulate fronts on {n}"; `face.imprimirVerso` "Imprimir versos" / "Print backs"; `face.imprimirVersoContagem` "Imprimir versos em {n}" / "Print backs on {n}"; `face.simularVerso` "Simular versos" / "Simulate backs"; `face.simularVersoContagem` "Simular versos em {n}" / "Simulate backs on {n}". Em `aria-label`, por extenso: "em 2 impressoras" / "on 2 printers".
10. **Dica que diz o estado, sem mudar a escolha** (`BlocoFace.tsx`). **A escolha nunca muda sozinha.** Estas duas não somem em `max-height: 560px` (não use `data-dica-modo`):
    - Frente escolhida e frentes prontas nas marcadas: "O trabalho das frentes terminou ou saiu da fila. Para os versos, escolha Verso." / "The fronts job finished or left the queue. For the backs, choose Back."
    - Verso escolhido e versos concluídos nas marcadas: "O trabalho dos versos terminou ou saiu da fila. Clique de novo só para repetir." / "The backs job finished or left the queue. Click again only to repeat it."
11. **Resumo com cópias**, só com 2 ou mais, no rodapé e na primeira linha da confirmação: "24 pág × 3 · 36 folhas" / "24 pages × 3 · 36 sheets". A linha "Cópias: {n}" fica. Com 1 cópia, nada muda.
12. **Janela mínima** (`Sidebar.module.css`, perto da linha 1025): nada cortado sem sinal de rolagem. Se o rodapé não couber em 640×420 com os itens acima, recolha também `[data-bloco-grupos]`, sem depender de haver cartão, e mostre ao menos uma impressora inteira.

## D. Cartão e confirmação

13. **Frentes prontas** (`cartoes.ts`, perto da linha 711; `CartaoImpressao.tsx`; `Sidebar.tsx`, perto da linha 240): título da seção por `cartaoSeguraAAba`, para "Em andamento" / "In progress", nunca "Encerrados". Tire a linha `cartao.simulada.clique` / `cartao.verso.clique` deste estado. O botão vem logo depois do gesto, e `face.espera` depois do botão, para ele aparecer sem rolar em 1280×800. O cartão de versos não muda a escolha do rodapé.
14. **Automático com cópias** (`cartoes.ts`, `CartaoImpressao.tsx`, `i18n.ts`; o cartão precisa saber o número de cópias). Em andamento: "Cópia {n} de {total} · {folhas} · A4" / "Copy {n} of {total} · {folhas} · A4". Com papéis mistos: "Cópia {n} de {total} · trecho {t} de {tt} · {folhas}" / "Copy {n} of {total} · section {t} of {tt} · {folhas}". Concluído: "{total} cópias · {folhas} no total · A4" / "{total} copies · {folhas} in total · A4", e nunca diz "Trecho". Lista: "Cada cópia: {lista}" / "Each copy: {lista}". Vire: "Em cada cópia, se a página de número menor não estiver em cima, vire a cópia." / "In each copy, if the lowest page number is not on top, flip that copy." `face.juntar` só aparece com mais de um papel por cópia. Reimprimir: "Reimprimir a cópia {n} de {total} em {nome}?" / "Reprint copy {n} of {total} on {nome}?"
15. **Confirmação do verso, com números diferentes:** logo abaixo do texto inicial, antes da caixa, 13 px, `--fg`, com ícone de aviso. Não entre as notas do fim.
16. **Cartão de documento mudado:** recapture `troca-*-01` com o cartão parado. Se o primeiro cartão seguir sem motivo, mostre `cartao.motivo.documentoMudou` como no segundo.

## E. Textos da revisão jurídica

- `face.copias.diferentes`: "A Frente está com {frente} {copiasFrente} e o Verso com {verso}: coloque um maço com {verso} {copiasVerso}." / "Front is set to {frente} {copiasFrente} and Back to {verso}: load a stack with {verso} {copiasVerso}."
- `cartao.motivo.documentoMudou`: "O arquivo mudou desde o pedido das frentes, e as folhas já impressas podem não corresponder a ele. Imprima as frentes de novo. Para imprimir só o verso mesmo assim, abra o arquivo em uma aba nova: o PlotS não confere o maço." / "The file changed since the fronts were requested, and sheets already printed may not match it. Print the fronts again. To print only the back anyway, open the file in a new tab: PlotS does not check the stack."
- `face.envio.jaSaiu`: "O trabalho das frentes deste documento já terminou ou saiu da fila em {lista}. Enviar de novo repete as frentes e, em impressora real, gasta papel. Para os versos, escolha Verso." / "The fronts job of this document already finished or left the queue on {lista}. Sending again repeats the fronts and, on a real printer, uses up paper. For the backs, choose Back."
- `cartao.concluido.simulado`: "A simulação deste envio terminou. Isso não confere nenhuma impressora real." / "The simulation of this job finished. This does not check any real printer."
- `face.dica.frente`: "Só as frentes, sem verso no meio." / "Fronts only, with no backs in between."
- `face.espera`: "Pode ser depois. Se fechar o PlotS, abra o mesmo arquivo e escolha Verso. Nesse caso, o PlotS não confere se ele mudou." / "It can wait. If you close PlotS, open the same file and choose Back. In that case, PlotS does not check whether it changed."
- `face.fechar.versos`: "Os versos ainda não foram pedidos. Depois, abra o mesmo arquivo, marque as impressoras e escolha Verso. O PlotS não confere se ele mudou." / "The backs have not been requested yet. Later, open the same file, tick the printers and choose Back. PlotS does not check whether it changed."
- O texto "em um trabalho só" (`i18n.ts:223` e `:748`): "As frentes estão indo para a fila." / "The fronts are going to the queue."
- "Frentes prontas" como rótulo: "Frentes concluídas na fila" / "Fronts done in the queue".
- "os anteriores saíram" (`i18n.ts:59` e `:587`): "os anteriores já foram enviados" / "the earlier ones were already sent".
- "imprime as duas faces" (`i18n.ts:516`, `:517`, `:519`, `:1041`, `:1042`, `:1044`): "está configurada para imprimir as duas faces de uma vez" e o equivalente em inglês, ajustando a concordância.

## Capturas

Em `saidas/shots`, por CDP, lendo cada uma. Refaça as dos itens acima e entregue também: hover do campo sem foco; hover e clique no seletor; foco por teclado em Frente/Verso, no botão principal e em "Dispensar", e a volta do foco ao fechar o diálogo; botões desabilitados com `cartao.recusa.frentePendente` e `face.recusa.soAutomaticas`; erro de Páginas nos dois temas; confirmação da frente com `jaSaiu`; maço acima de 30 folhas em frente e verso; papéis mistos; cartão com "Simular versos" à vista, em 1280 e 640; 640 sem cores forçadas, claro e escuro; diálogo de fechar com frentes prontas; automático em andamento com cópias, erro no meio de uma cópia e diálogo de reimprimir; saída simulada com folha de controle e 3 cópias; Quantidade com 999. Inglês e cores forçadas nos estados principais.

## Final

CI em um único log `saidas/ci.log`: `cargo fmt --all --check`, `cargo test --workspace --locked -j 4`, em `ui/` `npm ci`, `npm test`, `npm run build`, 0 `__plotsE2e` em `ui/dist`, `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR="%LOCALAPPDATA%/plots-build/pdfium/chromium-8125/bin"`, terminando com `CI_EXIT <código>`. Autoverificação `sh E:/PlotS/.claude/team/selfcheck.sh 3b6e31f ui src-tauri plots-core plots-print plots-pdf plots-windows PlotSim` não imprime nada. `git status` limpo com novo wip, apague `ui/node_modules` e `ui/dist`. Não deixe processo seu.

Resposta final, mesmo que falte algo: item a item, testes com grupo de falha prévia, linha do CI, capturas com caminho (lidas e não lidas) e o que não foi verificado. Se a física reprovar ou um item exigir decidir comportamento além do escrito, pare esse item e relate.
