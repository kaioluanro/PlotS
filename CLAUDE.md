# Instruções do PlotS

Leia e siga `AGENTS.md`, incluindo a regra de uso obrigatório da skill `.claude/skills/craft-team/SKILL.md` e a configuração em `.claude/team.json`.

## Team (craft-team)

Os agentes `plots-auditor`, `plots-implementer`, `plots-debugger`, `plots-reviewer`, `plots-legal`, `plots-interface`, `plots-research` e `plots-planner` estão em `.claude/agents/`. Para cada lote aplicável, siga o fluxo da skill: brief, implementação isolada, verificações, revisões, correções e atualização dos registros. Respeite as instruções de prioridade superior e as capacidades disponíveis no ambiente.

As regras estão em `AGENTS.md`. Os planos de implementação, a norma visual e o estado do projeto estão em `plan/` (comece por `plan/STATUS-MANUAIS.md`); `plan/estilo-ui-ux.md` e materiais privados não entram nos commits. A interface usa pt-BR e en. O comando de CI está em `.claude/team.json`.
