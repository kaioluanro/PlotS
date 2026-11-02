# windows-filas: listar as filas de impressão do Windows e seu estado

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master`. Faça commits wip; nunca dê push nem squash. Não altere nada em E:\PlotS fora do seu worktree.

## Ambiente
- Use em todo comando cargo `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/windows-filas"`: o disco E: está quase cheio. Logs ficam em `target/` do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- Se o sistema matar o build por falta de memória, repita uma vez; se morrer de novo, apague a pasta `debug/incremental` do target e tente mais uma vez. Só então relate.
- Depois da última edição, rode uma vez `cargo fmt --all --check` e `cargo test --workspace --locked`, gravando em `target/ci.log`, seguido de `CI_EXIT <código>`. Nunca faça wip depois de um CI vermelho. O workspace inclui `src-tauri`, e a compilação dele é demorada: use `CARGO_INCREMENTAL=0`.
- Log de falha prévia: `target/fail-before.log`, com execuções reais.
- **Dependência nova permitida:** só o crate `windows`, com versão exata e apenas as features necessárias, só para alvo Windows. Rede com `CARGO_NET_OFFLINE=false` só para isso. Declare a licença.
- **Este lote só lê.** Nada é enviado a nenhuma impressora, nem à "Microsoft Print to PDF". Nada de `OpenPrinterW` para escrita, `StartDocW` ou alteração de `DEVMODE`.
- Limpeza conforme "Limpeza constante do disco" em AGENTS.md.

## Navegação
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; depois leia só as linhas localizadas e confirme no worktree. Não rode `graphify update`.
- `plots-print/src/job.rs`: `trait Backend`
- `plots-cli/src/lib.rs`: despacho e `USO_*`

## Fontes
`plan/08-impressao-windows.md` e `plan/15-multiplas-impressoras.md`, no worktree, e a documentação pública da Microsoft das funções usadas. Não abra `E:\PlotS\plan\estilo-ui-ux.md` nem `E:\PlotS\codex-session-*.md`.

## Regras sobre identificadores do plano
Código, comentários e testes descrevem comportamento com palavras próprias: sem IDs de requisito, nome de lote, "auditoria" ou "brief".
Autoverificação, não pode imprimir nada: `sh E:/PlotS/.claude/team/selfcheck.sh <base> plots-windows plots-cli`, com `<base>` igual ao commit de `master` em que você começou.

## Escopo
Primeiro registre no relatório o que existe hoje.

1. **Crate novo `plots-windows`**, membro do workspace, com o mínimo possível de `unsafe`, cada bloco com comentário explicando a garantia. Fora do Windows, o crate compila e devolve uma lista vazia ou um erro claro.
2. **Listar filas** com `EnumPrintersW`, locais e conectadas: nome exato, nome do driver, porta, se é a padrão, e o estado lido. Use a chamada em duas etapas, com o tamanho pedido pelo sistema, um limite de tamanho e conversão UTF-16 segura. Lista vazia não é erro.
3. **Estado da fila** em um tipo próprio traduzido dos bits do Windows: pronta, pausada, offline, sem papel, atolada, erro, e outros bits como "outro" sem perder informação. Lógica de tradução em função pura e testada sem impressora. Registre que o estado reportado depende do driver.
4. **Comando `plots filas [--perfis ARQUIVO]`:** lista cada fila com estado em palavras e se tem perfil salvo; as simuladas aparecem em seção própria. Sem perfil, sugere `plots calibrar`. A saída deixa claro que nada foi enviado.
5. **Sem interface neste lote.**

## Regras
- **Testes:** tradução de estado, com bits isolados e combinados; conversão de nomes com acento e espaço; formatação da saída de `filas` a partir de uma lista fornecida, sem depender das impressoras da máquina. Um teste real de `EnumPrintersW` só pode exigir que a chamada não falhe, sem exigir que haja impressora.
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes. Tamanhos e ponteiros vindos do sistema são tratados com cuidado, e o código não confia que o buffer tenha o tamanho anunciado sem checar.
- **Textos:** CLI em pt-BR. Liste os textos novos. Atualize `USO_*` e a frase da ajuda sobre não enviar, mantendo os testes coerentes.
- **Nunca toque o `%APPDATA%` real:** use `--perfis` temporário no teste de uso.

## Teste de uso (binário real)
Rode `plots filas` nesta máquina, com `--perfis` temporário, e cole comando, saída e código de saída. A máquina tem pelo menos a fila "Microsoft Print to PDF". Confirme com `Get-PrintJob` que nenhum trabalho foi criado.

## Relatório
O que existia antes, cada item com testes e grupo de falha prévia, versão e licença da dependência, linha do CI, conferência de caminhos, autoverificação vazia, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
