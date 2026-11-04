# Impressão sem lote: o que falta na tela

Responda em português brasileiro. Worktree existente `E:\PlotS\.claude\worktrees\agent-acbf7ba0ce97216fc`, commit `85242f2`, sem criar outro. Build e saídas em `%LOCALAPPDATA%/plots-build/frentes-primeiro`, com `CARGO_TARGET_DIR` ali, `CARGO_INCREMENTAL=0`, `-j 4`; porta CDP 9352. Rode `npm ci` em `ui/`. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`. Limite de tempo em todo comando. Só impressoras simuladas e "Microsoft Print to PDF". Testes nunca tocam `%APPDATA%`.

**Faça todos os itens, um por um, e depois abra o aplicativo real por CDP.** O agente anterior parou sem abrir o aplicativo: este é o trabalho principal. As frases exatas e os detalhes estão em `E:\PlotS\plan\briefs\brief-sem-lote-revisoes.md`, seções C e D e item 8, e valem como escritas, com estas decisões:

- **Item 8:** faça por inteiro. Sem `aoSair`, o texto digitado fica. O estado inválido sai do campo para o rodapé, e o botão de envio fica desabilitado até corrigir. O envio nunca sai com um número diferente do que está na tela.
- **Item 7:** o campo já foi alterado, mas ninguém o viu. Confira na tela.
- **Item 9:** falta só o `aria-label` por extenso: "em 2 impressoras" / "on 2 printers".
- **Item 10:** dica de estado, com os textos da seção C do brief citado. A escolha nunca muda sozinha.
- **Item 11:** resumo com cópias.
- **Item 12:** janela mínima, medida em 640×420.
- **Item 13:** cartão de frentes prontas em "Em andamento", com o botão à vista. O rótulo "Frentes concluídas na fila" já foi trocado.
- **Item 14:** frases do automático com cópias.
- **Item 15:** posição do aviso de quantidades diferentes. O texto já foi trocado.
- **Item 16:** cartão de documento mudado, parado.

Não mexa em `plots-core`, nem no plano físico. Se precisar decidir comportamento além do escrito, pare esse item, siga para o próximo e relate.

## Capturas

A lista da seção "Capturas" do brief citado, por CDP, em `saidas/shots`, com prefixo `t5-`. Abra e leia cada imagem. As cenas prontas estão em `cena/` e em `saidas/`: adapte os textos. Meça com `contrast.py` (skill `ui-design-craft`) qualquer par novo.

## Final

CI em um único log `saidas/ci.log`: `cargo fmt --all --check`, `cargo test --workspace --locked -j 4`, em `ui/` `npm ci`, `npm test`, `npm run build`, 0 `__plotsE2e` em `ui/dist`, `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR="%LOCALAPPDATA%/plots-build/pdfium/chromium-8125/bin"`, terminando com `CI_EXIT <código>`. Autoverificação `sh E:/PlotS/.claude/team/selfcheck.sh 903a46e ui src-tauri plots-core plots-print plots-pdf plots-windows PlotSim` não imprime nada. `git status` limpo com novo wip, apague `ui/node_modules` e `ui/dist`. Não deixe processo seu.

Resposta final, curta: item a item (feito, parcial ou não feito), testes com grupo de falha prévia, linha do CI, capturas lidas e não lidas, com caminho, e o que não foi verificado.
