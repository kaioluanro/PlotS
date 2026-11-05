# Verso recebido: corrigir o que a interface apontou

Rodada de correção do lote `agent-verso-recebido`. Leia `E:\PlotS\AGENTS.md`,
`.claude/skills/craft-team/SKILL.md`, `.claude/agents/plots-implementer.md` e o brief original
`plan/briefs/brief-verso-recebido.md`. Worktree existente
`E:\PlotS\.claude\worktrees\agent-verso-recebido`, HEAD `17107f6`, sem criar outro. Build em
`%LOCALAPPDATA%\plots-build\verso-recebido\target`; CDP 9372; `npm ci` em `PlotSim/app/ui/`.

**Graphify:** antes do primeiro grep ou leitura de código, `graphify query "<pergunta>"
--graph E:/PlotS/graphify-out/graph.json`. Ao terminar, `graphify update .`.

## Obrigatório (da revisão de interface)

1. **Nenhuma superfície pode dizer "aguardando verso" quando o verso já chegou.**
   Hoje, com a lista em "Verso recebido", o rodapé diz "3 folhas, aguardando verso" e cada
   folha diz "Aguardando verso" (inclusive no `aria-label`).
   - `resumoDaBandeja` (`tela.ts`): com `verso_recebido`, usar chave nova
     "3 folhas, verso recebido" / "3 sheets, back side received" e o singular correspondente.
   - Situação da folha: com a passada do verso recebida e falta o gesto, usar
     "Verso recebido, falta recolocar" / "Back side received, reload" em vez de
     "Aguardando verso", inclusive no rótulo acessível.
   - Testes nos dois idiomas, com falha prévia.
2. **O rótulo do botão segue a vista.** `virarFolha` alterna. Se a folha está virada, o botão
   diz "Ver a frente da folha {n}" / "See the front of sheet {n}" (e "Ver a frente" no botão
   sem seleção); se não está, "Ver o outro lado da folha {n}" / "See the other side of sheet
   {n}". O `title`/descrição continua explicando que só muda a vista.
3. **Provas que faltam:**
   - captura em inglês de `vr-2` (verso recebido) e `vr-3` (folha virada);
   - captura no tema escuro das duas listas, com o ponto e a dica;
   - captura de "Ver o outro lado" **enquanto ainda está "Aguardando verso"** (o caso do
     relato);
   - tabela de contraste dos temas exigidos pela norma para os textos novos.

## Também, da revisão de código (barato)

4. `aria-description` do botão deve juntar a descrição e o motivo de estar desabilitado, como
   já acontece no `title`; hoje o leitor de tela perde o motivo.
5. `dicaDaEspera` devolve `null` quando a bandeja está `recebendo` ou tem aviso, para a linha
   não contradizer "Recebendo"/"Com aviso".
6. Teste de precedência: aviso ativo vence `verso_recebido` em `estado_da_bandeja`.
7. Comentário de `chaveDoDesenhoFalho` sem dizer que a frente "passa a ser o verso".

## Final

- CI em log único `saidas/ci.log`, mesmos passos, terminando em `CI_EXIT 0`; wip commit com
  árvore limpa; `node_modules`/`dist` apagados; filas `Uso *` removidas; seus PIDs encerrados.
- Não mexa em `plan/`.
- Resposta final item a item, com falha prévia (caminho), capturas lidas, contraste, linha do CI
  e o que não foi verificado.
