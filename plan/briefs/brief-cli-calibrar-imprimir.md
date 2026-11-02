# cli-calibrar-imprimir: calibrar uma fila e conduzir a impressão pelo terminal, ainda em simulação

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master` (base d4b804b). Faça commits wip; nunca dê push nem squash. Não altere nada em E:\PlotS fora do seu worktree. Outro agente trabalha em paralelo em `src-tauri/` e `ui/`: não mexa nessas pastas.

## Ambiente
- Use em todo comando cargo `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/cli-imprimir"`: o disco E: está quase cheio. Logs ficam em `target/` do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- Se o sistema matar o build por falta de memória, repita uma vez; se morrer de novo, apague a pasta `debug/incremental` do target e tente mais uma vez. Só então relate.
- Depois da última edição, rode uma vez `cargo fmt --all --check` e `cargo test --workspace --locked`, gravando em `target/ci.log`, seguido de `CI_EXIT <código>`. Nunca faça wip depois de um CI vermelho.
- Log de falha prévia: `target/fail-before.log`, com execuções reais. Classifique cada teste novo em: falha prévia registrada, API nova sem execução na base, ou guarda.
- Sem dependências novas além de `plots-perfis` como dependência de `plots-cli`.
- **Nunca envie nada a uma impressora, nem à "Microsoft Print to PDF", e nunca toque o `%APPDATA%` real** em testes ou no teste de uso: passe sempre `--perfis` com um caminho temporário.
- Limpeza conforme "Limpeza constante do disco" em AGENTS.md.

## Navegação
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; depois leia só as linhas localizadas e confirme no worktree. Não rode `graphify update`.
- `plots-cli/src/lib.rs`: despacho, `USO_*`, `ImpressoraVirtual`, `gerar_pdfs`, modo guiado
- `plots-perfis/src`: `carregar`, `salvar`, `caminho_padrao`, `obter`, `inserir`
- `plots-core/src/lib.rs`: `ajustar_perfil_por_calibracao`, `plano_lotes_por_papel`
- `plots-print/src/job.rs`: `Job`, `SimBackend`, `reiniciar_lote`

## Fontes
`plan/04-calibracao.md`, `plan/06-perfis.md`, `plan/07-cli-impressao.md` e `plan/13-fluxo-e-recolocacao.md`, no worktree. Não abra `E:\PlotS\plan\estilo-ui-ux.md` nem `E:\PlotS\codex-session-*.md`.

## Regras sobre identificadores do plano
Código, comentários e testes descrevem comportamento com palavras próprias: sem IDs de requisito, nome de lote, "auditoria" ou "brief".
Autoverificação, não pode imprimir nada: `sh E:/PlotS/.claude/team/selfcheck.sh d4b804b plots-cli`

## Escopo
Primeiro registre no relatório o que existe hoje.

1. **`plots calibrar NOME --saida DIR [--papel P] [--perfis ARQUIVO]`.** `NOME` é o nome exato da fila. Grava `calibracao.pdf` em `DIR`, sem sobrescrever, e guia em texto: imprimir as 4 páginas pelo programa que quiser, recolocar do jeito de costume e responder às duas perguntas: `O verso da folha 1 é o 2? (s/n)` e `O 2 está de cabeça para baixo? (s/n)`. Resposta inválida repete a pergunta, e fim da entrada cancela sem gravar nada. Com as respostas, usa `ajustar_perfil_por_calibracao` sobre o perfil já salvo da fila, ou o padrão, e grava com `plots-perfis`. Mostra o perfil resultante em palavras e diz que o resultado só vale depois de conferido em um lote. O padrão de `--perfis` é `caminho_padrao()`.
2. **`plots imprimir ARQUIVO --impressora NOME --lote K --saida DIR --simular [--ordem pares|impares] [--perfis ARQUIVO] [--folhas-finais]`.**
   - **Sem `--simular`:** erro `o envio à impressora ainda não está disponível; use --simular para conferir o fluxo sem imprimir`. Nada é gerado.
   - **Perfil:** `Simulada — Traseira` e `Simulada — Gaveta` (e os atalhos `traseira` e `gaveta`) usam os perfis embutidos. Qualquer outro nome lê o perfil salvo; sem perfil, erro que manda rodar `plots calibrar`.
   - **Fluxo por lote, com pausa real:** grava as passadas como `gerar-pdf`, mostra as páginas da frente, simula o envio e a confirmação do spooler pelo `Job`, mostra o gesto de recolocação e espera Enter; fim da entrada cancela, sem apagar o que foi gerado, e diz em que lote parou. Depois simula o verso e relata `N de M folhas corretas` e o próximo lote. Em um nome com perfil salvo, sem mecânica medida, a simulação usa a mecânica coerente com o perfil e diz que isso confere a sequência, não a máquina.
   - **Gesto:** usa a frase fixa da especificação, completando o lado: `Pegue o maço como saiu, não embaralhe, e coloque na entrada com a face impressa para ...`. Na gaveta, diz ao final para virar o maço antes de ler.
3. **Mensagens e ajuda.** Atualize `USO_*`, `--help` e a mensagem de comando desconhecido. A frase da ajuda geral sobre não enviar à impressora continua verdadeira neste lote; escreva de modo que a próxima mudança não a deixe falsa sem que um teste falhe. Avisos do `gerar-pdf`, como assinatura e orientação mista, aparecem também em `imprimir`.
4. **Reuso.** Compartilhe código com `gerar_pdfs` e com o modo guiado, sem duplicar a geração e a gravação sem sobrescrever.

## Regras
- **Testes:** com entrada simulada. `calibrar`: as 4 combinações de respostas com o perfil gravado, resposta inválida, fim da entrada sem gravar, arquivo de perfis inválido sem ser alterado, `calibracao.pdf` já existente. `imprimir`: sem `--simular`, nome desconhecido sem perfil, perfil salvo, cancelamento no meio, dois lotes completos nas duas impressoras simuladas, ordem ímpares, aviso de assinatura com fixture sintética. Registre a falha antes de implementar.
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes em código novo.
- **Textos:** CLI em pt-BR. Liste todos os textos novos. Sem i18n nesta etapa.

## Teste de uso (binário real)
Em pasta temporária dentro de `target/`, com `--perfis` temporário: rode `calibrar` e `imprimir --simular` passando respostas pela entrada padrão, nas duas impressoras simuladas e em um nome com perfil salvo. Cole comando, saída e código de saída. Use o `calibracao.pdf` gerado como documento de entrada.

## Relatório
O que existia antes, cada item com testes e grupo de falha prévia, linha do CI, conferência de caminhos, autoverificação vazia, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
