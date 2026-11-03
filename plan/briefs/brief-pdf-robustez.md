# pdf-robustez: PDFs gerados sem sobras e proteção contra arquivo malicioso

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master`. Faça commits wip; nunca dê push nem squash. Leia primeiro `E:\PlotS\plan\briefs\coordenacao-paralela.md`: você só mexe em `plots-pdf/`.

## Ambiente
- `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/pdf-robustez"`, `CARGO_INCREMENTAL=0`, `-j 4`.
- CI depois da última edição: `cargo fmt --all --check` e `cargo test --workspace --locked`, em `target/ci.log`, seguido de `CI_EXIT <código>`.
- Log de falha prévia: `target/fail-before.log`, com execuções reais.
- Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.
- Sem identificadores do plano em código e testes. Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh <base> plots-pdf`, com `<base>` igual ao commit de `master` em que começou.

## PDFs reais do usuário
Autorizados em `C:/Users/Kaio Luan/Downloads`. **Somente leitura**, sem copiar nada para o repositório, testes ou relatório, e sem nomes nem conteúdo no relatório: cada arquivo é um número. Saídas temporárias dentro de `target/`, apagadas ao fim. Python sempre com `-I`.

## Fontes
`plan/02-geracao-pdf.md`, no worktree. Não abra `E:\PlotS\plan\estilo-ui-ux.md` nem `E:\PlotS\codex-session-*.md`.

## Escopo
1. **Recursos de páginas de fora do lote.** Um PDF gerado não deve levar imagens usadas só em outras páginas, quando elas vêm de um `/Resources` compartilhado. Copie o dicionário de recursos de cada página e mantenha em `/XObject` só os nomes usados em `/Nome Do` no conteúdo daquela página, seguindo Form XObjects aninhados com profundidade limitada. Se a análise do conteúdo falhar, mantenha tudo. O resultado visual não pode mudar. Vale para passadas e folhas finais.
2. **Limites contra arquivo malicioso.** Teto de bytes por fluxo de conteúdo descomprimido, de total por passada e de profundidade, com erro claro em português, sem travar nem consumir memória sem limite. Escolha valores que não recusem documentos reais e diga quais escolheu e por quê.
3. **PDF com sangria.** Hoje o papel vem da `MediaBox`: PDF com `CropBox` A4 e `MediaBox` maior vira papel personalizado. Se o conteúdo visível for a `CropBox`, use a mesma caixa que o leitor mostra para medir a página, com teste. Se isso mudar o que é impresso, relate em vez de mudar.

## Testes
Fixtures sintéticas geradas no próprio teste: `/Resources` compartilhado com imagem usada só na página excluída (a imagem não pode estar na saída); Form aninhado; bomba de compressão; ciclo de Forms; `CropBox` menor que `MediaBox`. Registre a falha antes de corrigir.

## Teste de uso
Compare a imagem de cada página gerada com a original, a 30 dpi, com PyMuPDF, em todos os PDFs reais, e relate: aceitos, recusados por motivo, divergentes, e o tamanho total dos arquivos gerados antes e depois. Meta: 64 aceitos, sem divergência. O número de arquivos pode ter mudado; relate o real.

## Relatório
O que existia antes, cada item com testes e grupo de falha prévia, contagem nos PDFs reais, linha do CI, autoverificação, limpeza e o que não foi verificado. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
