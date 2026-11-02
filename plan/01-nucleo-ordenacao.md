# Núcleo de ordenação

**Estado:** Em andamento · OK 9 · PARTIAL 1 · MISSING 0 · BUG 0 · N/A 0

## Objetivo

Calcular, sem nenhuma impressora, em que ordem e com que rotação cada página é enviada.

## Plano de implementação

- [x] `plano_documento` e `plano_lotes_por_papel`, com página em branco no total ímpar
- [x] Ordem das passadas configurável e 8 combinações de perfil testadas
- [x] Limite de páginas e reserva de memória protegida
- [ ] Decidir se a assinatura pedida `plano_lote(paginas, perfil)` ainda vale ou se a API atual a substitui

## Pronto quando

Testes unitários cobrem lotes pares, ímpares, 1 folha e as 8 combinações de perfil.

## Depende de

Nenhuma.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A002 | Perfil com os 4 parâmetros: `passada1_inversa`, `passada2_inversa`, `verso_girar_180`, `gesto_recolocar` | OK |
| PS-A004 | Jato (traseira) inverte a passada 1; laser (gaveta) não | OK |
| PS-A006 | Total ímpar recebe página em branco no fim, que entra na passada das pares | OK |
| PS-A007 | Passada 1 = pares, passada 2 = ímpares, com ordem configurável | OK (cf22592) |
| PS-A008 | Aplicar inversão e giro de 180 graus (`/Rotate 180` no verso) | OK |
| PS-A009 | Função pura `plano_lote(paginas, perfil) -> (Vec<Pagina>, Vec<Pagina>)`, com testes | PARTIAL |
| PS-A068 | Testes unitários: lotes pares, ímpares, 1 folha e as 8 combinações de perfil | OK |
| PS-A069 | Crate `plots-core` com lógica pura, sem E/S | OK |
| PS-A086 | Limite de 100 000 páginas e lotes grandes sem estouro | OK |
| PS-A088 | "Pares primeiro" é o padrão | OK |

## Especificação

### Perfis de impressora e lógica de ordenação

Cada impressora vira um perfil com quatro parâmetros, todos medidos por um assistente de calibração (seção Fases, etapa 2). Com eles, um único algoritmo gera a ordem de envio para qualquer impressora.

| Parâmetro | O que significa | Jato de tinta (esperado) | Laser (esperado) |
| --- | --- | --- | --- |
| `passada1_inversa` | Enviar a frente da última para a primeira | sim | não |
| `passada2_inversa` | Enviar o verso da última para a primeira | a calibrar | a calibrar |
| `verso_girar_180` | Girar as páginas do verso 180° | a calibrar | a calibrar |
| `gesto_recolocar` | Como devolver o maço: virar ou não, qual borda entra primeiro | texto + figura | texto + figura |

Os valores "a calibrar" dependem de como você vira o maço. Por isso o programa não adivinha: imprime uma folha de teste e pergunta o que saiu.

**Algoritmo do lote** (lote de *k* folhas = 2*k* páginas):

1. Pegar as páginas do lote. Se o total for ímpar, completar com uma página em branco no fim.
2. Passada 1 = páginas pares (2, 4, 6…). Passada 2 = páginas ímpares (1, 3, 5…), no verso. A ordem das passadas é configurável.
3. Aplicar o perfil: inverter a lista da frente se `passada1_inversa`; inverter a do verso se `passada2_inversa`; marcar `/Rotate 180` nas páginas do verso se `verso_girar_180`.
4. Gerar dois PDFs temporários (`passada1.pdf`, `passada2.pdf`) e enviar o primeiro.
5. Mostrar o gesto de recolocação e esperar o clique em "Imprimir verso".
6. Enviar o segundo.

Exemplo, lote de 5 folhas na jato de tinta: as pares são enviadas primeiro, como 10, 8, 6, 4, 2; a pilha sai com a 2 por cima. As ímpares vão no verso, na ordem 1, 3, 5, 7, 9 ou na inversa, conforme a calibração.

A lógica fica numa função pura, `fn plano_lote(paginas, perfil) -> (Vec<Pagina>, Vec<Pagina>)`, testada com testes unitários sem nenhuma impressora.

A ordem das passadas fica configurável ("pares primeiro" é o padrão); com total ímpar, a página em branco entra na passada das pares.
