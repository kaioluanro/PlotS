# gesto-e-spooler: correções da revisão de interface

Responda em português brasileiro. Trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-a0909ac7237a0b0a7`, branch `worktree-agent-a0909ac7237a0b0a7`, sobre o wip `c616275`. Não crie outro worktree. Leia `E:\PlotS\plan\briefs\coordenacao-paralela-2.md`, `brief-gesto-e-spooler.md`, a tabela de `plan/19-modelos-de-impressora.md` e o relatório anterior em `%LOCALAPPDATA%/plots-build/gesto-e-spooler/relatorio.md`. Build e saídas: a mesma pasta `plots-build/gesto-e-spooler`. `ui/node_modules` foi apagado: rode `npm ci`. As cenas por CDP estão em `saidas/cena/`; use essa cópia.

A revisão de código aprovou. A jurídica só pede um registro, que o coordenador faz. A de interface rejeitou. Aplique a lista abaixo, nesta ordem; ela é obrigatória.

## Exceções ao mapa de arquivos

Pode mexer também em: `ui/src/components/Sidebar.tsx`, só para a linha do item 7 (um import e uma linha de uso, com o componente em arquivo novo); `src-tauri/src/trabalhos.rs` no que os itens 1 e 8 precisarem; `ui/src/i18n.ts` com prefixo `gesto.*` e `spooler.*`.

## Correções

1. **Avisos por passada**, decididos por um código estruturado da condição vindo do backend, nunca por busca na frase. Em pt-BR e en no cartão; em pt-BR no CLI.

| Condição e passada | Linha forte (pt / en) | Instrução (pt / en) |
|---|---|---|
| Sem papel, frente | A impressora está sem papel. / The printer is out of paper. | Coloque folhas em branco na entrada. / Add blank sheets to the feed. |
| Sem papel, verso | A impressora acusa falta de papel. / The printer reports it is out of paper. | Não coloque folhas em branco: na entrada fica só o maço impresso. Confira se ele está encaixado e retome na impressora. / Do not add blank sheets: the feed holds only the printed stack. Check that it is seated, then resume at the printer. |
| Atolado, frente | O papel atolou na impressora. / The paper is jammed in the printer. | Retire a folha presa. / Remove the stuck sheet. |
| Atolado, verso | O papel atolou na impressora. / The paper is jammed in the printer. | Retire a folha presa. O lote pode ter desalinhado: confira antes de seguir. / Remove the stuck sheet. The batch may be misaligned: check it before going on. |
| Sem resposta | A impressora não responde. / The printer is not responding. | Confira se ela está ligada e conectada. / Check that it is on and connected. |

   Ressalva só em "sem papel, verso": "Se o maço acabou antes do fim, o lote desalinhou: use Reimprimir lote." / "If the stack ran out early, the batch is misaligned: use Reprint batch." No CLI, em vez do botão, diga o que o CLI de fato oferece.
2. No estado, troque "Aguardando a impressora" por "Verifique a impressora" / "Check the printer" nas três condições. `.fraseForte` passa a 12/16, 600.
3. Ordem do cartão: estado, dados e sequência; ressalva e espera; **figura; gesto**; clique; botões.
4. Tipografia das frases: todas em 12/16; gesto em 600 com `--fg`; linha do clique em 400.
5. **Cena da entrada:** use `MacoDeCima` também na cena 2, em `x=104, y=30`; `DOBRA` de 14; linhas de tinta com passo de 4; seta `M118 27V18H149`, com ponta; impressora em `x=152, y=1`; `viewBox` 196×84. Corrija também o traço que perde meio pixel na borda direita, sem aumentar a área além do necessário.
6. **Qual extremidade entra primeiro: não mude a frase.** O coordenador conferiu: o modelo só distingue borda curta de longa, não cabeça de pé. Fica registrado como pendência do teste com impressora real. A seta sai da borda curta, sem marcar cabeça ou pé.
7. **Rolagem adiada:** enquanto houver rolagem adiada, mostre uma linha fixa fora da área rolável, entre a lista e o rodapé: mínimo de 28 px, 12/16, `--fg`, com ícone de seta. "Um cartão pede ação" / "A card needs you"; "{n} cartões pedem ação" / "{n} cards need you". O clique rola até o cartão, e a linha some ao rolar. É um botão de verdade, com foco e rótulo.
8. **Reimprimir no aviso.** Hoje o aviso só oferece Cancelar, e cancelar encerra o trabalho, sem como refazer o lote. **Decisão do coordenador:** com aviso em qualquer passada, o cartão também mostra "Reimprimir lote". O backend passa a aceitar esse comando no aviso. Em fila real, a confirmação que já existe continua, e cancelamento que falhar continua sem reenviar a frente. Confira se o texto dessa confirmação diz que a frente sai de novo e pede folhas em branco; se não, ajuste. Teste Rust: comando no aviso na frente e no verso, e cancelamento que falha não reenvia. Teste de interface: botão presente no aviso.
9. Gire 180° as linhas da cena 2 em `cima+girar` e `cima+borda_de_cima`, cobrindo isso no teste da tabela.
10. **"Não virar":** mesmo arco e escala de `ViraPelaBordaDeCima`, com o X no ponto médio e o maço em `x=26, y=40`. Não pode lembrar o ícone de desfazer.
11. Botão do verso visível e desabilitado no aviso com `passada === "verso"`.
12. `scroll-margin-top` e `scroll-margin-bottom` de 8 px no cartão.
13. **Opcionais da revisão de código, faça se forem de uma linha:** o rótulo `GetJobW` em `ler_com_repeticao`; pular `ler_estado` com o trabalho já terminal.
14. **Capturas** nos dois temas, em cores forçadas e em inglês: galeria das 8 combinações; os cartões jato e laser; aviso de cada condição, inclusive na passada de verso; a linha de rolagem adiada, com foco; a confirmação de reimprimir. Leia cada imagem no tamanho real. Confira contraste de qualquer par novo.

Depois: CI completo em um único log real, se o sistema permitir; autoverificação; `git status` limpo, com novo wip; apague `ui/node_modules` e `ui/dist`. Relatório novo em `plots-build/gesto-e-spooler/relatorio-correcoes.md`, com a mesma mensagem na resposta final: item a item, testes, linha do CI, capturas com caminho, textos novos, o que não foi verificado.
