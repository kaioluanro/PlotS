import type { Cartao } from "./cartoes";
import type { Chave } from "./i18n";
import type { Impressora } from "./impressoras";

/** O perfil de uma rodada: vem do backend e volta para ele, sem nada decidido aqui. */
export interface PerfilDaRodada {
  frenteInversa: boolean;
  versoInverso: boolean;
  versoGirado: boolean;
  virada: "nao_virar" | "lateral" | "borda_de_cima";
  borda: "curta" | "longa";
  driverInverte: boolean;
}

export interface AvaliacaoDaRodada {
  proximo: PerfilDaRodada;
  mudancas: string[];
  certo: boolean;
}

export interface FolhaVista {
  paraCima: number[];
  paraBaixo: number[];
  setasIguais: boolean;
}

export interface SaidaSimulada {
  saiuParaCima: boolean;
  primeiraASair: number | null;
  folhas: FolhaVista[];
}

export type ModeloInicial = "jato" | "laser";
export type PapelDoTeste = "a4" | "carta";
export type ConclusaoAutomatica = "certa" | "nao_vira_pela_borda_longa" | "ordem_nao_serve";

export const RODADAS_MAXIMAS = 5;
/** Cada rodada imprime o teste em folhas novas. */
export const FOLHAS_POR_RODADA = 2;
export const PASSOS_TOTAIS = 5;

export type Pergunta = "saida" | "lado" | "par" | "setas" | "leitura" | "primeira";

/** Cada resposta possivel tem o seu desenho. */
export type FiguraDaResposta =
  | "saida_a_vista"
  | "saida_escondida"
  | "lado_em_branco"
  | "lado_por_cima"
  | "par_certo"
  | "par_errado"
  | "setas_iguais"
  | "setas_opostas"
  | "leitura_sem_virar"
  | "leitura_virando"
  | "leitura_nao_fica"
  | "primeira_1"
  | "primeira_3"
  | "primeira_nao_reparei";

export interface Opcao {
  valor: string;
  rotulo: Chave;
  figura: FiguraDaResposta;
}

/** A pergunta em si. Em lado, setas e leitura, a instrucao vem antes, em outra chave. */
export const TEXTO_DA_PERGUNTA: Readonly<Record<Pergunta, Chave>> = {
  saida: "calibracao.pergunta.saida",
  lado: "calibracao.pergunta.lado",
  par: "calibracao.pergunta.par",
  setas: "calibracao.pergunta.setas",
  leitura: "calibracao.pergunta.leitura",
  primeira: "calibracao.pergunta.primeira",
};

export const INSTRUCAO_DA_PERGUNTA: Partial<Record<Pergunta, Chave>> = {
  lado: "calibracao.instrucao.lado",
  setas: "calibracao.instrucao.setas",
  leitura: "calibracao.instrucao.leitura",
  par: "calibracao.instrucao.par",
};

export const OPCOES: Readonly<Record<Pergunta, readonly Opcao[]>> = {
  saida: [
    { valor: "cima", rotulo: "calibracao.resposta.saidaAVista", figura: "saida_a_vista" },
    { valor: "baixo", rotulo: "calibracao.resposta.saidaEscondida", figura: "saida_escondida" },
  ],
  lado: [
    { valor: "em_branco", rotulo: "calibracao.resposta.emBranco", figura: "lado_em_branco" },
    { valor: "por_cima", rotulo: "calibracao.resposta.porCima", figura: "lado_por_cima" },
  ],
  par: [
    { valor: "sim", rotulo: "calibracao.resposta.parCerto", figura: "par_certo" },
    { valor: "nao", rotulo: "calibracao.resposta.parErrado", figura: "par_errado" },
  ],
  setas: [
    { valor: "sim", rotulo: "calibracao.resposta.setasIguais", figura: "setas_iguais" },
    { valor: "nao", rotulo: "calibracao.resposta.setasOpostas", figura: "setas_opostas" },
  ],
  leitura: [
    { valor: "sem_virar", rotulo: "calibracao.resposta.semVirar", figura: "leitura_sem_virar" },
    { valor: "virando", rotulo: "calibracao.resposta.virando", figura: "leitura_virando" },
    { valor: "nao_fica", rotulo: "calibracao.resposta.naoFica", figura: "leitura_nao_fica" },
  ],
  primeira: [
    { valor: "1", rotulo: "calibracao.resposta.primeira1", figura: "primeira_1" },
    { valor: "3", rotulo: "calibracao.resposta.primeira3", figura: "primeira_3" },
    { valor: "nao_reparei", rotulo: "calibracao.resposta.naoReparei", figura: "primeira_nao_reparei" },
  ],
};

export const TEXTO_DA_MUDANCA: ReadonlyMap<string, Chave> = new Map<string, Chave>([
  ["virada_nao_virar", "calibracao.mudou.viradaNaoVirar"],
  ["virada_lateral", "calibracao.mudou.viradaLateral"],
  ["virada_borda_de_cima", "calibracao.mudou.viradaBordaDeCima"],
  ["frente_inversa", "calibracao.mudou.frenteInversa"],
  ["frente_direta", "calibracao.mudou.frenteDireta"],
  ["verso_inverso", "calibracao.mudou.versoInverso"],
  ["verso_direto", "calibracao.mudou.versoDireto"],
  ["verso_girado", "calibracao.mudou.versoGirado"],
  ["verso_sem_giro", "calibracao.mudou.versoSemGiro"],
  ["driver_compensa", "calibracao.mudou.driverCompensa"],
  ["driver_nao_compensa", "calibracao.mudou.driverNaoCompensa"],
]);

export type Passo =
  | "preparar"
  | "confirmar"
  | "frente"
  | "saida"
  | "recolocar"
  | "verso"
  | "perguntas"
  | "ajustar"
  | "certo"
  | "gravada"
  | "nao_encaixou"
  | "conferida";

export interface Estado {
  passo: Passo;
  automatica: boolean;
  modelo: ModeloInicial;
  papel: PapelDoTeste;
  perfil: PerfilDaRodada | null;
  rodada: number;
  /** Como a folha saiu: fato da impressora, entao vale para as rodadas seguintes. */
  saiuParaCima: boolean | null;
  respostas: Partial<Record<Pergunta, string>>;
  pergunta: Pergunta | null;
  mudancas: string[];
  proximo: PerfilDaRodada | null;
  conclusao: ConclusaoAutomatica | null;
  cartao: Cartao | null;
  recusa: Recusa | null;
}

/** Cada falha tem a sua frase: quem nao conseguiu enviar nao pode ouvir que a impressora esta ocupada. */
export type Recusa = "ocupada" | "falha_envio" | "falha_perfil" | "falha_ao_gravar" | "falha_julgamento";

/** Parte do modelo escolhido na linha; sem modelo, do jato. */
export function estadoInicial(impressora: Impressora): Estado {
  const laser = impressora.id === "simulada:gaveta" || impressora.modo === "laser";
  return {
    passo: "preparar",
    automatica: impressora.modo === "automatico",
    modelo: laser ? "laser" : "jato",
    papel: "a4",
    perfil: null,
    rodada: 1,
    saiuParaCima: null,
    respostas: {},
    pergunta: null,
    mudancas: [],
    proximo: null,
    conclusao: null,
    cartao: null,
    recusa: null,
  };
}

/**
 * As perguntas desta rodada, na ordem. O que olha o maco inteiro vem antes do que manda pegar uma
 * folha, porque depois disso a ordem ja se perdeu. O que saiu por cima nao tem par nem seta a julgar;
 * a primeira folha a sair so importa quando o maco saiu invertido; a automatica nao tem gesto a
 * conferir.
 */
export function perguntasDaRodada(
  automatica: boolean,
  respostas: Partial<Record<Pergunta, string>>,
): Pergunta[] {
  if (automatica) return ["leitura", "par", "setas"];
  if (respostas.lado === "por_cima") return ["lado"];
  const primeira: Pergunta[] = respostas.leitura === "nao_fica" ? ["primeira"] : [];
  return ["lado", "leitura", ...primeira, "par", "setas"];
}

/** Em qual dos passos que a pessoa conta ela esta. */
export function numeroDoPasso(passo: Passo): number {
  switch (passo) {
    case "preparar":
      return 1;
    case "confirmar":
    case "frente":
      return 2;
    case "saida":
    case "recolocar":
    case "verso":
      return 3;
    case "perguntas":
      return 4;
    default:
      return 5;
  }
}

/** A passada segue na fila: quem desiste aqui pede para tira-la de la. */
export function temPassadaNaFila(estado: Estado): boolean {
  if (estado.passo !== "frente" && estado.passo !== "verso") return false;
  return estado.cartao?.estado.tipo !== "erro";
}

export type Acao =
  | { tipo: "chamada_iniciada" }
  | { tipo: "corrigir" }
  | { tipo: "falha_julgamento" }
  | { tipo: "modelo"; modelo: ModeloInicial }
  | { tipo: "papel"; papel: PapelDoTeste }
  | { tipo: "seguir_para_confirmar" }
  | { tipo: "rodada_iniciada"; perfil: PerfilDaRodada; rodada: number }
  | { tipo: "recusada"; recusa: Recusa }
  | { tipo: "verso_falhou" }
  | { tipo: "cartao"; cartao: Cartao }
  | { tipo: "responder"; pergunta: Pergunta; valor: string }
  | { tipo: "continuar" }
  | { tipo: "voltar" }
  | { tipo: "verso_pedido" }
  | { tipo: "avaliada"; avaliacao: AvaliacaoDaRodada }
  | { tipo: "conferida"; conclusao: ConclusaoAutomatica }
  | { tipo: "refazer" }
  | { tipo: "gravada" }
  | { tipo: "falha_ao_gravar" };

function aoReceberCartao(estado: Estado, cartao: Cartao): Estado {
  const atual = { ...estado, cartao };
  const emPassada = estado.passo === "frente" || estado.passo === "verso";
  if (!emPassada) return atual;
  if (cartao.estado.tipo === "aguardando" && estado.passo === "frente") {
    return { ...atual, passo: estado.saiuParaCima === null ? "saida" : "recolocar", pergunta: estado.saiuParaCima === null ? "saida" : null };
  }
  if (cartao.estado.tipo === "concluido") {
    const primeira = perguntasDaRodada(estado.automatica, {})[0] ?? null;
    return { ...atual, passo: "perguntas", pergunta: primeira, respostas: {} };
  }
  return atual;
}

/** Trocar uma resposta anterior invalida o que dependia dela: as que sairam da lista somem. */
function comResposta(estado: Estado, pergunta: Pergunta, valor: string): Estado {
  const respostas = { ...estado.respostas, [pergunta]: valor };
  const validas = perguntasDaRodada(estado.automatica, respostas);
  const filtradas: Partial<Record<Pergunta, string>> = {};
  for (const item of validas) {
    if (respostas[item] !== undefined) filtradas[item] = respostas[item];
  }
  return { ...estado, respostas: pergunta === "saida" ? respostas : filtradas };
}

function aoContinuar(estado: Estado): Estado {
  if (estado.pergunta === null) return estado;
  if (estado.passo === "saida") {
    const resposta = estado.respostas.saida;
    if (resposta === undefined) return estado;
    return { ...estado, passo: "recolocar", saiuParaCima: resposta === "cima", pergunta: null, respostas: {} };
  }
  const lista = perguntasDaRodada(estado.automatica, estado.respostas);
  const indice = lista.indexOf(estado.pergunta);
  if (indice < 0) return estado;
  const proxima = lista[indice + 1];
  if (estado.respostas[estado.pergunta] === undefined || proxima === undefined) return estado;
  return { ...estado, pergunta: proxima };
}

function aoVoltar(estado: Estado): Estado {
  if (estado.passo === "confirmar") return { ...estado, passo: "preparar" };
  if (estado.passo !== "perguntas" || estado.pergunta === null) return estado;
  const lista = perguntasDaRodada(estado.automatica, estado.respostas);
  const anterior = lista[lista.indexOf(estado.pergunta) - 1];
  return anterior === undefined ? estado : { ...estado, pergunta: anterior };
}

/** Da tela certa, volta a ultima pergunta com as respostas que ja estao la. Nada e reimpresso. */
function aoCorrigir(estado: Estado): Estado {
  if (estado.passo !== "certo") return estado;
  const lista = perguntasDaRodada(estado.automatica, estado.respostas);
  const ultima = lista[lista.length - 1];
  if (ultima === undefined) return estado;
  return { ...estado, passo: "perguntas", pergunta: ultima, recusa: null };
}

export function reduzir(estado: Estado, acao: Acao): Estado {
  switch (acao.tipo) {
    case "chamada_iniciada":
      return { ...estado, recusa: null };
    case "corrigir":
      return aoCorrigir(estado);
    case "falha_julgamento":
      return { ...estado, recusa: "falha_julgamento" };
    case "modelo":
      return { ...estado, modelo: acao.modelo };
    case "papel":
      return { ...estado, papel: acao.papel };
    case "seguir_para_confirmar":
      return { ...estado, passo: "confirmar", recusa: null };
    case "rodada_iniciada":
      return {
        ...estado,
        passo: "frente",
        perfil: acao.perfil,
        rodada: acao.rodada,
        cartao: null,
        recusa: null,
        respostas: {},
      };
    case "recusada":
      return { ...estado, recusa: acao.recusa };
    case "verso_falhou":
      return { ...estado, passo: "recolocar", pergunta: null, recusa: "falha_envio" };
    case "cartao":
      return aoReceberCartao(estado, acao.cartao);
    case "responder":
      return comResposta(estado, acao.pergunta, acao.valor);
    case "continuar":
      return aoContinuar(estado);
    case "voltar":
      return aoVoltar(estado);
    case "verso_pedido":
      return { ...estado, passo: "verso", cartao: null, recusa: null };
    case "avaliada": {
      const { avaliacao } = acao;
      if (avaliacao.certo) return { ...estado, passo: "certo", mudancas: [], proximo: null, recusa: null };
      if (estado.rodada >= RODADAS_MAXIMAS) return { ...estado, passo: "nao_encaixou", recusa: null };
      return { ...estado, passo: "ajustar", mudancas: avaliacao.mudancas, proximo: avaliacao.proximo, recusa: null };
    }
    case "conferida":
      return { ...estado, passo: "conferida", conclusao: acao.conclusao, recusa: null };
    case "refazer":
      return { ...estado, passo: "confirmar", cartao: null };
    case "gravada":
      return { ...estado, passo: "gravada", recusa: null };
    case "falha_ao_gravar":
      return { ...estado, recusa: "falha_ao_gravar" };
  }
}

/** O perfil da proxima rodada, ja contada. */
export function proximaRodada(estado: Estado): { perfil: PerfilDaRodada; rodada: number } | null {
  if (estado.passo !== "ajustar" || estado.proximo === null) return null;
  return { perfil: estado.proximo, rodada: estado.rodada + 1 };
}

/** As respostas no formato do backend. `null` enquanto faltar alguma das que esta rodada pede. */
export function respostasParaAvaliar(estado: Estado) {
  const r = estado.respostas;
  if (perguntasDaRodada(estado.automatica, r).some((pergunta) => r[pergunta] === undefined)) return null;
  // So vale a primeira folha que a rodada pergunta: resposta antiga nao entra na avaliacao.
  const primeira = perguntasDaRodada(estado.automatica, r).includes("primeira")
    ? r.primeira === "1"
      ? 1
      : r.primeira === "3"
        ? 3
        : null
    : null;
  return {
    porCima: r.lado === "por_cima",
    parCerto: r.par === "sim",
    setasIguais: r.setas === "sim",
    leitura: r.leitura ?? "sem_virar",
    primeiraASair: primeira,
  };
}

/** O maco certo e lido virado? So faz sentido ao fim, com a pergunta respondida. */
export function leituraFinal(estado: Estado): "sem_virar" | "virando" {
  return estado.respostas.leitura === "virando" ? "virando" : "sem_virar";
}

/** A fila pede o nome do arquivo em um dialogo: tentar de novo daria no mesmo. */
export function filaNaoServe(estado: Estado): boolean {
  return estado.cartao?.estado.tipo === "erro" && estado.cartao.estado.motivo.codigo === "fila_pede_arquivo";
}

/** Com o resultado na mao e as folhas ja gastas, desistir sem querer perderia tudo. */
export function pedeConfirmacaoAoDesistir(estado: Estado, simulada: boolean): boolean {
  return estado.passo === "certo" && !simulada;
}

/**
 * Trocar a compensacao em uma fila calibrada muda a ordem que a calibracao conferiu. Compara com o
 * valor que estava gravado na calibracao, nao com o da lista; sem esse valor (arquivo antigo), nao ha aviso.
 */
export function trocaPoeCalibracaoEmDuvida(
  modo: Impressora["modo"],
  naCalibracao: boolean | undefined,
  ligado: boolean,
): boolean {
  return modo === "calibrada" && naCalibracao !== undefined && naCalibracao !== ligado;
}

/** O que a instrucao de cada passo diz antes da pergunta. Os outros passos nao tem instrucao. */
export function instrucaoDoPasso(estado: Estado): Chave | null {
  if ((estado.passo === "perguntas" || estado.passo === "saida") && estado.pergunta !== null) {
    return INSTRUCAO_DA_PERGUNTA[estado.pergunta] ?? null;
  }
  return null;
}

/** O que o leitor de tela anuncia no passo. Na recusa da fila que pede arquivo, o alerta ja fala. */
export function anuncioDoPasso(progresso: string, textos: readonly string[], naoServe: boolean): string {
  if (naoServe) return "";
  return textos.length === 0 ? progresso : `${progresso}. ${textos.join(" ")}`;
}

const ROTULO_DO_RESUMO: Partial<Record<Pergunta, Chave>> = {
  lado: "calibracao.resumo.lado",
  leitura: "calibracao.resumo.leitura",
  primeira: "calibracao.resumo.primeira",
  par: "calibracao.resumo.par",
  setas: "calibracao.resumo.setas",
};

/** O que a tela certa resume: o rotulo de cada pergunta desta rodada e a resposta dada, na ordem. */
export function resumoDasRespostas(estado: Estado): { rotulo: Chave; valor: Chave }[] {
  const linhas: { rotulo: Chave; valor: Chave }[] = [];
  for (const pergunta of perguntasDaRodada(estado.automatica, estado.respostas)) {
    const valor = estado.respostas[pergunta];
    const opcao = OPCOES[pergunta].find((item) => item.valor === valor);
    const rotulo = ROTULO_DO_RESUMO[pergunta];
    if (opcao !== undefined && rotulo !== undefined) linhas.push({ rotulo, valor: opcao.rotulo });
  }
  return linhas;
}

/** Os textos que o leitor de tela ouve neste passo: a confirmacao de desistir traz a explicacao tambem. */
export function chavesDoAnuncio(estado: Estado, confirmando: boolean): Chave[] {
  if (confirmando) return ["calibracao.desistir.pergunta", "calibracao.desistir.explicacao"];
  const instrucao = instrucaoDoPasso(estado);
  const frase = fraseDoPasso(estado);
  return [instrucao, frase].filter((chave): chave is Chave => chave !== null);
}

/** A frase que abre o passo, para o anuncio do leitor de tela. Os outros passos tem so o progresso. */
export function fraseDoPasso(estado: Estado): Chave | null {
  if ((estado.passo === "perguntas" || estado.passo === "saida") && estado.pergunta !== null) {
    return TEXTO_DA_PERGUNTA[estado.pergunta];
  }
  switch (estado.passo) {
    case "ajustar":
      return "calibracao.ajustar.titulo";
    case "certo":
      return "calibracao.certo.titulo";
    case "gravada":
      return "calibracao.gravada";
    case "nao_encaixou":
      return "calibracao.naoEncaixou.titulo";
    case "conferida":
      return chaveDaConclusao(estado.conclusao);
    default:
      return null;
  }
}

export function chaveDaConclusao(conclusao: ConclusaoAutomatica | null): Chave {
  if (conclusao === "certa") return "calibracao.conferida.certa";
  return conclusao === "nao_vira_pela_borda_longa" ? "calibracao.conferida.borda" : "calibracao.conferida.ordem";
}

/** A ultima pergunta da rodada: ao continuar, o backend julga as respostas. */
export function eUltimaPergunta(estado: Estado): boolean {
  if (estado.passo !== "perguntas" || estado.pergunta === null) return false;
  const lista = perguntasDaRodada(estado.automatica, estado.respostas);
  return lista[lista.length - 1] === estado.pergunta;
}
