# avisos-no-topo: cartões viram avisos

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Comportamento: `E:\PlotS\plan\25-avisos-no-topo.md`. Leia também `plan/24-impressao-sem-lote.md`, as regras comuns de `plan/briefs/coordenacao-paralela-2.md` e a norma `E:\PlotS\plan\estilo-ui-ux.md`, com a skill `ui-design-craft`. Build e saídas em `%LOCALAPPDATA%/plots-build/avisos-no-topo`, com `CARGO_TARGET_DIR` ali, `CARGO_INCREMENTAL=0`, `-j 4`; porta CDP 9353. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`. Limite de tempo em todo comando.

**Outro agente ao mesmo tempo:** um depurador mexe em `src-tauri/src/trabalhos*.rs`, `plots-windows/`, `plots-print/` e no estado dos trabalhos. Mexa neles o mínimo. Seu mapa: `ui/src/components/`, `ui/src/App.tsx`, `ui/src/i18n.ts`, `ui/src/cartoes*.ts` (só apresentação), `src-tauri/src/impressoras.rs` e `src-tauri/Cargo.toml` (opção de compilação), `scripts/empacotar.mjs`.

## Itens

1. **Tirar os cartões da lista de impressoras** (`Sidebar.tsx`, `CartaoImpressao.tsx`). Reaproveite `cartoes.ts` para o texto: a lógica de estado não muda. A figura do gesto continua na confirmação do verso.
2. **Aviso no topo:** faixa no topo da área da janela, abaixo da barra de título, `role="status"` (erro: `role="alert"`). Some em 6 s, ou fica até fechar, se for erro ou pedir ação. Não rouba foco. Vários avisos empilham no máximo 3, e o resto só no painel. Movimento reduzido: sem animação.
3. **Painel de avisos:** ícone `Bell` ao lado do ícone de histórico, com contagem de não lidos. Abre como o histórico abre. Lista da sessão, do mais novo para o mais antigo, com hora, impressora e o texto completo do cartão, incluindo o gesto. Limite de 200, em memória; não grava em disco.
4. **Sem botão de imprimir nos avisos.** "Cancelar trabalho" e "Reimprimir" ficam só no painel, em trabalho em andamento ou com erro, com as mesmas confirmações de hoje. Nenhum envio pode perder o caminho de cancelar: confira cada estado que tinha botão no cartão e diga no relatório onde ele foi parar.
5. **Fechar a janela e a aba** continua com as mesmas confirmações, e qualquer estado que dependia do cartão para aparecer (recusa, papel, documento mudou) tem de aparecer como aviso.
6. **Opção de compilação sem simuladas:** feature Cargo `simuladas`, ligada por padrão em `plots-app`; sem ela, `src-tauri/src/impressoras.rs` não lista as três simuladas, e `ui/` não oferece nada que dependa delas. `scripts/empacotar.mjs` passa `--no-default-features`, ou o equivalente. Teste nos dois modos, e o CI segue com a padrão. Confirme que a interface não quebra com lista sem simuladas, incluindo lista vazia.
7. **Botão Calibrar:** ao lado da engrenagem, em cada fila em que hoje o seletor oferece "Calibrar", ícone com `title`. Mesmas regras de hoje. A opção do seletor fica.

## Testes e capturas

Vitest: o redutor de avisos, tempo, limite, não lidos, erro que fica. Atualize os testes de cartão sem perder a garantia: liste no relatório cada teste apagado e o motivo. App real por CDP, com simuladas: frente e verso com duas impressoras, erro, cancelar; dois temas, inglês, janela mínima, cores forçadas por emulação. Leia cada imagem, prefixo `av-`, em `saidas/shots`. Filas reais só "Microsoft Print to PDF" e nada a impressoras reais.

## Final

CI em um único log `saidas/ci.log`: `cargo fmt --all --check`, `cargo test --workspace --locked -j 4`, em `ui/` `npm ci`, `npm test`, `npm run build`, 0 `__plotsE2e` em `ui/dist`, `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR="%LOCALAPPDATA%/plots-build/pdfium/chromium-8125/bin"`, terminando com `CI_EXIT <código>`. Autoverificação com a sua base. Wip limpo, apague `ui/node_modules` e `ui/dist`. Resposta final, mesmo que falte algo: item a item, testes com falha prévia e apagados, linha do CI, capturas lidas e o que não foi verificado.
