# PlotS — regras do projeto

O **PlotS** faz qualquer impressora entregar o mesmo resultado físico no frente e verso manual. Este arquivo tem só as regras de trabalho. O que o programa faz e como implementar está em `plan/`: um arquivo por funcionalidade, com o índice e o estado em `plan/STATUS-MANUAIS.md`.

## Uso obrigatório da craft-team

Em tarefas de desenvolvimento, planejamento, auditoria, depuração e revisão deste projeto, use a skill local `.claude/skills/craft-team/SKILL.md` e consulte `.claude/team.json` no início da sessão. Siga o fluxo e use os agentes instalados em `.claude/agents/` quando o ambiente e as instruções de prioridade superior permitirem. Se alguma etapa não estiver disponível, informe a limitação e execute as verificações possíveis; não afirme que uma etapa foi realizada sem evidência.

- **Nenhum agente roda sem a craft-team.** Todo subagente recebe, no próprio prompt, a ordem de ler `.claude/skills/craft-team/SKILL.md` e o arquivo do seu papel em `.claude/agents/` antes de agir. Quando o ambiente não registra os agentes instalados como tipo de subagente, o coordenador usa o tipo genérico disponível e entrega o papel e o protocolo no prompt; isso é uma limitação do ambiente, não uma dispensa do fluxo.
- **Modelo:** `.claude/team.json` guarda três tabelas. `models` é a do Claude (haiku/sonnet/opus) e fica preservada; `models_openrouter` é a ativa neste ambiente, com todos os papéis em `openrouter/deepseek/deepseek-v4.1-flash-1017` (DeepSeek V4.1 Flash 1017), e `models_ativos` aponta para ela. `models_opencode` fica preservada com `opencode/deepseek-v4.1-flash-free` para voltar. Nenhum agente abre em outro modelo sem autorização do usuário. O instalador da skill lê `models`: depois de reinstalar com `--force`, reaplique `models_openrouter` nas linhas `model:` de `.claude/agents/`.

O comando de CI e a linha de sucesso estão em `.claude/team.json`.

## Planos de implementação

- Toda funcionalidade nova ganha um arquivo próprio em `plan/`, com objetivo, passos, critério de pronto, dependências e especificação.
- Ao concluir um lote, atualize o plano da funcionalidade, `plan/AUDIT-PlotS.md` e `plan/STATUS-MANUAIS.md`.
- Cada lote lê só os planos de que precisa; os briefs ficam em `plan/briefs/`.
- `plan/` entra nos commits, exceto `plan/estilo-ui-ux.md`.

## Limpeza constante do disco

O projeto não pode acumular no HD arquivos que já não servem. A limpeza faz parte do trabalho e vale para o coordenador e para todos os agentes.

- **Quando limpar**: ao terminar cada lote, antes de encerrar uma sessão e sempre que um agente concluir ou for interrompido. Confira com `du -sh target* .qa-* .deps-* .claude/worktrees 2>/dev/null`.
- **Pode apagar sem perguntar** (só o que o trabalho no projeto gerou e dá para gerar de novo): pastas de compilação extras (`target-*/`, `target/` de agentes e de worktrees), worktrees de lotes já encerrados, manifestos e crates temporários, e PDFs, capturas e logs de verificação depois que o resultado foi registrado.
- **Compilação e saídas fora do E:**: o disco `E:` está quase cheio. Toda compilação usa `CARGO_TARGET_DIR` em `%LOCALAPPDATA%/plots-build/<lote>`, e saídas de teste (PDFs, imagens) vão para `%LOCALAPPDATA%/plots-build/<lote>/saidas`. Essa pasta é apagada ao fim do lote. A raiz do projeto não mantém `target/`.
- **Nunca apagar sem perguntar**: código, especificações, `AUDIT-*.md`, briefs, registros de sessão, arquivos do usuário, trabalho sem commit e qualquer coisa que não dê para gerar de novo. Na dúvida, pergunte.
- **Antes de apagar**: confira o alvo e confirme que nenhum processo o está usando. Use caminhos exatos, nunca padrões amplos.
- **Arquivos temporários novos**: use uma pasta já coberta pelo `.gitignore` e informe no relatório do lote o que foi removido e o que ficou.

## Uso obrigatório do Graphify

Use o Graphify para navegar e atualizar o grafo de conhecimento do projeto em `graphify-out/`, seguindo as regras abaixo.

When the user types `/graphify`, use the installed graphify skill or instructions before doing anything else.

Rules:
- When graphify-out/graph.json exists, run `graphify query "<question>"` before your first grep, search or source-file read for a codebase question, and put this rule in every subagent prompt that explores code. Read raw files only for lines the graph or an earlier read already located. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- Dirty graphify-out/ files are expected after hooks or incremental updates; dirty graph files are not a reason to skip graphify. Only skip graphify if the task is about stale or incorrect graph output, or the user explicitly says not to use it.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
- A committed graphify-out/graph.json is rewritten whole on every build, so a large diff to it in a pull request is expected. Do not flag it as churn or suggest skipping the graph rebuild or its hooks.
