# pdfs-reais: a geração de passadas deve aceitar PDFs comuns

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master` (base 88d09f1). Faça commits wip; nunca dê push nem squash. Não altere nada em E:\PlotS fora do seu worktree. Outros agentes trabalham em paralelo na interface e em `plots-perfis`: mexa só em `plots-pdf`, e em `plots-cli` só se for preciso.

## Ambiente
- Use em todo comando cargo `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/pdfs-reais"`: o disco E: está quase cheio. Logs ficam em `target/` do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- Se o sistema matar o build por falta de memória, repita uma vez; se morrer de novo, apague a pasta `debug/incremental` do target e tente mais uma vez. Só então relate.
- Depois da última edição, rode uma vez `cargo fmt --all --check` e `cargo test --workspace --locked`, gravando em `target/ci.log`, seguido de `CI_EXIT <código>`. Nunca faça wip depois de um CI vermelho.
- Log de falha prévia: `target/fail-before.log`, com execuções reais.
- Sem dependências novas no Cargo. Não envie nada a impressoras.
- Limpeza conforme "Limpeza constante do disco" em AGENTS.md.

## PDFs reais: regras estritas
O usuário autorizou usar os PDFs de `C:/Users/Kaio Luan/Downloads` como teste.
- **Somente leitura:** nunca altere, mova ou apague esses arquivos.
- **Nunca copie** esses arquivos nem trechos deles para o repositório, testes, comentários, commits ou relatório. Fixtures são sempre sintéticas, geradas no próprio teste.
- **Sem conteúdo no relatório:** nada de nomes de arquivo, títulos ou texto dos documentos. Refira-se a cada um por número e use só nomes de chaves de estrutura do PDF.
- Saídas ficam em pasta temporária criada por você dentro de `target/`, apagada ao fim.
- Rode Python com `-I` e mantenha scripts fora da pasta dos PDFs.

## Navegação
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; depois leia só as linhas localizadas e confirme no worktree. Não rode `graphify update`.
- `plots-pdf/src/lib.rs`: origem da mensagem "referência insegura à árvore de páginas original", `gerar_passada`, `gerar_folhas_finais`, `remover_inacessiveis`

## Fontes
`plan/02-geracao-pdf.md` no worktree. Não abra `E:\PlotS\plan\estilo-ui-ux.md` nem `E:\PlotS\codex-session-*.md`.

## Regras sobre identificadores do plano
Código, comentários e testes descrevem comportamento com palavras próprias: sem IDs de requisito, nome de lote, "auditoria" ou "brief".
Autoverificação, não pode imprimir nada: `sh E:/PlotS/.claude/team/selfcheck.sh 88d09f1 plots-cli plots-pdf`

## O problema
Com PDFs reais, `plots gerar-pdf` recusou 10 de 12 arquivos com `PDF inválido: referência insegura à árvore de páginas original`. Os PyMuPDF abrem esses arquivos sem problema. A recusa existe para que um PDF de passada não carregue páginas fora do lote, e é preciso manter essa garantia. O erro é recusar documentos comuns.

## Escopo
1. **Reproduza e descubra a causa real.** Para cada arquivo recusado, ache qual chave e qual caminho levam à árvore de páginas. Candidatos a conferir, sem presumir: `/Parent`, `/StructParents`, `/P` e estrutura marcada, `/Dest` e `/A` em links, marcadores, `/Thumb`, `/OCProperties`, `/Metadata`, grupos e entradas específicas de aplicativo. Registre a contagem por causa.
2. **Escreva fixtures sintéticas** que reproduzam cada causa, com a falha registrada antes da correção.
3. **Corrija** tirando da cópia as referências que não fazem parte do que é impresso, de modo que o resultado imprima igual. Recusar continua sendo a resposta quando a referência faz parte do desenho da página e não pode ser retirada com segurança. A mensagem então diz o motivo em palavras que o usuário entenda.
4. **Mantenha as garantias:** nada de conteúdo de página excluída no PDF gerado, incluindo imagens e fontes usadas só nelas. O teste de páginas excluídas continua passando e ganha casos com as novas chaves. Trabalho limitado, sem recursão sem limite.
5. **Pontos relacionados, se couberem no mesmo código sem aumentar o risco:** tirar `/ID` do trailer gerado; retirar do `/Resources` compartilhado imagens não usadas pela página copiada, com fallback para manter tudo se a análise falhar. Se não couberem, relate.

## Teste de uso
Use `C:/Users/KAIOLU~1/AppData/Local/Temp/claude/E--PlotS--deps-fetch/b5987bb2-a817-4f78-967b-954552088948/scratchpad/real/conferir.py` como base: ele compara a imagem de cada página gerada com a original, sem mostrar conteúdo. Argumentos: executável, pasta dos PDFs, pasta de saída. Ele examina 12 arquivos; faça uma cópia sua, em `target/` do worktree, que examine **todos** os PDFs, com tempo limite por arquivo. Na cópia, trate também documentos com mais de um papel, em que a ordem é por lote.

Meta: nenhum PDF comum recusado sem motivo real, e nenhuma divergência. Relate a contagem final: aceitos, recusados por motivo, divergentes. Antes e depois.

## Relatório
Causa com a contagem, o que mudou, testes e grupo de falha prévia, contagem antes e depois nos PDFs reais, linha do CI, conferência de caminhos, autoverificação vazia, limpeza e o que não foi verificado. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
