# lateral-impressoras: escolher as filas de cada documento na barra lateral

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master` (base 762e40e). Faça commits wip; nunca dê push nem squash. Não altere nada em E:\PlotS fora do seu worktree.

## Ambiente
- Use em todo comando cargo `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/lateral"` e `CARGO_INCREMENTAL=0`: o disco E: está quase cheio. Logs e capturas ficam em `target/` do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- Se o sistema matar o build por falta de memória, repita uma vez; se morrer de novo, apague a pasta `debug/incremental` do target e tente mais uma vez. Só então relate.
- Em `ui/`, rode `npm ci` para instalar o que já está no lock. **Sem dependências novas de npm nem de Cargo**, salvo as internas `plots-windows`, `plots-perfis` e `plots-core` em `src-tauri`. Se o sistema negar algo, pare e relate; não tente contornar.
- CI depois da última edição: `cargo fmt --all --check`, `cargo test --workspace --locked`, e em `ui/` `npm test` e `npm run build`. Grave em `target/ci.log`, seguido de `CI_EXIT <código>`. Nunca faça wip depois de um CI vermelho.
- Log de falha prévia: `target/fail-before.log`, com execuções reais. Classifique cada teste novo.
- Limpeza conforme "Limpeza constante do disco" em AGENTS.md; capturas em `target/shots/` ficam para a revisão de interface.

## Segurança
**Este lote não envia nada a nenhuma fila** e não muda nenhuma impressora. Encerre só os seus próprios PIDs, por número exato; não deixe app rodando. Não use diálogos nativos nos testes nem envie teclas a outras janelas. Nunca grave no `%APPDATA%` real durante os testes: o app aceita `PLOTS_PERFIS` como caminho alternativo do arquivo de perfis, que você cria neste lote.

## Navegação
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; o grafo pode não cobrir a interface, e nesse caso leia os arquivos abaixo. Não rode `graphify update`.
- `ui/src/App.tsx`, `ui/src/tabs.ts`, `ui/src/components/Sidebar.tsx`, `ui/src/i18n.ts`, `ui/src/theme.css`
- `src-tauri/src/main.rs`, `src-tauri/src/documento.rs`
- `plots-windows/src/lib.rs`: `listar_filas`, `Estado`
- `plots-perfis/src`: `carregar`, `obter`
- Scripts de uso: não estão no repositório; escreva os seus em `target/e2e/`, operando o app por CDP com `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<porta livre>` e `WEBVIEW2_USER_DATA_FOLDER=<pasta em target/>`, com `Input.dispatchMouseEvent`, `Input.dispatchKeyEvent` e `Page.captureScreenshot`, pelo `WebSocket` nativo do Node. Confira que a porta é do seu processo.

## Fontes
`plan/11-lateral-impressoras.md`, `plan/14-padrao-visual-e-idiomas.md`, `plan/09-janela-e-abas.md` e `plan/03-simulacao.md`, no worktree, e a norma em `E:\PlotS\plan\estilo-ui-ux.md`. Antes de decidir detalhes visuais, carregue a skill `ui-design-craft`. Não abra `E:\PlotS\codex-session-*.md`.

## Regras sobre identificadores do plano
Código, comentários e testes descrevem comportamento com palavras próprias: sem IDs de requisito, nome de lote, "auditoria" ou "brief". Nada de marca fixa no código de produção.
Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh 762e40e src-tauri ui`

## Escopo
Primeiro registre no relatório o que existe hoje.

1. **Comando Rust `listar_impressoras`:** devolve as filas do Windows, com nome, estado e se têm perfil, mais `Simulada — Traseira` e `Simulada — Gaveta`. Fora do Windows, só as simuladas. Roda fora da thread da interface. Falha ao ler perfis ou filas vira aviso na tela, sem derrubar a lista. O comando só lê. Uma ação "Atualizar lista" chama de novo.
2. **Linhas de impressora** conforme o plano: caixa de seleção com área de clique de 24×24, nome com reticências e `title`, e à direita o seletor de modo. Estado com ponto e texto; nunca só cor. A especificação do plano vale: modos "Traseira ↑ face para cima" e "Gaveta ↓ face para baixo"; fila sem perfil mostra "Calibrar…". Neste lote, "Calibrar…" só explica em diálogo como calibrar pelo comando `plots calibrar`, sem executar nada. Fila com perfil salvo mostra o modo que o perfil implica, somente leitura, com `title` explicando. Impressoras simuladas mostram o modo fixo.
3. **Seleção por documento:** cada aba guarda as filas marcadas; aba nova começa sem nenhuma marcada; trocar de aba troca a seleção. Sem documento aberto, as caixas ficam desabilitadas, com explicação.
4. **Lote:** campo numérico por aba, padrão 10, de 1 a 50, com resumo em mono, como "24 pág · 2 lotes", quando o total for conhecido. O total de páginas vem de um comando Rust novo, `contar_paginas(caminho)`, que passa pela barreira de `PdfOrigem::carregar`; PDF recusado mostra o motivo na aba.
5. **Botão primário** "Imprimir frente" com a contagem ("Imprimir frente (2)"). Neste lote, **desabilitado** com `title` explicando que o envio ainda não está disponível, porque o envio real não existe. Não simule o envio aqui.
6. **Reavaliação:** confira a largura de 240 px e a altura mínima com o conteúdo real, mantendo o botão sempre visível, com rolagem só na lista. Se a linha não couber, proponha a menor mudança e registre. Libere o menu de contexto no campo de lote e decida o que Ctrl+P faz: por ora, nada, com bloqueio mantido.
7. **Textos** via `t()` em pt-BR e en.

## Regras
- **Testes:** Rust para `listar_impressoras` com lista fornecida, perfil por nome exato, falha de perfis e `contar_paginas` com PDF válido, inválido e criptografado, gerados no teste. Vitest para seleção por aba, resumo do lote, limites do campo e estado do botão.
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes no Rust.
- **Contraste:** meça cada par novo com `contrast.py` nos dois temas e liste. Nada de `--fg-faint` sobre `--panel` ou `--panel-hover`.

## Capturas e teste de uso
App real, com mouse e teclado por CDP. Em `target/shots/`, nos dois temas: lista com filas e simuladas; selecionada, hover e foco; sem documento; duas abas com seleções diferentes; nome longo; janela no mínimo; diálogo "Calibrar…"; PDF recusado; inglês; cores forçadas. Leia cada imagem.

## Relatório
O que existia antes, cada item com testes e grupo de falha prévia, linha do CI, conferência de caminhos, autoverificação, pares de contraste, capturas, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
