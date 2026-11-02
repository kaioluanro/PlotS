# modelos-virtuais: as impressoras simuladas passam a imitar os dois modelos do usuário

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master`. Faça commits wip; nunca dê push nem squash. Não altere nada em E:\PlotS fora do seu worktree. Outro agente trabalha em paralelo em `ui/` e `src-tauri/`: não mexa nessas pastas.

## Ambiente
- Use em todo comando cargo `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/modelos"` e `CARGO_INCREMENTAL=0`: o disco E: está quase cheio. Logs ficam em `target/` do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- Se o sistema matar o build por falta de memória, repita uma vez; se morrer de novo, apague a pasta `debug/incremental` do target e tente mais uma vez. Só então relate.
- Depois da última edição, rode uma vez `cargo fmt --all --check` e `cargo test --workspace --locked`, gravando em `target/ci.log`, seguido de `CI_EXIT <código>`. Nunca faça wip depois de um CI vermelho.
- Log de falha prévia: `target/fail-before.log`, com execuções reais.
- Sem dependências novas. **Não envie nada a nenhuma fila.** Nunca toque o `%APPDATA%` real: use `--perfis` temporário.
- Limpeza conforme "Limpeza constante do disco" em AGENTS.md.

## Navegação
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; depois leia só as linhas localizadas e confirme no worktree. Não rode `graphify update`.
- `plots-print/src/lib.rs`: `Mecanica::traseira`, `Mecanica::gaveta`, `Recolocacao`, `Simulador`, `folhas_para_leitura`, tabela de mecânicas
- `plots-cli/src/lib.rs`: `ImpressoraVirtual`, `GESTO_CORRETO`
- `plots-cli/src/imprimir.rs`: `gesto_do_perfil`, `gesto_de_recolocacao`

## Fontes
`plan/19-modelos-de-impressora.md`, no worktree: é a especificação deste lote. Não abra `E:\PlotS\plan\estilo-ui-ux.md` nem `E:\PlotS\codex-session-*.md`.

## Regras sobre identificadores do plano
Código, comentários e testes descrevem comportamento com palavras próprias: sem IDs de requisito, nome de lote, "auditoria" ou "brief". **Nada de marca ou modelo de impressora no código de produção nem nos testes**: use "jato de tinta" e "laser".
Autoverificação, não pode imprimir nada: `sh E:/PlotS/.claude/team/selfcheck.sh <base> plots-print plots-cli plots-core`, com `<base>` igual ao commit de `master` em que você começou.

## Escopo
Os nomes `Simulada — Traseira` e `Simulada — Gaveta` e os atalhos `traseira` e `gaveta` não mudam. Só o comportamento muda, conforme a tabela do plano.

1. **Virada pela borda de cima.** O gesto da jato de tinta vira o maço como bloco de notas. Represente isso no modelo físico, em `Recolocacao`, com um campo próprio ou com a combinação de virar e girar, o que for mais claro. Comentário curto explica por que a virada pela borda de cima deixa a folha de cabeça para baixo em relação à frente.
2. **Jato de tinta (`Simulada — Traseira`).** Mecânica inalterada: sai para cima, cai em cima, puxa a de cima, imprime na face de cima. Gesto: virar pela borda de cima. Perfil: frente inversa, verso inverso e **`verso_girar_180` verdadeiro**. Resultado para 6 páginas: frente 6, 4, 2; verso 5, 3, 1, girado.
3. **Laser (`Simulada — Gaveta`).** Mecânica: sai para baixo, cai em cima, puxa a de cima, **imprime na face de cima**. Gesto: **não vira**. Perfil: **frente inversa**, verso direto, sem giro. Resultado para 6 páginas: frente 6, 4, 2; verso 1, 3, 5. O maço final é lido virando-o.
4. **Frases.** A frase fixa continua: `Pegue o maço como saiu, não embaralhe, e coloque na entrada com a face impressa para ...`; ambas as simuladas dizem "para baixo". Acrescente uma segunda linha com o gesto, gerada do perfil: `Vire o maço pela borda de cima, como um bloco de notas.`, `Vire o maço pela lateral, como a página de um livro.` ou `Não vire o maço.`. Fila com perfil salvo continua sem afirmar lado. A linha `Na gaveta, vire o maço antes de ler as folhas.` continua na gaveta.
5. **Ordem ímpares.** `--ordem impares` continua produzindo folhas certas nas duas simuladas. Se a escolha de perfil precisar mudar para essa ordem, calcule isso, sem esconder o erro.
6. **Erro simulado.** `--simular-erro sem-virar` deve fazer sentido nas duas: na laser, que não vira, ele vira um gesto errado de virar, ou é recusado com mensagem clara. Decida o que for mais fiel e relate. Mantenha `girado` e `folha-a-menos`.
7. **Tabela e testes.** Atualize a tabela fixa de mecânicas, a propriedade com proptest e o laço exaustivo. Inclua os dois modelos com seus perfis. O teste da frase continua comparando com o estado real do simulador depois de recolocar. Acrescente testes de ponta a ponta para 6 páginas e para total ímpar nas duas simuladas, com a sequência exata do plano.

## Regras
- **Falha prévia:** registre a falha dos testes novos antes de mudar o comportamento.
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes em código novo.
- **Textos:** CLI em pt-BR. Liste os textos novos.

## Teste de uso (binário real)
Com `cargo run -p plots-cli --locked -- ...`, rode `simular` e `imprimir --simular` nas duas simuladas, com 6 e 5 páginas e nas duas ordens, e `--folhas-finais`. Cole comando, saída e código de saída, e diga se bate com a tabela do plano. Renderize as folhas finais com PyMuPDF (`python -I`) e leia uma imagem de cada modelo.

## Relatório
O que existia antes, cada item com testes e grupo de falha prévia, linha do CI, conferência de caminhos, autoverificação vazia, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
