# windows-envio, correção 1

Responda em português brasileiro. Você é um agente novo: trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-a5e60cb62e875b9df` (branch `worktree-agent-a5e60cb62e875b9df`, commit `dd91a9c` sobre `47a6969`); não crie outro nem mexa em E:\PlotS fora dele. Faça um commit wip ao terminar; nunca dê push nem squash.

As regras de `E:\PlotS\plan\briefs\brief-windows-envio.md` continuam valendo, em especial a **regra de segurança: só a fila "Microsoft Print to PDF", com arquivo de saída**. Leia esse brief primeiro. Use `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/windows-envio"`, `CARGO_INCREMENTAL=0` e `PLOTS_PDFIUM_DIR` apontando para `C:/Users/Kaio Luan/AppData/Local/plots-build/pdfium/chromium-8125`. Não baixe nada de novo. Não mexa em `ui/` nem em `src-tauri/`. Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh 47a6969 plots-windows plots-cli plots-print plots-pdf scripts`.

## Decisão do coordenador sobre o `DEVMODE`
O programa força os valores certos, em vez de pedir ao usuário que mude as preferências. O brief original limitava os campos, e esta decisão o substitui.

## Escopo
1. **`DEVMODE`.** Force `dmCopies = 1`, `dmScale = 100` e `dmOrientation` retrato, marcando `dmFields`, e também `dmNup` de uma por folha, se o campo existir. Depois do merge com `DocumentPropertiesW`, confira em função pura e testada: se o driver não aplicou cópias, escala, N-up, orientação ou simplex, recuse com erro claro dizendo qual valor não foi aceito e "nada foi enviado". Teste com buffer sintético, incluindo `aplicar` e `conferir_simplex`, que hoje não têm teste.
2. **Papel.** No ramo `Formato`, limpe `DM_PAPERLENGTH`, `DM_PAPERWIDTH` e `DM_FORMNAME` e zere largura e comprimento. No ramo `Medidas`, limpe `DM_FORMNAME`. Compare `conferir_papel` com a medida nominal do formato, e não com a medida da página.
3. **PDF protegido.** Em `plots-pdf/src/lib.rs`, a condição de `carregar` passa a ser `documento.is_encrypted() || documento.was_encrypted() || documento.trailer.has(b"Encrypt")`. O lopdf decifra sozinho quando a senha de usuário é vazia e apaga `/Encrypt`, então PDFs com senha de dono passavam. Corrija o comentário de `plots-windows/src/passada.rs`. Testes com `user_password: ""` e `Permissions::empty()`, em `plots-pdf` e no `plots imprimir` real, com falha registrada antes.
4. **Não apagar trabalho sem perguntar.**
   - Repita `GetJobW` algumas vezes em `ERROR_INSUFFICIENT_BUFFER`.
   - Tolere erros de consulta passageiros, até N seguidos, e nunca apague um trabalho por erro de consulta.
   - `OFFLINE`, `PAPEROUT` e `USER_INTERVENTION` viram aviso, e a espera continua até o usuário decidir: mostre o motivo e pergunte `Cancelar o trabalho na fila? (s/n)`. Com `n`, continue esperando. Com fim da entrada, deixe o trabalho na fila e diga isso.
   - Em `Falhou`, faça a mesma pergunta. Depois de cancelar, use: `O trabalho foi cancelado ou já tinha saído da fila; folhas já entregues ao dispositivo ainda podem sair. Nada mais será enviado.`
   - Ampliar o backend falso com erro de consulta, resposta `n` e fim da entrada, e ajuste `falha_na_fila_mostra_o_motivo...`.
5. **Textos que afirmam demais.**
   - `O trabalho da frente do lote {numero} terminou ou saiu da fila do spooler. Isso não garante que o papel já saiu.`
   - `O trabalho do verso do lote {numero} terminou ou saiu da fila do spooler.`
   - `Os trabalhos dos {total} lote(s) terminaram ou saíram da fila do spooler. Confira as folhas depois que a última sair.`
   - `USO_IMPRIMIR`: `O verso só é oferecido depois de o trabalho da frente terminar ou sair da fila do spooler. Isso não garante que o papel já saiu: espere a última folha antes de recolocar o maço.`
   - `--arquivo-saida`: `grava cada passada em um arquivo novo dentro de PASTA, pela própria fila; foi conferido só na fila de PDF do Windows.`
   - Ajuste os testes que fixam esses textos.
6. **Página em branco.** Na rasterização de `numero: None`, marque por função pura um pixel discreto, mas que sobreviva ao meio-tom, dentro do retângulo imprimível calculado de `Dispositivo` e `PlanoPagina`. Teste sem a DLL, com margem maior que 0, que o pixel está dentro da área e não é branco. Corrija `pagina_em_branco_sai_quase_toda_branca_com_o_ponto_discreto` para exigir ao menos um pixel não branco.
7. **Papel de cada lote.** `lotes_do_plano` passa a manter `LotePapel::papel`. O cabeçalho de cada lote diz o papel antes da primeira pergunta. Ao mudar de formato, escreva antes da pergunta: `Troque o papel para {papel} e retire as folhas do lote anterior.` Teste com o backend falso e dois formatos.
8. **Decisão de fila testável.** Separe de `conferir_fila` uma função pura que recebe a lista de filas. Teste: fila inexistente, nome com outra caixa ou espaço (recusa exata), porta que pede arquivo com e sem pasta, e fila normal. Recuse também `SHRFAX:`.
9. **Extensão do arquivo de saída.** O nome `.pdf` só vale para a fila de PDF: use `.saida` quando a fila não for da porta que pede arquivo, ou documente, o que for menor.

## Testes e teste de uso
Registre a falha prévia em `target/fail-before-fix.log`. Rode os testes `#[ignore]` à mão no build final. Repita o teste de uso do brief com `--arquivo-saida`, na fila de PDF, e confirme que não sobrou trabalho na fila. Não há impressora real.

## Relatório
Cada item com o que mudou e seu teste, linha do CI (`cargo fmt --all --check`, `cargo test --workspace --locked`), testes `#[ignore]`, autoverificação vazia, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo.
