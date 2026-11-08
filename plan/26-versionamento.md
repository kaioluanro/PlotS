# Versionamento

**Estado:** Concluído

## Objetivo

Decisão do usuário em 2026-11-04: numerar as versões. A que acabou de sair é a `1.0.0-alpha`; as correções do uso real entram em outra.

## Comportamento

- **Formato:** versão semântica, com a tag `v<versão>` no commit de que o instalador foi gerado. Tags são locais até o usuário pedir publicação.
- **Uma versão só:** PlotS e PlotSim usam o mesmo número. `node scripts/definir-versao.mjs <versão>` troca todos os manifestos, e `--conferir` diz se eles batem.
- **Em desenvolvimento:** o `master` já leva o número da próxima versão, e `CHANGELOG.md` registra o que entra nela.
- **Nome adotado:** o usuário disse "1-alpha"; a forma semântica é `1.0.0-alpha`, e as seguintes são `1.0.0-alpha.2`, `1.0.0-alpha.3`, até `1.0.0`.
- **Entrega da alpha.3:** por decisão do usuário em 2026-11-05, sem instalador: portátil em `.zip` com `PlotS.exe`, `pdfium.dll`, `LICENSE`, `THIRD-PARTY-NOTICES` e `RUST-STD-NOTICES.html`. O zip não inclui o PlotSim, que continua em pasta própria; na alpha.3, os dados de uso ficam em `%APPDATA%`, como no programa instalado. Artefato: `%LOCALAPPDATA%\plots-build\alpha3\saidas\PlotS_1.0.0-alpha.3_x64_portatil.zip` (10,36 MB, SHA-256 `C3BEC37A3F338D775690A6687FA36395F55367BF043CE9804A24CEB2E57DD4FC`).
- **Perfil no portátil (alpha.4):** com `PLOTS-PORTATIL.txt` ao lado do `PlotS.exe`, o `perfis.toml` fica nessa pasta e a primeira abertura importa o de `%APPDATA%` sem apagar o original; o instalado, sem o marcador, continua em `%APPDATA%`. O histórico continua em `%APPDATA%`. O zip não inclui o perfil do usuário. Artefato: `%LOCALAPPDATA%\plots-build\alpha4\saidas\PlotS_1.0.0-alpha.4_x64_portatil.zip` (10,37 MB, SHA-256 `4EB30324865E06CEB7AFC2E76EB0869160BCB2BAF8DC458CA67477B370DE24BB`), pasta `PlotS` fixa, para descompactar uma versão nova por cima sem perder o `perfis.toml`.
- **Entrega da alpha.5:** em 2026-11-06, com "Frente" = páginas ímpares, "Verso" = pares, 1 página sem folha em branco, desenho por bitmap compatível + `StretchBlt` e a chave de boca com bandeja/resolução/cor. Artefato: `%LOCALAPPDATA%\plots-build\alpha5\saidas\PlotS_1.0.0-alpha.5_x64_portatil.zip` (10,41 MB, SHA-256 `B81EFA6A20D972CF516E5E1F8275EEF4A9D04E84A59173235B6A8E3F45E06C0D`), pasta `PlotS` fixa, sem perfil dentro do zip.
- **Entrega da alpha.6:** em 2026-11-06, com a cor das imagens corrigida (o pdfium entregava RGBA e o DIB do GDI lia BGRA; laranja e amarelo saíam azuis). Artefato: `%LOCALAPPDATA%\plots-build\alpha6\saidas\PlotS_1.0.0-alpha.6_x64_portatil.zip` (10,41 MB, SHA-256 `9C89A13C51CE7B8DE962D49AC2EBD6B72823F9D7AA350A0C139B3907AD133E85`), pasta `PlotS` fixa, sem perfil dentro do zip.
- **Entrega da alpha.7:** em 2026-11-07, com imagens (PNG/JPEG/BMP/GIF/WebP/TIFF), painel PlotInfo, perfis HP/Epson, análise PDFium, `mutool/` CMYK separado e CSV. Artefato: `%LOCALAPPDATA%\plots-build\alpha7\saidas\PlotS_1.0.0-alpha.7_x64_portatil.zip` (42,25 MB, SHA-256 `52449AF14979E4AE78E860BDD0EBA3D55B302E9658266B28FA6A5E7EC18D0FD6`), pasta `PlotS` fixa, sem `perfis.toml`/`plotinfo.toml`; leva `mutool/` com AGPL, 26 licenças do `thirdparty`/fontes e a fonte correspondente fora do zip (`mupdf-1.28.5-source.tar.gz`, 68.919.385 bytes, SHA-256 `98A5C10CDA20C3992CDF76FF6B2A1149C32BD79CC796D3F703230B1185B7E934`).
- **Entrega da alpha.8:** em 2026-11-07, com a opção **Uma cópia por vez** acima da Quantidade, frentes e verso separados por cópia, confirmação da impressora antes da próxima, a mesma dica nas duas faces e branco entre cópias ímpares no automático juntado. Artefato: `%LOCALAPPDATA%\plots-build\alpha8\saidas\PlotS_1.0.0-alpha.8_x64_portatil.zip` (42,26 MB, SHA-256 `7FDD07AC22A3622DAE12FBD6B1B94379C62804B70F18EC168D692910083467B5`), pasta `PlotS` fixa, sem `perfis.toml`/`plotinfo.toml`; leva `mutool/` com AGPL, 26 licenças do `thirdparty`/fontes e a fonte correspondente fora do zip (`mupdf-1.28.5-source.tar.gz`, 68.919.385 bytes, SHA-256 `98A5C10CDA20C3992CDF76FF6B2A1149C32BD79CC796D3F703230B1185B7E934`). O portátil abriu e fechou com a versão `1.0.0-alpha.8`; o `mutool.exe` está presente, nenhum perfil veio dentro do zip e a dica da caixinha foi conferida na frente e no verso.

## Plano de implementação

- [x] `scripts/definir-versao.mjs` e `CHANGELOG.md`
- [x] Tag `v1.0.0-alpha` e instalador dessa versão
- [x] `master` em `1.0.0-alpha.2`
- [x] `master` em `1.0.0-alpha.3` e PlotS portátil em `.zip`
- [x] `master` em `1.0.0-alpha.4` e perfil ao lado do executável no portátil
- [x] `master` em `1.0.0-alpha.5` e correção da folha em branco + painel de opções no portátil
- [x] `master` em `1.0.0-alpha.6` e cor das imagens corrigida no portátil
- [x] `master` em `1.0.0-alpha.7` e portátil com imagens, painel PlotInfo, `mutool/` CMYK e CSV
- [x] `master` em `1.0.0-alpha.8` e portátil com a opção "Uma cópia por vez"

## Pronto quando

A tag aponta para o commit que gerou o instalador ou o portátil, e o programa diz o mesmo número.

## Depende de

`17-acabamento`.
