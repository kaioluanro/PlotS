# gesto-e-spooler: figura do gesto no cartão e avisos de papel acabado ou atolado

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia primeiro `E:\PlotS\plan\briefs\coordenacao-paralela-2.md`. Fontes: `plan/13-fluxo-e-recolocacao.md`, `plan/19-modelos-de-impressora.md`, `plan/15-multiplas-impressoras.md`, no worktree.

## Escopo

1. **Figura do gesto:** componente `FiguraGesto`, SVG em linhas desenhado por você, usando os tokens da interface, no cartão enquanto ele espera a recolocação. Mostra o maço, se vira ou não (e por qual borda) e o lado da face impressa na entrada. A impressora é genérica: nada que lembre um produto ou marca real.
   - Sai só dos dados que o backend já manda em `recolocar` (`lado`, `virada`). Toda combinação válida tem desenho próprio e correto. Sem dado (`lado` nulo, em fila não calibrada) ou com valor desconhecido, não há figura, e o texto segue sozinho. **Uma figura errada faz perder o lote**: confira cada combinação com a tabela do plano 19.
   - Sem animação obrigatória. Com a frase visível, a figura é decorativa para leitor de tela, sem repetir o texto. Funciona nos dois temas, em cores forçadas e na lateral na largura mínima.
2. **Avisos do spooler:** confira o que `plots-windows` e o cartão já fazem. Complete o que faltar para a consulta periódica distinguir **papel acabado**, **papel atolado** e **impressora sem resposta**, em cartão e CLI, lendo o estado da fila e do trabalho. O aviso some quando a condição passa; o botão do verso continua só depois da confirmação da frente. Estado desconhecido não vira alarme. Teste com backend falso; **não dá para provocar atolamento de verdade**, então relate isso como não verificado.
3. **Pendências do plano:**
   - não rolar a lista até o cartão com o ponteiro sobre ela;
   - o CLI diz "1 de 1 folhas": corrija para o que de fato ocorreu, sem inventar número;
   - restaurar o giro de `.girando` em movimento reduzido, que hoje para: um indicador de carregamento parado parece travado. Use mudança lenta de opacidade se a norma visual pedir.

## Testes

- Tabela: para cada par `lado` × `virada`, o desenho certo; par desconhecido não desenha nada.
- Rust com backend falso: cada condição do spooler vira o aviso certo, some ao passar e não libera o verso antes da hora.
- App real por CDP, nas simuladas: jato e laser no estado de recolocação, nos dois temas, em cores forçadas, em inglês e na largura mínima. Leia cada imagem. Com 6 páginas, as figuras precisam concordar com a frase e com a sequência do cartão.
