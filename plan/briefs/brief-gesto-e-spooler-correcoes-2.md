# gesto-e-spooler: segunda rodada de correções

Responda em português brasileiro. Trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-a0909ac7237a0b0a7`, branch `worktree-agent-a0909ac7237a0b0a7`, sobre o wip `63c91f2`. Não crie outro worktree. Leia `E:\PlotS\plan\briefs\coordenacao-paralela-2.md` (regras comuns) e `brief-gesto-e-spooler-correcoes.md` (exceções ao mapa de arquivos). Relatório da rodada anterior: `%LOCALAPPDATA%/plots-build/gesto-e-spooler/relatorio-correcoes.md`. Build e saídas: `%LOCALAPPDATA%/plots-build/gesto-e-spooler`; cenas em `saidas/cena/`. `ui/node_modules` foi apagado: rode `npm ci`. Antes do primeiro grep ou leitura de código, rode `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.

As revisões de código e jurídica aprovaram. A de interface pediu estes itens, nesta ordem, todos obrigatórios:

1. **Clique na linha "Um cartão pede ação":** a linha some sob o ponteiro, a lista rola, e o segundo clique de um clique duplo cai em um botão do cartão, que pode ser "Imprimir verso". No clique com ponteiro, mantenha o espaço de 28 px, inerte e sem texto, até `pointerleave`. Pelo teclado, some na hora. Teste de clique duplo: o segundo clique não aciona nada.
2. **`irParaAPrimeira`** (`ui/src/cartoes-gesto.ts`) tira da linha só o cartão de destino. Com outro pendente, a linha continua ativa e passa a "Um cartão pede ação" / "A card needs you". Depois do clique, sair da linha para fora da lista não pode tirar o primeiro cartão de vista descendo a outro.
3. **Texto do driver:** volta como última linha do aviso, em campo próprio do backend, nunca extraído de parênteses: "Mensagem da impressora: {texto}" / "Printer message: {texto}", papel `ressalva`, 12/16, no máximo 2 linhas, com `title` do texto inteiro, só quando existir, também no CLI. É entrada hostil: tamanho limitado e sem marcação. A frase principal continua só com a condição mais urgente.
4. **`cartao.reimprimirDialogo.texto`:**
   - pt-BR: "O que restar deste lote sai da fila e a frente é enviada de novo. Deixe na entrada só folhas em branco: tire qualquer folha já impressa deste lote."
   - en: "What is left of the batch leaves the queue, and the front is sent again. Leave only blank sheets in the feed: take out any sheet already printed for this batch."
5. **Ordem dos botões no aviso:** verso desabilitado, Continuar esperando, Reimprimir lote, Cancelar trabalho.
6. **Foco:** se a linha sumir com foco, o foco vai ao cartão correspondente, sem rolar além do necessário, ou à lista, se o cartão não existir mais.
7. **Hover da linha:** `background: var(--accent-bg-soft)` e texto sublinhado, nos dois temas. Meça de novo `--fg` sobre o fundo resultante.
8. **Recomendação jurídica, adotada:** "Retire a folha presa como indica o manual da impressora." / "Remove the stuck sheet as the printer manual describes.", em cartão, CLI e testes, inclusive a junção com "O lote pode ter desalinhado…" no verso.
9. **Capturas** nos dois temas, em cores forçadas e em inglês: depois do clique duplo; sequência com dois cartões; foco no cartão depois do teclado; aviso com mensagem longa do driver; confirmação de reimprimir; botões na nova ordem; hover no tema claro. Leia cada uma no tamanho real.

Não mexa em `plan/`: o coordenador registra a pendência de cabeça ou pé no plano 19.

Depois: CI completo em um único log, autoverificação (`selfcheck.sh f5d7707 plots-windows plots-print plots-cli src-tauri ui/src`), `git status` limpo com novo wip, apague `ui/node_modules` e `ui/dist`. Relatório em `%LOCALAPPDATA%/plots-build/gesto-e-spooler/relatorio-correcoes-2.md`, com a mesma mensagem na resposta final: item a item, testes com grupo de falha prévia, linha do CI, capturas com caminho, o que não foi verificado. Não espere processo em segundo plano sem limite de tempo.
