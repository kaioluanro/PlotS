import { idiomaAtual, type Chave, type Idioma } from "./i18n";
import { nomeDoModelo, type Modelo } from "./impressoras";
import { abasEmImpressao, type EstadoAbas } from "./tabs";

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

/** Monta o que o dialogo de fechamento diz; `null` quando nada espera resposta. */
export function conteudoDaConfirmacao(estado: EstadoAbas): ConteudoConfirmacao | null {
  if (estado.confirmandoJanela) {
    const imprimindo = abasEmImpressao(estado);
    const unica = imprimindo.length === 1 ? imprimindo[0] : undefined;
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
