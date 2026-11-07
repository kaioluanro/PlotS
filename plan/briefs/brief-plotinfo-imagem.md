# plotinfo-imagem: abrir e imprimir imagens no PlotS

Trabalhe no worktree isolado `E:\PlotS\.claude\worktrees\agent-plotinfo-imagem`, branch
`worktree-plotinfo-imagem`, criado a partir do `master` que contém este brief. Faça commits wip;
nunca faça push e nunca faça squash.

## Ambiente

- **Build:** `CARGO_TARGET_DIR=%LOCALAPPDATA%\plots-build\plotinfo-imagem\target`,
  `CARGO_INCREMENTAL=0`, `-j 4`. Um comando pesado por vez.
- **PDFium:** `PLOTS_PDFIUM_DIR=%LOCALAPPDATA%\plots-build\pdfium\chromium-8125\bin`.
- **Dependências novas:** `image 0.25.10`, `tiff 0.11.3` e `flate2 1.1.10` já estão no cache local
  do cargo. Se o cargo pedir rede, tente `--offline` e registre no relatório.
- **CI:** depois da última edição, rode o comando de `.claude/team.json` uma vez, no foreground,
  com log em `saidas/ci.log`; o log precisa terminar em
  `CI_SUCCESS: cargo test --workspace` e `CI_EXIT 0`.
- **Falha-antes:** `saidas/falha-antes.log`, com execuções reais e a saída. Classifique cada teste
  novo em: falha-antes registrada, API nova sem execução na base, ou guarda.
- **Saídas:** `saidas/`; capturas com prefixo `NN-`; leia cada captura.
- **App:** build com `npm ci` em `ui/`, `npm run build` em `ui/` e
  `cargo build -p plots-app` com o target acima. Rode o exe com o caminho da imagem como
  argumento (linha de comando), `PLOTS_PERFIS`/`PLOTS_HISTORICO` temporários e
  `PLOTS_PDFIUM_DIR`; espere a janela e capture. Feche só o PID que você abriu.

## Navegação de código

Antes do primeiro grep ou leitura de código:
`graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.
Confirme cada hit no worktree e leia só as linhas localizadas. Ao terminar, `graphify update .`.

Comece por:
- `src-tauri/src/documento.rs` — abertura, validação, contagem e leitura; é o ponto central.
- `src-tauri/src/leitura_pdf.rs` — o caminho que o resumo e a impressão usam.
- `src-tauri/src/main.rs` — comando `abrir_documento` e registro.
- `plots-pdf/src/lib.rs` — `PdfOrigem::carregar` e as dimensões.
- `ui/src/backend.ts` — filtro do seletor e comandos.
- `ui/src/tabs.ts`, `ui/src/erros.ts`, `ui/src/i18n.ts` — tipo do documento, erros e textos.
- `ui/src/components/EmptyState.tsx`/`DocumentArea.tsx` — estado vazio.
- `plots-windows/src/backend_testes.rs` — o teste de impressão na fila de PDF que serve de
  modelo.

## Segurança de processo

- Mate só os seus PIDs, por número exato. Nunca mate todas as instâncias do PlotS nem processos
  do usuário. Não toque nas filas `PlotSim — IMP1`/`IMP2`.
- Se uma ação for negada, pare e reporte; não peça a outra pessoa para fazer.
- Não deixe app rodando.

## Fontes (só estas)

- `AGENTS.md`, `.claude/team.json`, `.claude/skills/craft-team/SKILL.md`,
  `.claude/agents/plots-implementer.md`.
- `plan/30-plotinfo.md` e este brief.
- Os arquivos listados acima.
- Não leia `PlotInfo/` (material bruto do usuário); o plano 30 já traz o recorte aprovado.

## Regras de plano

- Comentários, nomes de teste e mensagens descrevem comportamento com nossas palavras. Nada de
  ids de requisito, sinais de seção, “manual”, “lote” ou “próximo lote”.
- **Self-check, precisa não imprimir nada:**
  `sh .claude/team/selfcheck.sh <base-sha> plots-pdf/src src-tauri/src ui/src`. Rode também nos
  arquivos novos não rastreados.

## Escopo

Primeiro registre no relatório o que o app faz hoje: só aceita `.pdf`; `documento::abrir` recusa
outra extensão; o seletor filtra `pdf`; a prévia e a impressão leem o PDF original.

1. **PDF derivado de imagem.** Em `plots-pdf/src/imagem.rs`, API pública que recebe os bytes, a
   extensão e as opções e devolve os bytes de um PDF com uma página por imagem:
   - PNG, JPEG, BMP, GIF e WebP via `image`; TIFF via `tiff` (todos os quadros);
   - alfa composto sobre branco;
   - página A4 retrato/paisagem conforme a proporção, imagem centralizada e ajustada sem
     distorcer; opção de usar o DPI do arquivo (PNG/JPEG/BMP/TIFF) fica preparada para o painel;
   - imagem RGB embutida como XObject `/DeviceRGB`, 8 bits, comprimida com `FlateDecode` (zlib);
   - limites de 1 GiB de arquivo e 100 MP por quadro, com erro claro;
   - erro próprio sem panic, com mensagens que a interface possa traduzir por código.
2. **Documento com tipo.** Em `src-tauri/src/documento.rs`:
   - `Documento` continua com `caminho` e `nome`, ganha `tipo` (`pdf`/`imagem`);
   - `abrir_para_leitura` devolve o documento e o caminho do conteúdo lido; para PDF é o
     original; para imagem é o PDF derivado;
   - o derivado vai para `%LOCALAPPDATA%\plots\cache\imagens\<hash>.pdf`, com escrita atômica;
     hash do conteúdo + extensão + opções; cache reaproveitado; limpeza best-effort de arquivos
     com mais de 30 dias;
   - `ler_documento`, `contar_paginas` e `leitura_pdf::ler_pdf` passam a ler o conteúdo do
     `abrir_para_leitura`, não o caminho original;
   - `abrir_documento` vira async em `spawn_blocking` para não travar a interface ao decodificar;
   - o nome/caminho/histórico continuam os do arquivo original.
3. **Interface de abertura.**
   - `escolherArquivo` aceita `pdf, png, jpg, jpeg, bmp, gif, tif, tiff, webp` e o nome do filtro
     muda para “PDF e imagens”/“PDF and images”;
   - arrasto e linha de comando já passam pelo `abrir_documento`, que agora aceita imagem;
   - `Documento` da interface ganha `tipo` (opcional para não quebrar testes antigos);
   - `erros.ts` traduz os códigos novos (`nao_suportado`, `imagem_invalida`);
   - estado vazio menciona “PDF ou imagem”;
   - todas as strings novas em pt-BR e en, na chave certa do `i18n.ts`.
4. **Impressão.** O envio continua chamando `leitura_pdf::ler_pdf` com o caminho original; esse
   caminho resolve o PDF derivado. Nenhuma mudança em `plots-core`, nas passadas de `plots-pdf`,
   no backend de `plots-windows` ou no `DEVMODE`.

## Regras

- **Testes:**
  - `plots-pdf`: cada formato vira PDF com a página certa; TIFF de 2 páginas vira 2 páginas;
    alfa sobre branco; limites recusados; PDF derivado abre em `PdfOrigem`; teste da imagem
    colorida registra os bytes/pixels esperados.
  - `src-tauri`: `abrir` de PNG, JPEG, BMP, GIF, WebP e TIFF; extensão desconhecida recusada;
    PDF continua igual; `contar_paginas` de TIFF multipágina; cache reaproveitado; bytes de
    `ler_documento` iguais ao derivado; falha-antes do PNG (antes da mudança, `nao_suportado`).
  - `#[ignore]` de uso real: abrir a imagem colorida, imprimir na fila “Microsoft Print to PDF”
    com `PLOTS_ARQUIVO_SAIDA` (sem diálogo nativo), reabrir o PDF gerado, rasterizar com
    `set_reverse_byte_order(false)` e exigir laranja/amarelo certos e zero pixels trocados.
    Registre o log e um PNG do resultado.
  - UI: filtro novo, `Documento.tipo`, mapeamento de erro, estado vazio. Não afrouxe nenhuma
    asserção.
- **Nunca crashar:** bytes de imagem são hostis; dimensões e contas com `checked_*`; nada de
  índice direto sem checar; limite de pixels antes de decodificar quando possível.
- **Strings:** liste todas as novas, em pt-BR e en.
- **Escopo proibido:** não mexa em `plan/`, `PlotInfo/`, `plots-core`, nas passadas de
  `plots-pdf`, no backend de impressão de `plots-windows`, no PlotSim, no zip, nos perfis do
  usuário ou nas filas dele. Não crie o painel do PlotInfo neste lote.

## Teste de uso (entrada real)

1. Gere um PNG de 800×600 com quatro faixas coloridas em `saidas/`.
2. Abra o PlotS com o caminho do PNG como argumento, perfil e histórico temporários, e espere a
   prévia.
3. Capture a janela com a imagem aberta e o nome da aba; confira que não há erro e que a prévia
   não está invertida.
4. Rode o teste de impressão `#[ignore]` e guarde `saidas/envio-imagem.log` e o PNG do PDF
   gerado.
5. Feche o app pelo PID exato.

## Relatório

- O que o app fazia antes.
- Cada item do escopo: feito, parcial ou pulado, com o porquê.
- Testes, cada um com seu grupo de falha-antes.
- Arquivos alterados e dependências novas.
- Capturas, com o que cada uma mostra.
- Strings novas, em pt-BR e en.
- Linha do CI, conferência de caminhos e self-check vazio.
- Retries e o que não foi verificado.