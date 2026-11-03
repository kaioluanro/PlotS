# STATUS

Requisitos: OK 52 · PARTIAL 32 · MISSING 6 · BUG 0 · N/A 3 (93). Detalhes em `AUDIT-PlotS.md`; norma visual em `estilo-ui-ux.md`.

| Plano | OK | PARTIAL | MISSING | BUG | N/A | Estado |
|---|---|---|---|---|---|---|
| [Visão geral, arquitetura e fases](00-visao-geral.md) | 1 | 2 | 0 | 0 | 1 | Em andamento |
| [Núcleo de ordenação](01-nucleo-ordenacao.md) | 9 | 1 | 0 | 0 | 0 | Em andamento |
| [Geração das passadas em PDF](02-geracao-pdf.md) | 5 | 2 | 0 | 0 | 0 | Em andamento |
| [Modo simulação](03-simulacao.md) | 7 | 3 | 0 | 0 | 1 | Em andamento |
| [Calibração](04-calibracao.md) | 2 | 3 | 0 | 0 | 0 | Em andamento |
| [Tamanhos de papel](05-tamanhos-de-papel.md) | 6 | 1 | 1 | 0 | 0 | Em andamento |
| [Perfis salvos](06-perfis.md) | 0 | 2 | 0 | 0 | 0 | Em andamento |
| [CLI de impressão](07-cli-impressao.md) | 2 | 2 | 1 | 0 | 0 | Em andamento |
| [Impressão no Windows](08-impressao-windows.md) | 1 | 6 | 1 | 0 | 0 | Em andamento |
| [Janela, barra superior e abas](09-janela-e-abas.md) | 1 | 3 | 0 | 0 | 0 | Em andamento |
| [Tela de abertura](10-splash.md) | 1 | 0 | 0 | 0 | 0 | Concluído |
| [Barra lateral de impressoras](11-lateral-impressoras.md) | 3 | 0 | 0 | 0 | 0 | Concluído |
| [Visualizador de PDF](12-visualizador.md) | 3 | 0 | 0 | 0 | 0 | Concluído |
| [Fluxo de impressão e recolocação](13-fluxo-e-recolocacao.md) | 5 | 1 | 0 | 0 | 0 | Em andamento |
| [Padrão visual e idiomas](14-padrao-visual-e-idiomas.md) | 2 | 1 | 0 | 0 | 0 | Em andamento |
| [Múltiplas impressoras](15-multiplas-impressoras.md) | 3 | 4 | 0 | 0 | 1 | Em andamento |
| [Impressora virtual](16-impressora-virtual.md) | 0 | 0 | 1 | 0 | 0 | Não iniciado |
| [Acabamento e instalador](17-acabamento.md) | 1 | 1 | 1 | 0 | 0 | Em andamento |
| [Linux e macOS](18-linux-e-macos.md) | 0 | 0 | 1 | 0 | 0 | Não iniciado |
| [Modelos de impressora](19-modelos-de-impressora.md) | — | — | — | — | — | Em andamento |
| [Preferências do driver](20-preferencias-do-driver.md) | — | — | — | — | — | Em andamento |
| [Frente e verso automático](21-frente-e-verso-automatico.md) | — | — | — | — | — | Concluído no código |
| [Opções de envio e histórico](22-opcoes-de-envio-e-historico.md) | — | — | — | — | — | Concluído no código |
| [PlotSim](23-plotsim.md) | — | — | — | — | — | Em andamento |
| [Impressão sem lote](24-impressao-sem-lote.md) | — | — | — | — | — | Em andamento |

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
| tela-abertura | `bd50a56` | concluído, sem a imagem definitiva |
| pdf-robustez | `8e11060` | concluído; retirada de recursos só com fixtures, sem PDF real que a exercite |
| interface-base | `353f54d` | concluído; janela aprovada pelo usuário |
| windows-filas | `a9f9185` | concluído |
| windows-envio | `126ac26` | concluído; conferido só na fila de PDF do Windows, sem impressora real |
| lateral-impressoras | `f6e3433` | concluído; botão de imprimir ainda desabilitado |
| modelos-virtuais | `166adee` | concluído; simuladas imitam os dois modelos, sem medição em papel |
| seletor-modelos | `826b726` | concluído; sem medição em papel |
| cartoes-impressao | `256e8ce` | concluído; simuladas e fila de PDF, sem impressora real; figura do gesto pendente |
| visualizador | `b29796e` | concluído; conferido com PDFs de teste, sem o intervalo de páginas |
| intervalo-paginas | `ccb1ebc` | concluído; simuladas e fila de PDF, gesto real de arrastar não verificado |
| instalador | `1a1f6d7` | concluído; instalador gerado, não executado |
| gesto-e-spooler | `ca10be1` | concluído; simuladas e backend falso, sem driver real nem papel |
| varias-impressoras | `62d51a4` | concluído; simuladas, fila real só até a confirmação, sem papel |
| preferencias-driver | `ec46630` | concluído no código; janela real aberta e fechada, sem clicar em OK |
| frente-e-verso-automatico | `0a8f451` | concluído no código; simulada e fila sem duas faces, sem envio real em duas faces nem papel |
| opcoes-de-envio | `1ad3307` | concluído no código; simuladas e fila de PDF, sem impressora real |
| laser-verso-girado | `364774d` | concluído; modelo físico e fila de PDF, sem papel |

**Próximo:**

1. Integrar a calibração pela janela (`04-calibracao`). Depois, as fases 4 a 6 do PlotSim (`23-plotsim`).
2. Revisar as linhas desatualizadas de `AUDIT-PlotS.md`, gerar o instalador final e fazer o teste do usuário.

Depois da primeira versão: impressora virtual, Linux e macOS, divisão por velocidade (`15-multiplas-impressoras`) e medir se o Graphify economiza tokens, com 3 perguntas respondidas com e sem ele.

## Bloqueado ou dependente do usuário

- Impressora real, que o usuário não tem à mão: confirmação dos planos `04-calibracao`, `07-cli-impressao` e `08-impressao-windows`.
- Executar o instalador em máquina ou conta de teste: plano `17-acabamento`.
- Conferir a janela real do driver, com OK e Cancelar: plano `20-preferencias-do-driver`.
- Conferir o frente e verso automático em impressora com duas faces: plano `21-frente-e-verso-automatico`.
