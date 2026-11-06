# Cor das imagens impressas

**Estado:** Em andamento

## Objetivo

Fazer a imagem sair com as cores certas: o que é laranja ou amarelo no PDF continua laranja ou
amarelo no papel, sem virar azul.

## Relatório da investigação

### Sintoma

Ao imprimir uma imagem, todos os tons de laranja e amarelo saíram em tons de azul. É a assinatura
de uma troca de canais **R↔B**: um laranja `RGB(255, 128, 0)` lido como BGR vira
`azul-esverdeado`, e um amarelo `RGB(255, 255, 0)` vira ciano.

### O que foi medido

O `pdfium.dll` foi chamado direto (ctypes), com um PDF de teste com um retângulo laranja
`1 0.5 0 rg`, em `FPDFBitmap_BGRA`:

| Flags de renderização | Bytes do pixel central | Como o GDI lê (DIB 32 bpp `BI_RGB`) |
|---|---|---|
| sem flag | `[0, 128, 255, 255]` — BGRA | laranja correto |
| `FPDF_REVERSE_BYTE_ORDER` (0x10) | `[255, 128, 0, 255]` — RGBA | azul (R e B trocados) |

### Causa

`plots-windows/src/backend.rs`, em `rasterizar`:

```rust
let configuracao = PdfRenderConfig::new()
    ...
    .set_format(PdfBitmapFormat::BGRA)
```

No `pdfium-render 0.9.5`, `PdfRenderConfig::new()` nasce com
`do_set_flag_reverse_byte_order: true`. A documentação da própria caixa diz: “The default is
`true`, so that Pdfium returns pixel data as four-channel RGBA rather than its default of
four-channel BGRA.” Ou seja, `set_format(PdfBitmapFormat::BGRA)` escolhe o bitmap, mas a flag de
inversão troca R e B na renderização, e `as_raw_bytes()` devolve **RGBA**.

O resto do caminho trata os bytes como **BGRA**:

- `marcar_ponto_discreto` escreve `[0, 0, 0, 255]` (preto, indiferente à ordem);
- `DocumentoAberto::desenhar_pagina` monta `BITMAPINFO` com `biBitCount = 32`,
  `biCompression = BI_RGB`, e um DIB de 32 bpp é lido pelo GDI na ordem **B, G, R, X**;
- `SetDIBits`/`StretchDIBits` recebem esses bytes sem conversão.

Resultado: o GDI lê o R do PDF como B e o B como R. O bug é anterior à alpha.5 — o mesmo
`PdfRenderConfig` já era usado com `StretchDIBits` —, mas só aparece com conteúdo colorido; os
testes existentes usam `0 g` (preto), que não distingue os canais.

### Correção

Em `rasterizar`, desligar a inversão para os bytes ficarem BGRA, que é o formato que o DIB e o
resto do código já declaram:

```rust
.set_reverse_byte_order(false)
```

Nada muda no GDI, no tamanho, no giro, no papel nem na marca da página em branco.

### Achado relacionado (PlotSim, fora do envio)

O mesmo `PdfRenderConfig::new()` é usado em `PlotSim/app/src/desenho.rs` e em
`PlotSim/plotsim-core/src/leitor.rs`:

- `desenho.rs` monta um BMP de 32 bpp com os bytes crus; o BMP espera BGRA, então o “Ver o outro
  lado” de uma página colorida também mostra R e B trocados.
- `leitor.rs` usa só o canal verde em `cinza_de_bgra`, então a leitura da marca não muda de
  resultado; o nome da função é que fica impreciso.

O produto PlotSim não entra neste lote; fica registrado para um lote próprio, para a conferência
da página recebida não contradizer a cor impressa.

## Comportamento

- Imagem colorida no PDF sai com as cores do PDF, inclusive laranja, amarelo, pele e vermelho.
- Preto, branco e cinza continuam iguais.
- O caminho de bitmap compatível (`SetDIBits` + `StretchBlt`) e a reserva (`StretchDIBits`)
  recebem o mesmo BGRA correto.
- Nenhuma mudança de tamanho, posição, resolução, papel, face ou `DEVMODE`.

## Testes

- Novo PDF de teste com quatro faixas: vermelho, laranja, amarelo e azul. Rasterizar a 72 dpi e
  conferir os bytes BGRA de cada faixa. Antes da correção, vermelho e laranja saem com R e B
  trocados; o teste tem de falhar.
- Teste de uso real (`#[ignore]`, fila "Microsoft Print to PDF"): enviar a página colorida,
  abrir o PDF gerado e conferir que o pixel laranja continua laranja.
- Testes existentes de preto/branco/giro continuam passando.
- CI completo do workspace e da UI.

## Depende de

`08-impressao-windows`, `27-impressao-pdfjs`.

## Pronto quando

Um PDF com laranja e amarelo sai com laranja e amarelo, e o teste de cor falha se a inversão de
bytes voltar ao padrão do pdfium-render.

## Resultado do lote

- Causa medida: o `pdfium.dll` devolve `[0, 128, 255, 255]` (BGRA) sem a flag
  `FPDF_REVERSE_BYTE_ORDER` e `[255, 128, 0, 255]` (RGBA) com ela. O `PdfRenderConfig::new()`
  do pdfium-render 0.9.5 liga a flag por padrão, e `as_raw_bytes()` devolvia RGBA para um DIB
  que o GDI lê como BGRA.
- Correção: `.set_reverse_byte_order(false)` em `plots-windows/src/backend.rs` (`rasterizar`);
  nenhum outro ponto do envio do PlotS usa `as_raw_bytes`.
- Testes: `as_faixas_de_cor_saem_com_os_bytes_bgra_certos` (vermelho, laranja, amarelo e azul,
  bytes BGRA exatos, alfa 255) e o teste de fila real
  `a_pagina_colorida_chega_ao_pdf_da_fila_com_as_cores_certas` (laranja 124616, amarelo 124616,
  laranja azulado 0, ciano 0; PNG `cor-impressa.png`). Falha anterior em `falha-antes.log`
  (`left: [255, 0, 0, 255]`, `right: [0, 0, 255, 255]`).
- Evidência: commit `6a972c6` (integrado em `4d09c34`); CI em
  `%LOCALAPPDATA%\plots-build\integracao-alpha6\saidas\ci.log`; capturas e logs em
  `%LOCALAPPDATA%\plots-build\cor-imagens\saidas` (worktree).
- Falta: impressão física em impressora real; a prova é a fila “Microsoft Print to PDF”. O
  mesmo defeito latente no PlotSim (`PlotSim/app/src/desenho.rs`) foi registrado no plano, mas
  não alterado neste lote.