# STATUS

Requisitos: OK 37 · PARTIAL 34 · MISSING 20 · BUG 0 · N/A 2 (93). Detalhes em `AUDIT-PlotS.md`; norma visual em `estilo-ui-ux.md`.

| Plano | OK | PARTIAL | MISSING | BUG | N/A | Estado |
|---|---|---|---|---|---|---|
| [Visão geral, arquitetura e fases](00-visao-geral.md) | 1 | 2 | 0 | 0 | 1 | Em andamento |
| [Núcleo de ordenação](01-nucleo-ordenacao.md) | 9 | 1 | 0 | 0 | 0 | Em andamento |
| [Geração das passadas em PDF](02-geracao-pdf.md) | 5 | 2 | 0 | 0 | 0 | Em andamento |
| [Modo simulação](03-simulacao.md) | 7 | 2 | 1 | 0 | 1 | Em andamento |
| [Calibração](04-calibracao.md) | 2 | 3 | 0 | 0 | 0 | Em andamento |
| [Tamanhos de papel](05-tamanhos-de-papel.md) | 5 | 2 | 1 | 0 | 0 | Em andamento |
| [Perfis salvos](06-perfis.md) | 0 | 2 | 0 | 0 | 0 | Em andamento |
| [CLI de impressão](07-cli-impressao.md) | 2 | 2 | 1 | 0 | 0 | Em andamento |
| [Impressão no Windows](08-impressao-windows.md) | 1 | 6 | 1 | 0 | 0 | Em andamento |
| [Janela, barra superior e abas](09-janela-e-abas.md) | 1 | 3 | 0 | 0 | 0 | Em andamento |
| [Tela de abertura](10-splash.md) | 0 | 0 | 1 | 0 | 0 | Não iniciado |
| [Barra lateral de impressoras](11-lateral-impressoras.md) | 2 | 1 | 0 | 0 | 0 | Em andamento |
| [Visualizador de PDF](12-visualizador.md) | 0 | 1 | 2 | 0 | 0 | Em andamento |
| [Fluxo de impressão e recolocação](13-fluxo-e-recolocacao.md) | 0 | 2 | 4 | 0 | 0 | Em andamento |
| [Padrão visual e idiomas](14-padrao-visual-e-idiomas.md) | 2 | 1 | 0 | 0 | 0 | Em andamento |
| [Múltiplas impressoras](15-multiplas-impressoras.md) | 0 | 3 | 5 | 0 | 0 | Em andamento |
| [Impressora virtual](16-impressora-virtual.md) | 0 | 0 | 1 | 0 | 0 | Não iniciado |
| [Acabamento e instalador](17-acabamento.md) | 0 | 1 | 2 | 0 | 0 | Em andamento |
| [Linux e macOS](18-linux-e-macos.md) | 0 | 0 | 1 | 0 | 0 | Não iniciado |
| [Modelos de impressora](19-modelos-de-impressora.md) | — | — | — | — | — | Em andamento |

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
| cli-calibrar-imprimir | `92772e5` | concluído, sem envio real |
| interface-base | `353f54d` | concluído; janela aprovada pelo usuário |
| windows-filas | `a9f9185` | concluído |
| windows-envio | `126ac26` | concluído; conferido só na fila de PDF do Windows, sem impressora real |
| lateral-impressoras | `f6e3433` | concluído; botão de imprimir ainda desabilitado |
| modelos-virtuais | `166adee` | concluído; simuladas imitam os dois modelos, sem medição em papel |

**Em andamento:** `seletor-modelos`: escolher jato de tinta ou laser para cada impressora.

**Próximo:** ligar o botão de imprimir ao envio, com cartões de recolocação (`13-fluxo-e-recolocacao`).

## Bloqueado ou dependente do usuário

- Impressora real, que o usuário não tem à mão: confirmação dos planos `04-calibracao`, `07-cli-impressao` e `08-impressao-windows`.
- Imagem da tela de abertura: plano `10-splash`.
