# Histórico de versões

O PlotS segue o [versionamento semântico](https://semver.org/lang/pt-BR/). As versões alfa ainda não têm teste em papel.

## 1.0.0-alpha.2

Em desenvolvimento: uso pela janela com várias impressoras, avisos no topo no lugar dos cartões e instalador sem as impressoras simuladas.

## 1.0.0-alpha

Primeira versão numerada, gerada em 2026-11-04.

- Impressão frente e verso manual em qualquer impressora, com os modelos jato de tinta e laser, e automática nas que imprimem as duas faces.
- Calibração de cada impressora na janela, com 2 folhas e figuras.
- Escolha de Frente ou Verso no rodapé, com quantidade de cópias e sem lotes na janela.
- Verso recusado se o arquivo mudou depois das frentes, na mesma aba.
- Folha de controle, ajuste ao papel, histórico e preferências do driver.
- PlotSim, ferramenta de teste com impressoras virtuais, fora do instalador.

**Problemas conhecidos:**

- A impressão pela janela com impressoras virtuais pode perder os versos ([relato do usuário](plan/25-avisos-no-topo.md), em investigação).
- Sem teste em papel, em impressora real nem execução do instalador.
- O instalador ainda mostra as impressoras simuladas.
