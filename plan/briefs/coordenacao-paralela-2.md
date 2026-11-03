# Coordenação do segundo grupo de quatro agentes

Base de todos: `f5d7707`. Cada agente mexe só nos arquivos do seu lote. Se precisar de outro arquivo, não edite: relate ao coordenador. Quem integra em `master`, na ordem abaixo, é só o coordenador, com rebase e CI a cada entrada.

| Ordem | Lote | Porta CDP | Pode mexer em | Não pode mexer em |
|---|---|---|---|---|
| 1 | `instalador` | 9344 | `Cargo.toml` da raiz e de cada crate (só `license` e `[profile.release]`), `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `ui/package.json`, `ui/package-lock.json` (só a CLI do Tauri e `license`), `scripts/`, `THIRD-PARTY-NOTICES` (novo), `README.md` (novo) | `ui/src/`, `src-tauri/src/`, `plots-*/src/` |
| 2 | `gesto-e-spooler` | 9342 | `plots-windows/`, `plots-print/`, `plots-cli/`, `ui/src/components/CartaoImpressao.*`, `ui/src/components/FiguraGesto.*` (novo), `ui/src/cartoes*.ts`, em `src-tauri/src/trabalhos.rs` só estados e avisos do cartão, e o CSS de `.girando` | `plots-core/`, `plots-perfis/`, `Sidebar.*`, `tabs.ts`, `comandos_trabalho.rs`, `Pedido` |
| 3 | `intervalo-paginas` | 9341 | `plots-core/src/lib.rs`, `src-tauri/src/comandos_trabalho.rs`, em `trabalhos.rs` só `Pedido`, `preparar_*` e `resumo_do_envio`, `ui/src/tabs*.ts`, `ui/src/confirmacao*.ts`, `ui/src/App.tsx`, `ui/src/components/Sidebar.*`, `ui/src/intervalo*.ts` e o componente do bloco (novos) | `plots-windows/`, `plots-perfis/`, `CartaoImpressao.*`, `distribuicao.rs` |
| 4 | `varias-impressoras` | 9343 | `plots-core/src/distribuicao.rs`, `plots-perfis/`, `src-tauri/src/comandos_trabalho.rs`, `src-tauri/src/grupos.rs` (novo), em `trabalhos.rs` só `Pedido`, `preparar_*` e `resumo_do_envio`, `src-tauri/src/main.rs` (só registro de comando), `ui/src/tabs*.ts`, `ui/src/confirmacao*.ts`, `ui/src/App.tsx`, `ui/src/components/Sidebar.*`, `ui/src/grupos*.ts` e o componente do bloco (novos); em `CartaoImpressao.tsx` e `cartoes.ts`, só a linha de dados do lote | `plots-windows/`, `FiguraGesto.*` |

`intervalo-paginas` e `varias-impressoras` dividem arquivos de propósito. Para o rebase ficar pequeno, sigam este contrato:

- **Rust:** `iniciar_impressao` e `resumo_do_envio` ganham argumentos no fim, cada lote o seu: `intervalo: Option<IntervaloPaginas>`; `modo: ModoDistribuicao`. `Pedido` ganha um campo por lote. Mantenha a mudança em `Pedido::novo` em poucas linhas.
- **Aba:** `intervalo` e `modo`, um campo e uma ação do redutor cada.
- **Sidebar:** cada lote põe o seu bloco em um componente próprio, em arquivo novo, e acrescenta uma única linha em `Sidebar.tsx`. O intervalo vem antes do tamanho do lote; o modo, logo depois do tamanho do lote e antes do botão. Com duas ou mais impressoras marcadas, o modo não cede espaço na janela curta.
- `varias-impressoras` será rebaseado sobre `intervalo-paginas`. Na divisão, o intervalo é aplicado primeiro, e os lotes do intervalo é que são repartidos.

## Regras comuns

- **`ui/src/i18n.ts`:** todos podem acrescentar chaves, só com o prefixo do lote (`gesto.*`, `spooler.*`, `intervalo.*`, `modo.*`, `grupo.*`), em bloco próprio, no fim de cada idioma. Não reordene nem altere as chaves existentes. Todo texto em pt-BR e en.
- **Dependências:** nenhum lote traz dependência Rust nova, e `Cargo.lock` não muda. A única npm nova é `@tauri-apps/cli`, do lote `instalador`. Se achar que precisa de outra, relate.
- **`plan/`:** agentes não editam. O coordenador atualiza planos e registros.
- **Compilação:** `CARGO_TARGET_DIR="%LOCALAPPDATA%/plots-build/<lote>"`, `CARGO_INCREMENTAL=0`, `-j 4`. Vários agentes compilam ao mesmo tempo: se o sistema matar um build por memória, espere 2 minutos antes de repetir.
- **Disco E:** vigie com `df -h /e`; há cerca de 2 GB livres. Abaixo de 500 MB, pare e relate. **Nenhuma saída no E::** logs, capturas, PDFs e instaladores vão para `%LOCALAPPDATA%/plots-build/<lote>/saidas`. Apague `ui/node_modules` e `ui/dist` do seu worktree ao terminar, depois do CI final.
- **CI, nesta ordem, depois da última edição:** `cargo fmt --all --check`, `cargo test --workspace --locked -j 4`, e em `ui/` `npm ci`, `npm test`, `npm run build`; 0 ocorrências de `__plotsE2e` em `ui/dist`. Grave em `saidas/ci.log`, terminando com `CI_EXIT <código>`. Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh f5d7707 <pastas>` não imprime nada.
- **Log de falha prévia:** `saidas/fail-before.log`, com execuções reais. Todo teste novo é falha prévia registrada, API nova ou guarda.
- **Graphify:** antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.
- **Sem identificadores do plano** em código, testes e comentários. Código e testes descrevem o comportamento.
- **Nunca travar:** nada de `unwrap`, `expect` ou indexação sem checagem fora de testes. Número vindo de entrada é hostil.
- **Processos:** encerre só os seus próprios PIDs, por número exato. Use a sua porta e uma pasta própria do WebView2. Se uma ação for negada pelo sistema de permissões, não contorne nem peça a outro agente: relate.
- **Capturas:** só por CDP (`Page.captureScreenshot`), nunca a tela do Windows. Movimento reduzido só por `Emulation.setEmulatedMedia`, sem mudar o Windows. Scripts de cena de exemplo: `%LOCALAPPDATA%/plots-build/cenas/`. A `pdfium.dll` está em `%LOCALAPPDATA%/plots-build/pdfium/chromium-8125/bin/`; use `PLOTS_PDFIUM_DIR` para apontar para ela.
- **Impressoras:** envie só às simuladas e à fila "Microsoft Print to PDF", com arquivo de saída. Nada a impressoras reais.
- **`%APPDATA%` real:** nunca. Use `PLOTS_PERFIS` com arquivo temporário.
- **PDFs do usuário** em `C:/Users/Kaio Luan/Downloads`: somente leitura, só para medir; sem nomes nem conteúdo em relatório, repositório, testes ou capturas. Capturas usam PDFs gerados por você.
- **Fontes proibidas:** não abra `E:\PlotS\codex-session-*.md`. Norma visual: `E:\PlotS\plan\estilo-ui-ux.md`, com a skill `ui-design-craft` carregada antes de decisões visuais. Meça cada par novo de cores com `contrast.py` da skill.
- **Commits:** só wip no worktree, sem push nem squash. Termine com `git status` limpo.
- **Relatório:** grave em `%LOCALAPPDATA%/plots-build/<lote>/relatorio.md` e devolva o mesmo texto na mensagem final: o que já existia, cada item com testes e grupo de falha prévia, linha do CI, autoverificação, contraste, capturas com caminho, textos novos, limpeza, o que não foi verificado, caminho do worktree e nome do branch.
