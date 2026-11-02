# fase2-folhas, correção 1: folhas finais na ordem do documento

Responda em português brasileiro. Trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-ac9bd4b6dd9dbcd79` (branch `worktree-agent-ac9bd4b6dd9dbcd79`); não crie outro. Faça um commit wip ao terminar; nunca dê push nem squash.

## Ambiente
- Deixe CARGO_TARGET_DIR sem valor, para usar `target/` do worktree. Um comando pesado por vez.
- Depois da última edição, rode uma vez `cargo fmt --all --check` e `cargo test --workspace --locked`, gravando em `target/ci.log`, seguido de `CI_EXIT <código>`.
- Código e testes descrevem comportamento: nada de IDs de requisito, nome de lote ou "brief".
- Autoverificação, não pode imprimir nada: `sh E:/PlotS/.claude/team/selfcheck.sh cb5be25 plots-cli plots-core plots-print plots-pdf`
- Use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json` antes do primeiro grep ou leitura de código.
- Apague os arquivos temporários que criar e liste o que removeu. Não instale nada. Não envie nada a impressoras.

## Fonte
`E:\PlotS\plan\03-simulacao.md`.

## O que corrigir
Hoje, `gerar-pdf --folhas-finais` mostra a primeira folha como `2 | 1`, tratando como frente a face da passada 1. O resultado físico pedido é o maço 1|2, 3|4…

1. Em cada folha do PDF, a página de número menor do documento fica à esquerda e a maior à direita, nas duas impressoras e nas duas ordens: `1 | 2`, `3 | 4`. Isso é o mesmo critério da listagem de `simular`. No total ímpar, a última folha é `5 | branca`.
2. Quem decide quais páginas estão em cada folha, e se o verso está de cabeça para baixo, continua sendo o simulador. Não reconstrua a folha a partir dos números das páginas: só escolha o lado.
3. As folhas continuam de cima para baixo no maço. Com a recolocação correta, a primeira folha do arquivo é a `1 | 2`.
4. A largura continua sendo a soma das faces, e a altura, a maior delas, com as faces na posição nova.

## Testes
Ajuste os testes de folhas finais para exigir a ordem acima e registre a falha antes de corrigir em `target/fail-before-fix.log`. Cubra as duas impressoras, as duas ordens, total par e ímpar, e faces de tamanhos diferentes.

## Relatório
O que mudou, testes, linha `CI_EXIT`, autoverificação vazia, limpeza e o que não foi verificado. Termine com `git status` limpo.
