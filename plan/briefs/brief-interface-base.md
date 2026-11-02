# interface-base: janela do PlotS com barra superior, abas, temas e idiomas

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master` (base fc0888b). Faça commits wip; nunca dê push nem squash. Não altere nada em E:\PlotS fora do seu worktree. Outros agentes estão trabalhando em paralelo em `plots-print` e em um crate novo de perfis: não mexa em `plots-print`, `plots-cli`, `plots-pdf` nem `plots-core`.

## Ambiente
- Deixe CARGO_TARGET_DIR sem valor, para usar `target/` do worktree. No mínimo 4 jobs. Um comando pesado por vez: esta máquina tem outros agentes compilando.
- Se o sistema matar o build por falta de memória, repita uma vez; se morrer de novo, apague `target/debug/incremental` e tente mais uma vez. Só então relate.
- **Dependências:** o usuário autorizou baixar as dependências do projeto, incluindo Tauri. Use `CARGO_NET_OFFLINE=false` no Cargo. Fixe versões exatas e versione `ui/package-lock.json`. Só as necessárias: `tauri`, `tauri-build`, `@tauri-apps/api`, `@tauri-apps/cli`, `react`, `react-dom`, `typescript`, `vite`, `@vitejs/plugin-react`, `lucide-react`, `@radix-ui/react-dialog` e `vitest`. Declare a licença de cada uma no relatório. **Se o sistema negar uma instalação, pare e relate; não tente contornar.**
- **CI:** depois da última edição, rode uma vez, nesta ordem: `cargo fmt --all --check`, `cargo test --workspace --locked`, `npm test` e `npm run build` em `ui/`, gravando em `target/ci.log`, seguido de `CI_EXIT <código>`. Nunca faça wip depois de um CI vermelho.
- **Log de falha prévia:** `target/fail-before.log`, com execuções reais. Classifique cada teste novo em: falha prévia registrada, API nova sem execução na base, ou guarda.
- **Limpeza** conforme "Limpeza constante do disco" em AGENTS.md. Acrescente ao `.gitignore` só o necessário (`/ui/node_modules/`, `/ui/dist/`, `/src-tauri/gen/`). Apague capturas e perfis temporários depois de examiná-los, exceto as capturas pedidas abaixo, que ficam em `target/shots/` para a revisão de interface.

## Navegação
Sem código de interface até agora. Se precisar de código existente, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json` antes do primeiro grep ou leitura de código. Não rode `graphify update`.

## Segurança de processos
Encerre só os seus próprios PIDs, por número exato; nunca um padrão que pegue todas as instâncias. Não use diálogos nativos nos testes. Não envie teclas a outras janelas. Se uma ação for negada, pare e relate; não peça a outro que a faça. Não feche a app deixando processos: não deixe nenhum app rodando no fim.

## Fontes (somente estas)
`plan/09-janela-e-abas.md`, `plan/14-padrao-visual-e-idiomas.md` e `plan/00-visao-geral.md` (seção de arquitetura), no worktree, e a norma de estilo em `E:\PlotS\plan\estilo-ui-ux.md` (fora do Git). Antes de decidir qualquer detalhe visual, carregue a skill `ui-design-craft`. Não abra `E:\PlotS\codex-session-*.md`.

## Regras sobre identificadores do plano
Código, comentários e testes descrevem comportamento com palavras próprias: sem IDs de requisito, nome de lote, "auditoria" ou "brief".
Autoverificação, não pode imprimir nada: `sh E:/PlotS/.claude/team/selfcheck.sh fc0888b src-tauri ui`

## Escopo
Primeiro registre no relatório o que existe hoje.

1. **App Tauri 2.** `src-tauri/` como membro do workspace e `ui/` com React, TypeScript, Vite e CSS Modules. Binário `plots-app`. Toda regra de negócio fica no Rust. Janela `main` com `decorations: false`.
2. **Barra superior** conforme os planos: espaço do ícone, botão da barra lateral, abas, botão "+", minimizar, maximizar e fechar, com `data-tauri-drag-region` na área vazia. Medidas e tokens da norma de estilo.
3. **Abas por documento.** Comando Rust `abrir_documento(caminho)` valida que o caminho existe e é um arquivo `.pdf` e devolve um documento com nome de exibição; erro claro caso contrário. Um caminho passado na linha de comando abre em uma aba ("Abrir com"). "+" e Ctrl+O/Ctrl+T abrem o seletor de arquivo do plugin de diálogo do Tauri (acrescente `tauri-plugin-dialog` e `@tauri-apps/plugin-dialog` às dependências); arrastar um PDF para a janela abre aba nova. Ctrl+W fecha, Ctrl+Tab troca, Ctrl+B alterna a lateral. Cada aba guarda o próprio estado, por enquanto só a impressão em andamento (falso por padrão); fechar uma aba com impressão em andamento pede confirmação em diálogo Radix, com ESC para cancelar.
4. **Conteúdo sem dados falsos.** A área do documento mostra só um estado vazio honesto: nome do arquivo aberto e, sem documento, um convite a abrir um PDF. A barra lateral aparece e some com o cabeçalho IMPRESSORAS e um estado vazio. O visualizador e a lista de impressoras são de outros lotes: nada de lista inventada.
5. **Padrão visual.** `ui/src/theme.css` com os tokens da norma e os temas `dark` (padrão) e `light`, troca pelo sistema e por um atributo em `<html>`. Foco visível em todo controle, ícones `lucide-react` de 12 a 14 px, nenhum emoji, botões de ícone com `aria-label`.
6. **Idiomas.** `t()` com pt-BR e en, escolhidos pelo idioma do sistema com pt-BR como padrão; todo texto passa por `t()`. Sem biblioteca de i18n.

Fora de escopo: tela de abertura, pdf.js, lista de impressoras, ajustes, instalador.

## Regras
- **Testes:** Rust para `abrir_documento` (arquivo válido, inexistente, pasta, outra extensão, caminho com acento e espaço). Vitest para a lógica de abas (abrir, fechar a ativa, trocar com Ctrl+Tab e sem abas, confirmação com impressão em andamento) e para `t()` (toda chave existe nos dois idiomas).
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes no Rust; `tauri::Builder` com erro tratado.
- **Contraste:** meça cada par de texto e fundo novo com o `contrast.py` da skill `ui-design-craft`, nos dois temas, e liste no relatório.

## Teste de uso (app real, entrada real)
Compile o app em debug e rode `npm run build` antes, de modo que a janela use os arquivos gerados. Sugestão para operar sem dependência nova: inicie o binário com `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<porta livre>` e `WEBVIEW2_USER_DATA_FOLDER=<pasta temporária em target/>`, e use um script node em `target/` com o `WebSocket` nativo do Node para falar CDP: `Input.dispatchMouseEvent`, `Input.dispatchKeyEvent` e `Page.captureScreenshot`. Confira que a porta é do seu processo. Abra PDFs passando o caminho na linha de comando. O seletor nativo fica fora do teste automático: diga isso no relatório.

Exercite e fotografe em `target/shots/`, com prefixo NN-, lendo cada imagem:
1. Sem documento, tema escuro, pt-BR. 2. O mesmo no tema claro. 3. Três abas abertas, com nome longo truncado. 4. Foco de teclado visível em uma aba. 5. Lateral aberta e fechada. 6. Diálogo de confirmação ao fechar aba com impressão em andamento (ative o estado por uma função só de teste, não por produto). 7. Interface em inglês. 8. Janela estreita (minLargura definida por você).

Teste também: Ctrl+W, Ctrl+Tab, Ctrl+B, ESC no diálogo, minimizar, maximizar e fechar.

## Relatório
O que existia antes, cada item com testes e grupo de falha prévia, versões e licenças das dependências, linha do CI, conferência de caminhos, autoverificação vazia, pares de contraste, cada captura com o que mostra, textos novos nos dois idiomas, limpeza, tamanho de `target/`, e o que não foi verificado. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
