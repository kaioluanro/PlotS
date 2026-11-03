import { idiomaAtual, traduzir, type Chave, type Idioma } from "./i18n";
import { nomeDeExibicao, nomeDoModelo, type Impressora, type Modelo } from "./impressoras";
import {
  cartaoEmAndamento,
  linhasDoEnvio,
  listaDeFora,
  loteNoDocumento,
  lotesSemImprimir,
  motivoDeFora,
  nomeDoPapel,
  textoDasPaginas,
  type Cartao,
  type EstadoDoEnvio,
  type FaixaDoLote,
  type PapelDoCartao,
  type ResumoDoEnvio,
} from "./cartoes";
import { linhasComTrecho, type LeituraIntervalo } from "./intervalo";
import {
  abasEmImpressao,
  type Aba,
  type DivisaoDoEnvio,
  type EstadoAbas,
  type ParteDaDivisao,
} from "./tabs";

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
  /** Na divisao, cada lote com a sua impressora, na ordem em que os macos se empilham. */
  pilha?: OrdemDaPilha;
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

/** Na divisao, o backend diz o que cabe a cada impressora, na ordem da selecao. */
export interface ParteDoResumo {
  impressora: string;
  lotes: number[];
  viraParaLer: boolean | null;
}

export type ResumoComDivisao = ResumoDoEnvio & {
  divisao: ParteDoResumo[];
  paginasDoDocumento: number;
  faixas: FaixaDoLote[];
};

/** As partes com o nome de cada impressora; as que nao tem lote ficam so na confirmacao. */
export function divisaoDoResumo(
  destinos: readonly Impressora[],
  resumo: ResumoComDivisao,
  idioma: Idioma = idiomaAtual,
): ParteDaDivisao[] {
  return resumo.divisao.flatMap((parte) => {
    const destino = destinos.find((item) => item.id === parte.impressora);
    if (!destino) return [];
    return [
      {
        ...parte,
        nome: nomeDeExibicao(destino, idioma),
        faixas: resumo.faixas.filter((faixa) => parte.lotes.includes(faixa.lote)),
      },
    ];
  });
}

export interface LinhaDaPilha {
  lote: number;
  paginas: string;
  impressora: string;
  /** "vire" ou "nao vire"; sem valor quando todos os lotes pedem o mesmo gesto. */
  gesto: string | null;
}

export interface OrdemDaPilha {
  linhas: readonly LinhaDaPilha[];
  /** Ha uma coluna de gesto: os lotes pedem gestos diferentes. */
  comGesto: boolean;
  /** A coluna de gesto tem cabecalho: so na confirmacao, antes de a pessoa imprimir. */
  cabecalhoDoGesto: boolean;
  /** O gesto comum, ou como ler a coluna. */
  frases: readonly string[];
  resumo: string;
}

const CHAVE_DO_GESTO = (vira: boolean | null): Chave =>
  vira === null ? "modo.pilha.confira" : vira ? "modo.pilha.vire" : "modo.pilha.naoVire";

/** Na confirmacao, o gesto ainda vai acontecer; na juncao, a pessoa ja esta com os macos na mao. */
export type UsoDaPilha = "confirmacao" | "juncao";

const FRASES_COMUNS: Record<UsoDaPilha, { confere: Chave; vira: Chave; naoVira: Chave }> = {
  confirmacao: {
    confere: "modo.pilha.todosConferem",
    vira: "modo.pilha.todosViram",
    naoVira: "modo.pilha.nenhumVira",
  },
  juncao: {
    confere: "modo.juntar.todosConferem",
    vira: "modo.juntar.todosViram",
    naoVira: "modo.juntar.nenhumVira",
  },
};

function frasesDoGestoDaPilha(
  partes: readonly ParteDaDivisao[],
  idioma: Idioma,
  uso: UsoDaPilha,
): string[] {
  const gestos = new Set(partes.map((parte) => parte.viraParaLer));
  if (gestos.size > 1) return gestos.has(null) ? [traduzir(idioma, "modo.pilha.conferir")] : [];
  const unico = partes[0]?.viraParaLer ?? null;
  const frases = FRASES_COMUNS[uso];
  if (unico === null) return [traduzir(idioma, frases.confere)];
  return [traduzir(idioma, unico ? frases.vira : frases.naoVira)];
}

function textoDoResumoDaPilha(
  paginas: number,
  lotes: number,
  papeis: readonly PapelDoCartao[],
  idioma: Idioma,
): string {
  const texto = traduzir(idioma, "modo.pilha.resumo", {
    paginas: traduzir(idioma, paginas === 1 ? "modo.pilha.qtdPaginas.um" : "modo.pilha.qtdPaginas.varios", {
      n: String(paginas),
    }),
    lotes: traduzir(idioma, lotes === 1 ? "modo.pilha.qtdLotes.um" : "modo.pilha.qtdLotes.varios", {
      n: String(lotes),
    }),
    papeis: papeis.map((papel) => nomeDoPapel(papel, idioma)).join(", "),
  });
  // Sem papel conhecido, a linha termina na contagem, sem o separador solto.
  return papeis.length === 0 ? texto.replace(/ · $/, "") : texto;
}

/** Cada lote, do primeiro ao ultimo, com as paginas e a impressora de onde ele sai. */
export function ordemDaPilha(
  partes: readonly ParteDaDivisao[],
  paginas: number,
  papeis: readonly PapelDoCartao[],
  idioma: Idioma = idiomaAtual,
  uso: UsoDaPilha = "confirmacao",
): OrdemDaPilha {
  const comLote = partes.filter((parte) => parte.lotes.length > 0);
  const comGesto = new Set(comLote.map((parte) => parte.viraParaLer)).size > 1;
  const linhas = comLote
    .flatMap((parte) =>
      parte.lotes.map((lote) => {
        const faixa = parte.faixas?.find((item) => item.lote === lote);
        return {
          lote,
          paginas: faixa ? textoDasPaginas(faixa, idioma) : "",
          impressora: parte.nome,
          gesto: comGesto ? traduzir(idioma, CHAVE_DO_GESTO(parte.viraParaLer)) : null,
        };
      }),
    )
    .sort((a, b) => a.lote - b.lote);
  return {
    linhas,
    comGesto,
    cabecalhoDoGesto: comGesto && uso === "confirmacao",
    frases: frasesDoGestoDaPilha(comLote, idioma, uso),
    resumo: textoDoResumoDaPilha(paginas, linhas.length, papeis, idioma),
  };
}

/** Um lote so nao se divide nem se junta: uma imprime tudo, e as outras ficam de fora. */
function conteudoDeLoteUnico(
  comLote: ParteDaDivisao,
  semLote: readonly ParteDaDivisao[],
  linhas: readonly string[],
  notas: readonly string[],
): ConteudoConfirmacao {
  return {
    titulo: "modo.unico.titulo",
    texto: semLote.length === 1 ? "modo.unico.texto.um" : "modo.unico.texto.varios",
    consequencia: null,
    manter: "cartao.envio.voltar",
    fechar: "modo.unico.confirmar",
    parametros: { nome: comLote.nome, outras: semLote.map((parte) => parte.nome).join(", ") },
    ehJanela: false,
    focoNaOrigemAoConfirmar: true,
    linhas,
    notas,
    confirmaAvancando: true,
  };
}

/** A confirmacao da divisao: de onde sai cada lote, quem fica de fora e como juntar os macos. */
export function conteudoDaDivisao(
  destinos: readonly Impressora[],
  resumo: ResumoComDivisao,
  leitura: LeituraIntervalo | null = null,
  idioma: Idioma = idiomaAtual,
): ConteudoConfirmacao {
  const partes = divisaoDoResumo(destinos, resumo, idioma);
  const comLote = partes.filter((parte) => parte.lotes.length > 0);
  const semLote = partes.filter((parte) => parte.lotes.length === 0);
  const envio = linhasDoEnvio(destinos, resumo, idioma);
  const reais = destinos.filter((destino) => destino.tipo === "fila").length;
  const aviso = traduzir(idioma, "cartao.envio.simuladas");
  // So com simuladas nao existe a fila real a que o aviso se compara.
  const notasDoEnvio = envio.notas.filter((nota) => reais > 0 || nota !== aviso);
  const pilha = ordemDaPilha(comLote, resumo.paginas, resumo.papeis, idioma);
  const trecho = leitura === null ? [] : linhasComTrecho(["", ""], leitura, idioma).slice(0, -2);
  const unica = comLote[0];
  if (resumo.lotes === 1 && unica !== undefined && comLote.length === 1) {
    return conteudoDeLoteUnico(unica, semLote, [...trecho, pilha.resumo], notasDoEnvio);
  }
  return {
    titulo: "modo.envio.titulo",
    texto: "modo.envio.texto",
    consequencia: null,
    manter: "cartao.envio.voltar",
    fechar: "modo.envio.confirmar",
    parametros: {},
    ehJanela: false,
    focoNaOrigemAoConfirmar: true,
    linhas: trecho,
    pilha,
    notas: [
      ...semLote.map((parte) => traduzir(idioma, "modo.envio.semLote", { nome: parte.nome })),
      traduzir(idioma, "modo.juntar.separar"),
      traduzir(idioma, "modo.juntar.conferencia"),
      ...notasDoEnvio,
    ],
    confirmaAvancando: true,
  };
}

/**
 * Regras do envio que so a divisao tem. Com uma divisao em curso, nada se envia desta aba: o envio
 * novo trocaria os numeros dos cartoes e a juncao. No modo divisao, menos de 2 destinos nao
 * dividem nada: quem quer enviar a uma so desmarca a outra, e o seletor e o "(1)" voltam a valer.
 */
export function envioComDivisao(
  envio: EstadoDoEnvio,
  aba: Aba | null,
  lista: readonly Impressora[],
  ocupadas: ReadonlySet<string>,
  idioma: Idioma = idiomaAtual,
): EstadoDoEnvio {
  if (aba === null) return envio;
  const marcadas = lista.filter((impressora) => aba.impressorasMarcadas.includes(impressora.id));
  const soSimuladas = marcadas.length > 0 && marcadas.every((item) => item.tipo === "simulada");
  const rotulo = traduzir(idioma, soSimuladas ? "cartao.simular.frente" : "imprimir.frente");
  if (aba.divisao !== null && aba.cartoes.some(cartaoEmAndamento)) {
    return {
      ...envio,
      rotulo,
      motivo: traduzir(idioma, "modo.envio.divisaoEmCurso"),
      desabilitado: true,
      semDestino: true,
    };
  }
  const contando = aba.paginas.estado === "conhecido";
  if (aba.modo !== "divisao" || marcadas.length < 2 || envio.destinos.length >= 2 || !contando) {
    return envio;
  }
  const fora = marcadas.filter((item) => motivoDeFora(item, ocupadas, idioma) !== null);
  const soOcupadas = fora.every((item) => ocupadas.has(item.id));
  const motivos = fora.map((item) =>
    soOcupadas ? nomeDeExibicao(item, idioma) : (motivoDeFora(item, ocupadas, idioma) ?? ""),
  );
  const chave: Chave = !soOcupadas
    ? "modo.envio.foraDaConta"
    : fora.length === 1
      ? "modo.envio.ocupada"
      : "modo.envio.ocupadas";
  return {
    ...envio,
    rotulo,
    motivo: traduzir(idioma, chave, { lista: listaDeFora(motivos, idioma) }),
    desabilitado: true,
  };
}

export interface JuncaoDaAba {
  /** Quais lotes nao sairam, e de que impressora. */
  faltas: string[];
  /** O que saiu, na ordem da pilha. */
  pilha: OrdemDaPilha;
  conferencia: string;
}

function faltasDaParte(
  parte: ParteDaDivisao,
  cartao: Cartao | undefined,
): readonly number[] | null {
  if (cartao) {
    if (cartaoEmAndamento(cartao) || !cartao.lotesNoDocumento) return null;
    return lotesSemImprimir(cartao);
  }
  return parte.semImprimirAoDispensar ?? null;
}

function fraseDaFalta(parte: ParteDaDivisao, faltantes: readonly number[], idioma: Idioma): string {
  const faixas = (parte.faixas ?? []).filter((faixa) => faltantes.includes(faixa.lote));
  const unica = faixas[0];
  const paginas =
    faixas.length === 1 && unica !== undefined
      ? textoDasPaginas(unica, idioma)
      : traduzir(idioma, "modo.pilha.paginas.lista", {
          lista: faixas
            .map((faixa) =>
              faixa.inicio === faixa.fim ? String(faixa.inicio) : `${faixa.inicio}–${faixa.fim}`,
            )
            .join(", "),
        });
  return traduzir(idioma, faltantes.length === 1 ? "modo.juntar.falta.um" : "modo.juntar.falta.varios", {
    lista: faltantes.join(", "),
    nome: parte.nome,
    paginas,
  });
}

const paginasDaFaixa = (faixa: FaixaDoLote) => faixa.fim - faixa.inicio + 1;

/** Enquanto algum trabalho da divisao segue, sem cartao na tela ou sem nada impresso, nao ha o que juntar. */
export function juncaoDaAba(aba: Aba, idioma: Idioma = idiomaAtual): JuncaoDaAba | null {
  const divisao: DivisaoDoEnvio | null = aba.divisao;
  if (divisao === null || aba.cartoes.length === 0) return null;
  // Um lote so nao se junta com nada.
  if (divisao.partes.reduce((soma, parte) => soma + parte.lotes.length, 0) <= 1) return null;
  const faltas: string[] = [];
  const impressas: ParteDaDivisao[] = [];
  for (const parte of divisao.partes) {
    const cartao = aba.cartoes.find((item) => item.impressora === parte.impressora);
    const faltantes = faltasDaParte(parte, cartao);
    if (faltantes === null) return null;
    if (faltantes.length > 0) faltas.push(fraseDaFalta(parte, faltantes, idioma));
    impressas.push({
      ...parte,
      lotes: parte.lotes.filter((lote) => !faltantes.includes(lote)),
    });
  }
  if (impressas.every((parte) => parte.lotes.length === 0)) return null;
  const paginas = impressas
    .flatMap((parte) => (parte.faixas ?? []).filter((faixa) => parte.lotes.includes(faixa.lote)))
    .reduce((soma, faixa) => soma + paginasDaFaixa(faixa), 0);
  return {
    faltas,
    pilha: ordemDaPilha(impressas, paginas, divisao.papeis ?? [], idioma, "juncao"),
    conferencia: traduzir(idioma, "modo.juntar.conferencia"),
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

/** Reimprimir cancela o que resta do lote na fila e envia a frente de novo: sempre pergunta antes. */
export function conteudoDaReimpressao(
  nome: string,
  lote: number,
  total: number,
  cartao?: Cartao,
): ConteudoConfirmacao {
  // Na divisao, o numero que importa e o do documento, nao a contagem desta impressora.
  const noDocumento = cartao ? loteNoDocumento(cartao) : null;
  if (noDocumento !== null) {
    return {
      titulo: "modo.reimprimir.titulo",
      texto: "cartao.reimprimirDialogo.texto",
      consequencia: null,
      manter: "cartao.envio.voltar",
      fechar: "cartao.reimprimir",
      parametros: { nome, n: String(noDocumento) },
      ehJanela: false,
      focoNaOrigemAoConfirmar: true,
    };
  }
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
