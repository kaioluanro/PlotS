# PlotSim

**Estado:** Em andamento

## Objetivo

Segundo programa, parte da suíte de testes: cria impressoras falsas no Windows que o PlotS vê como reais, segue ao pé da letra a instrução de recolocação e confere o maço final. A especificação do usuário está em `PlotSim/AGENTS.md`.

## O que pode e o que não pode ser reaproveitado

O usuário pediu para reaproveitar o código do PlotS. A especificação proíbe só uma parte, com razão: se a conferência usar o mesmo código que gerou a ordem, um erro passa despercebido nos dois.

| Reaproveita do PlotS | Nunca reaproveita na conferência |
|---|---|
| Rasterização e leitura com PDFium, geração de PDF (`plots-pdf`) | Planejador (`plano_*`) |
| Leitura do spooler e das filas (`plots-windows`) | `Simulador`, `SimBackend`, `SimGuiado`, `conferir_folhas` |
| Formato de arquivo e gravação atômica (`plots-perfis`) | Tabela de `Mecanica` e os gestos de `Instrucao` |
| Componentes, tokens e diálogos da interface (`ui/`) | |
| Inicialização do Tauri, janela e barra de título | |

Na suíte rápida, o Planejador e o gesto do PlotS são a **entrada** do teste, e o modelo físico do PlotSim é quem julga.

## Decisões

- **Pasta:** `PlotSim/`, com crates `PlotSim/plotsim-physics`, `PlotSim/plotsim-core` e o app em `PlotSim/app`, no mesmo workspace Cargo do PlotS.
- **Estilo:** a norma de `plan/estilo-ui-ux.md` e os tokens do PlotS.
- **Janela na primeira versão, depois da suíte**, como marcado na especificação.
- **Limite do que o PlotSim prova:** ele prova o caminho do Windows e a coerência da instrução com um modelo físico. O modelo físico é uma hipótese, como no plano 19: não substitui o teste em papel.

## Plano de implementação

- [ ] Fase 0: fila Print to PDF com porta de arquivo, 3 envios sem diálogo. Decide entre os planos A e B
- [ ] Fase 1: criar, listar e remover filas
- [ ] Fase 2: detectar trabalho concluído, guardar o PDF, extrair páginas e texto
- [ ] Fase 3: modelo físico e conferência, com a suíte rápida
- [ ] Fase 4: gesto como dado, canal local e `plots imprimir --teste`, depois de as opções de envio entrarem
- [ ] Fase 5: suíte ponta a ponta, PDFs de teste, matriz e relatório
- [ ] Fase 6: janela e modo manual

## Pronto quando

A matriz da especificação passa nos dois níveis, e os erros de propósito são acusados.

## Depende de

`08-impressao-windows`, `19-modelos-de-impressora`, `21-frente-e-verso-automatico`, `22-opcoes-de-envio-e-historico`.
