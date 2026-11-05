# Versionamento

**Estado:** Concluído

## Objetivo

Decisão do usuário em 2026-11-04: numerar as versões. A que acabou de sair é a `1.0.0-alpha`; as correções do uso real entram em outra.

## Comportamento

- **Formato:** versão semântica, com a tag `v<versão>` no commit de que o instalador foi gerado. Tags são locais até o usuário pedir publicação.
- **Uma versão só:** PlotS e PlotSim usam o mesmo número. `node scripts/definir-versao.mjs <versão>` troca todos os manifestos, e `--conferir` diz se eles batem.
- **Em desenvolvimento:** o `master` já leva o número da próxima versão, e `CHANGELOG.md` registra o que entra nela.
- **Nome adotado:** o usuário disse "1-alpha"; a forma semântica é `1.0.0-alpha`, e as seguintes são `1.0.0-alpha.2`, `1.0.0-alpha.3`, até `1.0.0`.
- **Entrega da alpha.3:** por decisão do usuário em 2026-11-05, sem instalador: portátil em `.zip` com `PlotS.exe`, `pdfium.dll`, `LICENSE`, `THIRD-PARTY-NOTICES` e `RUST-STD-NOTICES.html`. O zip não inclui o PlotSim, que continua em pasta própria; os dados de uso ficam em `%APPDATA%`, como no programa instalado. Artefato: `%LOCALAPPDATA%\plots-build\alpha3\saidas\PlotS_1.0.0-alpha.3_x64_portatil.zip` (10,36 MB, SHA-256 `C3BEC37A3F338D775690A6687FA36395F55367BF043CE9804A24CEB2E57DD4FC`).

## Plano de implementação

- [x] `scripts/definir-versao.mjs` e `CHANGELOG.md`
- [x] Tag `v1.0.0-alpha` e instalador dessa versão
- [x] `master` em `1.0.0-alpha.2`
- [x] `master` em `1.0.0-alpha.3` e PlotS portátil em `.zip`

## Pronto quando

A tag aponta para o commit que gerou o instalador ou o portátil, e o programa diz o mesmo número.

## Depende de

`17-acabamento`.
