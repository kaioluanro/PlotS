# plotsim-fisica: modelo físico, conferência e suíte rápida (fase 3)

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia `PlotSim/AGENTS.md` (especificação do usuário), `plan/23-plotsim.md` (o que não pode ser reaproveitado), `plan/19-modelos-de-impressora.md` e as regras comuns de `plan/briefs/coordenacao-paralela-2.md`. Build em `%LOCALAPPDATA%/plots-build/plotsim-fisica`. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.

**Arquivos:** só `PlotSim/plotsim-physics/` (novo), e no `Cargo.toml` da raiz só a linha do membro novo. Outro agente adiciona a sua própria linha lá e cria `PlotSim/plotsim-core/`; o coordenador resolve isso. Não mexa em `plots-*`, `src-tauri`, `ui`, `plan/`.

## Escopo

1. **Crate `plotsim-physics`**, sem depender de nenhum `plots-*` no código de produção. Escreva o modelo **a partir da especificação e do plano 19, sem ler nem copiar `plots-print/src/lib.rs` (`Simulador`, `conferir_folhas`), `fluxo.rs` nem as funções `plano_*`**, para os dois programas poderem discordar.
   - Folha com duas faces, cada uma com página (número ou em branco) e rotação. Bandeja de saída e entrada, pilha ordenada.
   - **Modo da impressora:** de onde puxa, em que face imprime, com que face sai, para onde a folha cai.
   - **Gesto como dado**, com os quatro campos da especificação: `entrada`, `face_impressa`, `borda_que_entra`, `inverter_ordem`. Operações físicas compostas: pegar a pilha, virar por uma borda, girar, colocar na entrada. A rotação de cada face tem de sair da geometria, nunca de tabela.
   - Duas faces automáticas como modo próprio: frente, volta, verso.
2. **Conferência** com as três condições da especificação: ordem (cada folha com *n* ímpar e *n*+1, ou em branco no total ímpar; maço em 1, 2, 3…), orientação (topo na mesma borda) e instrução executada ao pé da letra. O erro diz o motivo, em dado e em texto: "verso de cabeça para baixo", "verso trocado com a folha 3", "folha a menos".
3. **Suíte rápida**, em `PlotSim/plotsim-physics/tests/`, com o PlotS como **entrada** (`plots-core` e `plots-print` só em `[dev-dependencies]`):
   - matriz da especificação: todas as combinações de modo, páginas (1, 2, 3, 9, 10, 11, 24, 101), lote (1, 5, 10, inteiro), espelho e divisão, e o modelo automático;
   - mesmo documento, mesmo maço final em todas as impressoras;
   - erros de propósito: gesto errado, folha a menos, página girada: o modelo **tem** de acusar;
   - propriedades com `proptest`, que já é do projeto.

## O que relatar sem corrigir

**Se a suíte reprovar o PlotS, não corrija o PlotS nem ajuste o modelo para passar.** Registre cada divergência: entrada, o que o PlotS mandou, o que o modelo concluiu, e qual dos dois parece errado e por quê. É para isso que este programa existe. Teste que documenta divergência fica `#[ignore]`, com o motivo.

Atenção especial a `borda_que_entra`: o PlotS hoje só distingue borda curta de longa, sem cabeça nem pé, e o plano 19 tem isso em aberto. Mostre em que condições cada borda dá versos de cabeça para baixo, porque a resposta decide o que a frase e a figura do PlotS têm de dizer. Faça a tradução de `Instrucao` para o gesto em um adaptador dentro dos testes, sem mudar o PlotS, e diga o que faltou.

Sem interface. Nada de enviar a filas. Sem identificadores do plano no código.

## Final

CI do workspace em um único log (`cargo fmt --all --check`, `cargo test --workspace --locked -j 4`; não precisa de `ui/`), autoverificação, `git status` limpo com commits wip. Sem dependência nova além das já do projeto, salvo em `[dev-dependencies]` as dos próprios crates. Relatório em `%LOCALAPPDATA%/plots-build/plotsim-fisica/relatorio.md`, com a mesma mensagem na resposta final: o modelo em poucas linhas, tabela de resultados da matriz, cada divergência com o PlotS, resposta sobre `borda_que_entra`, linha do CI, e o que é hipótese. Limite de tempo em todo comando.
