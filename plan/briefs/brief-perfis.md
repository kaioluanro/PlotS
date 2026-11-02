# perfis: guardar por fila de impressão o que foi calibrado

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master` (base fc0888b). Faça commits wip; nunca dê push nem squash. Não altere nada em E:\PlotS fora do seu worktree. Outros agentes estão trabalhando em paralelo em `plots-print` e na interface: não mexa em `plots-print`, `plots-cli`, `plots-pdf`, nem em `src-tauri/` e `ui/`.

## Ambiente
- Deixe CARGO_TARGET_DIR sem valor, para usar `target/` do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- Se o sistema matar o build por falta de memória, repita uma vez; se morrer de novo, apague `target/debug/incremental` e tente mais uma vez. Só então relate.
- Depois da última edição, rode uma vez `cargo fmt --all --check` e `cargo test --workspace --locked`, gravando em `target/ci.log`, seguido de `CI_EXIT <código>`. Nunca faça wip depois de um CI vermelho.
- Log de falha prévia: `target/fail-before.log`, com execuções reais. Classifique cada teste novo em: falha prévia registrada, API nova sem execução na base, ou guarda.
- Rede: permitida só para baixar as dependências abaixo, com `CARGO_NET_OFFLINE=false`. O usuário autorizou essas dependências.
- Limpeza conforme "Limpeza constante do disco" em AGENTS.md: mantenha `target/` do worktree e apague o resto do que criar.

## Navegação
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; depois leia só as linhas localizadas e confirme no worktree. Não rode `graphify update`.
- `plots-core/src/lib.rs`: `PerfilImpressora`, `OrdemPassadas`, `ajustar_perfil_por_calibracao`

## Fontes
`plan/06-perfis.md` e `plan/04-calibracao.md`, no worktree. Não abra `E:\PlotS\plan\estilo-ui-ux.md` nem `E:\PlotS\codex-session-*.md`.

## Regras sobre identificadores do plano
Código, comentários e testes descrevem comportamento com palavras próprias: sem IDs de requisito, nome de lote, "auditoria" ou "brief".
Autoverificação, não pode imprimir nada: `sh E:/PlotS/.claude/team/selfcheck.sh fc0888b plots-perfis plots-core`

## Escopo
Primeiro registre no relatório o que existe hoje.

1. **Crate novo `plots-perfis`**, membro do workspace, dependendo de `plots-core`, `serde` (derive), `toml`, `directories` e `thiserror`, com versões exatas e licença declarada no relatório. `plots-core` continua sem acesso a disco e sem dependências novas: use tipos próprios de arquivo no crate novo e converta.
2. **Formato** de `perfis.toml`: um campo `versao = 1` e uma entrada por fila, com a chave pelo nome exato da fila no sistema, mais `apelido` opcional, `passada1_inversa`, `passada2_inversa`, `verso_girar_180` e o texto opcional do gesto de recolocação. Nomes de fila com espaço, acento, barra invertida e aspas funcionam.
3. **Operações:** `caminho_padrao()` (`%APPDATA%/plots/perfis.toml` no Windows, via `directories`), `carregar(caminho)`, `salvar(caminho, &perfis)`, obter, inserir e remover por fila. `caminho_padrao` não cria nada. Só `salvar` escreve e cria a pasta se preciso.
4. **Segurança dos dados:**
   - Arquivo ausente significa lista vazia, sem erro.
   - TOML inválido, versão desconhecida ou arquivo maior que 1 MB dá erro claro com o caminho, sem alterar nem apagar o arquivo.
   - `salvar` é atômico: grava em arquivo temporário na mesma pasta, faz flush e renomeia. Se falhar, o arquivo anterior fica intacto e o temporário é removido.
5. **Sem CLI nem interface neste lote:** só a biblioteca.

## Regras
- **Testes:** ida e volta de todos os campos e das 8 combinações de perfil; nome de fila hostil; arquivo ausente; TOML inválido, versão futura e arquivo grande demais sem alterar os bytes originais; pasta inexistente; falha de escrita sem perder o arquivo anterior. Usem só pastas temporárias e **nunca toquem o %APPDATA% real**.
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes.
- **Textos:** mensagens de erro em pt-BR, listadas no relatório.

## Teste de uso
Sem binário neste lote: rode um exemplo temporário de ida e volta em uma pasta dentro de `target/`, mostre o TOML gerado no relatório e apague o exemplo.

## Relatório
O que existia antes, cada item com testes e grupo de falha prévia, versões e licenças das dependências, linha do CI, conferência de caminhos, autoverificação vazia, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
