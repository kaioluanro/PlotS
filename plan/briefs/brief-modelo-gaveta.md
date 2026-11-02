# modelo-gaveta: o simulador da gaveta deve corresponder à impressora descrita

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master` (base fc0888b). Faça commits wip; nunca dê push nem squash. Não altere nada em E:\PlotS fora do seu worktree.

## Ambiente
- Deixe CARGO_TARGET_DIR sem valor, para usar `target/` do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- Se o sistema matar o build por falta de memória, repita uma vez; se morrer de novo, apague `target/debug/incremental` e tente mais uma vez. Só então relate.
- Depois da última edição, rode uma vez `cargo fmt --all --check` e `cargo test --workspace --locked`, gravando em `target/ci.log`, seguido de `CI_EXIT <código>`. Nunca faça wip depois de um CI vermelho.
- Log de falha prévia: `target/fail-before.log`, com execuções reais.
- Sem dependências novas: Cargo.toml e Cargo.lock não mudam. Não envie nada a impressoras.
- Limpeza conforme "Limpeza constante do disco" em AGENTS.md: mantenha `target/` do worktree e apague o resto do que criar.

## Navegação
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; depois leia só as linhas localizadas e confirme no worktree. Não rode `graphify update`.
- `plots-print/src/lib.rs`: `Mecanica`, `Mecanica::gaveta`, `Mecanica::traseira`, `Simulador`, `conferir_folhas`, tabela de mecânicas, laço exaustivo
- `plots-cli/src/lib.rs`: `ImpressoraVirtual::perfil`, `ImpressoraVirtual::mecanica`, recolocação usada em `simular`

## Fontes
`plan/01-nucleo-ordenacao.md`, `plan/03-simulacao.md`, `plan/04-calibracao.md` e `plan/11-lateral-impressoras.md`, no worktree. Não abra `E:\PlotS\plan\estilo-ui-ux.md` nem `E:\PlotS\codex-session-*.md`.

## Regras sobre identificadores do plano
Código, comentários e testes descrevem comportamento com palavras próprias: sem IDs de requisito, nome de lote, "auditoria" ou "brief".
Autoverificação, não pode imprimir nada: `sh E:/PlotS/.claude/team/selfcheck.sh fc0888b plots-cli plots-core plots-print plots-pdf`

## O problema
`Mecanica::gaveta()` tem `saida_no_topo = false` (cada folha sai embaixo da pilha) e `entrada_pelo_topo = false` (a entrada puxa a de baixo). A especificação descreve uma laser com entrada por gaveta inferior e saída no topo, face impressa para baixo: cada folha cai em cima da anterior, e a gaveta alimenta a folha de cima. Os testes podem estar passando porque dois comportamentos irreais se compensam.

## Escopo
1. **Reproduza primeiro.** Escreva um teste que descreva a pilha física da gaveta conforme a especificação e registre a falha antes de mudar o código.
2. **Corrija `Mecanica::gaveta()`** para o comportamento descrito. Qual face recebe tinta depende da máquina e é assunto da calibração: escolha um valor, justifique em comentário curto e diga no relatório o que muda se a máquina real fizer o contrário.
3. **Encontre o perfil e o gesto coerentes** que produzem o maço final 1|2, 3|4… sem folha de cabeça para baixo, em total par e ímpar e nas duas ordens. Defina em comentário de que lado o maço final é lido quando sai com a face para baixo. Use a recolocação real em `simular`, em vez de manter uma que só compense o modelo.
4. **Mantenha a regra da especificação:** no modo gaveta, face para baixo não inverte a frente (`passada1_inversa = false`). Se o modelo corrigido mostrar que essa regra não pode produzir o maço certo sem outro gesto, **não mude a regra**: pare e relate as opções, com a sequência física de cada uma, porque a decisão é do usuário.
5. **Atualize** a tabela de mecânicas e os números do laço exaustivo. O teste de `traseira` não pode mudar de comportamento. Revise se `traseira` tem o mesmo tipo de problema e relate sem corrigir, a menos que o defeito seja evidente e o teste prove.

## Regras
- Nunca travar: sem unwrap/expect/indexação sem checagem fora de testes em código novo.
- Sem mudar textos do CLI, salvo se a saída de `simular` ficar falsa; liste os que mudarem.

## Teste de uso (binário real)
Com `cargo run -p plots-cli --locked -- simular ...`, nas duas impressoras, total par e ímpar, nas duas ordens e com `--simular-erro`: cole comando, saída e código de saída.

## Relatório
Causa, sequência física passo a passo em um lote de 4 páginas na gaveta, o que mudou, testes e grupo de falha prévia, linha do CI, autoverificação vazia, o que não foi verificado e qualquer decisão pendente do usuário. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
