import { cartaoSeguraAAba, comCartao, ehRecusaDeOcupada, cartaoEmAndamento, type Cartao } from "./cartoes";
import {
  lerIntervaloDaAba,
  normalizarRascunho,
  paginasDoResumo,
  type RascunhoIntervalo,
  type TrechoDoEnvio,
} from "./intervalo";

export interface Documento {
  caminho: string;
  nome: string;
  /** O arquivo aberto; abas antigas e testes podem omitir. */
  tipo?: "pdf" | "imagem";
}

/** Total de paginas da aba, ou o motivo de o PDF ter sido recusado, ja traduzido. */
export type PaginasDaAba =
  | { estado: "contando" }
  | { estado: "conhecido"; total: number }
  | { estado: "recusado"; motivo: string };

export type PapelDeAjuste = "a4" | "a5" | "carta" | "oficio" | "legal";

/** O que o proximo envio imprime, em todas as marcadas. So a pessoa muda. */
export type FaceDoEnvio = "frente" | "verso";

/** Com o que saiu a ultima frente desta aba: o verso tem de usar o mesmo trecho e o mesmo papel. */
export interface FrenteEnviada {
  trecho: TrechoDoEnvio | null;
  papel: PapelDeAjuste | null;
}

/** Cópias do documento por face. Trocar a face não muda nenhum dos dois números. */
export interface CopiasDaAba {
  frente: number;
  verso: number;
}

/** Maior número de cópias que a janela aceita. */
export const COPIAS_MAXIMAS = 999;

export function ehQuantidadeDeCopias(valor: number): boolean {
  return Number.isInteger(valor) && valor >= 1 && valor <= COPIAS_MAXIMAS;
}

/** Le o texto digitado no campo: so digitos, com espacos em volta; qualquer outra coisa nao e numero. */
export function lerQuantidadeDeCopias(texto: string): number | null {
  const limpo = texto.trim();
  if (!/^\d{1,4}$/.test(limpo)) return null;
  const numero = Number(limpo);
  return ehQuantidadeDeCopias(numero) ? numero : null;
}

/** Abas sem o campo (antes da quantidade) valem uma cópia. */
export function copiasDaFace(aba: Pick<Aba, "copias">, face: FaceDoEnvio): number {
  return aba.copias?.[face] ?? 1;
}

export interface Aba {
  id: number;
  documento: Documento;
  impressaoEmAndamento: boolean;
  /** Identificadores das impressoras marcadas neste documento. */
  impressorasMarcadas: readonly string[];
  /** De qual a qual pagina, como foi escrito; `null` e o documento inteiro. */
  intervalo: RascunhoIntervalo | null;
  face: FaceDoEnvio;
  papelDeAjuste?: PapelDeAjuste | null;
  /** Escreve o lote e a folha no verso de cada folha. */
  controle?: boolean;
  /** A ultima frente enviada desta aba. */
  frenteEnviada: FrenteEnviada | null;
  /** Quem ja entregou as frentes desta aba: mandar de novo gasta papel, e a confirmacao avisa. */
  frentesProntas: readonly string[];
  /** Quantas cópias sai de cada face. O Verso começa com o número da Frente. */
  copias?: CopiasDaAba;
  paginas: PaginasDaAba;
  /** Um cartao por impressora; trocar de aba troca a lista, e os trabalhos seguem. */
  cartoes: readonly Cartao[];
}

/**
 * O envio da aba tem verso. Um trabalho de uma página só tem a frente; um trecho de uma página
 * também. Sem o total conhecido, a interface não esconde a escolha antes de saber.
 */
export function temVersoDaAba(aba: Aba | null): boolean {
  if (aba === null) return false;
  const quantidade = paginasDoResumo(lerIntervaloDaAba(aba));
  return quantidade === null || quantidade > 1;
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
  | { tipo: "definir_intervalo"; id: number; intervalo: RascunhoIntervalo | null }
  | { tipo: "definir_face"; id: number; face: FaceDoEnvio }
  | { tipo: "definir_copias"; id: number; face: FaceDoEnvio; valor: number }
  | { tipo: "definir_papel_ajuste"; id: number; papel: PapelDeAjuste | null }
  | { tipo: "definir_controle"; id: number; ligado: boolean }
  | { tipo: "aplicar_grupo"; id: number; impressoras: readonly string[] }
  | { tipo: "frente_enviada"; id: number; frente: FrenteEnviada }
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

export function reduzirAbas(estado: EstadoAbas, acao: AcaoAbas): EstadoAbas {
  switch (acao.tipo) {
    case "abrir": {
      const id = estado.proximoId;
      const aba: Aba = {
        id,
        documento: acao.documento,
        impressaoEmAndamento: false,
        impressorasMarcadas: [],
        intervalo: null,
        face: "frente",
        papelDeAjuste: null,
        controle: false,
        frenteEnviada: null,
        frentesProntas: [],
        copias: { frente: 1, verso: 1 },
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
    case "definir_intervalo":
      return mudarAba(estado, acao.id, (aba) => {
        if (aba.paginas.estado !== "conhecido") return aba;
        const depois = { ...aba, intervalo: normalizarRascunho(acao.intervalo, aba.paginas) };
        return temVersoDaAba(depois) ? depois : { ...depois, face: "frente" };
      });
    case "definir_face":
      return mudarAba(estado, acao.id, (aba) =>
        acao.face === "verso" && !temVersoDaAba(aba) ? aba : { ...aba, face: acao.face },
      );
    case "definir_copias":
      return mudarAba(estado, acao.id, (aba) => {
        if (!ehQuantidadeDeCopias(acao.valor)) return aba;
        const atuais = aba.copias ?? { frente: 1, verso: 1 };
        // A Frente leva o numero ao Verso; o Verso muda so o seu.
        return {
          ...aba,
          copias:
            acao.face === "frente"
              ? { frente: acao.valor, verso: acao.valor }
              : { ...atuais, verso: acao.valor },
        };
      });
    case "definir_papel_ajuste":
      return mudarAba(estado, acao.id, (aba) => ({ ...aba, papelDeAjuste: acao.papel }));
    case "definir_controle":
      return mudarAba(estado, acao.id, (aba) => ({ ...aba, controle: acao.ligado }));
    case "aplicar_grupo":
      return mudarAba(estado, acao.id, (aba) => ({ ...aba, impressorasMarcadas: acao.impressoras }));
    case "frente_enviada":
      return mudarAba(estado, acao.id, (aba) => ({ ...aba, frenteEnviada: acao.frente }));
    case "paginas_contadas":
      return mudarAba(estado, acao.id, (aba) => {
        const depois = {
          ...aba,
          paginas: { estado: "conhecido" as const, total: acao.total },
        };
        return temVersoDaAba(depois) ? depois : { ...depois, face: "frente" };
      });
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
        cartoes: comCartao(aba.cartoes, acao.cartao),
        frentesProntas:
          acao.cartao.estado.tipo === "frentes_prontas" && !aba.frentesProntas.includes(acao.cartao.impressora)
            ? [...aba.frentesProntas, acao.cartao.impressora]
            : aba.frentesProntas,
      }));
      const aba = depois.abas.find((candidata) => candidata.id === id);
      // O estado real dos trabalhos decide se ha o que perder ao fechar esta aba.
      return aba ? marcarImpressao(depois, id, aba.cartoes.some(cartaoSeguraAAba)) : depois;
    }
    case "dispensar_cartao": {
      const depois = mudarAba(estado, acao.id, (aba) => ({
        ...aba,
        cartoes: aba.cartoes.filter((cartao) => cartao.impressora !== acao.impressora),
      }));
      const aba = depois.abas.find((candidata) => candidata.id === acao.id);
      return aba ? marcarImpressao(depois, acao.id, aba.cartoes.some(cartaoSeguraAAba)) : depois;
    }
  }
}
