# plotinfo-mutool: CMYK nativo e CSV

Trabalhe no worktree isolado que o coordenador indicar, criado a partir do `master` que contém o
B2 e este brief. Faça commits wip; nunca faça push nem squash.

## Ambiente

- **Build:** `CARGO_TARGET_DIR=%LOCALAPPDATA%\plots-build\plotinfo-mutool\target`,
  `CARGO_INCREMENTAL=0`, `-j 4`.
- **PDFium:** `PLOTS_PDFIUM_DIR=%LOCALAPPDATA%\plots-build\pdfium\chromium-8125\bin`.
- **mutool de teste:** `PLOTS_MUTOOL=C:\Users\Kaio Luan\AppData\Local\plots-build\plotinfo-imagem-deps\extraido\mupdf-1.28.5-windows\mutool.exe` (já baixado e conferido; SHA-256 do zip `13476A53…`).
- **CI:** log em `saidas/ci.log`; precisa terminar em `CI_SUCCESS: cargo test --workspace` e
  `CI_EXIT 0`, com todas as linhas no worktree.
- **Falha-antes:** `saidas/falha-antes.log`, com execuções reais.
- **Saídas:** `saidas/`; guarde o log do mutool e, se útil, os PAMs/CSV de prova.

## Navegação de código

Antes do primeiro grep/leitura:
`graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.
Confirme no worktree. Ao terminar, `graphify update .`.

Comece por:
- `src-tauri/src/analise.rs` (B2), `analise_testes.rs`, `main.rs`;
- `plots-windows/src/analise.rs`, `backend.rs` (`biblioteca`);
- `plots-info/src/cobertura.rs` (`cobertura_de_pam`), `custo.rs`, `selecao.rs`;
- `plots-pdf/src/imagem.rs` (o PDF derivado é o que o mutool lê);
- `plan/30-plotinfo.md`, briefs do B1/B2.

## Segurança de processo

- Mate só os seus PIDs, por número exato. Nunca mate todas as instâncias do PlotS nem processos do
  usuário. O `mutool` que você abrir é seu; mate-o pelo PID exato.
- Se uma ação for negada, pare e reporte; não peça a outra pessoa para fazer.
- Não deixe app, `mutool` ou thread rodando.

## Fontes

- `AGENTS.md`, `.claude/team.json`, `craft-team`, `plots-implementer`.
- `plan/30-plotinfo.md`, briefs B1/B2 e este.
- Não leia `PlotInfo/`.

## Regras de plano

- Comentários/testes/mensagens com nossas palavras; sem ids, “manual”, “lote”, “batch”.
- **Self-check, precisa não imprimir nada:** `sh E:/PlotS/.claude/team/selfcheck.sh <base> src-tauri/src plots-info/src plots-windows/src`.

## Escopo

1. **Caminho do `mutool`.**
   - Procure, nesta ordem: `PLOTS_MUTOOL` (arquivo exe), `mutool\mutool.exe` ao lado do
     `PlotS.exe`, `mutool.exe` ao lado do `PlotS.exe`. Sem binário, o modo CMYK cai no PDFium e o
     resultado diz `modo_usado = simples`, sem erro fatal.
   - Não recuse um `mutool` modificado; se conferir hash, é só aviso (a AGPL permite alteração).
2. **Execução CMYK.**
   - Use o PDF derivado já resolvido por `documento::abrir_para_leitura`. Crie um diretório
     temporário exclusivo e rode:
     `mutool draw -q -c cmyk -r <dpi> -o <dir>\p-%04d.pam <pdf> <páginas>`.
   - As páginas são a seleção analisada; o nome do PAM é o número original da página (já
     verificado com `2-3` → `p-0002.pam`/`p-0003.pam`).
   - `CREATE_NO_WINDOW` no Windows; timeout (ex.: 120 s para o lote); em cancelamento, mate o
     processo pelo PID e limpe o diretório. Um processo por lote é aceitável nesta versão; não
     complique o paralelismo se ele puser a correção em risco.
   - Leia cada PAM e apague em seguida; o parser é o `cobertura_de_pam` do `plots-info`. Se um PAM
     faltar/estiver inválido, caia no PDFium para as páginas restantes ou devolva erro claro.
   - O resultado do modo CMYK deve dizer `modo_usado = cmyk`; o fallback, `simples`.
3. **Cache e resultado.**
   - A chave do cache do B2 já inclui o modo; confirme que CMYK e simples não compartilham
     cobertura. `recalcular_analise` continua instantâneo.
   - O `mutool` não pode rodar na thread da interface; reaproveite a thread do job do B2.
4. **CSV.**
   - Comando `exportar_csv(id, caminho)` sobre o resultado em cache: uma linha por página da
     seleção (`numero;largura_mm;altura_mm;ciano_pct;magenta_pct;amarelo_pct;preto_pct;ciano_ml;magenta_ml;amarelo_ml;preto_ml;total_ml;custo`) e uma linha de total. Separador `;`, decimal
     ponto, UTF-8. Recuse caminho inválido sem apagar nada; gravação atômica não é necessária, mas
     não deixe arquivo parcial em caso de erro.
   - Registre o comando no `main.rs`.
5. **Sem UI.** O painel é o lote seguinte. Sem CSV na interface ainda; só o comando.

## Regras

- **Testes:**
  - PDF de 3 páginas com `C 10/20/30` em página inteira, gerado no próprio teste com `lopdf` ou
    um escritor mínimo. Rodar `mutool` real (`#[ignore]`, `PLOTS_MUTOOL`): C exato 10/20/30,
    páginas e nomes certos; `modo_usado = cmyk`.
  - Sem `PLOTS_MUTOOL`: fallback, `modo_usado = simples`, números coerentes.
  - Cancelamento: mate o processo, não emita `fim` completo, não deixe temporário.
  - CSV: cabeçalho, uma linha por página, total, seleção `2,5,7,9`, cópias.
  - Guarda: o CI completo e o teste de cor da impressão continuam verdes.
- **Nunca crashar:** caminho do `mutool`, PDF, páginas, dpi, PAM e CSV são hostis; `checked_*`,
  limites, timeout, sem panic.
- **Escopo proibido:** não mexa em `plots-core`, nas passadas de `plots-pdf`, no backend de
  impressão, no PlotSim, no painel, no zip, nos perfis/filas do usuário. Não mexa na UI.
- **Licenças:** o binário não entra no repositório; o lote D empacota `mutool.exe` em
  `PlotS\mutool\` com AGPL/`MUPDF-SOURCE.txt`. Aqui só o código e o teste.

## Teste de uso

Sem tela. Rode o teste `#[ignore]` com o `mutool` real e registre o log; rode o fallback sem a
variável; rode o CSV e leia o conteúdo.

## Relatório

- O que o app fazia antes.
- Cada item do escopo.
- Testes e grupos de falha-antes.
- Arquivos/dependências.
- Linha do CI, caminhos, self-check.
- Retries e o que não foi verificado.