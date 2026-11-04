# Avisos no topo

**Estado:** Em andamento

## Objetivo

Decisão do usuário em 2026-11-04: os cartões de andamento saem da lista de impressoras. A lista volta a ser só lista, e o que o cartão dizia vira aviso no topo da janela, com um lugar para rever os avisos.

## Comportamento

- **Sem cartões na lista de impressoras.**
- **Aviso no topo da janela:** quando um trabalho muda de estado, um aviso aparece no topo e some sozinho, sem cobrir a barra de título. Erro e o que pede ação da pessoa ficam até serem fechados.
- **Sem botão de imprimir nos avisos:** frente e verso saem só pelo rodapé.
- **Ícone ao lado do histórico:** abre o painel com os avisos da sessão, do mais novo para o mais antigo, com o gesto de recolocar quando houver, e com um número de não lidos.
- **Cancelar e Reimprimir, escolha do coordenador:** ficam no painel, só em trabalho em andamento ou com erro, porque sem eles não haveria como parar um trabalho nem refazer um trecho. Se o usuário não quiser, saem.
- **O gesto continua na confirmação do verso.**
- **Versão final sem simuladas:** uma opção de compilação tira da lista "Simulada — Traseira", "Simulada — Gaveta" e "Simulada — Automática". `scripts/empacotar.mjs` compila sempre sem elas. Os testes e as cenas seguem com elas.
- **Calibrar visível:** botão ao lado da engrenagem nas filas, em vez de só uma opção escondida no seletor de modelo.

## Plano de implementação

- [ ] Avisos no topo e painel ao lado do histórico
- [ ] Tirar os cartões da lista de impressoras
- [ ] Opção de compilação sem simuladas, usada pelo instalador
- [ ] Botão Calibrar nas filas

## Pronto quando

Um fluxo de frente e verso com duas impressoras corre sem nenhum cartão na lista, com todos os avisos no topo e no painel, e o instalador não mostra as simuladas.

## Depende de

`13-fluxo-e-recolocacao`, `11-lateral-impressoras`, `17-acabamento`, `24-impressao-sem-lote`.
