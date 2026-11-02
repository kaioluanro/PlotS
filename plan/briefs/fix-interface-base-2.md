# interface-base, correção 2: a janela sempre fecha

Responda em português brasileiro. Você é um agente novo: trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-a6db429760e6413f3` (branch `worktree-agent-a6db429760e6413f3`, commit `ea8075d`); não crie outro nem mexa em E:\PlotS fora dele. Faça um commit wip ao terminar; nunca dê push nem squash.

## Ambiente
- Use `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/interface"` e `CARGO_INCREMENTAL=0`. Um comando pesado por vez, porque outro agente também compila.
- CI depois da última edição: `cargo fmt --all --check`, `cargo test --workspace --locked`, e em `ui/` `npm test` e `npm run build`. Grave em `target/ci.log`, seguido de `CI_EXIT <código>`.
- Autoverificação, não pode imprimir nada: `sh E:/PlotS/.claude/team/selfcheck.sh d4b804b src-tauri ui`.
- Sem identificadores do plano em código ou testes. Sem dependências novas.
- Encerre só os seus próprios PIDs, por número exato. Não deixe nenhum app rodando. Não envie teclas a outras janelas. Não use diálogos nativos nos testes.
- Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; o grafo pode não ter a interface, e nesse caso leia os arquivos abaixo.

## Onde olhar
`ui/src/App.tsx` (`onCloseRequested`, `assinar`), `ui/src/tabs.ts` (`abasEmImpressao`), `ui/src/confirmacao.ts`, `ui/src/lote.ts`, `ui/src/lote.test.ts`, `ui/src/i18n.ts`, `src-tauri/src/main.rs`. Os scripts de uso estão em `target/e2e/`.

## O problema
No Tauri 2, enquanto existe um listener JS de `tauri://close-requested`, o Rust sempre chama `prevent_close()`, e quem fecha é o JS. O app mantém esse listener a vida toda. Se a webview travar, nem o × nem o Alt+F4 fecham a janela, mesmo sem impressão em andamento.

## Escopo
1. **Listener só quando preciso.** Registre `onCloseRequested` somente enquanto houver aba com impressão em andamento. Sem impressão, o Rust fecha sem depender do JS. Extraia para função pura a decisão de quando o listener existe e teste.
2. **Proteção no Rust.** Com impressão em andamento, se o JS não responder ao pedido de fechamento dentro de um prazo curto, destrua a janela. Prazo: 5 segundos. O JS responde assim que recebe o pedido, por um comando que confirma que está vivo, mesmo que o diálogo fique aberto depois disso. Assim o diálogo pode ficar aberto o tempo que o usuário quiser, e só uma interface travada dispara o prazo. Decisão do prazo e do estado em função pura testada no Rust, sem panic nem espera que bloqueie a thread principal.
3. **Foco inicial.** Confira no app que `document.activeElement` é "Manter janela aberta" depois do × da janela e depois de um `WM_CLOSE` real. Informe o resultado. Se não for, corrija.
4. **Ponto final.** Troque `erro.varios` por "{n} arquivos não abriram." e "{n} files did not open.", com um teste do texto combinado com `lote.cortado`, nos dois idiomas.

## Teste de uso
App real, pelos scripts de `target/e2e/`: sem impressão, × e `WM_CLOSE` fecham com código 0; com impressão, o diálogo abre e fica aberto além de 5 segundos sem a janela fechar; ESC mantém; confirmar fecha. Para a interface travada, tente simular com uma espera longa no JS, pela porta de teste, e relate se a janela fechou após o prazo. Se não conseguir simular, diga isso.

## Relatório
O que mudou, testes e grupo de falha prévia, resultado do foco, resultado da interface travada, linha do CI, autoverificação vazia, textos, limpeza e o que não foi verificado. Termine com `git status` limpo.
