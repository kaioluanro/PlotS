# uso-real: correções da rodada integrada

Responda em português brasileiro. Worktree existente `E:\PlotS\.claude\worktrees\agent-a690fcada973242e0`, HEAD `6131b34`, sem criar outro. Build em `%LOCALAPPDATA%/plots-build/uso-real/target`, com `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, `-j 4`; CDP 9354 e 9355. Rode `npm ci` em `ui/` e `PlotSim/app/ui/`. Scripts e PDFs anteriores em `%LOCALAPPDATA%/plots-build/uso-real/` e `.../teste-simulacao/documentos/`. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`. Comportamento: `plan/24-impressao-sem-lote.md`, `plan/25-avisos-no-topo.md`. Limite de tempo em todo comando.

**Regras de uso:** toda confirmação é por clique ou tecla real na janela, por CDP, sem comando de terminal para imprimir ou configurar, sem gancho de verificação. Dados isolados: `PLOTS_PERFIS`, `PLOTS_HISTORICO`, `PLOTSIM_CONFIG`. Filas só `PlotSim — Uso *`, pela janela do PlotSim, removidas ao fim. Não toque em `IMP1`, `IMP2` nem nos processos do usuário. Nada a impressoras reais. **Capturas só por CDP: nunca capture a tela do Windows nem use SendKeys ou outra entrada no sistema**, porque isso alcança a área de trabalho do usuário. Se uma permissão for negada, relate, sem contornar.

Para cada defeito: teste que falha, causa, correção mínima, confirmação pela interface.

## 1. Trabalho perdido no envio simultâneo: causa provada

O registro `Microsoft-Windows-PrintService/Admin` mostra o evento **372** nos horários das perdas, inclusive na fila do usuário às 06:25:15: "não pôde ser impresso... Número de páginas impressas: 0... Código de erro do Win32 retornado pelo processador de impressão: 2291683334" (`0x88985006`, "a interface fornecida já está registrada", um erro do DirectWrite no processador de impressão). O driver "Microsoft Print To PDF" falha com dois trabalhos processados juntos. O PlotS só vê o trabalho sumir da fila e diz "Frentes concluídas".

- **Detectar, para qualquer impressora:** ao ver o trabalho sumir, em `plots-windows`, consulte o registro `Microsoft-Windows-PrintService/Admin` pelo evento 372 dessa fila, a partir do início do envio, com `EvtQuery` da API do Windows, usando o crate `windows` já no projeto, sem dependência nova. Encontrando, o trabalho vira erro: "O Windows não conseguiu imprimir este trabalho. Nenhuma página saiu desta fila." / "Windows could not print this job. No page came out of this queue.", com Reimprimir. Registro ilegível ou desligado: segue como hoje, sem travar nem atrasar mais que 2 s. Compare o nome da fila exato, e o tempo com folga pequena; o nome do documento é sempre igual.
- **Evitar, só nesse driver:** com o driver "Microsoft Print To PDF", envie uma fila por vez, esperando o trabalho anterior sair da fila antes de começar o próximo, com prazo. Filas de outros drivers continuam em paralelo: a impressão simultânea é o motivo de marcar várias impressoras.
- **Medida:** 10 rodadas com 2 filas e 5 páginas, pela interface: 0 perdas e 0 eventos novos 372. Force a detecção com teste de unidade que simula o evento, pois a perda deixa de ocorrer.

## 2. Papel misto mantém o lado (decisão do usuário)

Documento de 10 páginas, só a 5 de outro tamanho: **1/2, 3/4, 5/branco, 6/branco, 7/8, 9/10**. As páginas continuam no lado em que o documento foi diagramado: só o par com tamanhos diferentes se separa, e cada uma das duas ganha verso em branco. Corrija `emparelhar()` em `plots-core/src/lib.rs`. Testes com A4+Carta, A4+A5, A4+Ofício, página de outro tamanho no início, no meio, no fim, com total par e ímpar, e com intervalo. Retrato+paisagem do mesmo tamanho segue na mesma folha, com o aviso.

## 3. Rodapé travado depois de erro (bloqueia)

Fila removida durante o envio: o aviso é fechado, e "Imprimir frentes" segue desativado com "Em andamento nos cartões acima." sem saída, salvo cancelar no painel. Trabalho em erro, cancelado ou concluído não pode travar o rodapé.

## 4. Painel de avisos

- Cancelar e Reimprimir só na entrada mais recente do trabalho, conforme o estado dela. Cancelar só com trabalho em andamento, e desabilitado em "Cancelando…". Reimprimir só em erro. Hoje aparecem em até 16 entradas.
- A dica "Para os versos, escolha Verso" só nas entradas de frentes prontas. Ícone de cada entrada conforme o estado dela.
- Avisos do topo com o nome do documento: "{documento} · {fila}". Com duas abas, hoje não dá para saber de qual é cada um.
- Aviso fixo que perdeu a validade sai do topo, e fica no painel. Exemplo: "Troque o papel para A5" depois de o trecho avançar.

## 5. Textos que citam cartões

Sem a palavra "cartão" em PlotS e PlotSim, pt-BR e en. Definitivos:
- Rodapé em envio: "Em andamento. Veja os avisos." / "In progress. See the notices."
- Confirmação do verso: "As frentes já saíram? Coloque o maço na entrada, como mostrado abaixo." / "Are the fronts out? Place the stack in the feed, as shown below."
- Vários papéis: "O documento usa mais de um papel: um aviso diz quando trocar." / "The document uses more than one paper: a notice says when to change."
- PlotSim, recolocar: "Siga o que o PlotS pede na confirmação do verso." / "Follow what PlotS asks in the back confirmation."
- Cancelar: "Tirando o trabalho da fila. Isso pode levar até meio minuto." / "Removing the job from the queue. This can take up to half a minute."

Procure no `i18n.ts` qualquer outra ocorrência.

## 6. Contagem e janela pequena

- O resumo do rodapé bate com a confirmação em papel misto: hoje "10 pág · 5 folhas" no rodapé e "6 folhas" na confirmação. A contagem vem do plano, em um lugar só.
- Em 700×500, "Ajustar ao papel" e "Folha de controle" têm altura 0, sem rolagem: ou rolam com sinal claro, ou o bloco abre sob um botão "Mais opções" / "More options". Nada inalcançável.

## 7. Foco e campo

Depois de confirmar um envio por teclado, o foco volta ao botão de envio, e não ao BODY. O campo Quantidade tem rótulo acessível. O limite de caracteres do campo não pode transformar "12abc" em "12ab": deixe o texto como digitado, inválido. A aba do documento tem anel de foco.

## 8. PlotSim: veredito honesto em documento que não começa da 1

No uso manual, ele marca "Errada" em intervalo de páginas, cópias e papel misto, e trata o trecho seguinte como "verso recebido". Isso faz o usuário ler errado.
- Use a face do nome do trabalho (`PlotS - lote N - frente`/`verso`): frente que chega com a bandeja aguardando verso abre lote novo, em vez de virar verso.
- Em cada folha, confira o par e a orientação entre si. Quando as marcas não começarem na 1, ou se repetirem por cópias, ou houver mais de um lote, a conferência do maço inteiro não se aplica: a tela diz "Conferência por folha: este trabalho não começa da página 1 nem é um documento inteiro." / "Sheet-by-sheet check: this job is not a whole document starting at page 1.", sem acusar ordem. Um erro real numa folha continua "Errada". Use o resultado de `plotsim_physics` sem mudar a regra física.
- Depois de um `Fim` com instrução, tudo segue como hoje; a suíte tem de continuar passando.

## Final

Commit wip. CI em um único log `saidas/ci.log`: `cargo fmt --all --check`, `cargo test --workspace --locked -j 4`, `cargo test -p plots-app --no-default-features`, `cargo test -p plotsim-app -- --ignored`, `ui/` e `PlotSim/app/ui/` com `npm ci`, `npm test`, `npm run build`, 0 ganchos no `dist`, `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR="%LOCALAPPDATA%/plots-build/pdfium/chromium-8125/bin"`, terminando com `CI_EXIT <código>`. Rode também `plotsim suite --rapida`, em segundo plano, com prazo de 50 minutos, em build próprio, e informe a última linha. Apague `node_modules` e `dist`. Filas removidas, processos seus fechados. Resposta final: item a item, causa, testes com falha prévia, confirmação pela interface, capturas (`r4-`), linha do CI, resultado da suíte, e o que não foi verificado.
