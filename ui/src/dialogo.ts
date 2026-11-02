const PASSO_DA_SETA = 40;

export interface Rolavel {
  scrollTop: number;
  readonly clientHeight: number;
  readonly scrollHeight: number;
}

/**
 * O foco fica nos botoes do rodape, fora da area que rola, entao o navegador nao rola sozinho.
 * Devolve se a tecla foi usada; sem nada para rolar, ela segue seu caminho.
 */
export function rolarPeloTeclado(tecla: string, corpo: Rolavel | null): boolean {
  if (corpo === null) return false;
  const maximo = corpo.scrollHeight - corpo.clientHeight;
  if (maximo <= 0) return false;
  const pagina = corpo.clientHeight * 0.9;
  const destinos: Partial<Record<string, number>> = {
    ArrowDown: corpo.scrollTop + PASSO_DA_SETA,
    ArrowUp: corpo.scrollTop - PASSO_DA_SETA,
    PageDown: corpo.scrollTop + pagina,
    PageUp: corpo.scrollTop - pagina,
    Home: 0,
    End: maximo,
  };
  const destino = destinos[tecla];
  if (destino === undefined) return false;
  corpo.scrollTop = Math.min(maximo, Math.max(0, destino));
  return true;
}
