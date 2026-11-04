import type { ComparacaoDoEnvio } from "./opcoes-envio";
import { idiomaAtual, traduzir, type Chave, type Idioma } from "./i18n";
import { nomeDeExibicao, textoDoEstado, type Impressora } from "./impressoras";
import { filasComAjusteAgora, linhaDoAjusteNoEnvio } from "./preferencias";
import type { Aba } from "./tabs";

export type Passada = "frente" | "verso" | "duas_faces";

export interface Motivo {
  codigo: string;
  texto: string;
}

export type EstadoCartao =
  | { tipo: "preparando" }
  | { tipo: "imprimindo"; passada: Passada }
  | { tipo: "aguardando" }
  | { tipo: "frentes_prontas" }
  | { tipo: "proximo_lote" }
  | {
      tipo: "aviso";
      passada: Passada;
      motivo: Motivo;
      /** O que o proprio driver disse, ja limpo pelo backend. Nunca sai de dentro do motivo. */
      mensagemDoDriver?: string | null;
      respondido: boolean;
    }
  | { tipo: "erro"; motivo: Motivo }
  | { tipo: "concluido" }
  | { tipo: "cancelado" };

export interface PapelDoCartao {
  formato: string;
  larguraMm: number;
  alturaMm: number;
}

export interface Recolocar {
  lado: string | null;
  virada: string | null;
  textoDoPerfil: string | null;
}

export interface FaceDaSequencia {
  numero: number | null;
  girada: boolean;
}

export type AlcanceDoCartao = "so_frentes" | "so_versos" | "frente_e_verso";

/** O que a interface recebe de cada trabalho; toda decisao fica no backend. */
export interface Cartao {
  aba: number;
  impressora: string;
  nome: string;
  simulada: boolean;
  lote: number;
  total: number;
  papel: PapelDoCartao | null;
  estado: EstadoCartao;
  recolocar: Recolocar | null;
  trocaDePapel: PapelDoCartao | null;
  resultado: { corretas: number; folhas: number } | null;
  sequencia: { frente: FaceDaSequencia[]; verso: FaceDaSequencia[]; marcas?: string[] } | null;
  viraParaLer: boolean;
  /** A copia que o lote atual imprime, a partir de 1; nulo quando o lote mistura copias. */
  copia?: number | null;
  /** Quantas copias o trabalho inteiro imprime. */
  copias?: number;
  /** O que este trabalho imprime. */
  alcance?: AlcanceDoCartao;
  /** Folhas do trecho atual: e o que sai de novo ao reimprimir. */
  folhasDoLote?: number;
  /** As paginas do trecho atual: e por elas que a pessoa acha o maco. */
  faixaDoLote?: FaixaDoLote | null;
  /** O driver nao aceitou o ajuste salvo: este trabalho saiu com o padrao da impressora. */
  ajusteRecusado?: boolean;
  loteAutomatico?: boolean;
  anteriorAutomatico?: boolean;
  temAutomatico?: boolean;
}

/** De qual a qual pagina vai um lote, com os numeros do documento. */
export interface FaixaDoLote {
  lote: number;
  inicio: number;
  fim: number;
}

export type Comando =
  | "enviar_frente"
  | "enviar_verso"
  | "reimprimir"
  | "cancelar"
  | "continuar_esperando";

/** Motivos de erro que encerram o cartao sem nenhum trabalho na fila para acompanhar. */
const MOTIVOS_SEM_TRABALHO: ReadonlySet<string> = new Set([
  "interrompido",
  "ocupada",
  "documento_mudou",
  "duas_faces_recusadas",
]);

/**
 * Concluido e cancelado acabaram: o resto ainda depende de alguem ou da fila. Um cartao cuja
 * thread morreu, ou que foi recusado ao iniciar, nao espera nada: nao pode segurar a impressora.
 */
export function cartaoEmAndamento(cartao: Cartao): boolean {
  const { estado } = cartao;
  if (estado.tipo === "concluido" || estado.tipo === "cancelado") return false;
  // As frentes sairam e o trabalho acabou: a impressora fica livre ate alguem pedir os versos.
  if (estado.tipo === "frentes_prontas") return false;
  if (estado.tipo === "erro" && MOTIVOS_SEM_TRABALHO.has(estado.motivo.codigo)) return false;
  return true;
}

/** Ainda ha versos por imprimir: fechar perde o cartao, e por isso o fechamento pergunta. */
export function cartaoSeguraAAba(cartao: Cartao): boolean {
  return cartaoEmAndamento(cartao) || cartao.estado.tipo === "frentes_prontas";
}

/** A secao so diz "Encerrados" quando nenhum cartao ainda segura a aba, e frentes prontas seguram. */
export function tituloDaSecao(cartoes: readonly Cartao[]): Chave {
  return cartoes.some(cartaoSeguraAAba) ? "cartao.secao" : "cartao.secao.encerrados";
}

/** Cada impressora tem um cartao por aba: o novo fica no lugar do antigo. */
export function comCartao(cartoes: readonly Cartao[], novo: Cartao): readonly Cartao[] {
  return cartoes.some((cartao) => cartao.impressora === novo.impressora)
    ? cartoes.map((cartao) => (cartao.impressora === novo.impressora ? novo : cartao))
    : [...cartoes, novo];
}

/** Um trabalho por impressora vale entre todas as abas. */
export function impressorasOcupadas(abas: readonly Aba[]): ReadonlySet<string> {
  return new Set(
    abas.flatMap((aba) =>
      aba.cartoes.filter(cartaoEmAndamento).map((cartao) => cartao.impressora),
    ),
  );
}

const ESTADOS_QUE_PEDEM_ACAO: ReadonlySet<EstadoCartao["tipo"]> = new Set([
  "aguardando",
  "proximo_lote",
  "aviso",
  "erro",
]);

/** O cartao espera a pessoa: recolocar, responder um aviso, clicar em algo ou ver um erro. */
export function cartaoPedeAcao(cartao: Cartao): boolean {
  return ESTADOS_QUE_PEDEM_ACAO.has(cartao.estado.tipo);
}

export type SituacaoDaAba = "pede_acao" | "em_curso";

/** Uma aba com trabalho mostra se algum cartao pede acao, ou se algum ainda esta em curso. */
export function situacaoDaAba(aba: Aba): SituacaoDaAba | null {
  if (aba.cartoes.some(cartaoPedeAcao)) return "pede_acao";
  const emCurso = aba.cartoes.some(
    (cartao) => cartao.estado.tipo === "imprimindo" || cartao.estado.tipo === "preparando",
  );
  return emCurso ? "em_curso" : null;
}

/** A interface perdeu o contato com o trabalho. O que ja foi enviado nao foi tocado. */
export function cartaoInterrompido(cartao: Cartao): Cartao {
  return { ...cartao, estado: { tipo: "erro", motivo: { codigo: "interrompido", texto: "" } } };
}

function cartaoDaImpressora(aba: number, impressora: Impressora, estado: EstadoCartao): Cartao {
  return {
    aba,
    impressora: impressora.id,
    nome: impressora.nome,
    simulada: impressora.tipo === "simulada",
    lote: 0,
    total: 0,
    papel: null,
    estado,
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
  };
}

/** Cartao de um pedido que nem chegou ao backend. */
export function cartaoSemResposta(aba: number, impressora: Impressora): Cartao {
  return cartaoInterrompido(cartaoDaImpressora(aba, impressora, { tipo: "preparando" }));
}

/** A impressora ja tinha um trabalho quando o pedido chegou: o cartao explica e nao segura nada. */
export function cartaoRecusado(aba: number, impressora: Impressora): Cartao {
  return cartaoDaImpressora(aba, impressora, {
    tipo: "erro",
    motivo: { codigo: "ocupada", texto: "" },
  });
}

/** Os destinos que o backend nao aceitou ganham cartao de recusa; os aceitos esperam o proprio evento. */
export function cartoesRecusados(
  aba: number,
  destinos: readonly Impressora[],
  aceitas: readonly string[],
): Cartao[] {
  return destinos
    .filter((destino) => !aceitas.includes(destino.id))
    .map((destino) => cartaoRecusado(aba, destino));
}

/** A recusa por impressora ocupada: quem ja tem um cartao vivo para ela nao deve perde-lo por isso. */
export function ehRecusaDeOcupada(cartao: Cartao): boolean {
  return cartao.estado.tipo === "erro" && cartao.estado.motivo.codigo === "ocupada";
}

/** Rolagem do cartao: se ele for mais alto que a lista, quem le comeca pelo topo dele. */
export function alinhamentoDoCartao(alturaDoCartao: number, alturaDaLista: number): "start" | "nearest" {
  return alturaDoCartao > alturaDaLista ? "start" : "nearest";
}

/**
 * Com um cartao na tela, o rodape so sai quando nenhuma marcada da lista pode receber:
 * todas as marcadas que estao na lista estao ocupadas. Marcada que sumiu da lista nao conta.
 */
export function marcadasTodasOcupadas(
  lista: readonly Impressora[],
  marcadas: readonly string[],
  ocupadas: ReadonlySet<string>,
): boolean {
  const naLista = lista.filter((impressora) => marcadas.includes(impressora.id));
  return naLista.length > 0 && naLista.every((impressora) => ocupadas.has(impressora.id));
}

/**
 * Na altura curta, o rodape so sai quando nao ha o que fazer nele: toda marcada da lista tem trabalho
 * correndo aqui. Marcada ocupada por outra aba, ou cartao ja encerrado, nao tira o rodape: desmarcar
 * ainda resolve, e o motivo precisa ficar a vista.
 */
export function rodapeCedeAoCartao(
  aba: Aba | null,
  lista: readonly Impressora[],
  ocupadas: ReadonlySet<string>,
): boolean {
  if (aba === null) return false;
  if (!marcadasTodasOcupadas(lista, aba.impressorasMarcadas, ocupadas)) return false;
  const emAndamento = new Set(
    aba.cartoes.filter(cartaoEmAndamento).map((cartao) => cartao.impressora),
  );
  return lista
    .filter((impressora) => aba.impressorasMarcadas.includes(impressora.id))
    .every((impressora) => emAndamento.has(impressora.id));
}

export function motivoDeFora(
  impressora: Impressora,
  ocupadas: ReadonlySet<string>,
  idioma: Idioma,
): string | null {
  const nome = nomeDeExibicao(impressora, idioma);
  const com = (chave: Chave) => traduzir(idioma, chave, { nome });
  if (ocupadas.has(impressora.id)) return com("cartao.fora.ocupada");
  if (impressora.recusa === "fax") return com("cartao.fora.fax");
  if (impressora.recusa === "pede_arquivo") return com("cartao.fora.pedeArquivo");
  if (impressora.modo === "sem_perfil") return com("cartao.fora.semPerfil");
  if (impressora.modo === "desconhecido") return com("cartao.fora.perfilNaoLido");
  if (impressora.nivel === "problema") {
    return traduzir(idioma, "cartao.fora.problema", {
      nome,
      estado: textoDoEstado(impressora, idioma),
    });
  }
  return null;
}

export interface EstadoDoEnvio {
  rotulo: string;
  /** Por que o botao esta desabilitado, ou quem ficou fora da contagem. */
  motivo: string | null;
  desabilitado: boolean;
  destinos: readonly Impressora[];
  /** O rotulo com a contagem por extenso, para quem ouve o botao. So existe com contagem. */
  rotuloAcessivel?: string;
}

/** A contagem de impressoras com a palavra inteira: "em 2 impressoras", "em 1 impressora". */
export function contagemPorExtenso(chave: Chave, n: number, idioma: Idioma = idiomaAtual): string {
  return traduzir(idioma, chave, {
    n: String(n),
    impressoras: traduzir(idioma, n === 1 ? "impressoras.um" : "impressoras.varias"),
  });
}

const MAXIMO_DE_NOMES_NO_MOTIVO = 2;

export function listaDeFora(motivos: readonly string[], idioma: Idioma): string {
  const mostrados = motivos.slice(0, MAXIMO_DE_NOMES_NO_MOTIVO).join(", ");
  const resto = motivos.length - MAXIMO_DE_NOMES_NO_MOTIVO;
  return resto > 0
    ? `${mostrados}${traduzir(idioma, "cartao.botao.eMais", { n: String(resto) })}`
    : mostrados;
}

/** Conta so quem pode receber: marcada, com perfil, sem problema e com o PDF aceito. */
export function estadoDoEnvio(
  aba: Aba | null,
  lista: readonly Impressora[],
  ocupadas: ReadonlySet<string>,
  idioma: Idioma = idiomaAtual,
): EstadoDoEnvio {
  const marcadas = lista.filter((impressora) => aba?.impressorasMarcadas.includes(impressora.id));
  const fora = marcadas.flatMap((impressora) => {
    const motivo = motivoDeFora(impressora, ocupadas, idioma);
    return motivo === null ? [] : [{ impressora, motivo }];
  });
  const pdfAceito = aba?.paginas.estado === "conhecido";
  const destinos = pdfAceito
    ? marcadas.filter((impressora) => !fora.some((item) => item.impressora === impressora))
    : [];
  // O rotulo e a confirmacao seguem quem recebe de fato: uma fila invalida marcada nao muda nada.
  const base = destinos.length > 0 ? destinos : marcadas;
  const soSimuladas = base.length > 0 && base.every((i) => i.tipo === "simulada");
  const soAutomaticas = base.length > 0 && base.every((i) => i.modo === "automatico");
  const rotulo =
    soAutomaticas
      ? traduzir(idioma, soSimuladas ? "modo.automatico.simular" : "modo.automatico.enviar")
      : destinos.length > 0
      ? traduzir(idioma, soSimuladas ? "cartao.simular.frenteContagem" : "imprimir.frenteContagem", {
          n: String(destinos.length),
        })
      : traduzir(idioma, soSimuladas ? "cartao.simular.frente" : "imprimir.frente");
  const motivosDeFora = fora.map((item) => item.motivo);
  const todasOcupadas =
    marcadas.length > 0 && marcadas.every((impressora) => ocupadas.has(impressora.id));
  let motivo: string | null = null;
  if (aba === null) motivo = traduzir(idioma, "cartao.botao.semDocumento");
  else if (aba.paginas.estado === "contando") motivo = traduzir(idioma, "cartao.botao.contando");
  else if (aba.paginas.estado === "recusado") motivo = traduzir(idioma, "cartao.botao.recusado");
  else if (marcadas.length === 0) motivo = traduzir(idioma, "cartao.botao.semMarcada");
  else if (destinos.length === 0 && todasOcupadas) {
    motivo = traduzir(idioma, "cartao.botao.emAndamento");
  } else if (destinos.length === 0) {
    motivo = traduzir(idioma, "cartao.botao.nenhuma", {
      lista: listaDeFora(motivosDeFora, idioma),
    });
  } else if (fora.length > 0) {
    motivo = traduzir(idioma, "cartao.botao.foraDaConta", {
      lista: listaDeFora(motivosDeFora, idioma),
    });
  }
  const rotuloAcessivel =
    destinos.length > 0 && !soAutomaticas
      ? contagemPorExtenso(
          soSimuladas ? "cartao.simular.frenteContagem.aria" : "imprimir.frenteContagem.aria",
          destinos.length,
          idioma,
        )
      : undefined;
  return { rotulo, motivo, desabilitado: destinos.length === 0, destinos, rotuloAcessivel };
}

/**
 * O envio real pede confirmacao, e a impressora automatica tambem, mesmo simulada: e la que a pessoa
 * le quais paginas saem por qual fluxo. As outras simuladas nao mandam nada a lugar nenhum.
 */
export function precisaConfirmar(destinos: readonly Impressora[]): boolean {
  return destinos.some((impressora) => impressora.tipo === "fila" || impressora.modo === "automatico");
}

const CHAVES_DE_PAPEL: ReadonlyMap<string, Chave> = new Map<string, Chave>([
  ["a4", "cartao.papel.a4"],
  ["a5", "cartao.papel.a5"],
  ["carta", "cartao.papel.carta"],
  ["oficio", "cartao.papel.oficio"],
  ["legal", "cartao.papel.legal"],
]);

export function nomeDoPapel(papel: PapelDoCartao, idioma: Idioma = idiomaAtual): string {
  const chave = CHAVES_DE_PAPEL.get(papel.formato);
  if (chave) return traduzir(idioma, chave);
  return traduzir(idioma, "cartao.papel.outro", {
    largura: papel.larguraMm.toFixed(0),
    altura: papel.alturaMm.toFixed(0),
  });
}

/** As paginas de um lote, com os numeros do documento: "págs. 3–6". */
export function textoDasPaginas(faixa: FaixaDoLote, idioma: Idioma = idiomaAtual): string {
  return faixa.inicio === faixa.fim
    ? traduzir(idioma, "modo.pilha.paginas.uma", { a: String(faixa.inicio) })
    : traduzir(idioma, "modo.pilha.paginas.varias", {
        a: String(faixa.inicio),
        b: String(faixa.fim),
      });
}

/**
 * O numero da copia de um lote automatico, quando o trabalho tem varias copias e o lote sai de uma so.
 * Lote misturado, ou lote que nao e automatico, nao leva numero de copia.
 */
export function copiaDoLote(cartao: Cartao): number | null {
  return cartao.loteAutomatico === true && cartao.copia != null && (cartao.copias ?? 1) > 1
    ? cartao.copia
    : null;
}

/**
 * Quantos lotes cada copia ocupa: `total` lotes divididos pelas copias. Vale 1 quando cada copia sai
 * de um lote so; nunca menos que 1, para que a conta de trecho nao divida por zero.
 */
export function trechosPorCopia(cartao: Cartao): number {
  const copias = cartao.copias ?? 1;
  return copias > 0 ? Math.max(1, Math.floor(cartao.total / copias)) : 1;
}

/** "Copia 2 de 3 · 12 folhas · A4", ou com o trecho quando cada copia ocupa mais de um lote. */
export function linhaDaCopia(cartao: Cartao, idioma: Idioma = idiomaAtual): string | null {
  const copias = cartao.copias ?? 1;
  const tt = trechosPorCopia(cartao);
  if (cartao.estado.tipo === "concluido") {
    // Uma copia por lote: as folhas do ultimo lote sao as de uma copia, e o total sai da multiplicacao.
    if (!(cartao.temAutomatico === true && copias > 1 && tt === 1)) return null;
    return traduzir(idioma, "cartao.copia.concluido", {
      total: String(copias),
      folhas: textoDasFolhas((cartao.folhasDoLote ?? 0) * copias, idioma),
      papel: cartao.papel ? nomeDoPapel(cartao.papel, idioma) : "",
    });
  }
  const copia = copiaDoLote(cartao);
  if (copia === null) return null;
  const base = {
    n: String(copia),
    total: String(copias),
    folhas: textoDasFolhas(cartao.folhasDoLote ?? 0, idioma),
  };
  if (tt > 1) {
    const t = (Math.max(0, cartao.lote - 1) % tt) + 1;
    return traduzir(idioma, "cartao.copia.trechos", {
      ...base,
      t: String(t),
      tt: String(tt),
    });
  }
  return traduzir(idioma, "cartao.copia.andamento", {
    ...base,
    papel: cartao.papel ? nomeDoPapel(cartao.papel, idioma) : "",
  });
}

/** Varias faixas de uma vez: "págs. 3–4, 7–8". */
export function textoDasFaixas(
  faixas: readonly { inicio: number; fim: number }[],
  idioma: Idioma = idiomaAtual,
): string {
  const unica = faixas[0];
  if (faixas.length === 1 && unica !== undefined) {
    return textoDasPaginas({ lote: 0, inicio: unica.inicio, fim: unica.fim }, idioma);
  }
  return traduzir(idioma, "modo.pilha.paginas.lista", {
    lista: faixas
      .map((faixa) => (faixa.inicio === faixa.fim ? String(faixa.inicio) : `${faixa.inicio}–${faixa.fim}`))
      .join(", "),
  });
}

/** Com um trecho so, nao ha o que numerar. */
export function linhaDoLote(cartao: Cartao, idioma: Idioma = idiomaAtual): string | null {
  // Concluido com varias copias automaticas nunca diz "trecho": a linha de copias, quando existe, fala do trabalho.
  if (cartao.estado.tipo === "concluido" && cartao.temAutomatico === true && (cartao.copias ?? 1) > 1) return null;
  return cartao.total > 1
    ? traduzir(idioma, "cartao.lote", { n: String(cartao.lote), total: String(cartao.total) })
    : null;
}

export function textoDasFolhas(folhas: number, idioma: Idioma = idiomaAtual): string {
  return traduzir(idioma, folhas === 1 ? "face.folhas.um" : "face.folhas.varios", { n: String(folhas) });
}

/** Acima disto, o maco pode nao caber de uma vez na entrada. */
export const FOLHAS_QUE_CABEM = 30;

export function nomeDoCartao(cartao: Cartao, idioma: Idioma = idiomaAtual): string {
  if (!cartao.simulada) return cartao.nome;
  return traduzir(
    idioma,
    cartao.impressora === "simulada:automatica"
      ? "impressora.simulada.automatica"
      : cartao.impressora === "simulada:gaveta"
      ? "impressora.simulada.gaveta"
      : "impressora.simulada.traseira",
  );
}

export type TomDoCartao = "ativo" | "espera" | "problema" | "parado";

export function tomDoCartao(cartao: Cartao): TomDoCartao {
  switch (cartao.estado.tipo) {
    case "preparando":
    case "imprimindo":
      return "ativo";
    case "concluido": {
      // Simulacao que terminou com folha errada nao e um fim verde.
      const resultado = cartao.resultado;
      return resultado !== null && resultado.corretas < resultado.folhas ? "problema" : "ativo";
    }
    case "aguardando":
    case "frentes_prontas":
    case "proximo_lote":
    case "aviso":
      return "espera";
    case "erro":
      return "problema";
    case "cancelado":
      return "parado";
  }
}

/** O que dizer em uma condicao que o backend distinguiu: a decisao vem do codigo, nunca da frase. */
interface OrientacaoDoAviso {
  forte: Chave;
  instrucao: Chave;
  ressalva?: Chave;
}

const SEM_RESPOSTA: OrientacaoDoAviso = {
  forte: "spooler.semResposta.forte",
  instrucao: "spooler.semResposta.instrucao",
};

/** Na passada de verso, a entrada so tem o maco impresso: folha em branco ali desalinha o lote. */
const ORIENTACOES: ReadonlyMap<string, Readonly<Record<"frente" | "verso", OrientacaoDoAviso>>> = new Map([
  [
    "sem_papel",
    {
      frente: {
        forte: "spooler.semPapel.frente.forte",
        instrucao: "spooler.semPapel.frente.instrucao",
      },
      verso: {
        forte: "spooler.semPapel.verso.forte",
        instrucao: "spooler.semPapel.verso.instrucao",
        ressalva: "spooler.semPapel.verso.ressalva",
      },
    },
  ],
  [
    "atolado",
    {
      frente: { forte: "spooler.atolado.forte", instrucao: "spooler.atolado.frente.instrucao" },
      verso: { forte: "spooler.atolado.forte", instrucao: "spooler.atolado.verso.instrucao" },
    },
  ],
  ["sem_resposta", { frente: SEM_RESPOSTA, verso: SEM_RESPOSTA }],
]);

export function textoDoEstadoDoCartao(cartao: Cartao, idioma: Idioma = idiomaAtual): string {
  if (cartao.estado.tipo === "aviso" && ORIENTACOES.has(cartao.estado.motivo.codigo)) {
    return traduzir(idioma, "spooler.estado.verifique");
  }
  const chaves: Record<EstadoCartao["tipo"], Chave> = {
    preparando: "cartao.estado.preparando",
    imprimindo: cartao.simulada ? "cartao.estado.simulando" : "cartao.estado.imprimindo",
    aguardando: "cartao.estado.aguardando",
    frentes_prontas: "face.estado.frentesProntas",
    proximo_lote: "cartao.estado.proximo",
    aviso: "cartao.estado.aviso",
    erro: "cartao.estado.erro",
    concluido: "cartao.estado.concluido",
    cancelado: "cartao.estado.cancelado",
  };
  return traduzir(idioma, chaves[cartao.estado.tipo]);
}

/** Frases que a fila manda em portugues. O texto do driver, entre parenteses, fica como veio. */
const MOTIVOS_DA_FILA_EM_INGLES: readonly (readonly [string, string])[] = [
  ["a impressora está sem papel", "the printer is out of paper"],
  ["o papel atolou na impressora", "the paper is jammed in the printer"],
  ["a impressora não responde", "the printer is not responding"],
  ["a impressora pede atenção", "the printer needs attention"],
  ["o trabalho está bloqueado na impressora", "the job is blocked at the printer"],
  ["o trabalho deu erro", "the job failed"],
  ["o trabalho foi excluído da fila antes de terminar", "the job was deleted before it finished"],
  ["o spooler informou uma falha", "the print queue reported a failure"],
];

const CHAVES_DE_MOTIVO: ReadonlyMap<string, Chave> = new Map<string, Chave>([
  ["biblioteca", "cartao.motivo.biblioteca"],
  ["pdf", "cartao.motivo.pdf"],
  ["desenho", "cartao.motivo.desenho"],
  ["papel", "cartao.motivo.papel"],
  ["driver", "cartao.motivo.driver"],
  ["arquivo_saida", "cartao.motivo.arquivoSaida"],
  ["perfil", "cartao.motivo.perfil"],
  ["nao_cancelou", "cartao.motivo.naoCancelou"],
  ["sem_confirmacao", "cartao.motivo.semConfirmacao"],
  ["sem_acompanhamento", "cartao.motivo.semAcompanhamento"],
  ["sem_prazo", "cartao.motivo.semPrazo"],
  ["interrompido", "cartao.motivo.interrompido"],
  ["fila_inexistente", "cartao.motivo.filaInexistente"],
  ["fila_fax", "cartao.motivo.filaFax"],
  ["fila_pede_arquivo", "cartao.motivo.filaPedeArquivo"],
  ["ocupada", "cartao.motivo.ocupada"],
  ["documento_mudou", "cartao.motivo.documentoMudou"],
  ["duas_faces_recusadas", "modo.automatico.recusado"],
]);

function comInicialMaiuscula(texto: string): string {
  return texto.charAt(0).toUpperCase() + texto.slice(1);
}

/** Frase da fila sem ponto final ganha o ponto; a que ja termina em pontuacao fica como esta. */
function comPontoFinal(texto: string): string {
  return /[.!?…]$/.test(texto) ? texto : `${texto}.`;
}

/**
 * As mensagens do backend so existem em portugues. Em portugues, vale a mensagem exata; nos outros
 * idiomas, a frase traduzida pelo codigo, para nao misturar dois idiomas na mesma linha.
 */
export function textoDoMotivo(motivo: Motivo, idioma: Idioma = idiomaAtual): string {
  // A frase de quem recusou nao diz o que fazer: aqui, o caminho e outro modelo.
  if (motivo.codigo === "duas_faces_recusadas") return traduzir(idioma, "modo.automatico.recusado");
  if (motivo.codigo === "fila" && motivo.texto !== "") {
    if (idioma === "pt-BR") return comPontoFinal(comInicialMaiuscula(motivo.texto));
    const traduzido = MOTIVOS_DA_FILA_EM_INGLES.reduce(
      (texto, [de, para]) => texto.replaceAll(de, para),
      motivo.texto,
    );
    return comPontoFinal(comInicialMaiuscula(traduzido));
  }
  if (idioma === "pt-BR" && motivo.texto !== "") return comInicialMaiuscula(motivo.texto);
  return traduzir(idioma, CHAVES_DE_MOTIVO.get(motivo.codigo) ?? "cartao.motivo.sistema");
}

const CHAVES_DE_VIRADA: ReadonlyMap<string, Chave> = new Map<string, Chave>([
  ["borda_de_cima", "cartao.virada.bordaDeCima"],
  ["lateral", "cartao.virada.lateral"],
  ["nao_virar", "cartao.virada.naoVirar"],
  ["girar", "cartao.virada.girar"],
]);

/**
 * Com perfil personalizado, a frase nao afirma lado: vale o que foi feito na calibracao. O texto
 * do perfil e escrito em portugues, por isso so entra no portugues.
 */
export function frasesDoGesto(recolocar: Recolocar, idioma: Idioma = idiomaAtual): string[] {
  if (recolocar.lado === null) {
    const frases = [traduzir(idioma, "cartao.calibracao")];
    if (recolocar.textoDoPerfil && idioma === "pt-BR") frases.push(recolocar.textoDoPerfil);
    return frases;
  }
  // Sem gesto de virar, a colocacao vem sem "Depois": a frase nao segue uma negacao.
  const semGesto = !recolocar.virada || recolocar.virada === "nao_virar";
  const colocacao = traduzir(
    idioma,
    recolocar.lado === "cima"
      ? semGesto
        ? "cartao.colocacao.cima"
        : "cartao.gesto.cima"
      : semGesto
        ? "cartao.colocacao.baixo"
        : "cartao.gesto.baixo",
  );
  // Na ordem fisica: primeiro o gesto, depois a colocacao na entrada.
  const virada = CHAVES_DE_VIRADA.get(recolocar.virada ?? "");
  return virada ? [traduzir(idioma, virada), colocacao] : [colocacao];
}

export function resultadoDoCartao(cartao: Cartao, idioma: Idioma = idiomaAtual): string | null {
  if (cartao.resultado === null) return null;
  const chave = cartao.resultado.folhas === 1 ? "cartao.resultado.um" : "cartao.resultado.varios";
  return traduzir(idioma, chave, {
    n: String(cartao.resultado.corretas),
    m: String(cartao.resultado.folhas),
  });
}

function listaDeFaces(faces: readonly FaceDaSequencia[], idioma: Idioma): string {
  const numeros = encurtada(
    faces.map((face) => (face.numero === null ? traduzir(idioma, "cartao.sequencia.branca") : face.numero)),
  ).join(", ");
  const girada = faces.length > 0 && faces.every((face) => face.girada);
  return girada ? `${numeros} ${traduzir(idioma, "cartao.sequencia.girada")}` : numeros;
}

function folhasDoTrabalho(faces: readonly FaceDaSequencia[]): string {
  const folhas: string[] = [];
  for (let indice = 0; indice < faces.length; indice += 2) {
    const par = faces.slice(indice, indice + 2).map((face) => face.numero ?? "");
    folhas.push(par.join("|"));
  }
  return encurtada(folhas).join(", ");
}

/** As marcas chegam uma por folha, como saem no papel: "Folha 4". Na tela, viram uma lista so. */
function listaDeMarcas(marcas: readonly string[], idioma: Idioma): string {
  const folhas: string[] = [];
  const outras: string[] = [];
  for (const marca of marcas) {
    const [, folha] = /^\S+ (\d{1,9})$/.exec(marca) ?? [];
    if (folha === undefined) outras.push(marca);
    else folhas.push(folha);
  }
  const trecho =
    folhas.length === 0
      ? []
      : [
          traduzir(idioma, folhas.length === 1 ? "controle.marcas.umaFolha" : "controle.marcas.folhas", {
            folhas: encurtada(folhas).join(", "),
          }),
        ];
  return [...trecho, ...outras].join("; ");
}

const NUMEROS_NUMA_LINHA = 12;

/** Um trecho inteiro pode ter centenas de folhas: a linha mostra o comeco, o fim e nada no meio. */
function encurtada<T>(lista: readonly T[]): readonly (T | "…")[] {
  return lista.length <= NUMEROS_NUMA_LINHA ? lista : [...lista.slice(0, 8), "…", ...lista.slice(-2)];
}

/** So a simulacao mostra a ordem: e o que ela existe para conferir. */
export function sequenciaDoCartao(cartao: Cartao, idioma: Idioma = idiomaAtual): string[] {
  if (cartao.sequencia === null || cartao.estado.tipo === "frentes_prontas") return [];
  // Um trabalho so, sem verso a parte: o que se confere sao as folhas que ele forma.
  if (cartao.sequencia.verso.length === 0 && (cartao.loteAutomatico || cartao.temAutomatico)) {
    const lista = folhasDoTrabalho(cartao.sequencia.frente);
    return [traduzir(idioma, copiaDoLote(cartao) !== null ? "cartao.copia.cada" : "modo.automatico.folhas", { lista })];
  }
  const marcas = cartao.sequencia.marcas ?? [];
  return [
    traduzir(idioma, "cartao.sequencia.frente", {
      lista: listaDeFaces(cartao.sequencia.frente, idioma),
    }),
    traduzir(idioma, "cartao.sequencia.verso", {
      lista: listaDeFaces(cartao.sequencia.verso, idioma),
    }),
    ...(marcas.length > 0
      ? [traduzir(idioma, "controle.marcas", { lista: listaDeMarcas(marcas, idioma) })]
      : []),
  ];
}

/**
 * Papel de cada linha: `acao` diz o que clicar ou fazer; `ressalva` diz o que a fila confirma e o
 * que ela nao garante; `gesto` diz o que fazer com o maco; `contexto` explica o resto.
 */
export type PapelDaLinha = "acao" | "ressalva" | "gesto" | "contexto";

export interface LinhaDoCartao {
  texto: string;
  papel: PapelDaLinha;
  /** Texto de fora, de tamanho variavel: o cartao limita a altura e guarda o inteiro no titulo. */
  limitada?: boolean;
  /** Sai depois do botao da acao, e nao antes dele. */
  depoisDaAcao?: boolean;
}

/** O que o cartao explica em cada estado, com os termos de quem so sabe o que a fila diz. */
export function linhasDoCartao(cartao: Cartao, idioma: Idioma = idiomaAtual): LinhaDoCartao[] {
  const { estado } = cartao;
  const em = (chave: Chave, parametros?: Record<string, string>) =>
    traduzir(idioma, chave, parametros);
  const linha = (texto: string, papel: PapelDaLinha): LinhaDoCartao => ({ texto, papel });
  const gesto = (): LinhaDoCartao[] =>
    cartao.recolocar
      ? frasesDoGesto(cartao.recolocar, idioma).map((texto) => linha(texto, "gesto"))
      : [];
  const doMacoGrande = (): LinhaDoCartao[] =>
    (cartao.folhasDoLote ?? 0) > FOLHAS_QUE_CABEM ? [linha(em("face.verso.emPartes"), "contexto")] : [];
  switch (estado.tipo) {
    case "preparando":
      return [linha(em("cartao.preparando"), "contexto")];
    case "imprimindo":
      if (cartao.loteAutomatico) return [linha(em("modo.automatico.imprimindo"), "contexto")];
      if (cartao.simulada) {
        return [
          linha(
            em(estado.passada === "frente" ? "cartao.simulando.frente" : "cartao.simulando.verso"),
            "contexto",
          ),
        ];
      }
      return [
        estado.passada === "frente"
          ? linha(em("cartao.frente.espera"), "ressalva")
          : linha(em("cartao.verso.espera"), "ressalva"),
      ];
    case "aguardando": {
      // So os versos de outro papel chegam aqui: o trecho anterior acabou, e agora entra outro maco.
      const faixa = cartao.faixaDoLote;
      return [
        linha(em(cartao.simulada ? "cartao.verso.simulado" : "cartao.verso.confirmado"), "ressalva"),
        linha(em("face.verso.guardar"), "contexto"),
        ...(cartao.trocaDePapel
          ? [linha(em("face.verso.troca", { papel: nomeDoPapel(cartao.trocaDePapel, idioma) }), "contexto")]
          : []),
        ...(faixa ? [linha(em("face.verso.pegue", { paginas: textoDasPaginas(faixa, idioma) }), "acao")] : []),
        ...(cartao.simulada ? [linha(em("cartao.simulada.proximo"), "contexto")] : []),
        ...gesto(),
        ...doMacoGrande(),
        linha(em(cartao.simulada ? "cartao.simulada.clique" : "cartao.verso.clique"), "acao"),
      ];
    }
    case "frentes_prontas":
      // O botao de versos vem logo depois do gesto; a frase de espera, depois do botao.
      return [
        cartao.simulada
          ? linha(em("cartao.frente.simulada"), "contexto")
          : linha(em("cartao.frente.confirmada"), "ressalva"),
        ...(cartao.simulada ? [] : [linha(em("cartao.esperar"), "contexto")]),
        ...gesto(),
        ...doMacoGrande(),
        { ...linha(em("face.espera"), "contexto"), depoisDaAcao: true },
      ];
    case "proximo_lote": {
      const linhas = [
        linha(
          em(
            cartao.anteriorAutomatico
              ? "modo.automatico.trechoConcluido"
              : cartao.simulada
                ? "cartao.frente.simulada"
                : "cartao.frente.trechoPronto",
          ),
          "ressalva",
        ),
      ];
      if (cartao.trocaDePapel) {
        linhas.push(
          linha(em("cartao.trocaPapel", { papel: nomeDoPapel(cartao.trocaDePapel, idioma) }), "contexto"),
        );
      } else if (!cartao.simulada) {
        linhas.push(linha(em("cartao.proximo.semTroca"), "contexto"));
      }
      linhas.push(
        linha(em(cartao.loteAutomatico ? "modo.automatico.proximo" : cartao.simulada ? "cartao.proximo.cliqueSimulado" : "cartao.proximo.clique"), "acao"),
      );
      return linhas;
    }
    case "aviso": {
      const orientacao = ORIENTACOES.get(estado.motivo.codigo)?.[cartao.loteAutomatico || estado.passada === "duas_faces" ? "frente" : estado.passada];
      const linhas = orientacao
        ? [linha(em(orientacao.forte), "contexto"), linha(em(orientacao.instrucao), "acao")]
        : [linha(textoDoMotivo(estado.motivo, idioma), "contexto")];
      if (orientacao?.ressalva) linhas.push(linha(em(orientacao.ressalva), "ressalva"));
      if (estado.respondido) linhas.push(linha(em("cartao.aviso.seguindo"), "ressalva"));
      if (estado.mensagemDoDriver) {
        linhas.push({
          ...linha(em("spooler.mensagemDaImpressora", { texto: estado.mensagemDoDriver }), "ressalva"),
          limitada: true,
        });
      }
      return linhas;
    }
    case "erro":
      // Os lotes anteriores ja estao na bandeja: dizer que nada saiu faria a pessoa imprimir tudo de novo.
      if (estado.motivo.codigo === "duas_faces_recusadas" && cartao.lote > 1) {
        return [linha(em("modo.automatico.recusadoNoMeio"), "contexto")];
      }
      return [linha(textoDoMotivo(estado.motivo, idioma), "contexto")];
    case "concluido": {
      const linhas = [
        linha(em(cartao.simulada ? "cartao.concluido.simulado" : "cartao.concluido"), "contexto"),
      ];
      // Com um trecho automatico, como o maco saiu nao se sabe, mesmo que o ultimo lote tenha sido manual.
      if (cartao.temAutomatico) {
        // Com varias copias, a virada e de cada copia; sem elas, segue o texto de antes.
        const virada = (cartao.copias ?? 1) > 1 ? "cartao.copia.vire" : cartao.total >= 2 ? "modo.automatico.finalVarios" : "modo.automatico.final";
        linhas.push(linha(em(virada), "acao"));
      }
      else if (cartao.viraParaLer) linhas.push(linha(em("cartao.virarParaLer"), "acao"));
      // Juntar so importa com mais de um papel por copia: o total de lotes passa do numero de copias.
      if (cartao.total > (cartao.copias ?? 1)) linhas.push(linha(em("face.juntar"), "contexto"));
      return linhas;
    }
    case "cancelado":
      return [
        linha(em(cartao.simulada ? "cartao.cancelado.simulado" : "cartao.cancelado"), "contexto"),
      ];
  }
}

/** So os textos, para quem nao precisa do papel de cada linha. */
export function frasesDoCartao(cartao: Cartao, idioma: Idioma = idiomaAtual): string[] {
  return linhasDoCartao(cartao, idioma).map((linha) => linha.texto);
}

export interface AcoesDoCartao {
  /** Frente do proximo lote. */
  frente: boolean;
  /** Os versos sao outro pedido, com a confirmacao de sempre: as frentes ja acabaram. */
  pedirVerso: boolean;
  /**
   * O verso aparece durante a frente e durante o proprio verso, mas so libera quando a fila
   * confirma a frente: enquanto a passada de verso roda, fica visivel e desabilitado.
   */
  verso: { visivel: boolean; liberado: boolean };
  reimprimir: boolean;
  continuar: boolean;
  cancelar: boolean;
  /** Tira o cartao da lista: so para quem ja nao espera nada. */
  dispensar: boolean;
}

export function acoesDoCartao(cartao: Cartao): AcoesDoCartao {
  const { estado } = cartao;
  const emFrente =
    (estado.tipo === "imprimindo" && estado.passada === "frente") ||
    (estado.tipo === "aviso" && estado.passada === "frente");
  const emVerso =
    (estado.tipo === "imprimindo" && estado.passada === "verso") ||
    (estado.tipo === "aviso" && estado.passada === "verso");
  const semTrabalho = estado.tipo === "erro" && !cartaoEmAndamento(cartao);
  return {
    frente: estado.tipo === "proximo_lote",
    pedirVerso: estado.tipo === "frentes_prontas",
    verso: {
      visivel: !cartao.loteAutomatico && (emFrente || emVerso || estado.tipo === "aguardando"),
      liberado: estado.tipo === "aguardando",
    },
    // Sem lote ainda, a preparacao falhou: nao ha o que refazer sem pedir de novo. Thread morta
    // ou recusa ao iniciar tambem nao tem lote para refazer: o caminho e dispensar e imprimir de novo.
    // Com aviso, o lote pode ter desalinhado em qualquer passada: refazer precisa estar a mao.
    reimprimir: (estado.tipo === "erro" || estado.tipo === "aviso") && cartao.total > 0 && !semTrabalho,
    continuar: estado.tipo === "aviso" && !estado.respondido,
    cancelar: cartaoEmAndamento(cartao) && estado.tipo !== "preparando",
    dispensar: !cartaoEmAndamento(cartao),
  };
}

export function rotuloDoVerso(cartao: Cartao, idioma: Idioma = idiomaAtual): string {
  return traduzir(idioma, cartao.simulada ? "cartao.simular.verso" : "cartao.imprimir.verso");
}

export function rotuloDaFrente(cartao: Cartao, idioma: Idioma = idiomaAtual): string {
  if (cartao.loteAutomatico) return traduzir(idioma, cartao.simulada ? "modo.automatico.simular" : "modo.automatico.enviar");
  return traduzir(idioma, cartao.simulada ? "cartao.simular.frente" : "imprimir.frente");
}

export interface ResumoDoEnvio {
  paginas: number;
  lotes: number;
  /** Folhas do envio inteiro, em cada impressora. */
  folhas?: number;
  /** O maior maco que alguem vai ter de recolocar de uma vez. */
  maiorMaco?: number;
  papeis: PapelDoCartao[];
  paresMistos: [number, number][];
  assinado: boolean;
  fluxos?: { automatico: boolean; inicio: number; fim: number }[];
  lotesAutomaticos?: number;
  escalas?: [number, number] | null;
  comparacao?: ComparacaoDoEnvio | null;
  faixas?: FaixaDoLote[];
  gestos?: { impressora: string; recolocar: Recolocar }[];
}

/** O que as opcoes do envio acrescentam ao que a confirmacao diz. */
export interface EscolhasDoEnvio {
  /** O papel de destino, ja com a escala, quando o documento vai ajustado. */
  papel?: string | null;
  /** Copias da face que sai: com duas ou mais, a primeira linha mostra o numero. */
  copias?: number;
  /** O que vem entre a rota e as instrucoes gerais: folha de controle, driver e velocidade. */
  notasDoMeio?: readonly string[];
}

/** Quantas paginas e quantas folhas: e o papel que a pessoa precisa ter a mao. */
/** Paginas do documento, com o numero de copias quando passa de uma, e as folhas de todas elas. */
export function textoDoTamanhoDoEnvio(
  resumo: ResumoDoEnvio,
  idioma: Idioma = idiomaAtual,
  copias = 1,
): string {
  const paginas = traduzir(idioma, resumo.paginas === 1 ? "folhas.paginas.um" : "folhas.paginas.varios", {
    n: String(resumo.paginas),
  });
  const comCopias = copias > 1 ? `${paginas} × ${copias}` : paginas;
  return resumo.folhas === undefined ? comCopias : `${comCopias} · ${textoDasFolhas(resumo.folhas, idioma)}`;
}

/** As linhas da confirmacao: quem recebe, quantas paginas e em que papel. */
export function linhasDoEnvio(
  destinos: readonly Impressora[],
  resumo: ResumoDoEnvio,
  idioma: Idioma = idiomaAtual,
  filasComAjuste: ReadonlySet<string> = filasComAjusteAgora(),
  escolhas: EscolhasDoEnvio = {},
): {
  linhas: string[];
  notas: string[];
  notasDaRota: string[];
  notasGerais: string[];
  linhaDoPapel: string;
  papelComExtras: boolean;
} {
  const reais = destinos.filter((impressora) => impressora.tipo === "fila");
  const simuladas = destinos.length - reais.length;
  const automaticas = destinos.filter((impressora) => impressora.modo === "automatico");
  const soAutomaticas = destinos.length > 0 && automaticas.length === destinos.length;
  const fluxos = resumo.fluxos ?? [];
  // Uma rota so, para todas: cabe na linha do papel, e nenhuma nota precisa repetir o papel.
  const rotaUnica = soAutomaticas && fluxos.length === 1 ? fluxos[0] : undefined;
  const lista =
    escolhas.papel ?? resumo.papeis.map((papel) => nomeDoPapel(papel, idioma)).join(", ");
  const linhaDoPapel =
    rotaUnica === undefined
      ? traduzir(idioma, "cartao.envio.papel", { lista })
      : traduzir(idioma, "cartao.envio.papelRota", {
          lista,
          rota: traduzir(idioma, rotaUnica.automatico ? "modo.automatico.rotaAuto" : "modo.automatico.rotaManual"),
        });
  const linhas = [
    ...(reais.length > 0 ? reais : destinos).map((impressora) => nomeDeExibicao(impressora, idioma)),
    textoDoTamanhoDoEnvio(resumo, idioma, escolhas.copias),
    linhaDoPapel,
  ];
  const doAjuste = linhaDoAjusteNoEnvio(destinos, filasComAjuste, idioma);
  if (doAjuste !== null) linhas.push(doAjuste);
  const notasDaRota: string[] = [];
  const listaDeNomes = (impressoras: readonly Impressora[]) =>
    new Intl.ListFormat(idioma, { style: "long", type: "conjunction" }).format(
      impressoras.map((impressora) => nomeDeExibicao(impressora, idioma)),
    );
  if (automaticas.length > 0 && rotaUnica === undefined) {
    // Sozinha, nao ha de quem mais falar; com outra impressora, a linha diz de qual ela fala.
    const sozinha = destinos.length === 1;
    const nomes = listaDeNomes(automaticas);
    const notaDaRota = (automatico: boolean, nome: string, faixas: readonly { inicio: number; fim: number }[]) => {
      const chave: Chave = automatico
        ? sozinha ? "modo.automatico.trecho" : "modo.automatico.trechoNela"
        : sozinha ? "modo.automatico.manual" : "modo.automatico.manualNela";
      return traduzir(idioma, chave, { nomes: nome, paginas: textoDasFaixas(faixas, idioma) });
    };
    for (const fluxo of fluxos) notasDaRota.push(notaDaRota(fluxo.automatico, nomes, [fluxo]));
    const manuais = destinos.filter((impressora) => impressora.modo !== "automatico");
    if (manuais.length > 0) {
      notasDaRota.push(traduzir(idioma, "modo.automatico.manualEm", { nomes: listaDeNomes(manuais) }));
    }
  }
  const notas: string[] = [];
  if (automaticas.length > 0 && fluxos.some((fluxo) => fluxo.automatico)) {
    notas.push(traduzir(idioma, "modo.automatico.papelNormal"));
  }
  if (resumo.papeis.length > 1) notas.push(traduzir(idioma, "cartao.envio.variosPapeis"));
  for (const [a, b] of resumo.paresMistos) {
    notas.push(traduzir(idioma, "cartao.envio.misto", { a: String(a), b: String(b) }));
  }
  if (resumo.assinado) notas.push(traduzir(idioma, "cartao.envio.assinado"));
  if (reais.some((impressora) => impressora.modo === "jato" || impressora.modo === "laser")) {
    notas.push(traduzir(idioma, "modo.seletor.nota"));
  }
  if (reais.length === 0) notas.push(traduzir(idioma, "modo.automatico.soSimuladas"));
  else if (simuladas > 0) notas.push(traduzir(idioma, "cartao.envio.simuladas"));
  return {
    linhas,
    notas: [...notasDaRota, ...(escolhas.notasDoMeio ?? []), ...notas],
    notasDaRota,
    notasGerais: notas,
    linhaDoPapel,
    papelComExtras: rotaUnica !== undefined || (escolhas.papel ?? null) !== null,
  };
}
