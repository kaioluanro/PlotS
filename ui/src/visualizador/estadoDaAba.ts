import type { Documento } from "../tabs";
import type { Ancora, TamanhoPagina } from "./contas";

/**
 * O que a leitura lembra da aba: onde estava, se o zoom ainda acompanha a largura, e as medidas das
 * paginas ja lidas (num documento misto, o zoom de ajuste depende da pagina mais larga).
 */
export interface EstadoDaAba {
  ancora: Ancora;
  ajustando: boolean;
  zoomPedido: number;
  tamanhos: readonly TamanhoPagina[];
}

// Chave: o objeto da aba. Ele some quando a aba fecha, e o estado vai junto, sem limpeza manual.
const estados = new WeakMap<Documento, EstadoDaAba>();

export function lerEstadoDaAba(documento: Documento): EstadoDaAba | undefined {
  return estados.get(documento);
}

export function guardarEstadoDaAba(documento: Documento, estado: EstadoDaAba): void {
  estados.set(documento, estado);
}

/** Medidas de partida: as guardadas, se servem a este documento; senao, a primeira pagina repetida. */
export function tamanhosDaLeitura(
  lembrado: EstadoDaAba | undefined,
  total: number,
  primeiraPagina: TamanhoPagina,
): readonly TamanhoPagina[] {
  if (lembrado && lembrado.tamanhos.length === total) return lembrado.tamanhos;
  return new Array<TamanhoPagina>(total).fill(primeiraPagina);
}
