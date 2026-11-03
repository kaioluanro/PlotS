import {
  cartaoEmAndamento,
  comCartao,
  ehRecusaDeOcupada,
  lotesSemImprimir,
  type Cartao,
  type FaixaDoLote,
  type PapelDoCartao,
} from "./cartoes";
import { limitarAoIntervaloDoLote, LOTE_PADRAO } from "./impressoras";
import { normalizarRascunho, type RascunhoIntervalo } from "./intervalo";

export interface Documento {
  caminho: string;
  nome: string;
}

/** Total de paginas da aba, ou o motivo de o PDF ter sido recusado, ja traduzido. */
export type PaginasDaAba =
  | { estado: "contando" }
  | { estado: "conhecido"; total: number }
  | { estado: "recusado"; motivo: string };

/** Espelho: cada impressora imprime tudo. Divisao: os lotes sao repartidos entre elas. */
export type ModoDistribuicao = "espelho" | "divisao";

/** So quem recebe de fato conta: com menos de duas, o modo nao altera o envio. */
export function modoDoEnvio(modo: ModoDistribuicao, destinos: number): ModoDistribuicao {
  return destinos >= 2 ? modo : "espelho";
}

/** O que cabe a uma impressora na divisao, com os lotes numerados como no documento. */
export interface ParteDaDivisao {
  impressora: string;
  nome: string;
  lotes: readonly number[];
  /** As paginas de cada lote, como o backend as planejou. */
  faixas?: readonly FaixaDoLote[];
  /** Sem valor, o perfil nao afirma como o maco sai. */
  viraParaLer: boolean | null;
  /** O que faltava quando o cartao foi dispensado: a junção continua sabendo depois. */
  semImprimirAoDispensar?: readonly number[];
}

export interface DivisaoDoEnvio {
  partes: readonly ParteDaDivisao[];
  /** Os papeis do envio, para a linha que resume o que saiu. */
  papeis?: readonly PapelDoCartao[];
}

export interface Aba {
  id: number;
  documento: Documento;
  impressaoEmAndamento: boolean;
  /** Identificadores das impressoras marcadas neste documento. */
  impressorasMarcadas: readonly string[];
  /** Folhas por lote, sempre dentro do intervalo aceito. */
  tamanhoLote: number;
  /** De qual a qual pagina, como foi escrito; `null` e o documento inteiro. */
  intervalo: RascunhoIntervalo | null;
  modo: ModoDistribuicao;
  /** O ultimo envio em divisao; sem valor, o ultimo foi em espelho. */
  divisao: DivisaoDoEnvio | null;
  paginas: PaginasDaAba;
  /** Um cartao por impressora; trocar de aba troca a lista, e os trabalhos seguem. */
  cartoes: readonly Cartao[];
}

export interface EstadoAbas {
  abas: Aba[];
  ativa: number | null;
  proximoId: number;
  /** Aba cujo fechamento espera a resposta do usuario. */
  aguardandoConfirmacao: number | null;
  /** O fechamento da janela espera a resposta do usuario. */
  confirmandoJanela: boolean;
}

export type AcaoAbas =
  | { tipo: "abrir"; documento: Documento }
  | { tipo: "ativar"; id: number }
  | { tipo: "trocar" }
  | { tipo: "pedir_fechamento"; id: number }
  | { tipo: "pedir_fechamento_janela" }
  | { tipo: "confirmar_fechamento" }
  | { tipo: "cancelar_fechamento" }
  | { tipo: "marcar_impressao"; id: number; emAndamento: boolean }
  | { tipo: "alternar_impressora"; id: number; impressora: string }
  | { tipo: "definir_lote"; id: number; tamanho: number }
  | { tipo: "definir_intervalo"; id: number; intervalo: RascunhoIntervalo | null }
  | { tipo: "definir_modo"; id: number; modo: ModoDistribuicao }
  | { tipo: "aplicar_grupo"; id: number; impressoras: readonly string[]; modo: ModoDistribuicao }
  | { tipo: "divisao_enviada"; id: number; divisao: DivisaoDoEnvio | null }
  | { tipo: "paginas_contadas"; id: number; total: number }
  | { tipo: "paginas_recusadas"; id: number; motivo: string }
  | { tipo: "cartao"; cartao: Cartao }
  | { tipo: "dispensar_cartao"; id: number; impressora: string };

export const estadoInicial: EstadoAbas = {
  abas: [],
  ativa: null,
  proximoId: 1,
  aguardandoConfirmacao: null,
  confirmandoJanela: false,
};

export function abasEmImpressao(estado: EstadoAbas): Aba[] {
  return estado.abas.filter((aba) => aba.impressaoEmAndamento);
}

export function decidirFechamentoJanela(estado: EstadoAbas): "fechar" | "confirmar" {
  return abasEmImpressao(estado).length > 0 ? "confirmar" : "fechar";
}

/**
 * Um ouvinte de fechamento faz o sistema entregar o fechamento a interface, que pode travar.
 * Por isso ele so existe enquanto ha algo a confirmar; fora disso o backend fecha sozinho.
 */
export function deveOuvirFechamentoJanela(estado: EstadoAbas): boolean {
  return decidirFechamentoJanela(estado) === "confirmar";
}

export function vizinhaPorSeta(abas: readonly Aba[], posicao: number, tecla: string): Aba | null {
  const passo = tecla === "ArrowRight" ? 1 : tecla === "ArrowLeft" ? -1 : 0;
  if (passo === 0 || abas.length === 0) return null;
  return abas[(posicao + passo + abas.length) % abas.length] ?? null;
}

/** Foco itinerante: Tab entra na lista pela aba ativa; as outras se alcancam com as setas. */
export function indiceDeTabulacao(id: number, ativa: number | null): 0 | -1 {
  return id === ativa ? 0 : -1;
}

function fechar(estado: EstadoAbas, id: number): EstadoAbas {
  const posicao = estado.abas.findIndex((aba) => aba.id === id);
  if (posicao < 0) return estado;
  const abas = estado.abas.filter((aba) => aba.id !== id);
  // A vizinha que assume fica na mesma posicao; na ponta direita, e a nova ultima.
  const ativa =
    estado.ativa === id ? (abas[Math.min(posicao, abas.length - 1)]?.id ?? null) : estado.ativa;
  return { ...estado, abas, ativa, aguardandoConfirmacao: null };
}

function marcarImpressao(estado: EstadoAbas, id: number, emAndamento: boolean): EstadoAbas {
  if (!estado.abas.some((aba) => aba.id === id)) return estado;
  const marcado: EstadoAbas = {
    ...estado,
    abas: estado.abas.map((aba) =>
      aba.id === id ? { ...aba, impressaoEmAndamento: emAndamento } : aba,
    ),
  };
  // Um pedido aberto so faz sentido enquanto ainda existe a impressao que o motivou.
  return {
    ...marcado,
    aguardandoConfirmacao:
      marcado.aguardandoConfirmacao === id && !emAndamento ? null : marcado.aguardandoConfirmacao,
    confirmandoJanela: marcado.confirmandoJanela && abasEmImpressao(marcado).length > 0,
  };
}

/** Muda uma aba so. Aba que ja fechou fica como esta: a resposta pode chegar depois do fechamento. */
function mudarAba(estado: EstadoAbas, id: number, mudar: (aba: Aba) => Aba): EstadoAbas {
  if (!estado.abas.some((aba) => aba.id === id)) return estado;
  return { ...estado, abas: estado.abas.map((aba) => (aba.id === id ? mudar(aba) : aba)) };
}

/**
 * O backend conta os lotes por impressora; a interface guarda o numero deles no documento. Um
 * trabalho anterior que ainda corre fica com os numeros que ja tinha, mesmo depois de outro envio.
 */
function comLotesDoDocumento(aba: Aba, novo: Cartao): Cartao {
  const parte = aba.divisao?.partes.find((item) => item.impressora === novo.impressora);
  if (parte) {
    return {
      ...novo,
      lotesNoDocumento: parte.lotes,
      ...(parte.faixas ? { faixasNoDocumento: parte.faixas } : {}),
    };
  }
  if (novo.estado.tipo === "preparando") return novo;
  const anterior = aba.cartoes.find((cartao) => cartao.impressora === novo.impressora);
  return anterior?.lotesNoDocumento
    ? {
        ...novo,
        lotesNoDocumento: anterior.lotesNoDocumento,
        ...(anterior.faixasNoDocumento ? { faixasNoDocumento: anterior.faixasNoDocumento } : {}),
      }
    : novo;
}

function divisaoAoDispensar(aba: Aba, impressora: string): DivisaoDoEnvio | null {
  const cartao = aba.cartoes.find((item) => item.impressora === impressora);
  if (!aba.divisao || !cartao) return aba.divisao;
  return {
    ...aba.divisao,
    partes: aba.divisao.partes.map((parte) =>
      parte.impressora === impressora
        ? { ...parte, semImprimirAoDispensar: lotesSemImprimir(cartao) }
        : parte,
    ),
  };
}

export function reduzirAbas(estado: EstadoAbas, acao: AcaoAbas): EstadoAbas {
  switch (acao.tipo) {
    case "abrir": {
      const id = estado.proximoId;
      const aba: Aba = {
        id,
        documento: acao.documento,
        impressaoEmAndamento: false,
        impressorasMarcadas: [],
        tamanhoLote: LOTE_PADRAO,
        intervalo: null,
        modo: "espelho",
        divisao: null,
        paginas: { estado: "contando" },
        cartoes: [],
      };
      return { ...estado, abas: [...estado.abas, aba], ativa: id, proximoId: id + 1 };
    }
    case "ativar":
      return estado.abas.some((aba) => aba.id === acao.id) ? { ...estado, ativa: acao.id } : estado;
    case "trocar": {
      if (estado.abas.length === 0) return estado;
      const posicao = estado.abas.findIndex((aba) => aba.id === estado.ativa);
      const proxima = estado.abas[(posicao + 1) % estado.abas.length];
      return proxima ? { ...estado, ativa: proxima.id } : estado;
    }
    case "pedir_fechamento": {
      const aba = estado.abas.find((candidata) => candidata.id === acao.id);
      if (!aba) return estado;
      return aba.impressaoEmAndamento
        ? { ...estado, aguardandoConfirmacao: aba.id }
        : fechar(estado, aba.id);
    }
    case "pedir_fechamento_janela":
      return decidirFechamentoJanela(estado) === "confirmar"
        ? { ...estado, aguardandoConfirmacao: null, confirmandoJanela: true }
        : estado;
    case "confirmar_fechamento":
      return estado.aguardandoConfirmacao === null
        ? estado
        : fechar(estado, estado.aguardandoConfirmacao);
    case "cancelar_fechamento":
      return { ...estado, aguardandoConfirmacao: null, confirmandoJanela: false };
    case "marcar_impressao":
      return marcarImpressao(estado, acao.id, acao.emAndamento);
    case "alternar_impressora":
      return mudarAba(estado, acao.id, (aba) => ({
        ...aba,
        impressorasMarcadas: aba.impressorasMarcadas.includes(acao.impressora)
          ? aba.impressorasMarcadas.filter((impressora) => impressora !== acao.impressora)
          : [...aba.impressorasMarcadas, acao.impressora],
      }));
    case "definir_lote": {
      if (!Number.isInteger(acao.tamanho)) return estado;
      const tamanhoLote = limitarAoIntervaloDoLote(acao.tamanho);
      return mudarAba(estado, acao.id, (aba) => ({ ...aba, tamanhoLote }));
    }
    case "definir_intervalo":
      return mudarAba(estado, acao.id, (aba) =>
        aba.paginas.estado === "conhecido"
          ? { ...aba, intervalo: normalizarRascunho(acao.intervalo, aba.paginas) }
          : aba,
      );
    case "definir_modo":
      return mudarAba(estado, acao.id, (aba) => ({ ...aba, modo: acao.modo }));
    case "aplicar_grupo":
      return mudarAba(estado, acao.id, (aba) => ({
        ...aba,
        impressorasMarcadas: acao.impressoras,
        modo: acao.modo,
      }));
    case "divisao_enviada":
      // Os numeros e a juncao dependem da divisao em curso: nenhum envio a apaga enquanto ela corre.
      return mudarAba(estado, acao.id, (aba) =>
        aba.divisao !== null && aba.cartoes.some(cartaoEmAndamento)
          ? aba
          : { ...aba, divisao: acao.divisao },
      );
    case "paginas_contadas":
      return mudarAba(estado, acao.id, (aba) => ({
        ...aba,
        paginas: { estado: "conhecido", total: acao.total },
      }));
    case "paginas_recusadas":
      return mudarAba(estado, acao.id, (aba) => ({
        ...aba,
        paginas: { estado: "recusado", motivo: acao.motivo },
      }));
    case "cartao": {
      const id = acao.cartao.aba;
      // Um pedido duplicado chega recusado enquanto o cartao do primeiro ainda esta vivo: a recusa nao o substitui.
      const vivo = estado.abas
        .find((candidata) => candidata.id === id)
        ?.cartoes.some(
          (cartao) => cartao.impressora === acao.cartao.impressora && cartaoEmAndamento(cartao),
        );
      if (vivo && ehRecusaDeOcupada(acao.cartao)) return estado;
      const depois = mudarAba(estado, id, (aba) => ({
        ...aba,
        cartoes: comCartao(aba.cartoes, comLotesDoDocumento(aba, acao.cartao)),
      }));
      const aba = depois.abas.find((candidata) => candidata.id === id);
      // O estado real dos trabalhos decide se ha impressao em andamento nesta aba.
      return aba ? marcarImpressao(depois, id, aba.cartoes.some(cartaoEmAndamento)) : depois;
    }
    case "dispensar_cartao": {
      const depois = mudarAba(estado, acao.id, (aba) => ({
        ...aba,
        divisao: divisaoAoDispensar(aba, acao.impressora),
        cartoes: aba.cartoes.filter((cartao) => cartao.impressora !== acao.impressora),
      }));
      const aba = depois.abas.find((candidata) => candidata.id === acao.id);
      return aba ? marcarImpressao(depois, acao.id, aba.cartoes.some(cartaoEmAndamento)) : depois;
    }
  }
}
