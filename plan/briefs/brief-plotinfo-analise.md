# plotinfo-analise: tinta do documento com PDFium, cache e cancelamento

Trabalhe no worktree isolado que o coordenador indicar, criado a partir do `master` que contém o
B1 e este brief. Faça commits wip; nunca faça push nem squash.

## Ambiente

- **Build:** `CARGO_TARGET_DIR=%LOCALAPPDATA%\plots-build\plotinfo-analise\target`,
  `CARGO_INCREMENTAL=0`, `-j 4`. Um comando pesado por vez.
- **PDFium:** `PLOTS_PDFIUM_DIR=%LOCALAPPDATA%\plots-build\pdfium\chromium-8125\bin`.
- **CI:** depois da última edição, rode o comando de `.claude/team.json` uma vez, no foreground,
  com log em `saidas/ci.log`; precisa terminar em `CI_SUCCESS: cargo test --workspace` e
  `CI_EXIT 0`. Registre o `CI_EXIT` no próprio log.
- **Falha-antes:** `saidas/falha-antes.log`, com execuções reais. Classifique cada teste novo em:
  falha-antes registrada, API nova sem execução na base, ou guarda.
- **Saídas:** `saidas/`. Não há tela nova; as provas de renderização são logs e, se útil, um PNG
  da página rasterizada.

## Navegação de código

Antes do primeiro grep ou leitura de código:
`graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.
Confirme cada hit no worktree e leia só as linhas localizadas. Ao terminar, `graphify update .`.

Comece por:
- `plots-windows/src/backend.rs` — `biblioteca()` (`OnceLock<Pdfium>`) e o `PdfRenderConfig`
  existente com `BGRA`/`set_reverse_byte_order(false)`.
- `plots-windows/src/lib.rs` — módulos e reexports.
- `plots-windows/src/raster.rs` — contas de pixels/dpi e limites.
- `src-tauri/src/documento.rs`, `src-tauri/src/leitura_pdf.rs` — conteúdo derivado, hash,
  dimensões e `PdfLido`.
- `src-tauri/src/calibracao.rs`/`comandos_trabalho.rs` — modelo de evento e thread com
  `AppHandle::emit`.
- `src-tauri/src/plotinfo.rs` — perfis/configuração do B1.
- `plots-info/src/*` — seleção, cobertura, custo e configuração.
- `src-tauri/src/main.rs` — registro de comandos/estado.

## Segurança de processo

- Mate só os seus PIDs, por número exato. Nunca mate todas as instâncias do PlotS nem processos
  do usuário. Não deixe app rodando.
- Se uma ação for negada, pare e reporte; não peça a outra pessoa para fazer.

## Fontes (só estas)

- `AGENTS.md`, `.claude/team.json`, `.claude/skills/craft-team/SKILL.md`,
  `.claude/agents/plots-implementer.md`.
- `plan/30-plotinfo.md`, `plan/briefs/brief-plotinfo-motor.md` e este brief.
- O código do B1 já integrado.
- Não leia `PlotInfo/`; o plano 30 traz o recorte aprovado.

## Regras de plano

- Comentários, nomes de teste e mensagens descrevem comportamento com nossas palavras. Nada de
  ids de requisito, sinais de seção, “manual”, “lote” ou “próximo lote”.
- **Self-check, precisa não imprimir nada:**
  `sh E:/PlotS/.claude/team/selfcheck.sh <base-sha> plots-windows/src src-tauri/src plots-info/src`.
  Rode também nos arquivos novos não rastreados.

## Escopo

Primeiro registre no relatório o que o app faz hoje: existe o motor de cálculo e os perfis, mas
nada rasteriza páginas para análise, não há cache de cobertura, progresso, cancelamento nem
comando de análise.

1. **Renderizador de análise em `plots-windows`.**
   - Novo módulo público `analise` (Windows) que reaproveita a mesma `biblioteca()` do backend de
     impressão; não inicialize um segundo `Pdfium` (o teste de impressão já sofreu com
     `PdfiumLibraryBindingsAlreadyInitialized`).
   - Carregue o PDF uma vez e renderize as páginas pedidas em ordem, entregando cada buffer BGRA
     por callback e liberando antes da próxima. `PaginaRgb` com número, largura, altura e pixels.
   - `PdfRenderConfig` com `set_format(PdfFormat::BGRA)`, `set_reverse_byte_order(false)`,
     `clear_before_rendering(true)`, branco e tamanho fixo calculado pela página e pelo dpi
     (150/300). Não use qualidade de impressão; a cobertura é a média.
   - Sem panic: página fora do documento, PDF inválido, medidas absurdas, falta de memória e
     limite de pixels viram erro; nada de índice direto sem checagem.
   - Não altere `rasterizar`/`desenhar_paginas` da impressão. O teste de cor BGRA existente é
     guarda e precisa continuar passando.
   - Teste `#[ignore]` com a `pdfium.dll`: PDF de 3 páginas com `C 10/20/30` em página inteira;
     a 72 dpi, o pixel central de cada página tem o C certo; uma página fora do documento é
     recusada.
2. **Análise em `src-tauri/src/analise.rs`.**
   - `iniciar_analise(caminho, paginas: Option<String>, copias: u32, configuracao, pagina_atual:
     Option<usize>) -> Result<String, FalhaAnalise>`:
     resolve o conteúdo com `documento::abrir_para_leitura`, lê dimensões/hash com
     `leitura_pdf::ler_pdf`, interpreta a seleção com `plots-info` e valida as cópias (1..=999).
   - Job em thread própria, com um `Mutex<HashMap<id, Cancelamento>>` no estado Tauri. Eventos:
     `plotinfo://progresso` (feitas/total), `plotinfo://pagina` (resultado da página),
     `plotinfo://fim` (resumo do documento e da seleção) e `plotinfo://falha` (motivo). Ignore
     falha de emissão; cancelamento por `AtomicBool` e `cancelar_analise(id)`.
   - Ordem: página atual, páginas da seleção, restante do documento. Entregue os totais da
     seleção assim que as páginas dela estiverem prontas; os totais do documento podem ser
     parciais até o fim.
   - Cache em memória por `hash do conteúdo + dpi + modo de cor`: guarda a cobertura e as
     dimensões de cada página. `recalcular_analise(id, configuracao, paginas, copias)` usa o
     cache e não rasteriza de novo; trocar perfil, qualidade, preço por mL ou cópias é instantâneo.
   - Use `plots-info` para cobertura, mL, custo, cartucho e página mais cara. Nada de fórmula
     duplicada em `src-tauri`.
   - Neste lote, `ModoDeCor::Cmyk` ainda cai no caminho simples (PDFium) e o resultado diz
     `modo_usado = simples`; o `mutool` entra no B3. `ModoDeCor::Simples` idem.
   - `montar_resultado` separado do job, recebendo as coberturas/dimensões e a configuração, para
     ser testado sem PDFium. Valide páginas/índices e seleção hostis.
   - Registre os comandos no `main.rs` e o estado da análise em `.manage(...)`. Não há UI neste
     lote.
3. **Limites e segurança.**
   - Uma página A4 a 300 dpi pode passar de 8,7 MB em BGRA; processe uma por vez e libere.
   - O job não pode segurar a thread da interface; o comando só cria o job.
   - Cancelar no meio não pode gravar resultado parcial como completo nem deixar thread viva.
4. **Sem UI, sem mutool, sem CSV, sem mudanças na impressão.** O painel, os textos e o botão são
   do lote seguinte.

## Regras

- **Testes:**
  - `montar_resultado` com coberturas sintéticas: seleção `2,5,7,9` com 1 cópia; `1` com 2;
    `5-16` com 12; documento inteiro com 1; cópias 0/1000 recusadas; perfil trocado muda mL/custo
    sem mudar cobertura; cartucho e página mais cara.
  - Cache: analisar uma vez, recalcular com outro perfil/qualidade e conferir que a cobertura não
    muda e o custo muda; trocar dpi/modo invalida a chave.
  - Cancelamento: job cancelado termina em falha/cancelado e não emite `fim` completo.
  - Renderizador `#[ignore]` com PDFium: 3 páginas CMYK/RGB, pixel central, página fora, limite.
  - Guarda: `cargo test --workspace` continua verde; o teste de cor da impressão continua.
- **Nunca crashar:** bytes, páginas, dpi, seleção e cópias são hostis; use `checked_*`, limites e
  validação antes de alocar.
- **Escopo proibido:** não mexa em `plots-core`, nas passadas de `plots-pdf`, no backend de
  impressão de `plots-windows`, no PlotSim, no painel, no zip, nos perfis/filas do usuário. Não
  implemente mutool nem CSV.

## Teste de uso

Sem tela. Rode o teste `#[ignore]` do renderizador com `PLOTS_PDFIUM_DIR` e registre o log; rode
a suíte pura e o CI completo. Se possível, gere um PNG da página renderizada em `saidas/` e leia.

## Relatório

- O que o app fazia antes.
- Cada item do escopo: feito, parcial ou pulado, com o porquê.
- Testes, cada um com seu grupo de falha-antes.
- Arquivos alterados e dependências novas.
- Linha do CI, conferência de caminhos e self-check vazio.
- Retries e o que não foi verificado.