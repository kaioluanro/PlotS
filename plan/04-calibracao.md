# Calibração

**Estado:** Em andamento · OK 2 · PARTIAL 3 · MISSING 0 · BUG 0 · N/A 0

## Objetivo

Medir uma vez como cada impressora se comporta e preencher o perfil a partir de duas perguntas.

## Plano de implementação

- [x] `ajustar_perfil_por_calibracao` no núcleo, testado contra o simulador
- [x] `plots gerar-calibracao`, com número grande e seta
- [x] `plots calibrar X`: gerar o PDF, perguntar e gravar o perfil
- [x] Enviar a calibração à impressora pelo próprio PlotS (`c410c8c`), na linha de cada impressora, com figuras
- [x] A calibração identifica um driver que inverte quando a pessoa repara qual folha saiu primeiro (`c410c8c`)
- [ ] Confirmar o resultado em impressora real

**Limite conhecido:** um engano na última resposta que faltava pode ser aceito como certo. Nas simulações com um engano, isso ocorreu em 14% dos casos: cerca de 12% com o perfil certo e a instrução de leitura errada, e cerca de 2% com perfil errado gravado. Esse número não é a chance real, que depende de quanto as pessoas se enganam. A tela final mostra as respostas e deixa corrigir uma sem reimprimir. A hipótese de que os selos de certo e errado nas figuras induzem a resposta fica para o teste em papel.

## Pronto quando

As respostas preenchem `passada2_inversa` e `verso_girar_180`, e o perfil salvo produz o maço certo.

## Depende de

`06-perfis`, `08-impressao-windows`. A confirmação depende de impressora real.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A005 | `passada2_inversa` e `verso_girar_180` ficam "a calibrar", sem presunção | PARTIAL |
| PS-A070 | PDF de teste de calibração com números grandes e setas | OK (cf22592) |
| PS-A071 | Calibração: 2 folhas com páginas 1 a 4, duas perguntas e ajuste do perfil | PARTIAL |
| PS-A075 | `plots calibrar X` | PARTIAL |
| PS-A089 | Teste de calibração ajusta as respostas relativas ao perfil testado | OK |

## Especificação

| Parâmetro | O que significa | Jato de tinta (esperado) | Laser (esperado) |
| --- | --- | --- | --- |
| `passada1_inversa` | Enviar a frente da última para a primeira | sim | não |
| `passada2_inversa` | Enviar o verso da última para a primeira | a calibrar | a calibrar |
| `verso_girar_180` | Girar as páginas do verso 180° | a calibrar | a calibrar |
| `gesto_recolocar` | Como devolver o maço: virar ou não, qual borda entra primeiro | texto + figura | texto + figura |

Os valores "a calibrar" dependem de como você vira o maço. Por isso o programa não adivinha: imprime uma folha de teste e pergunta o que saiu.

O único ajuste que o PlotS não controla é a opção "inverter ordem" que alguns drivers têm no painel próprio. A calibração detecta se ela está ligada e o PlotS pede para desligar.
Calibração (Fase 2) em detalhe: o PlotS imprime 2 folhas com páginas "1" a "4" e setas; o usuário recoloca do jeito que já faz; depois responde duas perguntas — "o verso da folha 1 é o 2?" e "o 2 está de cabeça para baixo?". As respostas preenchem `passada2_inversa` e `verso_girar_180`.
