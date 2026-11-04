import { idiomaAtual, traduzir, type Chave, type Idioma } from "./i18n";
import { nomeDoModelo, type Modelo } from "./impressoras";
import { cartaoEmAndamento, copiaDoLote, textoDasFolhas, type Cartao } from "./cartoes";
import type { DesenhoDoGesto } from "./cartoes-gesto";
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
  /** A frase e a figura do gesto de cada impressora, antes de pedir os versos. */
  gestos?: readonly GestoNaConfirmacao[];
  /** Observacoes ja traduzidas, abaixo da lista. */
  notas?: readonly string[];
  /** Aviso ja traduzido logo abaixo do texto, antes da lista: o que o envio tem de diferente. */
  aviso?: string;
  /** O botao de confirmar segue em frente em vez de encerrar algo. */
  confirmaAvancando?: boolean;
}

export interface GestoNaConfirmacao {
  nome: string;
  frases: readonly string[];
  desenho: DesenhoDoGesto | null;
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

/** Ha frente saindo ou pronta, e os versos ainda nao foram pedidos. */
function esperaVersos(cartao: Cartao): boolean {
  const { estado } = cartao;
  return (
    !cartao.loteAutomatico &&
    (estado.tipo === "frentes_prontas" ||
      (cartao.alcance === "so_frentes" &&
        (estado.tipo === "imprimindo" || estado.tipo === "aviso" || estado.tipo === "proximo_lote")))
  );
}

/** Fechar perde o cartao: quem tem versos por imprimir precisa saber antes, e como continuar depois. */
function comVersosPorImprimir(
  conteudo: ConteudoConfirmacao | null,
  abas: readonly Aba[],
  idioma: Idioma,
): ConteudoConfirmacao | null {
  if (conteudo === null || !abas.some((aba) => aba.cartoes.some(esperaVersos))) return conteudo;
  return { ...conteudo, notas: [traduzir(idioma, "face.fechar.versos"), ...(conteudo.notas ?? [])] };
}

/** Monta o que o dialogo de fechamento diz; `null` quando nada espera resposta. */
export function conteudoDaConfirmacao(
  estado: EstadoAbas,
  idioma: Idioma = idiomaAtual,
): ConteudoConfirmacao | null {
  const emJogo = estado.confirmandoJanela
    ? estado.abas
    : estado.abas.filter((aba) => aba.id === estado.aguardandoConfirmacao);
  return comVersosPorImprimir(conteudoSemVersos(estado), emJogo, idioma);
}

function conteudoSemVersos(estado: EstadoAbas): ConteudoConfirmacao | null {
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
export function conteudoDoEnvio(
  linhas: readonly string[],
  notas: readonly string[],
  temAutomatica = false,
  soSimuladas = false,
): ConteudoConfirmacao {
  return {
    titulo: soSimuladas
      ? "modo.automatico.simulacaoTitulo"
      : temAutomatica
        ? "modo.automatico.envioTitulo"
        : "cartao.envio.titulo",
    texto: "cartao.envio.texto",
    consequencia: null,
    manter: "cartao.envio.voltar",
    fechar: soSimuladas
      ? "modo.automatico.simulacaoConfirmar"
      : temAutomatica
        ? "modo.automatico.envioConfirmar"
        : "cartao.envio.enviar",
    parametros: {},
    ehJanela: false,
    focoNaOrigemAoConfirmar: true,
    linhas,
    notas,
    confirmaAvancando: true,
  };
}

/** Apagar um grupo nao toca em impressora nenhuma, mas nao tem volta: pergunta antes. */
export function conteudoDoApagarGrupo(nome: string): ConteudoConfirmacao {
  return {
    titulo: "grupo.apagarDialogo.titulo",
    texto: "grupo.apagarDialogo.texto",
    consequencia: null,
    manter: "cartao.envio.voltar",
    fechar: "grupo.apagarDialogo.confirmar",
    parametros: { nome },
    ehJanela: false,
    focoNaOrigemAoConfirmar: true,
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

/** Reimprimir tira da fila o que resta do trecho e envia de novo so o que foi escolhido: sempre pergunta. */
export function conteudoDaReimpressao(
  nome: string,
  cartao: Cartao,
  idioma: Idioma = idiomaAtual,
): ConteudoConfirmacao {
  const copia = copiaDoLote(cartao);
  return {
    titulo: copia === null ? "cartao.reimprimirDialogo.titulo" : "cartao.copia.reimprimir",
    texto:
      cartao.alcance === "so_versos"
        ? "cartao.reimprimirDialogo.versos"
        : "cartao.reimprimirDialogo.texto",
    consequencia: null,
    manter: "cartao.envio.voltar",
    fechar: "cartao.reimprimir",
    parametros: {
      nome,
      folhas: textoDasFolhas(cartao.folhasDoLote ?? 0, idioma),
      ...(copia === null ? {} : { n: String(copia), total: String(cartao.copias ?? 1) }),
    },
    ehJanela: false,
    focoNaOrigemAoConfirmar: true,
  };
}
