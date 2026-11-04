# Versionamento

**Estado:** Concluído

## Objetivo

Decisão do usuário em 2026-11-04: numerar as versões. A que acabou de sair é a `1.0.0-alpha`; as correções do uso real entram em outra.

## Comportamento

- **Formato:** versão semântica, com a tag `v<versão>` no commit de que o instalador foi gerado. Tags são locais até o usuário pedir publicação.
- **Uma versão só:** PlotS e PlotSim usam o mesmo número. `node scripts/definir-versao.mjs <versão>` troca todos os manifestos, e `--conferir` diz se eles batem.
- **Em desenvolvimento:** o `master` já leva o número da próxima versão, e `CHANGELOG.md` registra o que entra nela.
- **Nome adotado:** o usuário disse "1-alpha"; a forma semântica é `1.0.0-alpha`, e as seguintes são `1.0.0-alpha.2`, `1.0.0-alpha.3`, até `1.0.0`.

## Plano de implementação

- [x] `scripts/definir-versao.mjs` e `CHANGELOG.md`
- [x] Tag `v1.0.0-alpha` e instalador dessa versão
- [x] `master` em `1.0.0-alpha.2`

## Pronto quando

A tag aponta para o commit que gerou o instalador, e o instalador e o programa dizem o mesmo número.

## Depende de

`17-acabamento`.
