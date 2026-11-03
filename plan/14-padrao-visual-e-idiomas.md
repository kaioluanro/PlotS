# Padrão visual e idiomas

**Estado:** Em andamento · OK 2 · PARTIAL 1 · MISSING 0 · BUG 0 · N/A 0

## Objetivo

Aplicar `estilo-ui-ux.md` em toda a interface, em pt-BR e en.

## Plano de implementação

- [x] `theme.css` com os tokens, temas escuro e claro, e Inter embutida
- [x] Componentes base com CSS Modules, `lucide-react` e Radix
- [x] Textos via `t()` em pt-BR e en
- [x] Foco visível, ESC em diálogos e contraste medido
- [ ] Estado da impressora com ponto e texto, no lote da lateral
- [ ] Escala real de 125% e 150% e a marca definitiva

## Decisões

- O tema segue o sistema, com escuro como reserva.
- O × da aba tem 12 px em caixa de 24×24, como exceção à §8.5 de `estilo-ui-ux.md`. O ícone do estado vazio tem 18 px.
- O nome da impressora vai até 2 linhas, com `line-clamp: 2`, `overflow-wrap: anywhere` e `title`, como exceção à §6.5 e à §8.4 de `estilo-ui-ux.md`: errar a impressora desperdiça papel, e nomes de fila costumam diferir só no fim. O estado também vai até 2 linhas, com `title`.
- "Calibrar…" usa padding `2px 8px` em 11 px, como exceção ao `4px 10px` do botão pequeno da norma: o botão fica na primeira linha, ao lado do nome, em uma lateral de 240 px.

## Pendente

Em movimento reduzido, a regra global de `theme.css` também para `.girando`, e a norma manda o indicador continuar girando. Restaurar a duração de 900 ms em `.girando`.

## Pronto quando

A autoridade de interface aprova as telas nos dois temas.

## Depende de

Nenhuma.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A026 | Tokens da norma de estilo, tema escuro, foco visível, ESC fecha diálogos, sem emoji, `aria-live` para o estado | PARTIAL (f6e3433) |
| PS-A027 | Textos em pt-BR e en via `t()` | OK (353f54d) |
| PS-A047 | Frontend React + TypeScript, CSS Modules, lucide-react, Radix | OK (353f54d) |

## Especificação

#### Padrão visual

A interface segue o `estilo-ui-ux.md`: tema escuro por padrão, acento monocromático, bordas de 1 px como estrutura, corpo de 12 px e dados em monoespaçado.

| Elemento | Especificação | Tokens |
| --- | --- | --- |
| Barra superior | 32 px; ícone 17×17 com raio 5; botão da lateral fantasma 28×28; controles de janela 32×36; fechar com fundo `#e81123` no hover | `--bg-elevated`, borda inferior `--border` |
| Abas de documento | `min-height` 26 px, raio 6, padding `0 8px`, 11 px/500; ativa com barra 2×16 px; "×" em caixa 24×24, revelado no hover e no foco | ativa `--panel-hover`, inativa `--fg-muted` |
| Botão "+" | Fantasma 28×28, ícone Lucide `plus` 14 px | `--fg-muted` → `--fg` |
| Barra lateral | Mesmo fundo do visualizador, separada por 1 px; padding 12 px; cabeçalho 34 px com IMPRESSORAS em caixa alta 10–11 px; contador mono em pílula | `--bg`, `--border`, `--fg-faint` |
| Linha de impressora | `min-height` 29 px, `padding-left` 4 px; checkbox 16 px com área de clique 24×24; duas fileiras independentes, `nome \| seletor` e `estado \| face`, com gap de 6 px; nome 12 px com até 2 linhas e `title`; estado 10 px com até 2 linhas e `title`; seletor na primeira fileira, à direita se o nome couber em 1 linha ao lado dele; se não couber, desce, alinhado em x=38 | `--border-control`, marcado `--accent` + glifo `--accent-on` |
| Seletor de modelo | Largura fixa de 108 px em todos os valores, sem reticências; 24 px de altura e raio de 4 px; 11 px/500; padding `0 20px 0 8px` (o rótulo mais largo, "Personalizado", mede 74,5 px e cabem 78 px); seta 10 px. Gravação que passa de 400 ms: a seta dá lugar a `LoaderCircle` de 12 px girando, em `--fg-muted` (`ButtonText` em cores forçadas), sem mudar o tamanho do seletor; `title` e um `role="status"` oculto dizem "Gravando modelo…" / "Saving model…", e `aria-busy` continua. Valores "Jato de tinta", "Laser", "Sem perfil", "Personalizado" / "Inkjet", "Laser", "No profile", "Custom"; o `title` mantém a palavra perfil. Na lista, depois de um `hr` de 1 px com margem de 4 px, "Calibrar…"; a opção marcada tem o check do sistema, e a lista entra só com opacidade, em 120 ms com `--ease-out-fluid`. **Exceção:** 24 px e raio de 4 px ficam abaixo do padrão de campo (32 px e raio de 8 px), porque o controle fica dentro de uma linha de 29 px; a área útil continua sendo a caixa inteira. **Exceção aceita:** em estado longo, a face desce para outra linha, e a altura da linha pode mudar entre um valor sem modelo e um modelo, sem mover o seletor | `--bg-sunken`, `--border-control`; divisor `--border`, `CanvasText` em cores forçadas; item ativo `--panel-hover` |
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
