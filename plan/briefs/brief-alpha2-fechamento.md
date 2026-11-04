# alpha.2: fechamento das revisões

Responda em português brasileiro. Leia `E:\PlotS\AGENTS.md`, `.claude/team.json` e siga
`.claude/agents/plots-debugger.md`. Worktree existente
`E:\PlotS\.claude\worktrees\agent-a690fcada973242e0`, HEAD `d85d143`, base do lote `861ab57`.
Não crie outro worktree. Build em `%LOCALAPPDATA%\plots-build\uso-real\target`, com
`CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, `-j 4`; CDP 9354 (PlotS) e 9355 (PlotSim). Rode
`npm ci` nas duas UIs. Scripts anteriores em `%LOCALAPPDATA%\plots-build\uso-real\` (`cap5/`,
`saidas/`, `*.mjs`). Não mexa em `plan/`.

**Graphify é obrigatório:** antes do primeiro grep ou leitura de código, `graphify query
"<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; use `graphify path`/`explain` para
relações. Leia arquivo cru só nas linhas que o grafo localizou. Ao terminar, `graphify update .`.

**Regras de uso:** confirmação por clique ou tecla real na janela, por CDP, sem comando de
terminal para imprimir ou configurar, sem gancho de verificação. Dados isolados:
`PLOTS_PERFIS`, `PLOTS_HISTORICO`, `PLOTSIM_CONFIG`. Filas só `PlotSim — Uso *`, criadas e
removidas pela janela do PlotSim. Não toque em `IMP1`, `IMP2` nem nos processos do usuário.
Nada a impressoras reais. **Capturas só por CDP: nunca capture a tela do Windows nem use
SendKeys.** Permissão negada: relate, sem contornar. Encerre só os PIDs que você abriu.

## A. Proteção do papel (obrigatório)

1. **A confirmação do verso em espera mostra o trabalho que vai sair, não o que está na aba.**
   Hoje `ui/src/App.tsx` (`prepararEnvio`, por volta de 510-534, e o ramo `enviar_verso` por
   volta de 590-597) monta o resumo com `lerIntervaloDaAba`, `aba.papelDeAjuste` e
   `copiasDaFace(aba, "verso")`. O comando `enviar_verso` só manda a passada 2 do lote já
   planejado. Cenário que gasta papel: frentes com 1 cópia, verso aguardando, a pessoa muda
   Quantidade para 2 e confirma; o diálogo manda montar maço de 2 e o trabalho imprime 1.
   - Use o que foi enviado (`aba.frenteEnviada`, e o plano/cópias do cartão que espera) para
     páginas, papel e cópias. Se o que está na aba diverge do enviado, mantenha o aviso de
     documento mudou que já existe; nunca troque os números do trabalho pelos da aba.
   - Teste: o verso em espera com a aba alterada depois da frente mostra os números do
     trabalho enviado. Falha prévia registrada.
2. **Aviso de trabalho anterior em erro também na confirmação do verso.**
   `face.envio.erroAnterior` só entra em `notasDaFrente`. Inclua a mesma nota em
   `conteudoDoVerso` (e na face automática, se ela substitui o cartão) quando
   `quemDeuErro(aba, destinos)` não for vazio. Teste que falhe sem a nota.
3. **Texto do cancelamento.** Em `ui/src/i18n.ts` (pt e en), troque "Nada mudou." /
   "Nothing changed." por "Nada foi cancelado." / "Nothing was cancelled." Mantenha o resto do
   texto do brief. Atualize o teste que fixa o texto.
4. **Faixa do topo: a fila não pode virar `PlotSim — Us…`.** Em
   `ui/src/components/AvisosDoTopo.module.css`, reserve a largura da fila (por exemplo
   `flex: 0 0 auto; min-width: max-content` na fila, com `max-width: 60%`) e deixe o documento
   ceder até 48 px, com reticências e `title`. Alinhe com o brief (documento `0 1 auto`).
   Teste de largura: para "Uso L" e "Uso J", `scrollWidth <= clientWidth` no elemento da fila.
5. **A 640×420 a faixa não pode cobrir a lateral.** A faixa ocupa a largura da janela e cobre a
   lista; limite-a à área do documento (ou ao espaço disponível) e prove por captura que a
   lista e o "Mais opções" continuam clicáveis com aviso no topo.
6. **`FOLGA_MS`**: volte para 1 s, salvo se você tiver medição que mostre 250 ms seguro. A
   janela de 1 s já era o valor revisado; reduzir sem dado pode perder um 372 real.
7. **Comentário** em `plots-windows/src/backend.rs:86`: "O driver desta fila falha com dois
   trabalhos juntos" → "Nos testes locais, este driver falhou com dois trabalhos juntos".

## B. Textos (obrigatório, do jurídico)

- `README.md`, em "Seus dados": cite também a face (frentes ou versos) no histórico; e no
  parágrafo do registro de impressão diga "quando consegue ler o registro" e que, se não
  consegue, o trabalho segue como concluído.
- Nada mais muda nos textos aprovados.

## C. Testes e falha prévia (obrigatório)

- O portão do rodapé não tem teste que falhe se o `if` de `App.tsx` for removido. Extraia a
  decisão para função pura (por exemplo `acaoDoEnvio(esperas)` devolvendo `confirmar` com os
  itens, ou `comandar` direto) usada pelo `App`, e teste: verso em espera → confirmar; frente
  em espera → comandar direto.
- Registre em `%LOCALAPPDATA%\plots-build\uso-real\saidas\` um log de falha prévia real para
  os itens A1, A2 e C (teste falhando no código anterior a esta rodada e passando depois).
  Sem log, o item não conta.

## D. Capturas finais (obrigatório)

Rebuild a partir do commit final. Prefixo `r6-`, em `cap6/`, lendo cada uma:
- nome longo real com duas impressoras: topo e painel, em 1280 e em 640, mostrando "Uso L" e
  "Uso J" legíveis;
- 640×420 com aviso no topo, provando a lista e o "Mais opções" clicáveis;
- 700×500 e 640×420 com "Mais opções" aberto e rolado até "Salvar seleção como grupo",
  "Ajustar ao papel" e "Folha de controle", com o botão de envio visível;
- prova de foco depois de Cancelar > Voltar > Fechar no painel: salve
  `document.activeElement` em arquivo e capture por teclado, com anel visível no sino;
- "1 não lido" e "99+";
- aviso sumindo aos 6 s, máximo de 3 no topo e pausa por `focus-within`;
- PlotSim: singular ("1 de 1 folha certa") e variantes com `semMarca`;
- `aguardando` com confirmação do verso (documento A4 + A5, 10 páginas), frente e verso até o
  fim, nas filas virtuais;
- `proximo_lote` com o aviso fechado e a dica de troca de papel.

Depois, repita as 10 rodadas de 2 filas e informe perdas e novos eventos 372.

## E. Fechamento

- Remova as filas `PlotSim — Uso J` e `PlotSim — Uso L` pela janela e confirme que só restam as
  do usuário.
- Apague `ui/node_modules`, `ui/dist`, `PlotSim/app/ui/node_modules` e `PlotSim/app/ui/dist`.
- `cargo fmt`, wip commit com árvore limpa.
- CI em um único log `saidas/ci.log`, com `cargo fmt --all --check`, `cargo test --workspace
  --locked -j 4`, `cargo clippy -p plots-app --no-default-features --all-targets -- -D
  warnings`, `cargo test -p plots-app --no-default-features`, `cargo test -p plotsim-app --
  --ignored`, as duas UIs com `npm ci`, `npm test`, `npm run build`, 0 ganchos no `dist`,
  `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR`, terminando com
  `CI_EXIT <código>`.
- Rode `plotsim suite --rapida` em segundo plano, prazo de 50 min, build próprio, e informe a
  última linha.
- Resposta final: item a item, causas, testes com falha prévia (caminho do log), capturas
  lidas e não lidas, linha do CI, resultado da suíte, filas/processos removidos e o que não foi
  verificado.
