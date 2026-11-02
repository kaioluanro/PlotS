import type { Chave } from "./i18n";
import { abasEmImpressao, type EstadoAbas } from "./tabs";

export interface ConteudoConfirmacao {
  titulo: Chave;
  texto: Chave;
  consequencia: Chave;
  manter: Chave;
  fechar: Chave;
  parametros: Record<string, string>;
  ehJanela: boolean;
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
  };
}
