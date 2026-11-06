const PASSO_DA_SETA = 40;

/** Controles que usam as setas/Home/End para o proprio valor, nao para rolar o corpo. */
const CONTROLES_INTERATIVOS =
  'select, input, button, a, textarea, [role="slider"], [role="radio"], [role="listbox"]';

export interface ComAncestral {
  closest(seletor: string): unknown;
}

/** Só o que não é um controle interativo deixa o corpo rolar com as teclas. */
export function deveRolarPeloTeclado(alvo: ComAncestral | null): boolean {
  return alvo === null || alvo.closest(CONTROLES_INTERATIVOS) === null;
}

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

/** Quem tem o foco, para devolver quando o dialogo fechar: so o que e um controle ainda na pagina. */
export interface ComFoco {
  readonly isConnected: boolean;
}

/**
 * Um dialogo que fecha e reabre por causa de outro (o painel, ao pedir Cancelar) guarda a origem da
 * primeira abertura: na reabertura o foco ja caiu no corpo da pagina, e nao e para la que ele volta.
 */
export function origemDoDialogo<T extends ComFoco>(guardada: T | null, ativa: T | null): T | null {
  return guardada !== null && guardada.isConnected ? guardada : ativa;
}
