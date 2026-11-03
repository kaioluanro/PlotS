# plotsim-suite: gesto como dado, modo de teste e suíte ponta a ponta (fases 4 e 5)

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia `PlotSim/AGENTS.md`, `plan/23-plotsim.md`, `plan/19-modelos-de-impressora.md` e as regras comuns de `plan/briefs/coordenacao-paralela-2.md`. Build e saídas em `%LOCALAPPDATA%/plots-build/plotsim-suite`. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`.

**Outros agentes ao mesmo tempo:** `calibracao-na-janela` (entra antes; mexe em `src-tauri`, `ui/`, perfil e recolocação calibrada) e `plotsim-janela` (só `PlotSim/app/` e dependerá de `plotsim-core`). Para o PlotS, mexa o mínimo e em poucos pontos: `plots-print/src/fluxo.rs` (`Instrucao`), `plots-cli`. Interface fora de escopo. O modelo físico e a conferência (`plotsim-physics/src/`) só mudam se a suíte exigir, e relate.

## Fase 4: contrato

1. **Gesto como dado no PlotS:** `Instrucao` passa a ter os quatro campos da especificação (`entrada`, `face_impressa`, `borda_que_entra`, `inverter_ordem`), e as frases saem desses campos. A borda vem da regra confirmada: sem giro, a aresta que entrou primeiro entra primeiro de novo; com giro, a oposta. **A frase visível não muda agora.**
2. **`plots imprimir --teste`:** sem perguntas, envia à fila do PlotSim, e publica o gesto com o número do lote em `\\.\pipe\plotsim`, antes de pedir o verso. Só funciona com `--teste`; sem o canal, falha claro. Nada disso roda sem a opção.
3. **Canal no `plotsim-core`:** servidor do pipe, com mensagem validada e tamanho limitado, entrada hostil, sem travar sem cliente. O modo manual do pipe fica para a janela.

## Fase 5: suíte

4. **PDFs de teste gerados pela suíte, em Rust:** cada página com número grande, seta para cima e **marca visual lida na imagem**, com o número da página e a orientação. O PlotS envia imagem, sem texto: a marca precisa sobreviver à rasterização, a qualquer DPI do driver e à rotação. Um leitor próprio em `plotsim-core`, sobre o bitmap do PDFium, sem dependência nova. Ele devolve número e rotação, ou erro claro; página sem marca é falha do cenário, nunca palpite. Teste com página girada, escala e PDF em branco.
5. **`plotsim suite`**, binário em `PlotSim/plotsim-suite/`: cria as filas, roda a matriz da especificação com o `plots-cli` em `--teste`, recebe cada PDF, lê as marcas, entrega as folhas ao `plotsim-physics`, executa o gesto recebido e confere. Remove as filas ao fim, mesmo com falha, e deixa só o que já existia. Prazo por trabalho; trabalho preso é falha do cenário.
6. **Matriz:** dois modelos (jato, laser) e as demais combinações do perfil; páginas 1, 2, 3, 9, 10, 11, 24, 101; lote 1, 5, 10, inteiro; A4, A5, Carta, Ofício e misto; 1, 2 em espelho, 2 em divisão; erros de propósito (gesto errado, folha a menos, página girada no PDF), que o PlotSim tem de acusar. A matriz inteira pode ser longa: `--rapida`, com um recorte representativo, para o uso comum, e a completa sob pedido. Modo automático: a fila do PlotSim não tem duas faces de verdade: relate o que dá para cobrir.
7. **Relatório HTML** em pasta dada: uma linha por cenário (passou ou falhou, e o motivo), com as folhas finais desenhadas. Sem marca de terceiros. Fica fora do repositório.

## Limites

Só filas `PlotSim — `, como já é regra de `plotsim-core`; nenhuma fila real. Sem elevação: se o Windows negar, pare e relate. Se a suíte reprovar o PlotS, **não corrija o PlotS**: relate entrada, saída e quem parece errado. Sem dependência nova, salvo se relatada antes. Limite de tempo em todo comando.

## Final

CI completo em um único log, incluindo `ui/` se algo do PlotS mudar, `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR` (o gerador exclui `plotsim-*`, então o novo crate tem de cair nessa regra). Rode `plotsim suite --rapida` de verdade e guarde o relatório. Autoverificação, `git status` limpo com wip. Relatório em `%LOCALAPPDATA%/plots-build/plotsim-suite/relatorio.md`, com a mesma mensagem na resposta final: item a item, resultado da suíte, cada divergência, tempo total, testes com grupo de falha prévia, o que restou no Windows e o que não foi verificado.
