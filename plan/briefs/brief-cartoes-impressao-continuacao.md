# cartoes-impressao: continuação do lote parado por falta de disco

Responda em português brasileiro. O agente anterior parou quando o disco E: encheu. O trabalho dele está salvo no commit wip `d7d3474`, branch do worktree `E:\PlotS\.claude\worktrees\agent-ab44ddabab049094e`. **Trabalhe só nesse worktree.** Base do lote: `c50e6d1`. Nunca dê push nem squash.

Leia antes, nesta ordem:
1. `E:\PlotS\plan\briefs\brief-cartoes-impressao.md`: escopo, segurança e relatório continuam valendo.
2. `E:\PlotS\plan\briefs\coordenacao-paralela.md`: arquivos que são seus e regras comuns.

## O que muda em relação ao brief original
- **Disco:** compilação em `C:/Users/Kaio Luan/AppData/Local/plots-build/cartoes`, como antes. PDFs de saída e capturas vão para `C:/Users/Kaio Luan/AppData/Local/plots-build/cartoes/saidas`, não para `target/` do worktree. Só `ci.log` e `fail-before.log` ficam em `target/`. Abaixo de 500 MB livres em E:, pare e relate.
- **Base já implementada:** não recomece. Antes de confiar no wip, confira com `git show --stat d7d3474` e pelo código se cada item do escopo existe. Um comando que falhou por falta de espaço pode ter deixado arquivo vazio ou truncado: procure por eles.

## O que falta
1. `npm ci` em `ui/`, rodar as cenas e examinar as 4 capturas que já estavam previstas. Depois, as capturas restantes do brief original.
2. Fluxo de 6 páginas pela janela, com as duas simuladas, conferindo a sequência do plano 19.
3. Envio à fila de PDF com `PLOTS_ARQUIVO_SAIDA` e renderização com PyMuPDF. Confirme que não sobrou trabalho na fila.
4. Contraste de cada par novo, CI completo e autoverificação. `ui/dist` pode estar em modo e2e: rode `npm run build` e confirme 0 ocorrências de `__plotsE2e`.
5. Relate como pendente, sem editar `plan/`, a figura do gesto.

## Desvios a resolver ou justificar no relatório
- `imprimir.indisponivel` está sem uso: remova ou justifique.
- `plots-windows` tem `pub use plots_print` em vez de dependência direta: diga por que, ou corrija.
- `leitura_pdf.rs` repete leitura que já existe: reaproveite se puder, sem mexer em `plots-pdf/`, que é de outro lote.
- Estados novos fora do brief: liste cada um e por que existe.
- Frase de fechar a janela no meio de uma passada: confira no código e no app se ela é verdadeira.

## Relatório
O do brief original, deixando claro o que veio do wip e o que você fez ou verificou agora. Termine com `git status` limpo, em um novo commit wip depois do CI verde.
