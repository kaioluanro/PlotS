import { idiomaAtual, type Chave, type Idioma } from "./i18n";
import { nomeDoModelo, type Modelo } from "./impressoras";
import { cartaoEmAndamento } from "./cartoes";
import { abasEmImpressao, type Aba, type EstadoAbas } from "./tabs";

export interface ConteudoConfirmacao {
  titulo: Chave;
  texto: Chave;
  /** Segundo paragrafo, quando o dialogo tem o que acrescentar ao texto. */
  consequencia: Chave | null;
  manter: Chave;
  fechar: Chave;
  parametros: Record<string, string>;
  ehJanela: boolean;
  /** O controle que abriu o dialogo continua na tela e recebe o foco tambem ao confirmar. */
  focoNaOrigemAoConfirmar: boolean;
  /** Linhas ja traduzidas que o texto apresenta, como uma lista. */
  linhas?: readonly string[];
  /** Observacoes ja traduzidas, abaixo da lista. */
  notas?: readonly string[];
  /** O botao de confirmar segue em frente em vez de encerrar algo. */
  confirmaAvancando?: boolean;
}

export interface PedidoDeSubstituicao {
  fila: string;
  modelo: Modelo;
}

/** O que o dialogo diz antes de um modelo tomar o lugar de um perfil personalizado. */
export function conteudoDaSubstituicao(
  pedido: PedidoDeSubstituicao,
  idioma: Idioma = idiomaAtual,
): ConteudoConfirmacao {
  return {
    titulo: "substituir.titulo",
    texto: "substituir.texto",
    consequencia: null,
    manter: "substituir.cancelar",
    fechar: "substituir.confirmar",
    parametros: { nome: pedido.fila, modelo: nomeDoModelo(pedido.modelo, idioma) },
    ehJanela: false,
    focoNaOrigemAoConfirmar: true,
  };
}

/** Sem nenhum envio real em andamento nao ha nada em fila para continuar la. */
function soSimulacao(abas: readonly Aba[]): boolean {
  const ativos = abas.flatMap((aba) => aba.cartoes.filter(cartaoEmAndamento));
  return ativos.length > 0 && ativos.every((cartao) => cartao.simulada);
}

/** Fechar a janela encerra o processo, e a passada que ainda estava indo para a fila para no meio. */
function haPassadaIndoParaAFila(abas: readonly Aba[]): boolean {
  return abas.some((aba) =>
    aba.cartoes.some((cartao) => !cartao.simulada && cartao.estado.tipo === "imprimindo"),
  );
}

/** Monta o que o dialogo de fechamento diz; `null` quando nada espera resposta. */
export function conteudoDaConfirmacao(estado: EstadoAbas): ConteudoConfirmacao | null {
  if (estado.confirmandoJanela) {
    const imprimindo = abasEmImpressao(estado);
    const unica = imprimindo.length === 1 ? imprimindo[0] : undefined;
    if (soSimulacao(imprimindo)) {
      return {
        titulo: "cartao.fechar.janelaTituloSimulacao",
        texto: unica ? "cartao.fechar.textoSimulacao" : "cartao.fechar.janelaTextoSimulacaoVarios",
        consequencia: "cartao.fechar.consequenciaSimulacao",
        manter: "confirmar.janela.manter",
        fechar: "confirmar.janela.fechar",
        parametros: unica ? { nome: unica.documento.nome } : { n: String(imprimindo.length) },
        ehJanela: true,
        focoNaOrigemAoConfirmar: false,
      };
    }
    if (haPassadaIndoParaAFila(imprimindo)) {
      return {
        titulo: "confirmar.janela.titulo",
        texto: unica ? "confirmar.texto" : "confirmar.janela.textoVarios",
        consequencia: "cartao.fechar.consequenciaEnviando",
        manter: "confirmar.janela.manter",
        fechar: "confirmar.janela.fechar",
        parametros: unica ? { nome: unica.documento.nome } : { n: String(imprimindo.length) },
        ehJanela: true,
        focoNaOrigemAoConfirmar: false,
      };
    }
    return {
      titulo: "confirmar.janela.titulo",
      texto: unica ? "confirmar.texto" : "confirmar.janela.textoVarios",
      consequencia: unica ? "confirmar.consequencia" : "confirmar.consequenciaVarios",
      manter: "confirmar.janela.manter",
      fechar: "confirmar.janela.fechar",
      parametros: unica ? { nome: unica.documento.nome } : { n: String(imprimindo.length) },
      ehJanela: true,
      focoNaOrigemAoConfirmar: false,
    };
  }
  const aba = estado.abas.find((candidata) => candidata.id === estado.aguardandoConfirmacao);
  if (!aba) return null;
  if (soSimulacao([aba])) {
    return {
      titulo: "cartao.fechar.tituloSimulacao",
      texto: "cartao.fechar.textoSimulacao",
      consequencia: "cartao.fechar.consequenciaSimulacao",
      manter: "confirmar.manter",
      fechar: "confirmar.fechar",
      parametros: { nome: aba.documento.nome },
      ehJanela: false,
      focoNaOrigemAoConfirmar: false,
    };
  }
  return {
    titulo: "confirmar.titulo",
    texto: "confirmar.texto",
    consequencia: "confirmar.consequencia",
    manter: "confirmar.manter",
    fechar: "confirmar.fechar",
    parametros: { nome: aba.documento.nome },
    ehJanela: false,
    focoNaOrigemAoConfirmar: false,
  };
}

/** A confirmacao antes de um envio real: quem recebe, as paginas e o papel. */
export function conteudoDoEnvio(linhas: readonly string[], notas: readonly string[]): ConteudoConfirmacao {
  return {
    titulo: "cartao.envio.titulo",
    texto: "cartao.envio.texto",
    consequencia: null,
    manter: "cartao.envio.voltar",
    fechar: "cartao.envio.enviar",
    parametros: {},
    ehJanela: false,
    focoNaOrigemAoConfirmar: true,
    linhas,
    notas,
    confirmaAvancando: true,
  };
}

/** Cancelar tira o trabalho da fila, entao sempre pergunta antes. */
export function conteudoDoCancelamento(nome: string, simulada: boolean): ConteudoConfirmacao {
  return {
    titulo: "cartao.cancelarDialogo.titulo",
    texto: simulada ? "cartao.cancelarDialogo.textoSimulado" : "cartao.cancelarDialogo.texto",
    consequencia: null,
    manter: "cartao.cancelarDialogo.manter",
    fechar: "cartao.cancelarDialogo.confirmar",
    parametros: { nome },
    ehJanela: false,
    focoNaOrigemAoConfirmar: true,
  };
}

/** Reimprimir cancela o que resta do lote na fila e envia a frente de novo: sempre pergunta antes. */
export function conteudoDaReimpressao(nome: string, lote: number, total: number): ConteudoConfirmacao {
  return {
    titulo: "cartao.reimprimirDialogo.titulo",
    texto: "cartao.reimprimirDialogo.texto",
    consequencia: null,
    manter: "cartao.envio.voltar",
    fechar: "cartao.reimprimir",
    parametros: { nome, n: String(lote), total: String(total) },
    ehJanela: false,
    focoNaOrigemAoConfirmar: true,
  };
}
