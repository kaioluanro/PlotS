# Brief: chave de boca e painel de opções da impressora

Leia `E:\PlotS\AGENTS.md`, `.claude/team.json`, `.claude/skills/craft-team/SKILL.md` e
`.claude/agents/plots-implementer.md` antes de agir; siga o método do papel. Não faça push nem
squash. Trabalhe no worktree isolado em que você foi iniciado.

**Graphify:** antes do primeiro grep ou leitura de código, `graphify query "<pergunta>"
--graph E:/PlotS/graphify-out/graph.json`. Ao terminar, `graphify update .` no worktree.

Base: `master` (commit dos planos, depois de `145a152`). Build em
`%LOCALAPPDATA%\plots-build\opcoes-impressora\target`, com `CARGO_TARGET_DIR`,
`CARGO_INCREMENTAL=0`, `-j 4`; `PLOTS_PDFIUM_DIR=%LOCALAPPDATA%\plots-build\pdfium\chromium-8125\bin`;
`npm ci` em `ui/`. CDP na porta **9374**.

**Capturas:** mudança visível. Capture o painel aberto, com opções e no estado vazio, nos temas
escuro e cinza, em 640×420 e no tamanho normal, em `saidas/`. Use cliques/teclas reais pelo CDP;
nada de tela do Windows nem SendKeys. Não toque nas filas `PlotSim - IMP1`/`IMP2` nem em
processos do usuário. Não use o PDF assinado. Use `PLOTS_PERFIS` temporário; não altere
`%APPDATA%\plots\perfis.toml`.

## 1. Pesquisa e decisão

Leia `plan/research/pdfjs-printing.md` (§6 e §12) e `plan/28-opcoes-da-impressora.md`. A
conclusão: o pdf.js não enumera impressoras; o PlotS usa a camada nativa. O painel mostra só
**bandeja**, **resolução/qualidade** e **cor** — o que o PlotS não força. Papel, face, cópias,
escala, orientação e uma página por folha continuam decisão do envio.

## 2. Backend (`plots-windows`)

- Em `spooler.rs`/`preferencias_driver.rs`, leia `DeviceCapabilitiesW` para a fila:
  `DC_BINS` + `DC_BINNAMES`, `DC_ENUMRESOLUTIONS`, `DC_COLORDEVICE`. Use `OpenPrinterW`/o handle
  já existente. `DeviceCapabilitiesW` vem de `windows::Win32::Storage::Xps` como no código atual.
- Tipos públicos em `plots-windows/src/lib.rs` (sem depender de serde):
  - `OpcoesDaFila { bandejas: Vec<Bandeja>, resolucoes: Vec<Resolucao>, cor: Option<Cor>, atual: Escolhas }`
  - `Bandeja { valor: i16, nome: String }`, `Resolucao { x: i32, y: i32 }`,
    `Cor { Colorida, Monocromatica }`, `Escolhas { bandeja: Option<i16>, resolucao: Option<(i32,i32)>, cor: Option<Cor> }`.
  - `opcoes_da_fila(fila: &str) -> Result<OpcoesDaFila, ErroPreferencias>`
  - `aplicar_opcoes_da_fila(fila: &str, salvo: Option<&[u8]>, escolhas: &Escolhas) -> Result<Vec<u8>, ErroPreferencias>`
- `aplicar_...` parte do ajuste salvo validado ou do padrão (`ler_padrao`/`validar_salvo`),
  marca só os campos escolhidos (`DM_DEFAULTSOURCE`, `DM_COLOR`, `DM_PRINTQUALITY` +
  `DM_YRESOLUTION`), valida com `DocumentPropertiesW` e devolve os bytes. Campo não escolhido
  não é tocado. Nunca marque `DM_DUPLEX`, `DM_PAPERSIZE`, `DM_COPIES`, `DM_SCALE`,
  `DM_ORIENTATION` ou `DM_NUP`.
- Se o driver recusar a fusão, devolva erro sem bytes; o chamador não grava.
- Sem opções, as listas ficam vazias e `cor` é `None`; não invente opção.
- Para resolução, use `dmYResolution` e `dmPrintQuality` com o valor em dpi do par
  `DC_ENUMRESOLUTIONS`; se o driver recusar, o erro sobe. Mantenha a resolução atual lida do
  DEVMODE como `Option` (só se o campo estiver marcado).
- Testes puros para o parsing dos arrays e para a montagem do DEVMODE com um driver falso
  (`DriverDaFila` já é testável). Teste que os campos forçados não são marcados e que a recusa
  não grava.

## 3. Tauri (`src-tauri`)

- Em `preferencias.rs`, comandos:
  - `opcoes_da_impressora(fila: String) -> Result<OpcoesDaImpressora, FalhaPreferencias>`
  - `aplicar_opcoes_da_impressora(fila: String, escolhas: EscolhasDaImpressora) -> Result<(), FalhaPreferencias>`
  - `voltar_ao_padrao_da_fila` já existe; o painel usa.
- Estruturas serde em camelCase para o frontend (`bandejas`, `resolucoes`, `cor`, `atual`).
- `aplicar` lê `perfis.ajuste_do_driver(fila)`, chama `plots_windows::aplicar_opcoes_da_fila`,
  grava com `atualizar`/`definir_ajuste_do_driver` sob a trava `GRAVACAO_DE_PERFIS`. Erro do
  driver vira `FalhaPreferencias` (código novo `opcao_recusada` ou o existente mais próximo).
- Registre os dois comandos em `main.rs`.
- Testes: aplicar grava no perfil temporário; recusa não grava; fila sumida devolve
  `fila_sumiu`; campos forçados não aparecem.

## 4. UI

- `ui/src/opcoes-impressora.ts`: tipos, `lerOpcoesDaImpressora`, `aplicarOpcoesDaImpressora`,
  normalização de códigos de falha.
- `ui/src/components/OpcoesDaImpressoraDialog.tsx` + CSS module:
  - Radix `Dialog` como `ConfirmDialog`; título "Opções da impressora" + nome; descrição com a
    nota de que papel, face, cópias, escala e orientação continuam com o PlotS.
  - Seções só para o que veio: Bandeja (select), Resolução (select "300 × 300 dpi"), Cor
    (radio Colorida/Monocromática). Cada uma com "Padrão da impressora" quando o campo atual é
    `null`.
  - Rodapé: "Usar padrão" (só se a fila tem ajuste salvo), "Cancelar" (fecha), "Aplicar"
    (grava; `aria-busy` durante). Erro no corpo, `role="alert"`; estado vazio com a frase e um
    botão "Abrir a engrenagem" que chama o callback da engrenagem.
  - Foco inicial no primeiro controle; ao fechar, volta para a chave de boca; Esc cancela.
  - Corpo com `overflow-y:auto`; largura `min(460px, calc(100vw - 48px))`; em 640×420 os botões
    ficam visíveis.
- `Sidebar.tsx`: importe `Wrench`; adicione o botão ao lado de `Settings`, com
  `aria-label={t("opcoes.rotulo", { nome })}`, `title={t("opcoes.titulo")}`,
  `aria-haspopup="dialog"`, `onClick={() => aoAbrirOpcoes(impressora.nome)}`. Nova prop
  `aoAbrirOpcoes`; não use `disabled`, preserve o foco.
- `App.tsx`: estado `filaDasOpcoes: string | null`; renderize o diálogo fora da lateral; ao
  aplicar/voltar ao padrão, chame `preferencias.recarregar()` (adicione ao hook
  `usePreferencias` retornando `ler`) e atualize a lista se a fila sumiu.
- `i18n.ts`: chaves pt-BR e en (`opcoes.*`); sem marca de terceiro. Use os tokens de
  `theme.css`; nada de cor literal.
- Testes Vitest/RTL: botão só em fila real, nome acessível, abre; painel com opções, vazio,
  aplica, cancela, usar padrão, erro, foco; 640×420 sem corte.
- Uso real CDP: abrir a chave de boca em "Microsoft Print to PDF", conferir as opções que o
  driver declarar (se nenhuma, o estado vazio), aplicar uma resolução e ver o `perfis.toml`
  temporário mudar; reabrir e conferir o valor; "Usar padrão" limpa. Capturas.

## 5. Final

- CI em um log único `saidas/ci.log`: `cargo fmt --all --check`,
  `cargo test --workspace --locked -j 4`, em `ui/` `npm ci`, `npm test`, `npm run build`,
  conferência de ganchos, terminando em `CI_SUCCESS: cargo test --workspace` e `CI_EXIT 0`.
- Wip commit com árvore limpa; `node_modules`/`dist`/`target` do lote apagados; nenhum
  `perfis.toml` do usuário no worktree ou no commit.
- Não mexa em `plan/`, no PlotSim nem no zip.
- Resposta final item a item: o que foi lido do driver, como aplica, falha prévia, provas de uso
  com capturas, linha do CI e o que não foi verificado.
