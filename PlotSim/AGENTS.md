# PlotSim — plano do simulador de impressoras

Nov 3, 2026 · @Kaio

## Visão geral e veredito

O PlotSim existe para uma coisa só: provar que o PlotS funciona. Ele cria impressoras falsas no Windows que o PlotS enxerga como reais, segue ao pé da letra a instrução de recolocação que o PlotS dá e confere se o maço final saiu em frente e verso correto — igual em todas as impressoras, mesmo com modos diferentes.

**Veredito: dá para fazer rápido, cerca de 3 semanas para uma pessoa, com uma condição:** as impressoras virtuais usam um driver genérico do próprio Windows, não o driver do fabricante. O comportamento físico (traseira ou gaveta, face para cima ou para baixo) é configurado no PlotSim, como no perfil do PlotS.

Drivers reais do fabricante ficam de fora: o objetivo é testar a ordem e as instruções do PlotS, não os drivers. O teste com bandeja e driver de verdade fica para quando houver a impressora.

O que o PlotSim testa que a simulação interna do PlotS não testa: o caminho real do Windows — lista de impressoras, `DEVMODE`, tamanho de papel, bandeja, spooler e status do trabalho.

## O que o PlotSim verifica

Um cenário só passa quando as três coisas estão certas:

1. **Ordem**: cada folha final tem a página ímpar *n* de um lado e *n*+1 do outro (ou a página em branco, no total ímpar), e o maço sai na sequência 1, 2, 3…
2. **Orientação**: frente e verso com o topo da página na mesma borda.
3. **Instrução**: o gesto que o PlotS mostrou, executado ao pé da letra pelo modelo físico, leva a esse resultado. Se o PlotS acertar a ordem mas mandar virar o maço errado, o cenário falha.

E o mesmo documento tem que dar o mesmo maço final em todas as impressoras virtuais, mesmo com modos diferentes. É isso que mostra que o PlotS está forçando as impressoras a trabalhar igual.

## Contrato com o PlotS

Para o PlotSim seguir a instrução sem interpretação humana, o PlotS passa a gerar o gesto como dado e monta o texto do cartão a partir desse dado.

| Campo | Valores | Exemplo |
| --- | --- | --- |
| `entrada` | traseira, gaveta, manual | traseira |
| `face_impressa` | para frente, para trás, para cima, para baixo | para trás |
| `borda_que_entra` | topo, base | topo |
| `inverter_ordem` | sim, não | não |

- O texto do cartão ("Pegue o maço como saiu, não embaralhe…") sai só desses campos; testar os campos é testar a instrução.
- Quando a impressora de destino é uma fila do PlotSim, o PlotS envia o gesto por um canal local (`\\.\pipe\plotsim`) junto com o número do lote. O PlotSim aplica o gesto na hora da recolocação.
- **Modo manual**: em vez do canal, você lê o cartão no PlotS e escolhe o gesto no PlotSim. Isso testa se a instrução é clara para uma pessoa.

## Suíte de testes

Dois níveis, com a mesma matriz de cenários:

- **Ponta a ponta (Windows)**: `plotsim suite` cria as filas virtuais, abre o PlotS em modo automático (`plots imprimir --teste`), imprime cada cenário de verdade pelo spooler, aplica os gestos e gera o relatório.
- **Rápida (sem spooler)**: o mesmo modelo físico roda em testes Rust comuns, alimentado pelo Planejador e pelo gesto do PlotS; milhares de casos em segundos, a cada mudança no código.

| Dimensão | Valores |
| --- | --- |
| Modo da impressora | Traseira face ↑, Gaveta face ↓ e as demais combinações do perfil |
| Páginas | 1, 2, 3, 9, 10, 11, 24, 101 |
| Lote | 1, 5 e 10 folhas, documento inteiro |
| Formato | A4, A5, Carta, Ofício, documento com formatos misturados |
| Impressoras ao mesmo tempo | 1; 2 em espelho; 2 em divisão |
| Erros de propósito | Gesto errado, folha a menos, página girada no PDF: o PlotSim tem que acusar |

- **Documentos de teste**: gerados pela suíte, cada página com número grande, seta para cima e o número também em texto, para a conferência ler.
- **Relatório**: página HTML com uma linha por cenário (passou ou falhou, e o motivo) e as folhas finais desenhadas como na janela.
- **Critério de pronto do PlotS**: a matriz inteira passando nos dois níveis.

## Como a impressora virtual funciona

Cada impressora virtual é uma fila normal do Windows: o driver "Microsoft Print to PDF" (já vem no Windows) ligado a uma porta local que grava num arquivo, sem abrir a janela "Salvar como".

1. Ao criar uma impressora, o PlotSim roda uma vez, como administrador: `Add-PrinterPort -Name "C:\ProgramData\PlotSim\jato\saida.pdf"` e `Add-Printer -Name "Jato (virtual)" -DriverName "Microsoft Print To PDF" -PortName …`.
2. O PlotS lista essa fila como qualquer impressora e imprime nela normalmente.
3. O Windows grava cada trabalho como PDF nesse arquivo.
4. O PlotSim observa a pasta, espera o spooler dar o trabalho como concluído, copia o PDF para o histórico e libera o arquivo.
5. Abre o PDF com `pdfium-render` (o mesmo do PlotS) e entrega as páginas ao modelo físico.

Como a saída é PDF, o texto das páginas sobrevive: o PlotSim consegue ler o número de cada página e conferir sozinho se cada folha ficou certa.

**Plano B**, se o Plano A falhar no teste do dia 1: driver "Microsoft PS Class Driver" numa porta TCP `127.0.0.1:9101`; o PlotSim escuta a porta e converte o PostScript com Ghostscript. A licença do Ghostscript (AGPL) serve para uso próprio; para vender, exige licença comercial.

## Simulação física e visualização

Cada impressora virtual tem um modo (Traseira, face para cima; ou Gaveta, face para baixo), os formatos de papel aceitos e o gesto de recolocação.

- **Passada 1 (pares)**: cada página recebida vira uma folha nova na bandeja de saída, empilhada conforme a face de saída.
- **Recolocação**: quando chega o segundo trabalho (ou ao clicar "Recolocar"), o PlotSim aplica o gesto: pega a pilha, vira e coloca na entrada. Na tela, o que já foi impresso passa para o verso: fica com 30% de opacidade e espelhado, como visto contra a luz. Se o gesto girar a folha, aparece também de cabeça para baixo.
- **Passada 2 (ímpares)**: cada página cai na próxima folha da entrada, na face em branco, com opacidade cheia, por cima da par espelhada.
- **Conferência**: lendo o número de cada página, marca cada folha como certa ou errada e diz o motivo ("verso de cabeça para baixo", "verso trocado com a folha 3").
- **Ver o outro lado**: "Virar folha" mostra o lado par em opacidade cheia e o ímpar espelhado.

O modelo físico é escrito à parte, sem reaproveitar o Planejador do PlotS. Se usasse o mesmo código, um erro de lógica passaria despercebido nos dois programas.

## Interface

Uma janela só, no mesmo padrão visual do PlotS.

&#91;embedded content: janela do PlotSim · exemplo após a passada 2\]

- **Lista à esquerda**: impressoras virtuais com estado (ponto + texto: "Ociosa", "Recebendo", "Aguardando verso"), modo e o botão "Nova impressora virtual".
- **Centro**: a pilha de saída da impressora selecionada, folha por folha, com o verso transparente e o resultado da conferência.
- **Rodapé**: resumo ("3 de 3 folhas corretas") e as ações "Virar folha", "Recolocar" e "Limpar bandeja".

## Arquitetura e stack

| Peça | Escolha |
| --- | --- |
| Janela | Tauri 2 + React, CSS Modules, tokens do PlotS |
| Criar e remover filas | PowerShell (`Add-PrinterPort`, `Add-Printer`, `Remove-Printer`), pedindo administrador só nessa hora |
| Receber trabalhos | `notify` (observar a pasta) + crate `windows` (`EnumJobsW`) para saber quando o trabalho terminou |
| Ler o PDF | `pdfium-render`: desenhar as páginas e extrair texto e ângulo |
| Modelo físico | Crate próprio `plotsim-physics`, com testes |
| Configuração | `serde` + `toml` em `%APPDATA%/plotsim` |

Reposítório: mesmo workspace Cargo do PlotS, como app separado (`plotsim/`), compartilhando só os tokens visuais e o wrapper do pdfium.

## Fases e estimativa

| Fase | Entrega | Dias úteis |
| --- | --- | --- |
| 0. Teste de viabilidade | Fila Print to PDF com porta arquivo imprimindo 3 vezes seguidas sem diálogo | 1 |
| 1. Filas virtuais | Criar, listar e remover impressoras virtuais pelo programa | 1–2 |
| 2. Receber trabalhos | Detectar trabalho concluído, guardar o PDF, extrair páginas e texto | 2 |
| 3. Modelo físico e conferência | Pilha, faces, gesto; checagem de ordem e orientação | 2–3 |
| 4. Contrato com o PlotS | Gesto como dado, canal local, modo `--teste` no PlotS | 2 |
| 5. Suíte | PDFs de teste, matriz de cenários, relatório | 2–3 |
| 6. Interface | Janela com lista e bandeja, modo manual | 3–4 |
| **Total** |  | **13–18** |

A suíte vem antes da janela, porque é ela que responde "o PlotS funciona?". A janela serve para ver os detalhes de um cenário e para o modo manual.

A Fase 0 decide tudo: se a porta arquivo gravar sem diálogo, segue o Plano A; se não, o Plano B soma cerca de 2 dias.

## Riscos e perguntas

| Risco | Efeito | Mitigação |
| --- | --- | --- |
| A porta arquivo sobrescreve o mesmo arquivo a cada trabalho | Dois trabalhos seguidos podem se perder | Copiar assim que o spooler conclui; uma pasta por impressora |
| Gravar sem diálogo pela porta arquivo não é documentado pela Microsoft | Pode mudar numa atualização do Windows | Plano B pronto (PS Class + TCP + Ghostscript) |
| Driver genérico tem bandejas e papéis genéricos | Não testa o nome exato da bandeja da sua impressora | Testar a bandeja de verdade quando houver a máquina |
| Criar filas exige administrador | Aparece o aviso do Windows (UAC) | Só na criação e remoção |

Decisões e pendências:

- [x] Nome confirmado: PlotSim.
- [x] A janela entra já na primeira versão, ou primeiro só a suíte com relatório?
