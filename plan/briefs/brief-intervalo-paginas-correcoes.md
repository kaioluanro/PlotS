# intervalo-paginas: correções da revisão de interface

Responda em português brasileiro. Trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-ab7e164962d03f27d`, branch `worktree-agent-ab7e164962d03f27d`, sobre o wip `926935b`. Não crie outro worktree. Leia `E:\PlotS\plan\briefs\coordenacao-paralela-2.md` (regras comuns e arquivos permitidos), o brief original `brief-intervalo-paginas.md` e o relatório anterior em `%LOCALAPPDATA%/plots-build/intervalo-paginas/relatorio.md`. Pasta de build e saídas: a mesma, `plots-build/intervalo-paginas`. `ui/node_modules` foi apagado: rode `npm ci`. A cena por CDP está em `saidas/cenas/`; use essa cópia, porque ela já escolhe a janela certa.

A revisão de código e a jurídica aprovaram. A de interface rejeitou. Aplique a lista abaixo, nesta ordem; ela é obrigatória. **Não mude o planejamento no Rust.**

1. Remova a regra que esconde o bloco em `max-height: 560px` (`BlocoIntervalo.module.css`). Com rodapé presente, a linha "Páginas [ ] a [ ]" fica sempre visível, porque ela decide quais páginas saem. Lote e resumo continuam cedendo espaço. `data-sem-destino` continua escondendo o rodapé inteiro.
2. Teste por CDP em 640x420, com cartão e erro: em um PDF de 200 páginas, de "100 a 20" para "10 a 20". O foco fica no campo, e nenhuma tecla se perde.
3. Troque o botão de texto "Usar o documento inteiro" por botão de ícone `RotateCcw`: ícone de 14 px, caixa de 28x28, na linha dos campos, à esquerda deles, com vão de 4 px. `title` e `aria-label` com o texto atual. Só aparece com `aba.intervalo != null`. Sem fundo em repouso, ícone `--fg-muted`; no hover, `--panel-hover` com ícone `--fg`; em cores forçadas, `ButtonText`. Ao usar, o foco vai para o primeiro campo.
4. Erro e motivo do botão só aparecem depois de 600 ms sem digitar, ao sair do campo ou com Enter. O botão desabilita na hora. Ao ficar válido, os dois somem na hora. Um envio nunca pode sair com o valor do meio da digitação. Teste com Vitest e temporizador falso.
5. Reordene o rodapé: Páginas, erro de páginas, Folhas por lote, resumo ou estado, botão, motivo. 8 px entre grupos e 4 px dentro do grupo. A ordem de Tab acompanha.
6. Em `.campoInvalido`, some `box-shadow: inset 0 0 0 1px var(--status-offline)`, para o erro não depender só da cor. Em cores forçadas, `Highlight`.
7. `intervalo.botao.invalido`: "Corrija as páginas para imprimir." / "Fix the pages to print."
8. Omita `aria-valuemax` quando `total === 0`. Use `aria-valuetext` com o texto atual quando não houver `aria-valuenow`.
9. Hover do campo em cores forçadas: `.campoLote:hover:not(:disabled) { outline: 1px solid Highlight; }`.
10. Capturas nos dois temas e idiomas, por CDP: 03, 04, 05, 07, 15, 16, 17 e 18 refeitas; meio da digitação, antes de 600 ms; hover do ícone sem foco; sem documento; `naoNumero`; `foraUma`; totais de 4 e de 5 dígitos (confira se o número cabe no campo de 56 px; se cortar, ajuste a largura sem quebrar a linha); cores forçadas por `Emulation.setEmulatedMedia`. Leia cada imagem, inclusive a área do documento quando for relevante.

Duas conferências da revisão de código, sem obrigatoriedade: rode o `prettier` nos arquivos seus, se for regra do projeto, em `Sidebar.tsx`; não mude mais nada.

Depois: CI completo, com todas as etapas em um único log real, se o sistema permitir; autoverificação; `git status` limpo, com novo wip; apague `ui/node_modules` e `ui/dist`. Relatório novo em `plots-build/intervalo-paginas/relatorio-correcoes.md`, com a mesma mensagem na resposta final: item a item, testes, linha do CI, capturas com caminho, o que não foi verificado.
