# cartoes-impressao: rodada de correções

Responda em português brasileiro. Worktree: `E:\PlotS\.claude\worktrees\agent-ab44ddabab049094e`, branch `worktree-agent-ab44ddabab049094e`, último wip `ea1fd28`, base `c50e6d1`. **Trabalhe só nele.** Nunca dê push nem squash. Leia antes `brief-cartoes-impressao.md` e `coordenacao-paralela.md`, nesta pasta, `plan/13-fluxo-e-recolocacao.md` e `plan/19-modelos-de-impressora.md`, e os módulos citados abaixo. Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; o grafo pode não cobrir a interface.

**Ambiente:** compilação em `C:/Users/Kaio Luan/AppData/Local/plots-build/cartoes`, `CARGO_INCREMENTAL=0`, `-j 4`. Scripts de cena e capturas ficam em `.../plots-build/cartoes/saidas` (`e2e/`, `shots/`). Só `ci.log` e `fail-before.log` ficam em `target/`. Rode `npm ci` em `ui/`. `PLOTS_PDFIUM_DIR=C:/Users/Kaio Luan/AppData/Local/plots-build/pdfium/chromium-8125/bin`. Abaixo de 500 MB livres em E:, pare e relate.

**Segurança:** envie só às simuladas e à fila "Microsoft Print to PDF", com `PLOTS_ARQUIVO_SAIDA`. Confira a fila depois. Capturas por CDP, `PLOTS_PERFIS` temporário, nunca o `%APPDATA%` real. Encerre só seus PIDs, com porta e pasta do WebView2 exclusivas. Se o sistema negar algo, pare e relate. Não edite `plan/`: relate as exceções, que eu registro.

Registre a falha prévia de cada teste novo, com execução real. A autoverificação recusa as expressões inglesas "next batch" e "this batch": use os textos abaixo, que as evitam.

## A. Código e segurança (primeiro)
1. **Fechar a aba durante `preparando` ainda envia a frente.** Em `src-tauri/src/trabalhos.rs`, `conduzir`: antes do primeiro `enviar`, esvazie os recados com `try_recv`. Com `Soltar`, `Cancelar` ou canal fechado, retorne sem enviar. Teste: solta durante a preparação e o backend não recebe nenhum `enviar`.
2. **Cartão de thread morta é um beco sem saída.** Em `ui/src/App.tsx` (`comandar`) e `ui/src/cartoes.ts` (`cartaoInterrompido`, `acoesDoCartao`): se o comando falhar com a thread já morta, o cartão precisa poder ser descartado e liberar a impressora, sem diálogo, em vez de recriar o erro. Teste de `reduzirAbas`.
3. **Recusa muda do `Gerente`.** Em `App.tsx`, `iniciar`: compare o retorno de `iniciar_impressao` com os destinos e, para cada recusada, mostre cartão de erro com motivo "ocupada" (nova chave `cartao.motivo.*`, nos dois idiomas). Teste.
4. **Cartão diz "cancelado" com o trabalho ainda na fila.** `plots-windows/src/backend.rs:325`: `cancelar` faz `self.trabalho.take()` antes de cancelar. Se o cancelamento falhar, o id se perde e a chamada seguinte devolve `Ok` sem cancelar nada. Só solte o id depois do sucesso, e propague a falha até o cartão, com texto que mande conferir a fila. Teste com backend falso; se o trecho não for testável sem spooler, extraia a decisão para uma função pura.
5. **"Reimprimir lote" em fila real pede confirmação**, pelo `ConfirmDialog`, porque cancela e reenvia. Simulada não confirma.
   - Título: "Reimprimir o lote {n} de {total} em {nome}?"
   - Texto: "O que restar deste lote sai da fila e a frente é enviada de novo. Coloque papel em branco na entrada."
   - Botões: "Voltar" e "Reimprimir lote". Equivalentes em inglês.
6. **Rótulo do botão pelos `destinos`, não pelas `marcadas`** (`cartoes.ts:163`). Hoje, com uma simulada válida e uma fila real inválida marcadas, o botão diz "Imprimir frente (1)" e só simula, sem confirmação. Teste.
7. **Chaves de lista** em `ConfirmDialog.tsx` e `CartaoImpressao.tsx` não podem ser o próprio texto: linhas iguais geram chaves duplicadas.

## B. Texto do gesto e do cartão
8. **A frase da jato manda colocar e só depois virar: quem segue na ordem perde o lote.** Mantenha a frase fixa do plano e troque a segunda linha por "Antes de colocar, vire o maço pela borda de cima, como um bloco de notas." / "Before placing it, flip the stack over its top edge, like a notepad." Faça o mesmo em lateral e girar, **na interface e no CLI**, pelo mesmo código (`plots-print/src/fluxo.rs:179-186`, `ui/src/cartoes.ts:317-322`, testes do CLI). Com perfil personalizado, continue sem afirmar lado. Confira cada combinação do plano 19.
9. **Última linha:** "Com o maço na entrada, clique em Imprimir verso." / "With the stack in the feed, click Print back."
10. **Motivo do verso desabilitado visível**, sem depender de `title`: "A frente está indo para a fila. O verso libera quando a fila confirmar." Na simulação, frase equivalente (`CartaoImpressao.tsx:89`).
11. **Hierarquia:** `frasesDoCartao` devolve o papel de cada linha. Ação em 12/16 px `--fg`; ressalva da fila em 11 px `--fg-muted`; 8 px entre os grupos; `.sequencia` logo abaixo de lote e papel.
12. **Simulação:** antes do gesto, "Na impressora real, o próximo passo seria:" / "On a real printer, the next step would be:". Depois, "Clique em Simular verso para conferir." / "Click Simulate back to check."
13. **`proximo_lote`:**
    - Estado "Pronto para o próximo lote" / "Ready for the following batch".
    - "O verso do lote anterior terminou ou saiu da fila."
    - Sem troca de papel, "Retire as folhas do lote anterior e coloque papel em branco."
    - "Depois, clique em Imprimir frente."
    - No inglês, troque "set of sheets" por "batch" em tudo.
14. **`preparando`:** tom `ativo`, frase "Preparando as páginas para enviar." / "Preparing the pages to send.", sem botão de verso.
15. **Estado "Imprimindo" (jurídico):** o programa só conhece a fila. Mude só o texto de `cartao.estado.imprimindo` para "Enviando à fila" / "Sending to the queue", e ajuste `cartoes.test.ts:240`.
16. **Cancelar (jurídico e interface):** `cartao.cancelarDialogo.texto` em inglês: "PlotS removes from the queue whatever of the current batch is still waiting there." Confira que o código cancela mesmo o lote atual.
17. **Fechar:** `confirmar.texto` e `textoVarios` em inglês: "The print of {nome} has not finished.", porque "is still printing" é falso em recolocação. Troque "passada" e "pass" por "Um envio em curso pode ficar incompleto na fila; confira a fila do Windows." / "A job still being sent may stay incomplete in the queue; check the Windows queue."
18. **Termos em inglês:** `Back` ao lado de `Send the front` vira `Cancel`; `Waiting for reload` vira `Waiting for you to reload the stack`. "Na gaveta, vire…" vira "Vire o maço antes de ler as folhas." / "Flip the stack before reading the sheets."

## C. Layout (`Sidebar.*`, `CartaoImpressao.*`, `Botao.*`, `TitleBar.*`, `theme.css`)
19. **Rodapé:** uma só linha quando todas as marcadas estão ocupadas: "Em andamento nos cartões acima." / "In progress in the cards above." Em `max-height: 560px` com cartão ativo, oculte a linha de lote e o resumo (relate, porque registro como exceção ao plano 11). Quando um cartão passa a pedir ação (`aguardando`, `proximo_lote`, `aviso`, `erro`), `scrollIntoView({ block: "nearest" })` sem tirar o foco.
20. **Aba com trabalho:** ponto de 6 px em `TitleBar`, `--status-waiting` quando pede ação e `--status-working` em curso, com `title` e `aria-label`: "seis.pdf, aguardando você" / "seis.pdf, waiting for you".
21. **Tokens do tema claro:** `--status-working #059669`, `--status-waiting #c2610a`, com os `-soft` alinhados. Comentário da exceção em `theme.css`, sem editar a norma. Remeça com `contrast.py` sobre `#eef2f7`, `#f6f7fb`, `#e9eaee` e `#ffffff`. O tema escuro não muda.
22. **Botões do cartão:** classe própria com `min-height: 28px`, `padding: 4px 10px`.
23. **Selo SIMULAÇÃO:** 10 px/600.
24. **`.rolagem { scrollbar-gutter: stable }`**, `padding-left: 4px` no título, `gap: 8px` em `.emAndamento`.
25. **Verso visível e desabilitado** durante a passada de verso, para o clique duplo não cair em outra ação (`cartoes.ts:407-416`).
26. **Cartão encerrado:** ação "Dispensar" / "Dismiss". Se todos estiverem encerrados, título "Encerrados" / "Finished".
27. **Texto de perfil em português na interface em inglês** (`en-light-28`) e **motivos da fila sem ponto final** (`textoDoMotivo`): corrija, se for simples. Relate se não for.

**Conflitos previsíveis:** o lote da tela de abertura altera `main.rs` em `ScaleFactorChanged` e `CloseRequested`; o do visualizador altera `DocumentArea.*`. Não mexa nisso. Eu integro.

## Capturas
Refaça as cenas (4 conjuntos) e acrescente:
- fila real em `proximo_lote` sem troca de papel;
- fila real com o gesto da jato de tinta;
- fila real cancelada;
- marcadas mistas (simulada e fila real, simulada e fila inválida);
- resultado com N < M (`tomDoCartao` fica verde em Concluído nesse caso: se enganar, corrija);
- `preparando` em tema normal;
- aguardando em 640×420;
- trabalho vivo em outra aba;
- cores forçadas com selo;
- `consequenciaEnviando` em inglês;
- Reimprimir com confirmação.

Repita o fluxo de 6 páginas nas duas simuladas e o envio à fila de PDF, renderizando com PyMuPDF (`python -I`). Confirme que não sobrou trabalho na fila. Leia as capturas que mudaram.

## Relatório
Cada item com testes e falha prévia, CI (`target/ci.log` terminando em `CI_EXIT 0`, depois da última edição), autoverificação (`sh E:/PlotS/.claude/team/selfcheck.sh c50e6d1 plots-print plots-cli plots-windows plots-core src-tauri ui`), contraste, caminho das capturas, textos novos, limpeza e o que não verificou. Apague `ui/node_modules` ao fim. Termine com `git status` limpo, em um novo commit wip.
