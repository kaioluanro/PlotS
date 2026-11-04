# Janela, barra superior e abas

**Estado:** Em andamento · OK 4 · PARTIAL 0 · MISSING 0 · BUG 0 · N/A 0

## Objetivo

Janela sem menus, com uma barra única e um documento por aba.

## Plano de implementação

- [x] Criar o app Tauri 2 com React e TypeScript
- [x] Barra própria com arrasto da janela e controles
- [x] Abas com estado próprio e confirmação ao fechar com impressão em andamento, na aba e na janela; a janela nunca fica presa se a interface travar
- [x] Atalhos de teclado e arrastar PDF para abrir aba; recarga do WebView2 bloqueada
- [ ] Confirmar no lote de impressão que fechar mantém a fila e perde o verso, como diz o diálogo
- [x] Verificação manual da janela pelo usuário (2026-11-02): aprovada, sem problemas relatados

## Pronto quando

Abrir, trocar e fechar documentos sem terminal, só com teclado ou mouse.

## Depende de

`14-padrao-visual-e-idiomas`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A019 | Janela sem menus; barra superior única (ícone, botão da lateral, abas, "+", controles) | OK (353f54d) |
| PS-A022 | Abas por documento com estado próprio; confirmação ao fechar com impressão em andamento | PARTIAL (353f54d) |
| PS-A023 | Atalhos Ctrl+O, Ctrl+T, Ctrl+W, Ctrl+Tab, Ctrl+B; arrastar PDF abre aba | PARTIAL (353f54d) |
| PS-A079 | Interface Tauri + pdf.js: visualizador, painel de impressoras, cartões com figura, fluxo sem terminal | PARTIAL (353f54d) |

## Especificação

### Interface

A janela não tem barra de menus (Arquivo, Editar…): uma única barra no topo, no estilo do Firefox, reúne ícone, botão da barra lateral, abas, botão "+" e os controles da janela.

&#91;embedded content: janela principal · barra de abas e barra lateral\]

**Barra superior**, da esquerda para a direita:

1. Espaço do ícone do app.
2. Botão da barra lateral (mostra/oculta a lista de impressoras).
3. Abas, uma por documento aberto, cada uma com "×" para fechar.
4. Botão "+" para abrir outro PDF.
5. Minimizar, maximizar e fechar, à direita.

No Tauri isso é uma barra de título própria: `decorations: false` na janela e `data-tauri-drag-region` na área vazia da barra, para arrastar a janela. Atalhos: Ctrl+O ou Ctrl+T abre, Ctrl+W fecha a aba, Ctrl+Tab troca de aba, Ctrl+B alterna a barra lateral. Arrastar um PDF para a janela abre uma aba nova.

Cada aba guarda o próprio estado: impressoras marcadas e seus modos, intervalo de páginas, tamanho do lote e progresso. Fechar uma aba com impressão em andamento pede confirmação.
