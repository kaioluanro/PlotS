import {
  acoesDoCartao,
  cartaoPedeAcao,
  frasesDoCartao,
  nomeDoCartao,
  tomDoCartao,
  type Cartao,
  type TomDoCartao,
} from "./cartoes";

/** Quanto tempo um aviso passageiro fica no topo da janela. */
export const ESPERA_DO_AVISO_MS = 6000;
/** No topo cabem tres avisos ao mesmo tempo; os demais so aparecem no painel. */
export const MAXIMO_NO_TOPO = 3;
/** Quantos avisos da sessao ficam guardados em memoria; os mais antigos saem primeiro. */
export const LIMITE_DE_AVISOS = 200;

/** Um aviso e um registro do cartao no instante em que ele mudou de estado. */
export interface AvisoDoTrabalho {
  id: number;
  quando: number;
  /** O cartao como estava naquele instante: o painel mostra o texto dele, mesmo depois. */
  cartao: Cartao;
  /** O documento da aba na hora: com duas abas abertas, e o que diz de qual e o aviso. */
  documento: string | null;
  /** Fica no topo ate ser fechado: erro, ou um estado que pede a acao da pessoa. */
  fixo: boolean;
  lido: boolean;
  fechado: boolean;
}

export interface EstadoDosAvisos {
  /** Do mais novo para o mais antigo. */
  itens: readonly AvisoDoTrabalho[];
  proximoId: number;
}

export const ESTADO_INICIAL_DOS_AVISOS: EstadoDosAvisos = { itens: [], proximoId: 1 };

export type AcaoDosAvisos =
  | { tipo: "publicar"; cartao: Cartao; agora: number; documento?: string | null }
  | { tipo: "fechar"; id: number }
  | { tipo: "ler_todos" };

/** Erro e o que pede acao ficam no topo ate a pessoa fechar; o resto some sozinho. */
export function ehAvisoFixo(cartao: Cartao): boolean {
  return cartao.estado.tipo === "erro" || cartaoPedeAcao(cartao);
}

export function reduzirAvisos(estado: EstadoDosAvisos, acao: AcaoDosAvisos): EstadoDosAvisos {
  switch (acao.tipo) {
    case "publicar": {
      const novo: AvisoDoTrabalho = {
        id: estado.proximoId,
        quando: acao.agora,
        cartao: acao.cartao,
        documento: acao.documento ?? null,
        fixo: ehAvisoFixo(acao.cartao),
        lido: false,
        fechado: false,
      };
      return {
        itens: [novo, ...estado.itens].slice(0, LIMITE_DE_AVISOS),
        proximoId: estado.proximoId + 1,
      };
    }
    case "fechar":
      return {
        ...estado,
        itens: estado.itens.map((item) => (item.id === acao.id ? { ...item, fechado: true } : item)),
      };
    case "ler_todos":
      return {
        ...estado,
        itens: estado.itens.map((item) => (item.lido ? item : { ...item, lido: true })),
      };
  }
}

/** Um intervalo em que o prazo dos avisos passageiros ficou parado (ponteiro ou foco sobre a faixa). */
export interface PausaDoTopo {
  de: number;
  /** Nulo enquanto a pausa dura. */
  ate: number | null;
}

/** Quanto do tempo do aviso correu de fato: o que passou, menos as pausas dentro desse intervalo. */
export function tempoDecorrido(
  quando: number,
  agora: number,
  pausas: readonly PausaDoTopo[] = [],
): number {
  let parado = 0;
  for (const pausa of pausas) {
    const inicio = Math.max(pausa.de, quando);
    const fim = Math.min(pausa.ate ?? agora, agora);
    if (fim > inicio) parado += fim - inicio;
  }
  return Math.max(0, agora - quando - parado);
}

/** Ainda deve estar no topo: nao fechado, e passageiro dentro da espera ou fixo. */
function visivelNoTopo(aviso: AvisoDoTrabalho, agora: number, pausas: readonly PausaDoTopo[]): boolean {
  if (aviso.fechado) return false;
  return aviso.fixo || tempoDecorrido(aviso.quando, agora, pausas) < ESPERA_DO_AVISO_MS;
}

/**
 * Os avisos do topo, no maximo tres: so o mais novo de cada trabalho (o anterior ja foi superado e
 * fica no painel), com os fixos primeiro e cada grupo do mais novo para o mais antigo.
 */
export function avisosNoTopo(
  itens: readonly AvisoDoTrabalho[],
  agora: number,
  pausas: readonly PausaDoTopo[] = [],
): AvisoDoTrabalho[] {
  const vistos = new Set<string>();
  const atuais = itens.filter((aviso) => {
    const chave = chaveDoCartao(aviso.cartao);
    const superado = vistos.has(chave);
    vistos.add(chave);
    return !superado && visivelNoTopo(aviso, agora, pausas);
  });
  return [...atuais.filter((aviso) => aviso.fixo), ...atuais.filter((aviso) => !aviso.fixo)].slice(
    0,
    MAXIMO_NO_TOPO,
  );
}

/** "documento · fila": com duas abas abertas, o aviso diz de qual e. */
export function nomeDoAvisoNoTopo(aviso: AvisoDoTrabalho): string {
  const fila = nomeDoCartao(aviso.cartao);
  return aviso.documento === null || aviso.documento === "" ? fila : `${aviso.documento} · ${fila}`;
}

/** Quantos milissegundos faltam para o aviso passageiro mais novo sair; nulo se nenhum sai. */
export function esperaAteSair(
  itens: readonly AvisoDoTrabalho[],
  agora: number,
  pausas: readonly PausaDoTopo[] = [],
): number | null {
  const prazos = itens
    .filter((aviso) => !aviso.fechado && !aviso.fixo)
    .map((aviso) => ESPERA_DO_AVISO_MS - tempoDecorrido(aviso.quando, agora, pausas))
    .filter((restante) => restante > 0);
  return prazos.length === 0 ? null : Math.min(...prazos);
}

export function naoLidos(itens: readonly AvisoDoTrabalho[]): number {
  return itens.filter((aviso) => !aviso.lido).length;
}

/** O numero que cabe no contador do sino: acima de 99, "99+". */
export function contadorDoSino(n: number): string {
  return n > 99 ? "99+" : String(n);
}

/** Chave de um cartao: cada impressora tem um cartao por aba. */
export function chaveDoCartao(cartao: Cartao): string {
  return `${cartao.aba}:${cartao.impressora}`;
}

/**
 * O que muda o estado de um cartao: a passada, o tipo, o motivo e o lote. Mensagens do driver e
 * respostas nao contam, para nao gerar um aviso a cada atualizacao de linha.
 */
export function assinaturaDoEstado(cartao: Cartao): string {
  const { estado } = cartao;
  const passada = "passada" in estado ? estado.passada : "";
  const motivo = "motivo" in estado ? estado.motivo.codigo : "";
  return [estado.tipo, passada, motivo, cartao.lote, cartao.copia ?? ""].join("|");
}

/**
 * Compara os cartoes de agora com a ultima leitura: devolve os que mudaram e a leitura nova.
 * Cartao que saiu da tela nao gera aviso, e some da leitura.
 */
export function cartoesComMudanca(
  anteriores: ReadonlyMap<string, string>,
  cartoes: readonly Cartao[],
): { novos: Cartao[]; proximo: Map<string, string> } {
  const novos: Cartao[] = [];
  const proximo = new Map<string, string>();
  for (const cartao of cartoes) {
    const chave = chaveDoCartao(cartao);
    const assinatura = assinaturaDoEstado(cartao);
    proximo.set(chave, assinatura);
    if (anteriores.get(chave) !== assinatura) novos.push(cartao);
  }
  return { novos, proximo };
}

/** Linhas de acao do cartao para o topo: o que a pessoa precisa fazer, sem o resto. */
export function frasesDeAcaoDoAviso(cartao: Cartao): string[] {
  return frasesDoCartao(cartao).slice(0, 2);
}

/**
 * Quais acoes o painel oferece para um trabalho que ainda existe na janela. Frente e verso nao entram:
 * saem so pelo rodape, que e de onde a pessoa sabe qual face e qual trecho vao sair.
 */
export interface AcoesDoPainel {
  continuar: boolean;
  reimprimir: boolean;
  cancelar: boolean;
}

export function acoesDoPainel(cartao: Cartao): AcoesDoPainel {
  const acoes = acoesDoCartao(cartao);
  return {
    continuar: acoes.continuar,
    reimprimir: acoes.reimprimir,
    cancelar: acoes.cancelar,
  };
}

/** O que o painel oferece em uma entrada: so a mais recente do trabalho tem acoes. */
export interface AcoesDaEntrada {
  continuar: boolean;
  /** So em erro: em aviso ou em andamento o trabalho segue na fila. */
  reimprimir: boolean;
  /** So com trabalho em andamento; desabilitado enquanto o cancelamento corre. */
  cancelar: { desabilitado: boolean } | null;
}

export interface EntradaDoPainel {
  aviso: AvisoDoTrabalho;
  acoes: AcoesDaEntrada | null;
  /** O cartao cujo gesto a dica explica: so a entrada atual, no estado que pede o proximo passo. */
  dicaDe: Cartao | null;
  /** E a entrada mais recente do trabalho, e o trabalho ja nao esta na janela. */
  semTrabalho: boolean;
}

const ESTADOS_COM_DICA: ReadonlySet<Cartao["estado"]["tipo"]> = new Set([
  "frentes_prontas",
  "aguardando",
  "proximo_lote",
]);

/**
 * Casa a sessao com os trabalhos que ainda existem. `itens` vem do mais novo para o mais antigo: a
 * primeira entrada de cada trabalho e a unica que recebe acoes e dica.
 */
export function entradasDoPainel(
  itens: readonly AvisoDoTrabalho[],
  cartoesVivos: readonly Cartao[],
): EntradaDoPainel[] {
  const vistos = new Set<string>();
  return itens.map((aviso) => {
    const chave = chaveDoCartao(aviso.cartao);
    const maisRecente = !vistos.has(chave);
    vistos.add(chave);
    if (!maisRecente) return { aviso, acoes: null, dicaDe: null, semTrabalho: false };
    const vivo = cartoesVivos.find((atual) => chaveDoCartao(atual) === chave);
    if (vivo === undefined) return { aviso, acoes: null, dicaDe: null, semTrabalho: true };
    const acoes = acoesDoCartao(vivo);
    return {
      aviso,
      acoes: {
        continuar: acoes.continuar,
        reimprimir: vivo.estado.tipo === "erro" && acoes.reimprimir,
        cancelar: acoes.cancelar ? { desabilitado: acoes.cancelando } : null,
      },
      dicaDe:
        vivo.estado.tipo === aviso.cartao.estado.tipo && ESTADOS_COM_DICA.has(vivo.estado.tipo)
          ? aviso.cartao
          : null,
      semTrabalho: false,
    };
  });
}

/** O tom do cartao, para o icone: a cor nunca diz sozinha o que aconteceu. */
export function tomDoAviso(cartao: Cartao): TomDoCartao {
  return tomDoCartao(cartao);
}
