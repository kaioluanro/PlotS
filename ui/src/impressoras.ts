import { idiomaAtual, traduzir, type Chave, type Idioma } from "./i18n";
import type { PaginasDaAba } from "./tabs";

export type Nivel = "pronta" | "ativa" | "atencao" | "problema";
export type Modo = "traseira" | "gaveta" | "sem_perfil" | "desconhecido";

export interface CondicaoLida {
  codigo: string;
  detalhe?: string;
}

export interface Impressora {
  id: string;
  nome: string;
  tipo: "fila" | "simulada";
  nivel: Nivel;
  condicoes: CondicaoLida[];
  modo: Modo;
}

export interface Aviso {
  codigo: string;
  detalhe?: string;
  /** Arquivo de perfis envolvido na falha, quando se sabe qual e. */
  caminho?: string;
}

export interface ListaImpressoras {
  impressoras: Impressora[];
  avisos: Aviso[];
}

export const LOTE_MINIMO = 1;
export const LOTE_MAXIMO = 50;
export const LOTE_PADRAO = 10;

export function limitarAoIntervaloDoLote(valor: number): number {
  return Math.min(LOTE_MAXIMO, Math.max(LOTE_MINIMO, valor));
}

/** Enquanto se digita: so vale um inteiro dentro do intervalo. */
export function interpretarLote(texto: string): number | null {
  const limpo = texto.trim();
  if (!/^[0-9]{1,3}$/.test(limpo)) return null;
  const valor = Number(limpo);
  return valor >= LOTE_MINIMO && valor <= LOTE_MAXIMO ? valor : null;
}

/** Ao sair do campo: numero fora do intervalo vai ao limite; o que nao e numero volta ao atual. */
export function confirmarLote(texto: string, atual: number): number {
  const limpo = texto.trim();
  if (!/^[0-9]+$/.test(limpo)) return atual;
  return limitarAoIntervaloDoLote(Number(limpo));
}

/** Cada folha leva duas paginas; pagina impar ocupa uma folha inteira. Mesma conta do plano. */
export function contarLotes(paginas: number, folhasPorLote: number): number {
  if (!Number.isInteger(paginas) || !Number.isInteger(folhasPorLote)) return 0;
  if (paginas <= 0 || folhasPorLote <= 0) return 0;
  return Math.ceil(Math.ceil(paginas / 2) / folhasPorLote);
}

export function resumoDoLote(
  paginas: number,
  folhasPorLote: number,
  idioma: Idioma = idiomaAtual,
): string {
  const lotes = contarLotes(paginas, folhasPorLote);
  const textoPaginas = traduzir(idioma, paginas === 1 ? "folhas.paginas.um" : "folhas.paginas.varios", {
    n: String(paginas),
  });
  const textoLotes = traduzir(idioma, lotes === 1 ? "folhas.lotes.um" : "folhas.lotes.varios", {
    n: String(lotes),
  });
  return `${textoPaginas} · ${textoLotes}`;
}

/** Uma marca so conta enquanto a impressora continuar na lista. */
export function impressorasValidas(
  marcadas: readonly string[],
  lista: readonly Impressora[],
): string[] {
  return marcadas.filter((id) => lista.some((impressora) => impressora.id === id));
}

export interface EstadoBotao {
  rotulo: string;
  titulo: string;
  desabilitado: boolean;
}

/** O envio real ainda nao existe, entao o botao nunca fica ativo. */
export function estadoDoBotao(marcadas: number, idioma: Idioma = idiomaAtual): EstadoBotao {
  return {
    rotulo:
      marcadas > 0
        ? traduzir(idioma, "imprimir.frenteContagem", { n: String(marcadas) })
        : traduzir(idioma, "imprimir.frente"),
    titulo: traduzir(idioma, "imprimir.indisponivel"),
    desabilitado: true,
  };
}

const CHAVES_DE_CONDICAO: ReadonlyMap<string, Chave> = new Map<string, Chave>([
  ["pausada", "condicao.pausada"],
  ["offline", "condicao.offline"],
  ["sem_papel", "condicao.sem_papel"],
  ["atolada", "condicao.atolada"],
  ["erro", "condicao.erro"],
  ["exclusao_pendente", "condicao.exclusao_pendente"],
  ["alimentacao_manual", "condicao.alimentacao_manual"],
  ["problema_com_papel", "condicao.problema_com_papel"],
  ["recebendo_dados", "condicao.recebendo_dados"],
  ["ocupada", "condicao.ocupada"],
  ["imprimindo", "condicao.imprimindo"],
  ["bandeja_cheia", "condicao.bandeja_cheia"],
  ["indisponivel", "condicao.indisponivel"],
  ["em_espera", "condicao.em_espera"],
  ["processando", "condicao.processando"],
  ["inicializando", "condicao.inicializando"],
  ["aquecendo", "condicao.aquecendo"],
  ["pouco_toner", "condicao.pouco_toner"],
  ["sem_toner", "condicao.sem_toner"],
  ["pagina_nao_impressa", "condicao.pagina_nao_impressa"],
  ["intervencao_do_usuario", "condicao.intervencao_do_usuario"],
  ["sem_memoria", "condicao.sem_memoria"],
  ["tampa_aberta", "condicao.tampa_aberta"],
  ["servidor_desconhecido", "condicao.servidor_desconhecido"],
  ["economia_de_energia", "condicao.economia_de_energia"],
  ["servidor_offline", "condicao.servidor_offline"],
  ["driver_desatualizado", "condicao.driver_desatualizado"],
  ["outro", "condicao.outro"],
]);

function comInicialMaiuscula(texto: string): string {
  return texto.charAt(0).toUpperCase() + texto.slice(1);
}

/** Estado em palavras: "Pronta", ou as condicoes separadas por virgula. */
export function textoDoEstado(impressora: Impressora, idioma: Idioma = idiomaAtual): string {
  if (impressora.condicoes.length === 0) return traduzir(idioma, "estado.pronta");
  const partes = impressora.condicoes.map((condicao) => {
    const chave = CHAVES_DE_CONDICAO.get(condicao.codigo);
    if (!chave) return traduzir(idioma, "estado.desconhecido");
    return traduzir(idioma, chave, { detalhe: condicao.detalhe ?? "" });
  });
  return comInicialMaiuscula(partes.join(", "));
}

/** O nome das simuladas e traduzido; o de uma fila e sempre o nome exato dado pelo sistema. */
export function nomeDeExibicao(impressora: Impressora, idioma: Idioma = idiomaAtual): string {
  if (impressora.tipo === "fila") return impressora.nome;
  return traduzir(
    idioma,
    impressora.modo === "gaveta" ? "impressora.simulada.gaveta" : "impressora.simulada.traseira",
  );
}

const CHAVES_DE_AVISO: ReadonlyMap<string, Chave> = new Map<string, Chave>([
  ["filas", "aviso.filas"],
  ["perfis_pasta", "aviso.perfis_pasta"],
  ["perfis_leitura", "aviso.perfis_leitura"],
  ["perfis_grande", "aviso.perfis_grande"],
  ["perfis_invalido", "aviso.perfis_invalido"],
  ["perfis_versao", "aviso.perfis_versao"],
]);

export function textoDoAviso(aviso: Aviso, idioma: Idioma = idiomaAtual): string {
  if (aviso.codigo === "filas" && aviso.detalhe) {
    return traduzir(idioma, "aviso.filasCodigo", { detalhe: aviso.detalhe });
  }
  return traduzir(idioma, CHAVES_DE_AVISO.get(aviso.codigo) ?? "aviso.comunicacao");
}

const AVISOS_QUE_MANDAM_AGIR_NO_ARQUIVO: ReadonlySet<string> = new Set([
  "perfis_leitura",
  "perfis_grande",
  "perfis_invalido",
]);

/** O caminho so aparece quando o texto manda fazer algo com o arquivo. */
export function caminhoDoAviso(aviso: Aviso): string | null {
  if (!AVISOS_QUE_MANDAM_AGIR_NO_ARQUIVO.has(aviso.codigo)) return null;
  return aviso.caminho ? aviso.caminho : null;
}

/** O erro do campo fica no lugar do resumo: as duas informacoes nao aparecem juntas. */
export function conteudoAbaixoDoLote(
  invalido: boolean,
  paginas: PaginasDaAba | null,
): "erro" | "resumo" | "nada" {
  if (invalido) return "erro";
  return paginas === null ? "nada" : "resumo";
}

/** Marca, no HTML, o que pode abrir o menu de contexto do sistema. */
export const ATRIBUTO_MENU_DE_CONTEXTO = "data-menu-de-contexto";

/** O menu fica bloqueado na janela toda, menos onde editar texto precisa dele. */
export function liberaMenuDeContexto(alvo: unknown): boolean {
  if (typeof alvo !== "object" || alvo === null || !("closest" in alvo)) return false;
  const { closest } = alvo;
  if (typeof closest !== "function") return false;
  return closest.call(alvo, `[${ATRIBUTO_MENU_DE_CONTEXTO}]`) !== null;
}
