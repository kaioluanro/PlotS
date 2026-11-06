import { invoke } from "@tauri-apps/api/core";

/** Cor que o driver aceita para a fila. */
export type CorDaImpressora = "colorida" | "monocromatica";

export interface BandejaDaImpressora {
  valor: number;
  nome: string;
}

export interface ResolucaoDaImpressora {
  x: number;
  y: number;
}

/** O que a pessoa escolheu; `null` deixa o campo como o driver o entregou. */
export interface EscolhasDaImpressora {
  bandeja: number | null;
  resolucao: ResolucaoDaImpressora | null;
  cor: CorDaImpressora | null;
}

/** O que o driver declara para a fila, com os valores que o ajuste atual usa. */
export interface OpcoesDaImpressora {
  bandejas: BandejaDaImpressora[];
  resolucoes: ResolucaoDaImpressora[];
  cor: CorDaImpressora | null;
  atual: EscolhasDaImpressora;
}

export const ESCOLHAS_VAZIAS: EscolhasDaImpressora = {
  bandeja: null,
  resolucao: null,
  cor: null,
};

/** Códigos que o painel sabe explicar; o resto vira falha genérica do driver. */
export type CodigoFalhaDasOpcoes =
  | "fila_sumiu"
  | "opcao_recusada"
  | "nao_abriu"
  | "nao_gravou"
  | "grande";

const CODIGOS: readonly CodigoFalhaDasOpcoes[] = [
  "fila_sumiu",
  "opcao_recusada",
  "nao_abriu",
  "nao_gravou",
  "grande",
];

interface FalhaDasOpcoes {
  codigo: string;
  fila: string;
}

/** A falha vem do backend; o que não tiver a forma esperada conta como recusa do driver. */
export function codigoDaFalhaDasOpcoes(erro: unknown): CodigoFalhaDasOpcoes {
  const codigo = (erro as Partial<FalhaDasOpcoes> | null | undefined)?.codigo;
  return CODIGOS.find((conhecido) => conhecido === codigo) ?? "opcao_recusada";
}

export type DestinoDaFalha = "fechar" | "mostrar";

/** Fila que sumiu fecha o painel; as outras falhas ficam no corpo para a pessoa corrigir. */
export function destinoDaFalha(codigo: CodigoFalhaDasOpcoes): DestinoDaFalha {
  return codigo === "fila_sumiu" ? "fechar" : "mostrar";
}

export function lerOpcoesDaImpressora(fila: string): Promise<OpcoesDaImpressora> {
  return invoke<OpcoesDaImpressora>("opcoes_da_impressora", { fila });
}

export function aplicarOpcoesDaImpressora(
  fila: string,
  escolhas: EscolhasDaImpressora,
): Promise<void> {
  return invoke<void>("aplicar_opcoes_da_impressora", { fila, escolhas });
}

export function mesmaResolucao(
  a: ResolucaoDaImpressora | null,
  b: ResolucaoDaImpressora | null,
): boolean {
  return a === null ? b === null : b !== null && a.x === b.x && a.y === b.y;
}

export function rotuloDaResolucao(resolucao: ResolucaoDaImpressora): string {
  return `${resolucao.x} × ${resolucao.y} dpi`;
}

/**
 * Cores que o driver aceita. Uma impressora que declara cor também imprime em preto; uma que só
 * declara preto não ganha a opção de cor.
 */
export function coresOferecidas(capacidade: CorDaImpressora | null): CorDaImpressora[] {
  if (capacidade === "colorida") return ["colorida", "monocromatica"];
  if (capacidade === "monocromatica") return ["monocromatica"];
  return [];
}

/**
 * O ajuste atual só vira escolha se o driver ainda declarar aquele valor. Um ajuste antigo que
 * aponta para uma opção que sumiu começa em "Padrão da impressora", sem inventar valor.
 */
export function escolhasIniciais(opcoes: OpcoesDaImpressora): EscolhasDaImpressora {
  const { atual } = opcoes;
  const bandeja =
    atual.bandeja !== null && opcoes.bandejas.some((item) => item.valor === atual.bandeja)
      ? atual.bandeja
      : null;
  const resolucao =
    atual.resolucao !== null &&
    opcoes.resolucoes.some((item) => mesmaResolucao(item, atual.resolucao))
      ? atual.resolucao
      : null;
  const cor =
    atual.cor !== null && coresOferecidas(opcoes.cor).includes(atual.cor) ? atual.cor : null;
  return { bandeja, resolucao, cor };
}

/** O painel só mostra o que o driver declarou; sem nada, diz isso e aponta a engrenagem. */
export function temOpcoes(opcoes: OpcoesDaImpressora): boolean {
  return opcoes.bandejas.length > 0 || opcoes.resolucoes.length > 0 || opcoes.cor !== null;
}
