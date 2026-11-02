# Visualizador de PDF

**Estado:** Não iniciado · OK 0 · PARTIAL 0 · MISSING 3 · BUG 0 · N/A 0

## Objetivo

Mostrar o documento e as miniaturas na janela; toda a decisão de ordem continua no Rust.

## Plano de implementação

- [ ] Integrar `pdfjs-dist` na webview
- [ ] Barra com página e zoom
- [ ] Miniaturas e escolha do intervalo de páginas

## Pronto quando

Abrir um PDF mostra páginas e miniaturas, e o intervalo escolhido chega ao planejador.

## Depende de

`09-janela-e-abas`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A013 | Abrir PDF (arrastar, "Abrir com") e escolher o intervalo de páginas | MISSING |
| PS-A025 | Visualizador pdf.js com miniaturas, página e zoom | MISSING |
| PS-A039 | App Tauri 2 com webview e pdf.js | MISSING |

## Especificação

1. Abrir o PDF no PlotS (arrastar, "Abrir com" ou imprimir de outro programa na impressora virtual — Fase 5).
2. Ver as páginas no visualizador e escolher o intervalo.
| Peça | Escolha | Por quê |
| --- | --- | --- |
| Visualização | `pdfjs-dist` no frontend | Leitor do Firefox, miniaturas prontas |
App desktop em **Tauri 2**: backend em Rust e interface numa webview, onde roda o **pdf.js** — o mesmo leitor de PDF do Firefox. O pdf.js só exibe; toda a decisão de ordem fica no Rust.
O pdf.js tem um único papel: mostrar o documento e as miniaturas dentro da janela do PlotS.
