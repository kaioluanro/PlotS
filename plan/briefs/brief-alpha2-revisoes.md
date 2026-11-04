# alpha.2: correções das revisões

Responda em português brasileiro. Worktree existente `E:\PlotS\.claude\worktrees\agent-a690fcada973242e0`, HEAD `d494088`, sem criar outro. Build em `%LOCALAPPDATA%/plots-build/uso-real/target`, com `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, `-j 4`; CDP 9354 e 9355. Rode `npm ci` em `ui/` e `PlotSim/app/ui/`. Scripts anteriores em `%LOCALAPPDATA%/plots-build/uso-real/` (`cap4/`, `saidas/`, `*.mjs`). Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`. Comportamento: `plan/24-impressao-sem-lote.md`, `plan/25-avisos-no-topo.md`. Limite de tempo em todo comando.

**Regras de uso:** confirmação por clique ou tecla real na janela, por CDP, sem comando de terminal para imprimir ou configurar, sem gancho de verificação. Dados isolados: `PLOTS_PERFIS`, `PLOTS_HISTORICO`, `PLOTSIM_CONFIG`. Filas só `PlotSim — Uso *`, pela janela do PlotSim, removidas ao fim. Não toque em `IMP1`, `IMP2` nem nos processos do usuário. Nada a impressoras reais. **Capturas só por CDP: nunca capture a tela do Windows nem use SendKeys ou outra entrada no sistema.** Se uma permissão for negada, relate, sem contornar.

Os textos são definitivos. Testes que fixam texto mudam junto, com a mesma garantia.

## A. Proteção do papel (obrigatório)

1. **Verso em `aguardando` passa pela confirmação com o gesto.** `ui/src/App.tsx` (`aoImprimir`) e `ui/src/rodape-espera.ts`: hoje `enviar_verso` sai direto. Abra o mesmo diálogo de `conteudoDoVerso`, com gesto, páginas e troca de papel, e só comande depois de confirmar. Teste com falha prévia.
2. **Troca de papel em `proximo_lote`:** o envio sai sem confirmar, e, fechado o aviso, a troca some do rodapé. Em `rodape-espera.ts` e `i18n.ts`:
   - Dica que persiste: "Troque o papel para {papel}. Depois, continua o próximo trecho em {lista}." / "Change the paper to {papel}. This then continues the next section on {lista}." Sem troca, use antes o texto de `cartao.proximo.semTroca`. Troque "stretch" por "section" em `cartao.espera.frente`.
   - Rótulo do botão nesse estado: "Continuar frentes em {n}" / "Continue fronts on {n}"; simulada: "Continuar simulação em {n}" / "Continue simulation on {n}".
   - `cartao.proximo.clique`: "Depois, clique em Continuar frentes." / "Then click Continue fronts."; simulada: "Depois, clique em Continuar simulação." / "Then click Continue simulation."
3. **Falso positivo do registro** (`plots-windows/src/registro.rs`): além da fila e do tempo, compare `Param1` com o nome do documento do trabalho, para um 372 antigo da mesma fila não marcar um trabalho que imprimiu. Se a política do Windows ocultar o nome, trate como: sem comparação, segue pela fila e pelo tempo. Reduza `FOLGA_MS` se ela não for necessária. Teste com falha prévia. Troque no teste `<Param2>Kaio</Param2>` por `<Param2>Pessoa</Param2>` e o código por um fictício. Comentários: "o nome do documento não é usado" sai, conforme a mudança; em `vez.rs` e `registro.rs`, em vez de afirmar que o driver falha, escreva "nos testes locais, falhou".
4. **Trabalho em erro substituído:** o antigo não sai da fila do Windows e pode imprimir além do novo. Na confirmação de um novo envio sobre um trabalho em erro, diga: "O trabalho anterior deu erro e pode continuar na fila do Windows. Confira a fila antes de enviar de novo, para não imprimir duas vezes." / "The previous job failed and may still be in the Windows queue. Check the queue before sending again, so it does not print twice."
5. **Par separado em papel misto:** a análise do documento avisa: "As páginas {a} e {b} têm tamanhos diferentes: cada uma sai na sua folha, com o verso em branco." / "Pages {a} and {b} have different sizes: each comes out on its own sheet, with a blank back." Na confirmação, junto dos outros avisos do documento.

## B. Textos da revisão jurídica (obrigatório)

- Evento 372, nos quatro lugares (`plots-windows/src/backend.rs`, perto da linha 195, `ui/src/cartoes.ts`, perto da linha 612, seu teste, e `CHANGELOG.md`): "o registro do Windows indica que a impressão falhou nesta fila. Confira o que saiu antes de reimprimir" / "the Windows log indicates that printing failed on this queue. Check what came out before reprinting". CHANGELOG: "Trabalho que sai da fila com falha registrada pelo Windows para aquela fila (evento 372 do registro de impressão) vira erro com Reimprimir, em qualquer impressora."
- `cartao.cancelarDialogo.jaTerminou`: título "Nada para cancelar" / "Nothing to cancel"; texto "Neste momento, não há trabalho de {nome} para cancelar. Nada mudou." / "Right now, there is no job on {nome} to cancel. Nothing changed."
- Cancelar: "Tirando o trabalho da fila. Isso pode demorar um pouco." / "Removing the job from the queue. This can take a while."
- PlotSim, `centro.porFolha`: "Conferência por folha: pelas marcas, parece um trecho, cópias ou mais de um lote." / "Sheet-by-sheet check: from the marks, this looks like a page range, copies or more than one batch."
- `README.md`, em "Seus dados": "Quando um trabalho sai da fila, o PlotS consulta no registro de impressão do Windows se houve falha naquela fila. O conteúdo do evento é usado só nessa comparação: não é gravado, mostrado nem enviado."
- Os textos de `cartao.frente`, `face.fechar.versos` etc. já revistos não mudam.

## C. Leitura (obrigatório)

6. **A fila nunca some no aviso** (`AvisosDoTopo.tsx` e `.module.css`): dois `span`, documento com `flex: 0 1 auto; min-width: 48px`, fila com `flex: 0 1 auto; max-width: 60%`, ambos com reticências, e `title` completo.
7. **Documento em cada entrada do painel** (`PainelDeAvisos.tsx`, em `.cabeca`, depois da hora): 12 px, `--fg-muted`, com reticências e `title`.
8. **PlotSim:** com `conferenciaPorFolha`, o resumo diz: "{certas} de {total} folhas certas uma a uma; o maço inteiro não foi conferido" / "{certas} of {total} sheets correct one by one; the whole stack was not checked", com singular e variantes com `semMarca` (`tela.ts`, `resumoDaBandeja`, `i18n.ts`).
9. **Leitor de tela** (`AvisosDoTopo.tsx`): `aria-live="polite"` e `aria-relevant="additions"` no `.faixa` permanente; tire o `role` dos itens que não são erro; erro mantém `role="alert"`.

## D. Também, de uma vez

10. Em `avisosNoTopo`, só o aviso mais novo de cada trabalho, com os fixos primeiro.
11. Ícone `LoaderCircle`, estático com movimento reduzido, em "Preparando", "Enviando" e "Cancelando…", em vez de `Check`.
12. O prazo do aviso passageiro pausa em hover e `focus-within`.
13. X do aviso e "Mais opções" com 28 px.
14. "1 não lido" / "1 unread", singular e plural; contador limitado a "99+". Dica do painel: "Para os versos, escolha Verso." / "For the backs, choose Back."
15. **CI:** acrescente `cargo test -p plots-app --no-default-features` ao passo de teste de `scripts/` que exista; se não houver, deixe no log do CI deste lote e diga. Registre o limite conhecido em comentário de `conferir_por_folha_vale`: um documento inteiro sem a folha 1 cai na conferência por folha, sem instrução do PlotS.

## Capturas e uso

Pela interface, prefixo `r5-`, lendo cada uma, sobre o build final, nos dois temas e em inglês onde couber:
- `aguardando` com a confirmação do verso, em documento A4 + A5 com 10 páginas, do começo ao fim, frente e verso, nas filas virtuais;
- `proximo_lote` com o aviso fechado;
- nome longo real com duas impressoras, no topo e no painel;
- 640×420 com impressora marcada e "Mais opções" aberto;
- foco depois de Cancelar > Voltar > Fechar no painel: deve voltar ao sino;
- `r4-sim-misto` com o resumo novo;
- depois de erro, novo envio com o aviso do item 4.

Depois, repita as 10 rodadas de 2 filas e informe perdas e novos eventos 372.

## Final

Commit wip. CI em um único log `saidas/ci.log`, com `cargo fmt --all --check`, `cargo test --workspace --locked -j 4`, `cargo clippy -p plots-app --no-default-features --all-targets -- -D warnings`, `cargo test -p plots-app --no-default-features`, `cargo test -p plotsim-app -- --ignored`, `ui/` e `PlotSim/app/ui/` com `npm ci`, `npm test`, `npm run build`, 0 ganchos no `dist`, `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR="%LOCALAPPDATA%/plots-build/pdfium/chromium-8125/bin"`, terminando com `CI_EXIT <código>`. Rode também `plotsim suite --rapida`, em segundo plano, com prazo de 50 minutos, em build próprio, e informe a última linha. Apague `node_modules` e `dist`. Filas removidas, processos seus fechados. Resposta final: item a item, testes com falha prévia, confirmação pela interface, capturas lidas e não lidas, linha do CI, resultado da suíte e o que não foi verificado.
