# cli-mensagens: o CLI diz a verdade sobre o que faz e avisa sobre folha com orientação mista

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master`
(`git merge --ff-only master` primeiro; base 191b29e). Faça commits wip; nunca dê push nem squash.
Não altere nada em E:\PlotS fora do seu worktree.

## Ambiente
- **Build:** deixe CARGO_TARGET_DIR sem valor, para usar `target/` dentro do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- **Antes do primeiro build:** rode `touch plots-cli/src/lib.rs` e confira que as linhas de compilação apontam para o worktree.
- **Pouca memória:** se o sistema matar o build, repita o comando uma vez; se morrer de novo, apague `target/debug/incremental` e tente mais uma vez. Só então relate.
- **CI:** depois da última edição, rode uma vez em primeiro plano, no PowerShell:
  `cargo fmt --all --check; if ($LASTEXITCODE -ne 0) { exit 1 }; cargo test --workspace --locked; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; Write-Output 'CI_SUCCESS: cargo test --workspace'`
  Grave a saída em `target/ci.log` e acrescente `CI_EXIT <código>`. Tem de terminar com `CI_SUCCESS: cargo test --workspace` e `CI_EXIT 0`. Nunca faça wip depois de um CI vermelho.
- **Log de falha prévia:** `target/fail-before.log`, com execuções reais. Classifique cada teste novo em: falha prévia registrada, API nova sem execução na base, ou guarda.
- **Limpeza** (seção "Limpeza constante do disco" em E:\PlotS\AGENTS.md): apague os PDFs e pastas temporários do teste de uso e liste no relatório o que removeu. Mantenha `target/` do worktree: o coordenador precisa dele e o remove junto com o worktree.
- **Sem novas dependências:** Cargo.toml e Cargo.lock não mudam.

## Navegação no código
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; depois leia só as linhas localizadas e confirme no worktree. Não rode `graphify update`.
- `plots-cli/src/lib.rs`: `executar_com_io` (despacho e mensagens), `gerar_pdfs`
- `plots-cli/src/main.rs`: texto de uso em todo erro
- `plots-cli/tests/cli.rs`: `rejeita_pdf_com_mensagem_explicita`
- `plots-core/src/lib.rs`: `plano_lotes_por_papel`, `reconhecer_papel`, `OrientacaoPapel`, `DimensoesPagina`
- `plots-print/src/job.rs`: comentários por volta das linhas 23 e 147-158

## Segurança de processos
Encerre só os seus próprios PIDs, por número exato. Se uma ação for negada, pare e relate; não peça a outro que a faça. Não envie nada a nenhuma impressora, nem à "Microsoft Print to PDF".

## Fontes (somente estas)
E:\PlotS\AGENTS.md, seções "Tamanhos de papel" e "Fases de implementação". Não abra E:\PlotS\estilo-ui-ux.md nem E:\PlotS\codex-session-*.md.

## Regras sobre identificadores do plano
Comentários, testes e mensagens descrevem comportamento com palavras próprias: nada de IDs de requisito (PS-A...), BUG-n, nome de lote, "auditoria" ou "brief".
**Autoverificação, não pode imprimir nada:** `sh E:/PlotS/.claude/team/selfcheck.sh 191b29e plots-cli plots-core plots-print plots-pdf`

## Escopo
Primeiro registre no relatório o que o CLI faz hoje em cada caso.

1. **Aviso de orientação mista.** Frente e verso com o mesmo papel, mas orientações diferentes (ex.: 595×842 e 842×595), continuam aceitos porque cabem na mesma folha física. Hoje passam em silêncio. Crie em `plots-core` uma função pura que receba `&[DimensoesPagina]` e devolva os pares (frente, verso) de números de página com orientação diferente; sem panic, com reserva protegida como no restante do módulo. `gerar-pdf` escreve na saída, antes da linha "Gerados...", uma linha por folha: `Aviso: a folha das páginas 1 e 2 mistura retrato e paisagem; confira o resultado antes de imprimir.` A geração continua e o código de saída é 0. Página quadrada não gera aviso. Pares com tamanhos diferentes continuam com o erro atual.
2. **Comandos que ainda não existem.** `imprimir` e `calibrar` respondem, com ou sem argumentos: `o comando 'imprimir' ainda não está disponível; use 'simular' ou 'gerar-pdf'` (idem para `calibrar`). Nada diz "PDF não é aceito".
3. **Comando desconhecido ou ausente.** Responde `comando ausente ou desconhecido; use 'simular' ou 'gerar-pdf'`, mesmo com um argumento .pdf.
4. **`simular` com PDF.** `simular arquivo.pdf ...` e `simular --pdf ...` respondem `'simular' não lê PDF: informe --paginas N, ou use 'gerar-pdf' para gerar as passadas de um arquivo`.
5. **Texto de uso.** Em erro, `main.rs` mostra só o uso do comando pedido: `simular`, `gerar-pdf` ou, nos demais casos, os dois. `--help` e `-h` funcionam como primeiro argumento (todos os comandos) e logo depois do subcomando (só aquele comando), e saem com código 0. O texto de ajuda geral deixa claro que nada é enviado à impressora.
6. **Comentários.** Corrija "sa?da" em `plots-cli/src/lib.rs` e "n?o" em `plots-print/src/job.rs`, procure outros `?` no lugar de acento em comentários dos crates e recoloque em `job.rs` o comentário de `SimBackend`, partido em duas partes, em cima do struct. Só comentários: sem mudar comportamento.

## Regras
- **Testes em `plots-cli/tests/cli.rs` e nos crates:** cada item 1 a 5 com teste de entrada e saída. Inclua também: `--help` citando `gerar-pdf`; `gerar-pdf` com opção repetida; `gerar-pdf` com PDF criptografado (erro explícito, sem arquivo criado), reaproveitando a forma de gerar PDF criptografado dos testes de `plots-pdf` ou criando-o no próprio teste; folha retrato+paisagem com aviso e PDFs gerados; documento sem mistura sem nenhum aviso. Atualize `rejeita_pdf_com_mensagem_explicita` para o novo texto.
- **Falha prévia:** registre a falha dos testes dos itens 1 a 5 antes de implementar.
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes em código novo; lista vazia e página única sem verso não geram aviso nem panic.
- **Textos:** CLI em pt-BR, com os textos acima palavra por palavra. Liste todos os textos novos. Sem i18n nesta etapa.

## Teste de uso (binário real)
Com `cargo run -p plots-cli --locked -- ...`, em uma pasta temporária dentro de `target/`: rode cada caso dos itens 1 a 5 e, em `gerar-pdf`, confira os PDFs gerados. Cole no relatório comando, saída e código de saída.

## Relatório
O que o CLI fazia antes; cada item com seus testes e grupo de falha prévia; linha do CI, conferência de caminhos e autoverificação vazia; textos novos; o que foi removido na limpeza; o que não foi verificado. Termine com tudo em commits wip e `git status` limpo. Informe o caminho do worktree e o nome do branch.
