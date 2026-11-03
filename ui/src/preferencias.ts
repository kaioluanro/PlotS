import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ConteudoConfirmacao } from "./confirmacao";
import { traduzir, idiomaAtual, type Chave, type Idioma } from "./i18n";
import type { Impressora } from "./impressoras";

export type FimDaJanela = "salvo" | "cancelado" | "ja_aberta";

/** Por que o ajuste da impressora nao foi usado ou nao foi salvo. */
export type CodigoAvisoDoAjuste =
  | "fila_sumiu"
  | "nao_abriu"
  | "nao_gravou"
  | "grande"
  | "recusado"
  | "corrompido";

export interface EstadoDasPreferencias {
  salvas: string[];
  ignoradas: string[];
  /** Filas cujo ajuste diz duas faces no campo publico; o envio continua em uma face so. */
  duasFaces: string[];
}

interface FalhaPreferencias {
  codigo: CodigoAvisoDoAjuste;
  fila: string;
}

/** Abre a janela do proprio driver e responde quando ela fechar. */
export function abrirPreferencias(fila: string): Promise<FimDaJanela> {
  return invoke<FimDaJanela>("abrir_preferencias", { fila });
}

export function lerEstadoDasPreferencias(): Promise<EstadoDasPreferencias> {
  return invoke<EstadoDasPreferencias>("estado_das_preferencias");
}

export function voltarAoPadraoDaFila(fila: string): Promise<void> {
  return invoke<void>("voltar_ao_padrao_da_fila", { fila });
}

/** O driver nao aceitou o ajuste de uma fila, e o envio seguiu com o padrao dela. */
export function ouvirRecusaDoAjuste(aoReceber: (fila: string) => void): Promise<() => void> {
  return listen<string>("preferencias-recusada", (evento) => aoReceber(evento.payload));
}

/** So fila real tem driver: uma simulada nao tem o que abrir. */
export function temPreferencias(impressora: Impressora): boolean {
  return impressora.tipo === "fila";
}

const CODIGOS: readonly CodigoAvisoDoAjuste[] = [
  "fila_sumiu",
  "nao_abriu",
  "nao_gravou",
  "grande",
  "recusado",
  "corrompido",
];

/** A recusa vem do backend; o que nao tiver a forma esperada conta como falha ao abrir. */
export function codigoDaFalha(erro: unknown): CodigoAvisoDoAjuste {
  const codigo = (erro as Partial<FalhaPreferencias> | null | undefined)?.codigo;
  return CODIGOS.find((conhecido) => conhecido === codigo) ?? "nao_abriu";
}

const CHAVE_DO_AVISO: Record<CodigoAvisoDoAjuste, Chave> = {
  fila_sumiu: "preferencias.aviso.filaSumiu",
  nao_abriu: "preferencias.aviso.naoAbriu",
  nao_gravou: "preferencias.aviso.naoGravou",
  grande: "preferencias.aviso.grande",
  recusado: "preferencias.aviso.recusado",
  corrompido: "preferencias.aviso.corrompido",
};

export function textoDoAvisoDoAjuste(
  codigo: CodigoAvisoDoAjuste,
  fila: string,
  idioma: Idioma = idiomaAtual,
): string {
  return traduzir(idioma, CHAVE_DO_AVISO[codigo], { nome: fila });
}

/**
 * Soma os ajustes corrompidos da leitura aos avisos atuais. Todo aviso fica ate a pessoa agir de novo
 * naquela impressora: a proxima gravacao tira do arquivo o que estava corrompido, e isso nao pode
 * apagar o aviso em silencio. O que a pessoa dispensou nesta sessao nao volta.
 */
export function avisosDepoisDaLeitura(
  atuais: ReadonlyMap<string, CodigoAvisoDoAjuste>,
  ignoradas: readonly string[],
  dispensadas: ReadonlySet<string> = new Set(),
): ReadonlyMap<string, CodigoAvisoDoAjuste> {
  const novos = new Map(atuais);
  for (const fila of ignoradas) {
    if (!novos.has(fila) && !dispensadas.has(fila)) novos.set(fila, "corrompido");
  }
  return novos;
}

function nomesComLinha(impressoras: readonly Impressora[]): ReadonlySet<string> {
  return new Set(impressoras.filter(temPreferencias).map((impressora) => impressora.nome));
}

/**
 * Aviso de uma impressora que nao esta na lista nao tem linha onde aparecer: vai para o topo. O de
 * fila que sumiu nao: ele pede para atualizar a lista, e fora dela ja nao ha o que atualizar.
 */
export function avisosSemLinha(
  avisos: ReadonlyMap<string, CodigoAvisoDoAjuste>,
  impressoras: readonly Impressora[],
): [string, CodigoAvisoDoAjuste][] {
  const comLinha = nomesComLinha(impressoras);
  return [...avisos].filter(([fila, codigo]) => !comLinha.has(fila) && codigo !== "fila_sumiu");
}

/** A fila deixou a lista: o aviso de que ela sumiu ja cumpriu o papel dele. */
export function semFilasQueSairam(
  avisos: ReadonlyMap<string, CodigoAvisoDoAjuste>,
  impressoras: readonly Impressora[],
): ReadonlyMap<string, CodigoAvisoDoAjuste> {
  const comLinha = nomesComLinha(impressoras);
  const saiu = [...avisos].some(([fila, codigo]) => codigo === "fila_sumiu" && !comLinha.has(fila));
  if (!saiu) return avisos;
  return new Map(
    [...avisos].filter(([fila, codigo]) => codigo !== "fila_sumiu" || comLinha.has(fila)),
  );
}

/** Ajuste de fila que nao esta na lista nao tem linha: sem isto, ninguem conseguiria apaga-lo. */
export function ajustesSemLinha(
  salvas: ReadonlySet<string>,
  impressoras: readonly Impressora[],
): string[] {
  const comLinha = nomesComLinha(impressoras);
  return [...salvas].filter((fila) => !comLinha.has(fila));
}

export function semAviso(
  avisos: ReadonlyMap<string, CodigoAvisoDoAjuste>,
  fila: string,
): ReadonlyMap<string, CodigoAvisoDoAjuste> {
  if (!avisos.has(fila)) return avisos;
  const novos = new Map(avisos);
  novos.delete(fila);
  return novos;
}

export function comAviso(
  avisos: ReadonlyMap<string, CodigoAvisoDoAjuste>,
  fila: string,
  codigo: CodigoAvisoDoAjuste,
): ReadonlyMap<string, CodigoAvisoDoAjuste> {
  return new Map(avisos).set(fila, codigo);
}

let filasComAjuste: ReadonlySet<string> = new Set();

/** Guarda quem tem ajuste, para a confirmacao do envio dizer isso sem ler o arquivo de novo. */
export function registrarFilasComAjuste(filas: ReadonlySet<string>): void {
  filasComAjuste = filas;
}

export function filasComAjusteAgora(): ReadonlySet<string> {
  return filasComAjuste;
}

/** A linha da confirmacao do envio, quando alguma fila real da lista tem ajuste. */
export function linhaDoAjusteNoEnvio(
  destinos: readonly Impressora[],
  filas: ReadonlySet<string>,
  idioma: Idioma = idiomaAtual,
): string | null {
  const algum = destinos.some((destino) => temPreferencias(destino) && filas.has(destino.nome));
  return algum ? traduzir(idioma, "preferencias.envio.linha") : null;
}

/** O que o dialogo diz antes de apagar o ajuste salvo. */
export function conteudoDoVoltarAoPadrao(nome: string): ConteudoConfirmacao {
  return {
    titulo: "preferencias.voltarDialogo.titulo",
    texto: "preferencias.voltarDialogo.texto",
    consequencia: null,
    manter: "preferencias.voltarDialogo.manter",
    fechar: "preferencias.voltarDialogo.confirmar",
    parametros: { nome },
    ehJanela: false,
    focoNaOrigemAoConfirmar: true,
  };
}

/**
 * Roda `acao` quando o foco voltar a `origem`, depois de um dialogo fechar. Sem esse aviso a origem
 * poderia sair da tela antes, e o foco cairia longe. O prazo cobre uma janela sem foco do sistema,
 * em que o evento nao chega.
 */
export function aoVoltarOFoco(origem: HTMLElement, acao: () => void, prazoMs = 1000): void {
  let feito = false;
  const rodar = () => {
    if (feito) return;
    feito = true;
    origem.removeEventListener("focus", rodar);
    clearTimeout(prazo);
    acao();
  };
  origem.addEventListener("focus", rodar);
  const prazo = setTimeout(rodar, prazoMs);
}
