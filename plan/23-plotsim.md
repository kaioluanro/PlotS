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

## Resultado da suíte rápida

- **Divergência com o PlotS:** na laser, recolocar o maço como saiu, com o verso sem giro, deixa todo verso de cabeça para baixo. A revisão refez a conta à mão e confirmou. Hipóteses: a curva em U vira a folha uma vez; o gesto é um transporte sem girar; o verso é lido como página de livro; a folha entra pela borda curta. Falta confirmar em papel. Correção à espera da decisão do usuário.
- **Regra da borda:** sem giro, a aresta que entrou primeiro na frente tem de entrar primeiro de novo; com giro, tem de ser a oposta. "Borda curta" não basta, porque as duas bordas curtas dão resultados opostos.
- **Fontes:** guia da laser e manual do jato com duas faces, em leitura de fatos, sem cópia. O manual da HP não foi lido: o site recusou, e a tentativa seguinte foi negada. Se o usuário o entregar, entra. As cópias locais dos manuais foram apagadas.
- **Pendente, menor:** comentários nos testes que documentam a divergência e têm de ser invertidos com a correção; diagnóstico da conferência em total ímpar; pilha com `VecDeque` para documentos grandes. A matriz ainda não cobre formatos de papel.

## Plano de implementação

- [ ] Fase 0: fila Print to PDF com porta de arquivo, 3 envios sem diálogo. Decide entre os planos A e B
- [ ] Fase 1: criar, listar e remover filas
- [ ] Fase 2: detectar trabalho concluído, guardar o PDF, extrair páginas e texto
- [x] Fase 3: modelo físico e conferência, com a suíte rápida (`f1afcee`)
- [ ] Fase 4: gesto como dado, canal local e `plots imprimir --teste`, depois de as opções de envio entrarem
- [ ] Fase 5: suíte ponta a ponta, PDFs de teste, matriz e relatório
- [ ] Fase 6: janela e modo manual

## Pronto quando

A matriz da especificação passa nos dois níveis, e os erros de propósito são acusados.

## Depende de

`08-impressao-windows`, `19-modelos-de-impressora`, `21-frente-e-verso-automatico`, `22-opcoes-de-envio-e-historico`.
