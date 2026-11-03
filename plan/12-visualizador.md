# Visualizador de PDF

**Estado:** Concluído · OK 3 · PARTIAL 0 · MISSING 0 · BUG 0 · N/A 0

## Objetivo

Mostrar o documento e as miniaturas na janela; toda a decisão de ordem continua no Rust.

## Plano de implementação

- [x] Integrar `pdfjs-dist` na webview
- [x] Barra com página e zoom
- [x] Miniaturas
- [x] Escolha do intervalo de páginas

## Decisões e limitações

- O documento abre em "ajustar à largura", medido pela maior página, com piso de 2%; 100% equivale a 96 dpi; os degraus vão de 25% a 400%. Com uma página gigante, as menores ficam pequenas: é o custo aceito para não cortar nada.
- Trocar de aba solta o PDF para liberar memória e o relê ao voltar, na mesma página e zoom.
- `pdfjs-dist` 6.5.438. Fontes base-14 não embutidas usam a fonte do sistema; a Liberation do pacote fica de fora (ver `licencas-terceiros.md`).
- JPX e JBIG2 funcionam sem `wasm-unsafe-eval`, porque o Tauri 2.12.5 só manda a CSP em respostas HTML e o wasm roda no worker. **Se uma versão nova do Tauri mandar CSP nos assets, essas imagens quebram:** conferir ao atualizar.
- Cores ICC e CMYK na prévia são aproximadas. Imagens acima de 200 MP não são desenhadas.
- **Pendente:** medir a troca de aba com um PDF de 20 MB e, se passar de 300 ms, manter vivo o documento da última aba; Ctrl+roda; ir à página digitando o número; `--panel-hover` no tema claro; primeiro "+" quase não muda o zoom; foco em "Tentar de novo"; ícones de "−" e "+" em cinza nas cores forçadas do tema escuro; rever os 8 `Foxit` Serif e Fixed, que parecem sem uso com `useSystemFonts`.
- **Memória:** PDF de 425 MB, base de 185 MB, pico de 1,39 GB, 414 MB depois de fechar. O resíduo está na GPU do WebView2, causa não determinada.
- **Conferido no exe de release:** abertura, página e miniatura de um PDF com JPX. **Não verificado:** JBIG2 aritmético (sem codificador para gerar o teste; pelo código usa o mesmo módulo do MMR), instalador, captura de erro de página, escalas de 125% e 150%, leitor de tela real.

- **Intervalo de páginas (`ccb1ebc`):** um trecho contínuo por aba, nas opções da lateral. Vários trechos separados e marcar o trecho nas miniaturas ficaram de fora. A linha "Páginas" nunca cede espaço enquanto há botão de envio. Texto válido deixa o botão ativo na hora, então "10" a caminho de "100" sai como 10 se a pessoa clicar nesse instante; no envio real, a confirmação mostra o trecho.
- **Não verificado no intervalo:** gesto real de arrastar, que só foi exercitado pelo evento interno; "Reimprimir" de um trecho no app, só em teste Rust; papéis mistos no diálogo, só em teste Rust; leitor de tela e cores forçadas reais do Windows.

## Pronto quando

Abrir um PDF mostra páginas e miniaturas, e o intervalo escolhido chega ao planejador.

## Depende de

`09-janela-e-abas`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A013 | Abrir PDF (arrastar, "Abrir com") e escolher o intervalo de páginas | OK |
| PS-A025 | Visualizador pdf.js com miniaturas, página e zoom | OK |
| PS-A039 | App Tauri 2 com webview e pdf.js | OK |

## Especificação

1. Abrir o PDF no PlotS (arrastar, "Abrir com" ou imprimir de outro programa na impressora virtual — Fase 5).
2. Ver as páginas no visualizador e escolher o intervalo.
| Peça | Escolha | Por quê |
| --- | --- | --- |
| Visualização | `pdfjs-dist` no frontend | Leitor do Firefox, miniaturas prontas |
App desktop em **Tauri 2**: backend em Rust e interface numa webview, onde roda o **pdf.js** — o mesmo leitor de PDF do Firefox. O pdf.js só exibe; toda a decisão de ordem fica no Rust.
O pdf.js tem um único papel: mostrar o documento e as miniaturas dentro da janela do PlotS.
