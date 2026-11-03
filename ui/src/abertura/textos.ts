import { idiomaAtual, traduzir, type Chave, type Idioma } from "../i18n";

export type Etapa = "impressoras" | "documento" | "componente_impressao" | "pronto";

export interface Andamento {
  etapa: Etapa;
  concluidas: number;
  total: number;
}

export type CodigoAviso = "filas" | "perfis" | "documento" | "componente_impressao" | "limite";

export interface AvisoAbertura {
  codigo: CodigoAviso;
  detalhe?: string;
}

/** A partir daqui, a mesma etapa mostra os segundos decorridos. */
const SEGUNDOS_ATE_MOSTRAR_MS = 2000;

const textoDaEtapa: Record<Etapa, Chave> = {
  impressoras: "abertura.etapa.impressoras",
  documento: "abertura.etapa.documento",
  componente_impressao: "abertura.etapa.componente",
  pronto: "abertura.etapa.pronto",
};

const textoDoLimite: Record<Etapa, Chave> = {
  impressoras: "abertura.limite.impressoras",
  documento: "abertura.limite.documento",
  componente_impressao: "abertura.limite.componente",
  // O prazo so estoura com a carga incompleta; a etapa final nunca chega aqui.
  pronto: "abertura.limite.impressoras",
};

/** Avisos que so o toast mostra. Filas e perfis ficam na barra lateral; o documento, no banner. */
export type AvisoDoToast = AvisoAbertura & { codigo: "limite" | "componente_impressao" };

export function avisosDoToast(avisos: readonly AvisoAbertura[]): AvisoDoToast[] {
  return avisos.filter(
    (aviso): aviso is AvisoDoToast =>
      aviso.codigo === "limite" || aviso.codigo === "componente_impressao",
  );
}

function etapaConhecida(valor: string | undefined): Etapa {
  return valor !== undefined && valor in textoDaEtapa ? (valor as Etapa) : "impressoras";
}

/** Parte do trabalho ja feita, de 0 a 1. Valores fora da faixa ou invalidos nao quebram a barra. */
export function fracaoDoProgresso(andamento: Pick<Andamento, "concluidas" | "total">): number {
  const { concluidas, total } = andamento;
  if (!Number.isFinite(concluidas) || !Number.isFinite(total) || total <= 0) return 0;
  return Math.min(1, Math.max(0, concluidas / total));
}

/** Segundos na mesma etapa, so depois de 2 s; antes disso, ou com valor invalido, nada. */
export function textoDoTempo(decorridoMs: number): string | null {
  if (!Number.isFinite(decorridoMs) || decorridoMs < SEGUNDOS_ATE_MOSTRAR_MS) return null;
  return `${Math.floor(decorridoMs / 1000)} s`;
}

export function linhaDeEstado(etapa: string | undefined, idioma: Idioma = idiomaAtual): string {
  return traduzir(idioma, textoDaEtapa[etapaConhecida(etapa)]);
}

export function mensagemDoAviso(aviso: AvisoDoToast, idioma: Idioma = idiomaAtual): string {
  if (aviso.codigo === "limite") {
    return traduzir(idioma, textoDoLimite[etapaConhecida(aviso.detalhe)]);
  }
  return traduzir(idioma, "abertura.aviso.componente");
}
