import { idiomaAtual, traduzir, type Chave, type Idioma } from "./i18n";
import type { PaginasDaAba } from "./tabs";

export type Nivel = "pronta" | "ativa" | "atencao" | "problema";
export type Modelo = "jato" | "laser";
export type Modo = Modelo | "personalizado" | "sem_perfil" | "desconhecido";

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
  /** Esta fila nunca recebe da janela. */
  recusa?: "pede_arquivo" | "fax";
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
    impressora.modo === "laser" ? "impressora.simulada.gaveta" : "impressora.simulada.traseira",
  );
}

const CHAVES_DE_AVISO: ReadonlyMap<string, Chave> = new Map<string, Chave>([
  ["filas", "aviso.filas"],
  ["perfis_pasta", "aviso.perfis_pasta"],
  ["perfis_leitura", "aviso.perfis_leitura"],
  ["perfis_grande", "aviso.perfis_grande"],
  ["perfis_invalido", "aviso.perfis_invalido"],
  ["perfis_versao", "aviso.perfis_versao"],
  ["modelo_nao_gravado", "aviso.modelo_nao_gravado"],
]);

export function textoDoAviso(aviso: Aviso, idioma: Idioma = idiomaAtual): string {
  if (aviso.codigo === "filas" && aviso.detalhe) {
    return traduzir(idioma, "aviso.filasCodigo", { detalhe: aviso.detalhe });
  }
  return traduzir(idioma, CHAVES_DE_AVISO.get(aviso.codigo) ?? "aviso.comunicacao", {
    detalhe: aviso.detalhe ?? "",
  });
}

const AVISOS_QUE_MANDAM_AGIR_NO_ARQUIVO: ReadonlySet<string> = new Set([
  "perfis_leitura",
  "perfis_grande",
  "perfis_invalido",
  "modelo_nao_gravado",
]);

/** O caminho so aparece quando o texto manda fazer algo com o arquivo. */
export function caminhoDoAviso(aviso: Aviso): string | null {
  if (!AVISOS_QUE_MANDAM_AGIR_NO_ARQUIVO.has(aviso.codigo)) return null;
  return aviso.caminho ? aviso.caminho : null;
}

const CHAVES_DO_MODELO: Readonly<Record<Modelo, Chave>> = {
  jato: "modo.jato",
  laser: "modo.laser",
};

export function nomeDoModelo(modelo: Modelo, idioma: Idioma = idiomaAtual): string {
  return traduzir(idioma, CHAVES_DO_MODELO[modelo]);
}

export function eModelo(modo: Modo): modo is Modelo {
  return modo === "jato" || modo === "laser";
}

export const CALIBRAR = "calibrar";

export interface OpcaoDoSeletor {
  valor: string;
  rotulo: string;
  /** So mostra o estado atual, sem aparecer na lista para ser escolhida. */
  soEstado: boolean;
  /** Vem depois de um divisor: nao e uma escolha do mesmo tipo das anteriores. */
  divisorAntes?: boolean;
}

export interface EstadoDoSeletor {
  valor: string;
  titulo: string;
  opcoes: OpcaoDoSeletor[];
}

/**
 * O que o seletor mostra: a escolha sendo gravada ou, sem ela, o que esta salvo. Simulada tem
 * modelo fixo; com o arquivo de perfis ilegivel nao se sabe o que ha para substituir.
 */
export function estadoDoSeletor(
  impressora: Impressora,
  gravando: Modelo | undefined,
  idioma: Idioma = idiomaAtual,
): EstadoDoSeletor | null {
  if (impressora.tipo !== "fila" || impressora.modo === "desconhecido") return null;
  const { modo } = impressora;
  const opcoes: OpcaoDoSeletor[] = [
    { valor: "jato", rotulo: nomeDoModelo("jato", idioma), soEstado: false },
    { valor: "laser", rotulo: nomeDoModelo("laser", idioma), soEstado: false },
    { valor: CALIBRAR, rotulo: traduzir(idioma, "modo.calibrar"), soEstado: false, divisorAntes: true },
  ];
  if (eModelo(modo)) {
    const rotulo = nomeDoModelo(modo, idioma);
    return {
      valor: gravando ?? modo,
      titulo: traduzir(idioma, "modo.perfil.titulo", {
        modo: rotulo,
        face: traduzir(idioma, "modo.entrada"),
      }),
      opcoes,
    };
  }
  const personalizado = modo === "personalizado";
  return {
    valor: gravando ?? modo,
    titulo: traduzir(idioma, personalizado ? "modo.seletor.personalizado" : "modo.seletor.semPerfil"),
    opcoes: [
      {
        valor: modo,
        rotulo: traduzir(idioma, personalizado ? "modo.personalizado" : "modo.semPerfil"),
        soEstado: true,
      },
      ...opcoes,
    ],
  };
}

export type EscolhaNoSeletor =
  | { acao: "gravar"; modelo: Modelo }
  | { acao: "calibrar" }
  | { acao: "nada" };

export function interpretarEscolha(valor: string): EscolhaNoSeletor {
  if (valor === "jato" || valor === "laser") return { acao: "gravar", modelo: valor };
  return valor === CALIBRAR ? { acao: "calibrar" } : { acao: "nada" };
}

/** Gravacao rapida nao pisca: o indicador so aparece depois desta espera. */
export const ESPERA_DO_INDICADOR_MS = 400;

export function indicaGravacao(gravando: Modelo | undefined, decorridoMs: number): boolean {
  return gravando !== undefined && decorridoMs >= ESPERA_DO_INDICADOR_MS;
}

/**
 * So um perfil personalizado tem valores que nao voltam sem calibrar de novo. Com uma gravacao em
 * andamento nao ha o que confirmar: a escolha seria ignorada.
 */
export function pedeConfirmacao(modo: Modo, gravando: Modelo | undefined): boolean {
  return modo === "personalizado" && gravando === undefined;
}

/** Com a lista fechada, as teclas de andar trocariam o valor a cada passo, e Enter nao faria nada. */
export function teclaAbreALista(tecla: string): boolean {
  return ["ArrowDown", "ArrowUp", "Home", "End", "PageDown", "PageUp", "Enter"].includes(tecla);
}

export interface SeletorComLista {
  matches(consulta: string): boolean;
  showPicker(): void;
}

/**
 * Abre a lista em vez de deixar a tecla trocar o valor; devolve se a tecla foi usada aqui. Sem saber
 * se a lista ja esta aberta, a tecla segue seu caminho para nao travar quem esta andando nela.
 */
export function abrirListaPeloTeclado(seletor: SeletorComLista): boolean {
  try {
    if (seletor.matches(":open")) return false;
  } catch {
    return false;
  }
  try {
    seletor.showPicker();
  } catch {
    // O sistema pode recusar a abertura; o valor continua sem mudar e o clique ainda abre a lista.
  }
  return true;
}

/** A linha da fila mostra a propria falha; sem a linha na lista, ela vai para o topo. */
export function avisosDoTopo(
  avisos: readonly Aviso[],
  falha: Aviso | null,
  impressoras: readonly Impressora[],
): readonly Aviso[] {
  if (falha === null || linhaDaFalha(falha, impressoras) !== null) return avisos;
  return [falha, ...avisos];
}

/** Id da linha em que a falha aparece: a fila real com o nome que nao gravou. */
export function linhaDaFalha(falha: Aviso | null, impressoras: readonly Impressora[]): string | null {
  if (falha === null) return null;
  const linha = impressoras.find(
    (impressora) => impressora.tipo === "fila" && impressora.nome === falha.detalhe,
  );
  return linha?.id ?? null;
}

/** A linha mostra o modelo gravado sem esperar uma nova leitura das filas. */
export function comModeloGravado(
  lista: ListaImpressoras,
  fila: string,
  modelo: Modelo,
): ListaImpressoras {
  return {
    ...lista,
    impressoras: lista.impressoras.map((impressora) =>
      impressora.tipo === "fila" && impressora.nome === fila
        ? { ...impressora, modo: modelo }
        : impressora,
    ),
  };
}

/** O backend responde a falha em forma de aviso; qualquer outra resposta perde so o caminho. */
export function avisoDeFalhaAoGravar(falha: unknown, fila: string): Aviso {
  const caminho =
    typeof falha === "object" && falha !== null && "caminho" in falha ? falha.caminho : undefined;
  return {
    codigo: "modelo_nao_gravado",
    detalhe: fila,
    ...(typeof caminho === "string" && caminho !== "" ? { caminho } : {}),
  };
}

/** Grava a escolha e diz o que mostrar depois. Nunca rejeita: a falha vira aviso. */
export async function gravarEscolha(
  fila: string,
  modelo: Modelo,
  gravar: (fila: string, modelo: Modelo) => Promise<void>,
): Promise<{ gravado: true } | { gravado: false; aviso: Aviso }> {
  try {
    await gravar(fila, modelo);
    return { gravado: true };
  } catch (falha: unknown) {
    return { gravado: false, aviso: avisoDeFalhaAoGravar(falha, fila) };
  }
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
