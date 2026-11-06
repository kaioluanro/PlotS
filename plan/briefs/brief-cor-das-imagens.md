# Brief: cor das imagens impressas

Leia `E:\PlotS\AGENTS.md`, `.claude/team.json`, `.claude/skills/craft-team/SKILL.md` e
`.claude/agents/plots-implementer.md` antes de agir; siga o método do papel. Não faça push nem
squash. Trabalhe no worktree isolado em que você foi iniciado.

**Graphify:** antes do primeiro grep ou leitura de código, `graphify query "<pergunta>"
--graph E:/PlotS/graphify-out/graph.json`. Ao terminar, `graphify update .` no worktree.

Base: `master` `6f08e00`. Build em `%LOCALAPPDATA%\plots-build\cor-imagens\target`, com
`CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, `-j 4`;
`PLOTS_PDFIUM_DIR=%LOCALAPPDATA%\plots-build\pdfium\chromium-8125\bin`; `npm ci` em `ui/`.

**Capturas:** a mudança não tem tela nova. Capture só a saída impressa colorida (PNG do PDF
gerado pela fila) em `saidas/`, mostrando as faixas laranja/amarelo na cor certa. Nada de tela do
Windows nem SendKeys.

Não toque nas filas `PlotSim — IMP1`/`IMP2` nem em processos do usuário. Não use o PDF assinado.
Use `PLOTS_PERFIS`/`PLOTS_HISTORICO` temporários; não altere `%APPDATA%\plots\perfis.toml`.

## 1. Causa já confirmada (não gaste tempo reabrir)

`plots-windows/src/backend.rs`, `rasterizar`: `PdfRenderConfig::new()` do pdfium-render 0.9.5 vem
com `reverse_byte_order = true`; `as_raw_bytes()` devolve **RGBA**. O DIB 32 bpp `BI_RGB` do GDI e
`marcar_ponto_discreto` esperam **BGRA**. Medição com o `pdfium.dll` real, retângulo laranja:

| flags | pixel central | leitura |
|---|---|---|
| 0 | `[0, 128, 255, 255]` | BGRA, laranja certo |
| 0x10 (`FPDF_REVERSE_BYTE_ORDER`) | `[255, 128, 0, 255]` | RGBA, laranja vira azul no GDI |

## 2. Correção

- Em `rasterizar`, acrescente `.set_reverse_byte_order(false)` à configuração, com um comentário
  curto dizendo que o DIB do GDI é BGRA e que a inversão padrão do pdfium-render entregaria RGBA.
- Não mude `set_format(PdfBitmapFormat::BGRA)`, o `BITMAPINFO`, `SetDIBits`, `StretchBlt`,
  `StretchDIBits`, o giro, o tamanho, a posição, o papel, as cópias, a escala, a orientação nem o
  `DEVMODE`.
- Se houver um lugar único que documente o contrato dos pixels, ajuste a frase para “BGRA, de
  cima para baixo”; não renomeie tipos públicos.

## 3. Testes

### 3.1 Cor na rasterização (novo)

Em `plots-windows/src/backend_testes.rs`, crie um PDF de teste com quatro faixas verticais de
25 pontos em uma página de 100×100, a 72 dpi:

- vermelho `1 0 0 rg`;
- laranja `1 0.5 0 rg`;
- amarelo `1 1 0 rg`;
- azul `0 0 1 rg`.

Rasterize com `desenhar()` e confira os bytes BGRA no centro de cada faixa:

| faixa | BGRA esperado |
|---|---|
| vermelho | `[0, 0, 255, 255]` |
| laranja | `[0, 128, 255, 255]` |
| amarelo | `[0, 255, 255, 255]` |
| azul | `[255, 0, 0, 255]` |

O alfa de todas é 255. O teste é `#[ignore = "precisa da pdfium.dll"]`, como os vizinhos.

**Falha anterior obrigatória:** escreva o teste antes da correção, rode com
`PLOTS_PDFIUM_DIR` e registre em `saidas/falha-antes.log` a execução que falha (vermelho/laranja
com R e B trocados). Depois aplique a correção e mostre o mesmo teste passando.

### 3.2 Cor no envio real (`#[ignore]`)

Acrecente um teste que envia a página colorida à fila “Microsoft Print to PDF” usando
`PLOTS_ARQUIVO_SAIDA`, espera `Concluido`, abre o PDF gerado com a pdfium e o rasteriza com
`PdfRenderConfig::new().set_reverse_byte_order(false)`. No bitmap do PDF gerado:

- conte pixels de laranja (`R > 200`, `80 < G < 180`, `B < 60`) e de amarelo
  (`R > 200`, `G > 200`, `B < 60`); ambos precisam existir em quantidade (> 1000 pixels cada);
- conte os pixels trocados (laranja azulado `B > 200`, `80 < G < 180`, `R < 60` e ciano
  `B > 200`, `G > 200`, `R < 60`); ambos precisam ser **zero**;
- registre no log só o nome do arquivo (não o caminho absoluto), a contagem e o resultado;
- apague a pasta `teste-cor-<pid>` no fim, como o teste vizinho.

Rode esse teste manualmente e guarde o log em `saidas/envio-cor.log`; gere também um PNG da
página colorida impressa em `saidas/cor-impressa.png` (use a pdfium para renderizar o PDF gerado).

### 3.3 Guardas existentes

- Os testes de preto/branco/giro e o `uma_pagina_sai_com_conteudo_no_arquivo_da_fila_de_pdf`
  continuam passando.
- Não afrouxe nenhuma asserção para o teste novo passar.

## 4. Final

- CI em um log único `saidas/ci.log`: `cargo fmt --all --check`,
  `cargo test --workspace --locked -j 4`, em `ui/` `npm ci`, `npm test`, `npm run build`,
  conferência de ganchos, terminando em `CI_SUCCESS: cargo test --workspace` e `CI_EXIT 0`.
- Wip commit com árvore limpa; `node_modules`/`dist`/`target` do lote apagados; nenhum
  `perfis.toml`/`historico.toml` do usuário no worktree ou no commit.
- Não mexa em `plan/`, no PlotSim nem no zip.
- Resposta final item a item: causa, correção, falha anterior (caminho e trecho), provas de cor
  (logs, PNG e contagens), linha do CI e o que não foi verificado.