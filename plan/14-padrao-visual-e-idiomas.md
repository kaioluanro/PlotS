# Padrão visual e idiomas

**Estado:** Em andamento · OK 0 · PARTIAL 1 · MISSING 2 · BUG 0 · N/A 0

## Objetivo

Aplicar `estilo-ui-ux.md` em toda a interface, em pt-BR e en.

## Plano de implementação

- [ ] `theme.css` com os tokens, temas escuro e claro
- [ ] Componentes base com CSS Modules, `lucide-react` e Radix
- [ ] Textos via `t()` em pt-BR e en
- [ ] Foco visível, ESC em diálogos, estado com ponto e texto, contraste medido

## Pronto quando

A autoridade de interface aprova as telas nos dois temas.

## Depende de

Nenhuma.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A026 | Tokens da norma de estilo, tema escuro, foco visível, ESC fecha diálogos, sem emoji, `aria-live` para o estado | MISSING |
| PS-A027 | Textos em pt-BR e en via `t()` | PARTIAL |
| PS-A047 | Frontend React + TypeScript, CSS Modules, lucide-react, Radix | MISSING |

## Especificação

#### Padrão visual

A interface segue o `estilo-ui-ux.md`: tema escuro por padrão, acento monocromático, bordas de 1 px como estrutura, corpo de 12 px e dados em monoespaçado.

| Elemento | Especificação | Tokens |
| --- | --- | --- |
| Barra superior | 32 px; ícone 17×17 com raio 5; botão da lateral fantasma 28×28; controles de janela 32×36; fechar com fundo `#e81123` no hover | `--bg-elevated`, borda inferior `--border` |
| Abas de documento | `min-height` 26 px, raio 6, padding `0 8px`, 11 px/500; ativa com barra 2×16 px; "×" em caixa 24×24, revelado no hover e no foco | ativa `--panel-hover`, inativa `--fg-muted` |
| Botão "+" | Fantasma 28×28, ícone Lucide `plus` 14 px | `--fg-muted` → `--fg` |
| Barra lateral | Mesmo fundo do visualizador, separada por 1 px; padding 12 px; cabeçalho 34 px com IMPRESSORAS em caixa alta 10–11 px; contador mono em pílula | `--bg`, `--border`, `--fg-faint` |
| Linha de impressora | `min-height` 29 px; checkbox 16 px com área de clique 24×24; nome 12 px com reticências; seletor com `flex-shrink: 0` | `--border-control`, marcado `--accent` + glifo `--accent-on` |
| Seletor de modo | 24 px dentro da linha, 11 px, seta 10×6; opções "Traseira ↑ face para cima", "Gaveta ↓ face para baixo", "Calibrar…" | `--bg-sunken`, `--border-control` |
| Lote | Campo 32 px com número mono 13 px; resumo mono 11 px ("24 pág · 2 lotes") | `--font-mono`, `--fg-faint` |
| Imprimir frente | Único botão primário da janela; padding `8px 14px`; 13 px/500 | `--accent`, `--accent-on` |
| Cartão de impressão | Card de ação: padding `12px 14px`, raio 8; estado = ponto 8 px + texto; "Imprimir verso" é botão secundário pequeno | `--bg-sunken`, `--status-waiting`, `--status-working`, `--status-offline` |
| Visualizador | Barra de 28 px com página e zoom em mono; área do pdf.js no `--bg` | `--bg`, `--border` |

Regras que mais pesam aqui:

- Estado da impressora sempre com ponto + texto (Imprimindo, Aguardando recolocação, Concluído, Erro), anunciado com `aria-live="polite"`.
- Foco de teclado `outline: 2px solid var(--accent)` em todo controle; ESC fecha diálogos.
- Nada de `--fg-faint` sobre `--panel` ou `--panel-hover`: em hover e seleção o metadado passa a `--fg-muted`.
- Ícones `lucide-react` de 12–14 px; nenhum emoji na interface.
- Começar com os temas `dark` e `light` e o estilo Normal; os outros temas e o estilo Clean entram depois.
- Strings via `t()`, em `pt-BR` e `en` (`zh-CN` é opcional).

Frontend: React + TypeScript com CSS Modules e os tokens da norma de estilo (`theme.css`), ícones `lucide-react` e Radix para diálogos e menus. Nenhuma regra de negócio fica no frontend. No Windows, vale avaliar o SumatraPDF por linha de comando para imprimir PDF sem rasterizar.
