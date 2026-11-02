# seletor-modelos: escolher uma das duas configurações para cada impressora

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master`. Faça commits wip; nunca dê push nem squash. Não altere nada em E:\PlotS fora do seu worktree.

## Ambiente
- Use em todo comando cargo `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/seletor"` e `CARGO_INCREMENTAL=0`: o disco E: está quase cheio. Logs e capturas ficam em `target/` do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- Se o sistema matar o build por falta de memória, repita uma vez; se morrer de novo, apague a pasta `debug/incremental` do target e tente mais uma vez. Só então relate.
- Em `ui/`, rode `npm ci`. Sem dependências novas de npm nem de Cargo. Se o sistema negar algo, pare e relate; não contorne.
- CI depois da última edição: `cargo fmt --all --check`, `cargo test --workspace --locked`, e em `ui/` `npm test` e `npm run build`. Grave em `target/ci.log`, seguido de `CI_EXIT <código>`. Nunca faça wip depois de um CI vermelho.
- Log de falha prévia: `target/fail-before.log`, com execuções reais.
- Limpeza conforme "Limpeza constante do disco" em AGENTS.md; capturas em `target/shots/` ficam para a revisão de interface.

## Segurança
**Este lote não envia nada a nenhuma fila.** Encerre só os seus próprios PIDs, por número exato; não deixe app rodando. Não use diálogos nativos nos testes nem envie teclas a outras janelas. Nunca grave no `%APPDATA%` real durante os testes: use `PLOTS_PERFIS` no app e `--perfis` no CLI, com caminho temporário.

## Navegação
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; o grafo pode não cobrir a interface, e nesse caso leia os arquivos abaixo. Não rode `graphify update`.
- `plots-core/src/lib.rs`: `PerfilImpressora`, `GestoRecolocar`
- `plots-perfis/src/formato.rs`, `plots-perfis/src/perfis.rs`
- `plots-cli/src/lib.rs`: `ImpressoraVirtual`; `plots-cli/src/imprimir.rs`: `gesto_do_perfil`, `resolver_fila`, `mecanica_coerente`, `gesto_de_recolocacao`
- `plots-print/src/lib.rs`: `Mecanica::traseira`, `Mecanica::gaveta`, `Recolocacao::virada_pela_borda_de_cima`
- `src-tauri/src/impressoras.rs`: dedução do modo por `passada1_inversa`
- `ui/src/components/Sidebar.tsx`, `ui/src/impressoras.ts`, `ui/src/i18n.ts`

## Fontes
`plan/19-modelos-de-impressora.md`, `plan/11-lateral-impressoras.md` e `plan/14-padrao-visual-e-idiomas.md`, no worktree, e a norma em `E:\PlotS\plan\estilo-ui-ux.md`. Antes de decidir detalhes visuais, carregue a skill `ui-design-craft`. Não abra `E:\PlotS\codex-session-*.md`.

## Regras sobre identificadores do plano
Código, comentários e testes descrevem comportamento com palavras próprias: sem IDs de requisito, nome de lote, "auditoria" ou "brief". **Sem marca nem modelo de impressora no produto e nos testes**: use "Jato de tinta" e "Laser".
Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh <base> plots-core plots-perfis plots-print plots-cli src-tauri ui`, com `<base>` igual ao commit de `master` em que você começou.

## O problema
O usuário não tem as impressoras à mão e quer duas configurações prontas, sem calibrar. Hoje, uma fila real só ganha perfil por `plots calibrar`. A barra lateral deduz o modo de `passada1_inversa`, e isso ficou errado: a laser também inverte a frente. `GestoRecolocar` também não sabe dizer "virar pela borda de cima".

## Escopo
Primeiro registre no relatório o que existe hoje.

1. **Modelo como tipo, em um único lugar.** Em `plots-core`, um tipo `ModeloImpressora` com `JatoDeTinta` e `Laser`, com perfil e gesto de cada um conforme a tabela de `plan/19-modelos-de-impressora.md`. `GestoRecolocar` passa a expressar a virada pela borda de cima e o "não virar". As impressoras simuladas do CLI e os testes usam esse tipo, sem duplicar a tabela.
2. **Perfil salvo guarda o modelo.** Em `perfis.toml`, um campo opcional `modelo = "jato"` ou `"laser"`, e o que for preciso para guardar a borda da virada. Arquivos existentes, sem esses campos, continuam abrindo: mantenha `versao = 1` e use valores padrão. Teste de ida e volta, e de arquivo antigo.
3. **CLI:** `plots perfil NOME --modelo jato|laser [--perfis ARQUIVO]` grava o perfil pronto da fila, preservando o apelido e sem exigir que a fila exista. Recusa nomes de impressora simulada, com a mesma mensagem de `calibrar`. Em `plots imprimir`, um perfil com modelo usa a frase de gesto do modelo, com o lado, e o simulador com a mecânica daquele modelo, sem `mecanica_coerente`. Perfil sem modelo continua como está. A saída diz que o modelo não foi medido naquela máquina. O `calibrar` limpa o modelo do perfil, porque os valores deixam de ser os do modelo. Atualize `USO_*`.
4. **Barra lateral:**
   - Corrija `src-tauri/src/impressoras.rs`: o modo vem do campo `modelo`. Perfil sem modelo mostra "Perfil personalizado", sem deduzir.
   - Seletor de modelo de 24 px em cada fila real, nos estados sem perfil, com modelo e com perfil personalizado: "Jato de tinta" e "Laser", no lugar de "Calibrar…". Opção "Calibrar…" no mesmo seletor abre o diálogo atual.
   - Comando Rust `definir_modelo(fila, modelo)`, com gravação atômica de `plots-perfis`. Falha vira aviso na tela, e o seletor volta ao valor salvo. Depois de gravar, a linha mostra o modelo e a face da entrada.
   - Simuladas continuam fixas: a de traseira mostra "Jato de tinta", e a de gaveta, "Laser".
   - Use `<select>` nativo estilizado ou Radix, o que for mais simples sem dependência nova. Foco, teclado e cores forçadas têm de funcionar.
   - Marcas nos dados de `ui/src/calibrar.test.ts` saem: troque por nomes genéricos, mantendo os casos.
5. **Textos** via `t()` em pt-BR e en, com uma frase curta dizendo que o modelo é um ponto de partida e deve ser conferido com 2 ou 3 folhas.

Fora de escopo: ligar o botão "Imprimir frente" e cartões de impressão.

## Regras
- **Testes:** Rust para o tipo de modelo (sequência exata de 6 páginas contra o simulador, nos dois modelos), perfis (ida e volta, arquivo antigo, inválido), `plots perfil` e o modo em `impressoras.rs`. Vitest para o estado do seletor e a falha ao gravar.
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes.
- **Contraste:** meça cada par novo com `contrast.py` nos dois temas.

## Capturas e teste de uso
App real, com mouse e teclado por CDP, nos dois temas, em `target/shots/`: seletor fechado e aberto, com foco; fila sem perfil, com cada modelo e com perfil personalizado; escolha gravada; falha ao gravar; janela no mínimo; inglês; cores forçadas. Binário real: `plots perfil` e depois `plots imprimir --simular` nos dois modelos, com arquivo de perfis temporário. Cole comando e saída. Leia cada imagem.

## Relatório
O que existia antes, cada item com testes e grupo de falha prévia, linha do CI, conferência de caminhos, autoverificação, contraste, capturas, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
