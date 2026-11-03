# visualizador: ver o PDF e as miniaturas dentro da janela

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master`. Faça commits wip; nunca dê push nem squash. Leia primeiro `E:\PlotS\plan\briefs\coordenacao-paralela.md`: ele diz exatamente quais arquivos são seus.

## Ambiente
- `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/visualizador"`, `CARGO_INCREMENTAL=0`, `-j 4`. Em `ui/`, rode `npm ci`.
- **Dependência:** o usuário autorizou o pdf.js. Instale `pdfjs-dist` com versão exata e informe a licença. **Se o sistema negar a instalação, pare e relate; não contorne.**
- CI depois da última edição: `cargo fmt --all --check`, `cargo test --workspace --locked`, e em `ui/` `npm test` e `npm run build`, em `target/ci.log`, seguido de `CI_EXIT <código>`.
- Log de falha prévia: `target/fail-before.log`, com execuções reais.
- Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; o grafo pode não cobrir a interface.
- Sem identificadores do plano em código e testes. Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh <base> src-tauri ui`, com `<base>` igual ao commit de `master` em que começou.
- Não envie nada a nenhuma fila. Não grave no `%APPDATA%` real.

## PDFs reais do usuário
Autorizados em `C:/Users/Kaio Luan/Downloads`, para medir o desempenho. **Somente leitura**, nunca em capturas, repositório, testes ou relatório: capturas usam PDFs de teste gerados por você. Sem nomes nem conteúdo no relatório.

## Fontes
`plan/12-visualizador.md`, `plan/14-padrao-visual-e-idiomas.md`, no worktree, e a norma em `E:\PlotS\plan\estilo-ui-ux.md`. Antes de decidir detalhes visuais, carregue a skill `ui-design-craft`. Não abra `E:\PlotS\codex-session-*.md`.

## Escopo
1. **Comando Rust `ler_documento(caminho)`** em `src-tauri/src/documento.rs`: devolve os bytes com o mesmo limite de 1 GiB e os mesmos erros da abertura, fora da thread da interface, recusando PDF que não passe por `PdfOrigem::carregar`. O pdf.js nunca recebe senha nem abre PDF protegido.
2. **Área do documento:** com a aba aberta e o PDF aceito, mostra as páginas em rolagem contínua, renderizando só o que está visível, com memória limitada: páginas fora da tela são descartadas. Texto selecionável não é necessário.
3. **Barra de 28 px** com página atual, total e zoom, em mono: diminuir, aumentar, ajustar à largura. Atalhos Ctrl+`+`, Ctrl+`-` e Ctrl+`0`, só com o foco na área do documento e sem brigar com os atalhos globais.
4. **Miniaturas** à direita da área, recolhíveis, carregadas sob demanda, com a página atual marcada por mais do que cor. Clique ou Enter vai à página.
5. **Estados:** carregando, PDF recusado (mostra o motivo que a aba já tem) e erro de renderização, sempre sem travar a janela. A renderização roda em worker, para a janela continuar respondendo.
6. **CSP:** ajuste o que for preciso em `tauri.conf.json` apenas se não houver outra saída, e relate, porque esse arquivo é do lote `tela-abertura`. Prefira um worker servido pelo próprio bundle.
7. **Textos** em pt-BR e en, com prefixo `visualizador.*`.

Fora de escopo: escolher intervalo de páginas (fica para depois do lote `cartoes-impressao`), impressão, anotações, busca.

## Regras
- **Testes:** Rust para `ler_documento` (válido, inexistente, protegido, grande demais com limite pequeno). Vitest para a lógica pura: página visível a partir da rolagem, janela de páginas a manter, zoom com limites, estado das miniaturas.
- **Nunca travar:** PDF hostil não derruba a janela; sem unwrap/expect/indexação sem checagem no Rust.
- **Contraste:** meça cada par novo com `contrast.py`.

## Capturas e teste de uso
App real por CDP, com porta própria, nos dois temas, em `target/shots/`, com PDFs de teste gerados por você: documento aberto; miniaturas abertas e recolhidas; zoom; rotação `/Rotate`; carregando; recusado; foco; janela no mínimo; inglês. Meça com um PDF real grande: tempo até a primeira página, memória aproximada e se a janela responde durante a rolagem. Leia cada imagem.

## Relatório
O que existia antes, cada item com testes e grupo de falha prévia, versão e licença do pdf.js, medidas, linha do CI, autoverificação, contraste, capturas, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
