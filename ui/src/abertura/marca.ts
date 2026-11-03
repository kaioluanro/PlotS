/**
 * Marca provisoria da tela de abertura: as duas folhas do icone atual, desenhadas em SVG, com o
 * nome em texto ao lado. E um substituto: a imagem definitiva (PNG de 960 x 600 px, para telas de
 * alta resolucao) entra no lugar quando existir. Nenhum arquivo binario e usado por enquanto.
 */
export const MARCA_PROVISORIA = `<svg class="abertura-folhas" viewBox="0 0 64 64" width="72" height="72" aria-hidden="true" focusable="false"><rect x="11" y="21" width="28" height="32" rx="2" fill="var(--fg-faint)"/><rect x="22" y="9" width="34" height="38" rx="4" fill="var(--bg)"/><rect x="25" y="12" width="28" height="32" rx="2" fill="var(--fg)"/></svg>`;
