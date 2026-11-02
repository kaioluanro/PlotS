# STATUS

Requisitos: OK 29 · PARTIAL 23 · MISSING 39 · BUG 0 · N/A 2 (93). Detalhes em `AUDIT-PlotS.md`; norma visual em `estilo-ui-ux.md`.

| Plano | OK | PARTIAL | MISSING | BUG | N/A | Estado |
|---|---|---|---|---|---|---|
| [Visão geral, arquitetura e fases](00-visao-geral.md) | 0 | 3 | 0 | 0 | 1 | Em andamento |
| [Núcleo de ordenação](01-nucleo-ordenacao.md) | 9 | 1 | 0 | 0 | 0 | Em andamento |
| [Geração das passadas em PDF](02-geracao-pdf.md) | 4 | 3 | 0 | 0 | 0 | Em andamento |
| [Modo simulação](03-simulacao.md) | 7 | 2 | 1 | 0 | 1 | Em andamento |
| [Calibração](04-calibracao.md) | 2 | 2 | 1 | 0 | 0 | Em andamento |
| [Tamanhos de papel](05-tamanhos-de-papel.md) | 5 | 1 | 2 | 0 | 0 | Em andamento |
| [Perfis salvos](06-perfis.md) | 0 | 2 | 0 | 0 | 0 | Em andamento |
| [CLI de impressão](07-cli-impressao.md) | 2 | 1 | 2 | 0 | 0 | Em andamento |
| [Impressão no Windows](08-impressao-windows.md) | 0 | 1 | 7 | 0 | 0 | Em andamento |
| [Janela, barra superior e abas](09-janela-e-abas.md) | 0 | 0 | 4 | 0 | 0 | Não iniciado |
| [Tela de abertura](10-splash.md) | 0 | 0 | 1 | 0 | 0 | Não iniciado |
| [Barra lateral de impressoras](11-lateral-impressoras.md) | 0 | 1 | 2 | 0 | 0 | Em andamento |
| [Visualizador de PDF](12-visualizador.md) | 0 | 0 | 3 | 0 | 0 | Não iniciado |
| [Fluxo de impressão e recolocação](13-fluxo-e-recolocacao.md) | 0 | 2 | 4 | 0 | 0 | Em andamento |
| [Padrão visual e idiomas](14-padrao-visual-e-idiomas.md) | 0 | 1 | 2 | 0 | 0 | Em andamento |
| [Múltiplas impressoras](15-multiplas-impressoras.md) | 0 | 2 | 6 | 0 | 0 | Em andamento |
| [Impressora virtual](16-impressora-virtual.md) | 0 | 0 | 1 | 0 | 0 | Não iniciado |
| [Acabamento e instalador](17-acabamento.md) | 0 | 1 | 2 | 0 | 0 | Em andamento |
| [Linux e macOS](18-linux-e-macos.md) | 0 | 0 | 1 | 0 | 0 | Não iniciado |

## Lotes

| Lote | Commit | Estado |
|---|---|---|
| Base do projeto | `bb69d1a` | concluído |
| Regras e planos | cb5be25 | concluído |
| cli-mensagens | `882ba68` | concluído |
| fase2-calibracao | `cf22592` | concluído |
| fase2-folhas | `6baa24a` | concluído |
| modelo-gaveta | `1301405` | concluído |
| perfis | `72030fb` | concluído |
| pdfs-reais | `25557c9` | concluído: 64 de 64 PDFs reais aceitos, conferidos no PyMuPDF |
| interface-base | — | em correção |

**Próximo:** `plots imprimir` com backend simulado e `plots calibrar` (`04-calibracao`, `07-cli-impressao`).

## Bloqueado ou dependente do usuário

- Impressora real: confirmação dos planos `04-calibracao`, `07-cli-impressao` e `08-impressao-windows`.
- Imagem da tela de abertura: plano `10-splash`.
- Validação das passadas no Firefox: plano `02-geracao-pdf`.
