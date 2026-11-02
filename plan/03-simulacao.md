# Modo simulação

**Estado:** Em andamento · OK 7 · PARTIAL 2 · MISSING 1 · BUG 0 · N/A 1

## Objetivo

Provar a lógica inteira sem impressora, inclusive os erros de recolocação.

## Plano de implementação

- [x] `Simulador` e `SimBackend` com o mesmo trait dos backends reais
- [x] `plots simular` com `--guiado`, `--ordem` e `--simular-erro`
- [x] Resultado certa/errada por folha e por lote
- [x] Propriedade com `proptest` nos 8 perfis
- [x] PDF de folhas finais com `--folhas-finais`
- [x] Tabela fixa de mecânica por perfil e contagem do laço exaustivo
- [x] Corrigir o modelo da gaveta: saída no topo e entrada pela folha de cima
- [ ] Teste de folha final com face girada
- [ ] Criar impressoras virtuais com qualquer combinação de parâmetros
- [ ] Vista Simulação, chave por documento e selo na interface

## Pronto quando

Maço final 1|2, 3|4… sem folha de cabeça para baixo em todos os perfis, e fluxo completo testável na interface.

## Depende de

`01-nucleo-ordenacao`, `02-geracao-pdf`; a parte visual depende de `09-janela-e-abas`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A028 | Rodar o fluxo inteiro sem impressora (`simular`) | OK |
| PS-A029 | Chave "Simular impressão" por documento; botão vira "Simular frente"; selo SIMULAÇÃO | MISSING |
| PS-A030 | Duas impressoras virtuais "Simulada — Traseira" e "Simulada — Gaveta", com outras combinações possíveis | PARTIAL |
| PS-A031 | `SimBackend` implementa o mesmo trait dos backends reais | OK |
| PS-A032 | Modelo físico: folha `{frente, verso, girada}`, pilha de saída, gesto como operação | OK |
| PS-A033 | "Simular erro": recolocar sem virar, girado, ou com uma folha a menos | OK (cf22592) |
| PS-A034 | Vista por lote com passada 1, pilha, passada 2 e folhas finais marcadas certo/errado ("3 de 3 folhas corretas") | PARTIAL |
| PS-A037 | Exportar PDF de folhas finais (frente e verso lado a lado) | OK (6baa24a) |
| PS-A038 | Modelo com `proptest`: 8 perfis, 1 a 200 páginas, lotes de 1 a 50 folhas, maço final 1\|2, 3\|4 | OK (6baa24a) |
| PS-A073 | Backend Simulado com impressoras virtuais | OK |
| PS-A093 | "Limite: a simulação prova a lógica; só a impressora real confirma o perfil" | N/A |

## Especificação

### Modo simulação

Com "Simular impressão" ligado, o PlotS faz tudo igual — ordena, divide em lotes, pede a recolocação — mas, em vez de mandar ao spooler, mostra o que sairia de cada impressora. Assim o programa inteiro pode ser feito e testado sem impressora.

&#91;embedded content: vista Simulação · exemplo com 6 páginas\]

**Na interface**

- Uma chave "Simular impressão" no topo da barra lateral, por documento (como a seleção de impressoras).
- Com ela ligada, o botão vira "Simular frente", a aba ganha a vista **Simulação** ao lado de **Documento** e um selo SIMULAÇÃO fica visível o tempo todo.
- A vista mostra, por lote: o envio da passada 1, a pilha na saída, o envio da passada 2 e as folhas finais com frente e verso. Cada folha recebe ícone de certo/errado e texto (ex.: "3 de 3 folhas corretas").
- O cartão EM ANDAMENTO funciona igual: "Imprimir verso" vira "Simular verso", então o fluxo de recolocação também é testado.

**Impressoras virtuais**: aparecem na lista duas impressoras de teste, "Simulada — Traseira" (face para cima) e "Simulada — Gaveta" (face para baixo). Dá para criar outras com qualquer combinação dos quatro parâmetros do perfil.

**Por dentro**

- `SimBackend` implementa o mesmo `trait Backend` dos backends reais; o resto do programa não sabe que é simulação.
- Modelo físico: cada folha é `{frente, verso, girada}`; a bandeja de saída é uma pilha; o gesto de recolocação é uma operação sobre ela (inverter a ordem, virar as folhas, girar 180°).
- "Simular erro": recolocar sem virar, girado ou com uma folha a menos, para ver o resultado errado e testar "Reimprimir lote".
- Exportar `passada1.pdf`, `passada2.pdf` e um PDF de folhas finais (frente e verso lado a lado) para abrir no Firefox.
- O mesmo modelo roda em testes com `proptest`: para as 8 combinações de perfil, totais de 1 a 200 páginas e lotes de 1 a 50 folhas, o maço final tem que ser 1|2, 3|4… sem folha de cabeça para baixo.

**Limite**: a simulação prova que a lógica está certa para um perfil; só a impressora real confirma que o perfil corresponde à máquina. Quando houver impressora, basta rodar a folha de calibração uma vez.

O exemplo do desenho usa a passada 2 em ordem crescente; a ordem real depende do gesto escolhido no perfil.
