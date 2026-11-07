# STATUS

Requisitos: OK 76 · PARTIAL 28 · MISSING 2 · BUG 0 · N/A 4 (110). Detalhes em `AUDIT-PlotS.md`; norma visual em `estilo-ui-ux.md`.

| Plano | OK | PARTIAL | MISSING | BUG | N/A | Estado |
|---|---|---|---|---|---|---|
| [Visão geral, arquitetura e fases](00-visao-geral.md) | 1 | 2 | 0 | 0 | 1 | Em andamento |
| [Núcleo de ordenação](01-nucleo-ordenacao.md) | 9 | 1 | 0 | 0 | 0 | Em andamento |
| [Geração das passadas em PDF](02-geracao-pdf.md) | 5 | 2 | 0 | 0 | 0 | Em andamento |
| [Modo simulação](03-simulacao.md) | 9 | 1 | 0 | 0 | 1 | Em andamento |
| [Calibração](04-calibracao.md) | 2 | 3 | 0 | 0 | 0 | Em andamento |
| [Tamanhos de papel](05-tamanhos-de-papel.md) | 8 | 1 | 0 | 0 | 0 | Em andamento |
| [Perfis salvos](06-perfis.md) | 1 | 1 | 0 | 0 | 0 | Em andamento |
| [CLI de impressão](07-cli-impressao.md) | 3 | 2 | 0 | 0 | 0 | Em andamento |
| [Impressão no Windows](08-impressao-windows.md) | 2 | 7 | 0 | 0 | 0 | Em andamento |
| [Janela, barra superior e abas](09-janela-e-abas.md) | 4 | 0 | 0 | 0 | 0 | Em andamento |
| [Tela de abertura](10-splash.md) | 1 | 0 | 0 | 0 | 0 | Concluído |
| [Barra lateral de impressoras](11-lateral-impressoras.md) | 2 | 0 | 0 | 0 | 1 | Concluído |
| [Visualizador de PDF](12-visualizador.md) | 3 | 0 | 0 | 0 | 0 | Concluído |
| [Fluxo de impressão e recolocação](13-fluxo-e-recolocacao.md) | 7 | 1 | 0 | 0 | 0 | Em andamento |
| [Padrão visual e idiomas](14-padrao-visual-e-idiomas.md) | 4 | 0 | 0 | 0 | 0 | Em andamento |
| [Múltiplas impressoras](15-multiplas-impressoras.md) | 4 | 3 | 0 | 0 | 1 | Em andamento |
| [Impressora virtual](16-impressora-virtual.md) | 0 | 0 | 1 | 0 | 0 | Não iniciado |
| [Acabamento e instalador](17-acabamento.md) | 2 | 1 | 0 | 0 | 0 | Em andamento |
| [Linux e macOS](18-linux-e-macos.md) | 0 | 0 | 1 | 0 | 0 | Não iniciado |
| [Modelos de impressora](19-modelos-de-impressora.md) | — | — | — | — | — | Em andamento |
| [Preferências do driver](20-preferencias-do-driver.md) | — | — | — | — | — | Em andamento |
| [Frente e verso automático](21-frente-e-verso-automatico.md) | — | — | — | — | — | Concluído no código |
| [Opções de envio e histórico](22-opcoes-de-envio-e-historico.md) | — | — | — | — | — | Concluído no código |
| [PlotSim](23-plotsim.md) | — | — | — | — | — | Em andamento |
| [Impressão sem lote](24-impressao-sem-lote.md) | — | — | — | — | — | Em andamento |
| [Avisos no topo](25-avisos-no-topo.md) | 2 | 1 | 0 | 0 | 0 | Concluído no código |
| [Versionamento](26-versionamento.md) | 1 | 0 | 0 | 0 | 0 | Concluído |
| [Impressão no Windows: faces e desenho](27-impressao-pdfjs.md) | 1 | 1 | 0 | 0 | 0 | Concluído no código; falta impressora real |
| [Opções da impressora](28-opcoes-da-impressora.md) | 0 | 1 | 0 | 0 | 0 | Concluído no código; falta driver com bandeja |
| [Cor das imagens impressas](29-cor-das-imagens.md) | 1 | 0 | 0 | 0 | 0 | Concluído no código; falta impressora real |
| [PlotInfo: imagens e custo de tinta](30-plotinfo.md) | 1 | 0 | 0 | 0 | 0 | Concluído no código; falta impressora real e prova de tinta em papel |
| [Impressão de uma cópia por vez](31-copia-por-vez.md) | 1 | 0 | 0 | 0 | 0 | Concluído no código; falta impressora real |

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
| plotsim-suite | `108bede` | concluído; rápida com 42 de 44, 2 sem cobertura, sem papel |
| calibracao-na-janela | `c410c8c` | concluído no código; simuladas e fila de PDF, sem papel nem leitor de tela real |
| impressao-sem-lote | `18bb5cc` | concluído no código; simuladas, sem papel |
| plotsim-janela | `7c47c9e` | concluído; filas virtuais pelo spooler real, sem papel |
| 1.0.0-alpha | `11bc3e3` | tag local; instalador gerado, não executado |
| 1.0.0-alpha.2 | `7b16a4f` | concluído no código; avisos no topo, proteção do verso e envio simultâneo; sem papel |
| 1.0.0-alpha.3 | `c248cf8` | concluído no código; PlotSim com "Verso recebido" e "Ver o outro lado"; PlotS com tema cinza neutro e alerta no verso; portátil em `.zip`, sem instalador |
| 1.0.0-alpha.4 | `97643ec` | concluído no código; perfil portátil ao lado do exe; portátil em `.zip` |
| 1.0.0-alpha.5 | `559a9c8` | concluído no código; frente/verso corretos, 1 página sem verso, desenho por bitmap compatível e painel de opções; portátil em `.zip` |
| 1.0.0-alpha.6 | `6a972c6` | concluído no código; cor das imagens (BGRA) corrigida; portátil em `.zip` |
| plotinfo-imagem | `d6ae384` | concluído no código; PNG/JPEG/BMP/GIF/WebP/TIFF viram PDF derivado, abrem e imprimem |
| plotinfo-motor | `be877c1` | concluído no código; `plots-info` com seleção, cobertura, custo e perfis, mais `plotinfo.toml` e comandos |
| plotinfo-analise | `8a9866f` | concluído no código; PDFium, cache, eventos, recálculo e cancelamento; revisões de código e jurídico aprovadas |
| plotinfo-mutool | `eab8d30` | concluído no código; mutool separado, CMYK, fallback com aviso e CSV; prova DeviceCMYK e job completo |
| plotinfo-painel | `15bc485` + `0f66aa7` + `7acd500` + `a699b22` + `8bfd509` | concluído no código; gaveta, resumo alternável, engrenagem, CSV liberado, aviso traduzível, reanálise por dpi/modo/cinzas, foco e acessibilidade; 1.165 testes de UI |
| copia-por-vez | `09727ac` | concluído no código; caixinha acima da Quantidade, frentes uma cópia por vez, automático junto/separado e cartão com “Cópia N de M”; 1.173 testes de UI |

**Próximo:**

1. Feito em 2026-11-04: calibração, impressão sem lote e PlotSim integrados. Suíte rápida no `master` `a34a54e`: 42 de 44, 2 sem cobertura, 0 falhas.
2. Feito em 2026-11-04: `1.0.0-alpha.2` (`7b16a4f`), com avisos no topo, confirmação do verso com o trabalho enviado, envio simultâneo sem perda e papel misto mantendo o lado. CI verde; suíte rápida 42 de 44, 2 sem cobertura, 0 falhas.
3. Auditoria revista e instalador final gerado em 2026-11-04, a partir de `b0c7302`. Falta o teste do usuário: papel pela calibração, janela do driver com OK e Cancelar, e instalador em máquina ou conta de teste. O instalador da alpha.2, sem as simuladas, foi gerado (`PlotS_1.0.0-alpha.2_x64-setup.exe`) e ainda não foi executado.
4. Feito em 2026-11-04: PlotSim avisa quando o verso chegou e separa a vista do gesto (`ade950c`). Próximo: testar o fluxo frente → verso com o PDF marcado, olhando o estado "Verso recebido" na janela real.
5. Feito em 2026-11-05: `1.0.0-alpha.3` (`c248cf8`), com o cinza neutro no lugar do branco e o alerta no diálogo do verso. CI verde; portátil em `PlotS_1.0.0-alpha.3_x64_portatil.zip`, aberto e fechado sem erro. Falta o teste do usuário com papel e com o PDF marcado.
6. Feito em 2026-11-05: `1.0.0-alpha.4` (`97643ec`), com o `perfis.toml` ao lado do `PlotS.exe` no portátil e a importação única de `%APPDATA%`; o instalado não muda. CI verde; portátil em `PlotS` com `PLOTS-PORTATIL.txt` e sem o perfil dentro do zip. Falta mover a pasta para outro computador e conferir o perfil na prática.
7. Feito em 2026-11-06: `1.0.0-alpha.5` (`559a9c8`), com "Frente" = páginas ímpares e "Verso" = pares, 1 página sem folha em branco, desenho por bitmap compatível + `StretchBlt` e a chave de boca com bandeja/resolução/cor. CI verde; portátil em `PlotS_1.0.0-alpha.5_x64_portatil.zip`. Falta o teste em impressora real (L6170), um driver com bandeja/zero opções e um envio usando o `DEVMODE` salvo.
8. Feito em 2026-11-06: `1.0.0-alpha.6` (`6a972c6`), com a cor das imagens corrigida (o pdfium entregava RGBA e o GDI lia BGRA; laranja/amarelo saíam azuis). CI verde; portátil em `PlotS_1.0.0-alpha.6_x64_portatil.zip`. Falta conferir em impressora real e corrigir a prévia colorida do PlotSim.
9. Feito em 2026-11-07: Lote A do PlotInfo (`d6ae384`), com abertura e impressão de PNG/JPEG/BMP/GIF/WebP/TIFF por PDF derivado, cache reaproveitado e textos de “documento”. CI verde; painel de tinta e mutool ficam para os próximos lotes.
10. Feito em 2026-11-07: B1 do PlotInfo (`be877c1`), com a crate `plots-info` (seleção, cobertura, custo, perfis e preço por mL) e o `plotinfo.toml` com cinco comandos. CI verde; faltavam a análise, o mutool e o painel.
11. Feito em 2026-11-07: B2/B3/C do PlotInfo (`8a9866f`, `eab8d30`, `15bc485`, `0f66aa7`, `a699b22`, `8bfd509`), com análise PDFium, mutool/CSV, painel com gaveta/engrenagem, aviso traduzível, reanálise por dpi/modo/cinzas, CSV liberado e acessibilidade. Revisões de código e jurídico; falta a impressora real e a prova de tinta em papel.
12. Feito em 2026-11-07: `1.0.0-alpha.7` (`8bfd509`), portátil de 42,25 MB com `mutool/`, AGPL, 26 licenças do `thirdparty`/fontes, fonte fora do zip e SHA-256 `52449AF1…`; zip sem `perfis.toml`/`plotinfo.toml`, aberto e fechado com a versão certa. Falta testar em impressora real e provar a tinta em papel.
13. Feito em 2026-11-07: impressão de uma cópia por vez (`31-copia-por-vez`), com a caixinha acima da Quantidade, frentes separadas por cópia, avanço depois da confirmação da impressora, verso junto e automático junto/separado. CI verde, 1.173 testes de UI e build concluído. Falta conferir em impressora real.

Depois da primeira versão: impressora virtual, Linux e macOS, divisão por velocidade (`15-multiplas-impressoras`) e medir se o Graphify economiza tokens, com 3 perguntas respondidas com e sem ele.

## Bloqueado ou dependente do usuário

- Impressora real, que o usuário não tem à mão: confirmação dos planos `04-calibracao`, `07-cli-impressao` e `08-impressao-windows`.
- Executar o instalador em máquina ou conta de teste: plano `17-acabamento`.
- Conferir a janela real do driver, com OK e Cancelar: plano `20-preferencias-do-driver`.
- Conferir o frente e verso automático em impressora com duas faces: plano `21-frente-e-verso-automatico`.
