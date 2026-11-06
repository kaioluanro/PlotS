# Brief: faces corretas e desenho GDI compatível

Leia `E:\PlotS\AGENTS.md`, `.claude/team.json`, `.claude/skills/craft-team/SKILL.md` e
`.claude/agents/plots-implementer.md` antes de agir; siga o método do papel. Não faça push nem
squash. Trabalhe no worktree isolado em que você foi iniciado.

**Graphify:** antes do primeiro grep ou leitura de código, `graphify query "<pergunta>"
--graph E:/PlotS/graphify-out/graph.json`. Ao terminar, `graphify update .` no worktree.

Base: `master` (commit dos planos, depois de `e130777`). Build em
`%LOCALAPPDATA%\plots-build\impressao-pdfjs\target`, com `CARGO_TARGET_DIR`,
`CARGO_INCREMENTAL=0`, `-j 4`; `PLOTS_PDFIUM_DIR=%LOCALAPPDATA%\plots-build\pdfium\chromium-8125\bin`;
`npm ci` em `ui/`. CDP na porta **9374**.

**Capturas:** a mudança de face altera o texto dos cartões (páginas da frente/verso). Capture a
janela com a lista e o cartão de um documento de 1 página e de 3 páginas, nos temas escuro e
cinza, em `saidas/`. Não use tela do Windows nem SendKeys. Não toque nas filas
`PlotSim - IMP1`/`IMP2` nem em processos do usuário. Não use o PDF assinado.

## Contexto

O caminho do pdf.js está em `plan/research/pdfjs-printing.md`: ele rasteriza cada página em
150 dpi e deixa o navegador/OS compor; não entrega um DIB de 600 dpi ao driver. O PlotS usa GDI
nativo. O relato do usuário: toda impressora puxa a folha e devolve em branco. A causa
confirmada no código é a opção "Frente" mandar `passada1`, que são as páginas **pares** (o verso
físico); em documento de 1 página, `passada1` é a face em branco.

## 1. Faces isoladas

Em `plots-core`/`plots-print`, crie funções puras que devolvem as páginas de uma face para envio
isolado, sem quebrar o `Lote`:

- `frentes_do_lote(&Lote) -> Vec<Pagina>`: páginas ímpares, na ordem de `passada1` (o driver
  ejeta a primeira passada nessa ordem), com `girar_180 = false`.
- `versos_do_lote(&Lote) -> Vec<Pagina>`: páginas pares + face em branco final, exatamente
  `lote.passada1` (ordem e giro já aplicados).

Derive a ordem de `lote.paginas` (pares frente/verso na ordem das folhas) e de `lote.passada1`
(que é a sequência dos versos, possivelmente invertida pelo perfil). Se `passada1` casar com os
versos invertidos, inverta as frentes também; se não casar nem direta nem invertida, use
`passada2` sem o giro como reserva. Não compare por identidade de página sem a cópia; use a
estrutura das folhas.

No `Job` (`plots-print/src/job.rs`):
- `enviar_frente` com `Alcance::SoFrentes` envia `frentes_do_lote`.
- `enviar_verso` com `Alcance::SoVersos` envia `versos_do_lote` e passa as frentes em
  `maco_de_fora` (é o maço que já saiu).
- `Alcance::FrenteEVerso` continua enviando `passada1`/`passada2` como hoje.
- `Lote` com `passada2` vazio ou `SoFrentes` não pode pedir verso: depois da frente, o lote
  termina como concluído (sem `AguardandoRecolocacao`). Documento de 1 página: `Frente` termina
  em `FrentesProntas`; `Verso` não é oferecido.

No `src-tauri/src/trabalhos.rs`, a `sequencia` do cartão (`frente`/`verso`) passa a usar
`frentes_do_lote`/`versos_do_lote`, para a tela dizer as páginas certas. O `planejar_pedido` e o
`Lote.paginas` não mudam.

A simulação (`plots-print`/`plotsim`) precisa fechar o maço de uma página e de três páginas; o
`maco_de_fora` do verso usa as frentes.

## 2. Desenho GDI

Em `plots-windows/src/spooler.rs` (`DocumentoAberto::desenhar_pagina`):
- Crie um DC de memória compatível (`CreateCompatibleDC`), um bitmap compatível
  (`CreateCompatibleBitmap`) do tamanho do plano, selecione-o, copie os pixels BGRA com
  `SetDIBits` (DIB com `biHeight` negativo, 32 bpp) e desenhe com `StretchBlt` no DC da
  impressora em `HALFTONE`, com `SetStretchBltMode`.
- Restaure o bitmap anterior e apague bitmap/DC de memória em todos os caminhos, inclusive erro.
- Se qualquer passo do caminho de memória falhar, use o `StretchDIBits` atual como fallback.
- `aceita_bitmap` não pode mais recusar o envio antes de tentar: `RC_STRETCHDIB` só é exigido
  pelo fallback. Se ambos falharem, devolva `SemSuporteABitmap` ou o erro do sistema.
- Não mude o tamanho físico, a posição, o giro, o papel, as cópias, a escala, a orientação nem
  o `DEVMODE`. Não mexa na biblioteca de rasterização.

## 3. Testes

- Falha anterior em `saidas/falha-antes.log` para os testes novos de face (uma página, três
  páginas, duas páginas, ordem invertida do perfil). Rode antes da implementação e registre as
  falhas; se a API é nova, diga "API nova" e registre a execução que compila e falha.
- Testes de `plots-print`/`plots-core` para `frentes_do_lote`/`versos_do_lote`, incluindo perfil
  com `passada1_inversa` e `verso_girar_180`.
- `src-tauri/src/trabalhos_testes.rs`: documento de 1 página com `SoFrentes` termina sem verso
  e com a página 1; de 3 páginas, frente `[1,3]` e verso `[2, branco]`.
- `plots-windows`: mantenha os testes existentes; acrescente um teste que envie uma página real
  à fila "Microsoft Print to PDF" com `PLOTS_ARQUIVO_SAIDA` e confira que o PDF gerado tem uma
  página e conteúdo (não só tamanho > 0). Use a pdfium ou o lopdf já disponível; se precisar de
  PyMuPDF, use o Python do ambiente e registre o comando no log. Esse teste é `#[ignore]`.
- Uso real: app de desenvolvimento com `ui/dist`, fila "Microsoft Print to PDF", PDF de 1
  página; clique em Imprimir, confira que o arquivo gerado tem a página 1 com conteúdo e que o
  cartão não oferece verso. Repita com 3 páginas e confira os números nas duas faces. Capture as
  telas em `saidas/`.

## 4. Final

- CI em um log único `saidas/ci.log`: `cargo fmt --all --check`,
  `cargo test --workspace --locked -j 4`, em `ui/` `npm ci`, `npm test`, `npm run build`,
  conferência de ganchos, terminando em `CI_SUCCESS: cargo test --workspace` e `CI_EXIT 0`.
- Wip commit com árvore limpa; `node_modules`/`dist` e `target` do lote apagados; nenhum
  `perfis.toml` do usuário no worktree ou no commit.
- Não mexa em `plan/`, no PlotSim nem no zip.
- Resposta final item a item: causa, falha prévia (caminho), provas de uso (arquivos e capturas),
  linha do CI e o que não foi verificado.
