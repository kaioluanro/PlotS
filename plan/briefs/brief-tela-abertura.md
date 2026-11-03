# tela-abertura: janela pequena enquanto o programa prepara tudo

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master`. Faça commits wip; nunca dê push nem squash. Leia primeiro `E:\PlotS\plan\briefs\coordenacao-paralela.md`: ele diz exatamente quais arquivos são seus.

## Ambiente
- `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/tela-abertura"`, `CARGO_INCREMENTAL=0`, `-j 4`. Em `ui/`, rode `npm ci`. Sem dependências novas.
- CI depois da última edição: `cargo fmt --all --check`, `cargo test --workspace --locked`, e em `ui/` `npm test` e `npm run build`, em `target/ci.log`, seguido de `CI_EXIT <código>`.
- Log de falha prévia: `target/fail-before.log`, com execuções reais.
- Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.
- Sem identificadores do plano em código e testes. Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh <base> src-tauri ui`, com `<base>` igual ao commit de `master` em que começou.
- Não envie nada a nenhuma fila. Não grave no `%APPDATA%` real: use `PLOTS_PERFIS`.

## Fontes
`plan/10-splash.md` e `plan/14-padrao-visual-e-idiomas.md`, no worktree, e a norma em `E:\PlotS\plan\estilo-ui-ux.md`. Antes de decidir detalhes visuais, carregue a skill `ui-design-craft`. Não abra `E:\PlotS\codex-session-*.md`.

## Escopo
1. **Janela `abertura`:** 480 × 300, centralizada, sem bordas nem barra de título, fora da barra de tarefas, fundo `--bg`. Janela `main` começa com `visible: false`.
2. **Imagem provisória:** o usuário ainda não forneceu a imagem. Desenhe em SVG a marca provisória existente, com o nome PlotS em texto, sem arquivo binário novo. Comentário e constante deixam claro que é provisório. O plano fala em PNG de 960 × 600: registre que entra quando a imagem existir.
3. **Abaixo da imagem:** barra de progresso de 2 px (`--accent` sobre `--border`) e uma linha de estado em mono 11 px, em `t()`: "Carregando impressoras…" etc.
4. **Carregamento em segundo plano no `setup`:** perfis, lista de impressoras e, se houver, o PDF passado por "Abrir com". A pré-carga serve para falhar cedo e mostrar o aviso; a janela principal pode reler depois. A `pdfium.dll` só é verificada se existe, sem carregar.
5. **Tempo:** pelo menos 800 ms, no máximo 10 s. Passado o limite, abre a janela principal mesmo assim e mostra um aviso do que falhou. Lógica de tempo em função pura testada, sem panic e sem bloquear a thread principal. Se a abertura falhar, a janela principal abre igual: nunca deixe o programa sem janela.
6. **Passagem:** quando o carregamento termina e a janela principal avisa que montou, um comando fecha `abertura` e mostra `main`, com fade de 220 ms na principal. Com `prefers-reduced-motion`, troca direta. Para a interface avisar que montou, sem editar `App.tsx`, use um módulo novo em `ui/src/abertura/` chamado por `ui/src/main.tsx` em uma única linha.
7. **Textos** em pt-BR e en.

## Capturas e teste de uso
App real por CDP, com porta própria. Nos dois temas, em `target/shots/`: abertura em carregamento; aviso depois do limite (force com uma variável de teste só no build e2e); janela principal depois da passagem; inglês. Meça o tempo mínimo e o máximo no app real. Meça cada par novo com `contrast.py`. Leia cada imagem.

## Relatório
O que existia antes, cada item com testes e grupo de falha prévia, tempos medidos, linha do CI, autoverificação, contraste, capturas, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
