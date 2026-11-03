import { idiomaAtual, traduzir, type Chave, type Idioma } from "./i18n";
import { nomeDeExibicao, textoDoEstado, type Impressora } from "./impressoras";
import { filasComAjusteAgora, linhaDoAjusteNoEnvio } from "./preferencias";
import type { Aba } from "./tabs";

export type Passada = "frente" | "verso";

export interface Motivo {
  codigo: string;
  texto: string;
}

export type EstadoCartao =
  | { tipo: "preparando" }
  | { tipo: "imprimindo"; passada: Passada }
  | { tipo: "aguardando" }
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
  sequencia: { frente: FaceDaSequencia[]; verso: FaceDaSequencia[] } | null;
  viraParaLer: boolean;
  /** Na divisao, o numero de cada lote desta impressora no documento. */
  lotesNoDocumento?: readonly number[];
  /** As paginas desses lotes, como o backend as planejou. */
  faixasNoDocumento?: readonly FaixaDoLote[];
  /** O driver nao aceitou o ajuste salvo: este trabalho saiu com o padrao da impressora. */
  ajusteRecusado?: boolean;
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
]);

/**
 * Concluido e cancelado acabaram: o resto ainda depende de alguem ou da fila. Um cartao cuja
 * thread morreu, ou que foi recusado ao iniciar, nao espera nada: nao pode segurar a impressora.
 */
export function cartaoEmAndamento(cartao: Cartao): boolean {
  const { estado } = cartao;
  if (estado.tipo === "concluido" || estado.tipo === "cancelado") return false;
  if (estado.tipo === "erro" && MOTIVOS_SEM_TRABALHO.has(estado.motivo.codigo)) return false;
  return true;
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
 * Na altura curta, o rodape so sai quando nao ha o que fazer nele: a divisao desta aba ainda corre,
 * ou toda marcada da lista tem trabalho correndo aqui. Marcada ocupada por outra aba, ou cartao
 * ja encerrado, nao tira o rodape: desmarcar ainda resolve, e o motivo precisa ficar a vista.
 */
export function rodapeCedeAoCartao(
  envio: EstadoDoEnvio,
  aba: Aba | null,
  lista: readonly Impressora[],
  ocupadas: ReadonlySet<string>,
): boolean {
  if (aba === null) return false;
  if (envio.semDestino === true) return true;
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
  /** Esta aba ainda corre uma divisao: nada se envia dela ate o fim, e a altura curta tira o rodape da frente. */
  semDestino?: boolean;
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
  const rotulo =
    destinos.length > 0
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
  return { rotulo, motivo, desabilitado: destinos.length === 0, destinos };
}

/** So o envio real pede confirmacao: a simulacao nao manda nada a lugar nenhum. */
export function precisaConfirmar(destinos: readonly Impressora[]): boolean {
  return destinos.some((impressora) => impressora.tipo === "fila");
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

const LOTES_NUMA_LISTA = 8;

/** Numeros de lote em uma linha; muitos viram o comeco, o fim e a contagem. */
export function listaDeLotes(lotes: readonly number[], idioma: Idioma = idiomaAtual): string {
  if (lotes.length <= LOTES_NUMA_LISTA) return lotes.join(", ");
  return traduzir(idioma, "modo.lotes.resumo", {
    inicio: lotes.slice(0, LOTES_NUMA_LISTA - 2).join(", "),
    ultimo: String(lotes.at(-1) ?? ""),
    n: String(lotes.length),
  });
}

/** O que ainda nao terminou nesta impressora, numerado como no documento. */
export function lotesSemImprimir(cartao: Cartao): readonly number[] {
  if (cartao.estado.tipo === "concluido") return [];
  return (cartao.lotesNoDocumento ?? []).slice(Math.max(cartao.lote - 1, 0));
}

/** O numero do lote atual no documento; sem divisao, ou fora da lista, nao ha. */
export function loteNoDocumento(cartao: Cartao): number | null {
  if (cartao.lote < 1) return null;
  return cartao.lotesNoDocumento?.[cartao.lote - 1] ?? null;
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

/** Em "proximo lote", o trabalho ja avancou: o maco a retirar e o do lote anterior a este. */
function fraseDeRetirar(cartao: Cartao, idioma: Idioma): string | null {
  const lote = cartao.lotesNoDocumento?.[cartao.lote - 2];
  const faixa = cartao.faixasNoDocumento?.find((item) => item.lote === lote);
  if (lote === undefined || faixa === undefined) return null;
  return traduzir(idioma, "modo.proximo.retirar", {
    n: String(lote),
    paginas: textoDasPaginas(faixa, idioma),
  });
}

/** Na divisao, as linhas do lote no cartao. Fora dela, ou se a contagem nao bate, nao ha. */
export interface LinhasDoLote {
  titulo: string | null;
  detalhe: string | null;
  semImprimir: string | null;
}

function trechoDosLotes(
  lotes: readonly number[],
  chaves: { um: Chave; varios: Chave },
  idioma: Idioma,
): string {
  return traduzir(idioma, lotes.length === 1 ? chaves.um : chaves.varios, {
    lista: listaDeLotes(lotes, idioma),
  });
}

/** O numero que vale e o do documento: e por ele que os macos se juntam. */
export function linhasDoLote(cartao: Cartao, idioma: Idioma = idiomaAtual): LinhasDoLote | null {
  const lotes = cartao.lotesNoDocumento;
  // Uma contagem que nao bate com a do envio nao autoriza afirmar numero nenhum.
  if (!lotes || (cartao.total > 0 && cartao.total !== lotes.length)) return null;
  const todos = trechoDosLotes(lotes, { um: "modo.lotes.um", varios: "modo.lotes.varios" }, idioma);
  const { tipo } = cartao.estado;
  if (tipo === "concluido") return { titulo: todos, detalhe: null, semImprimir: null };
  const atual = loteNoDocumento(cartao);
  const titulo =
    atual === null ? null : traduzir(idioma, "modo.lote.documento", { n: String(atual) });
  if (tipo === "erro" || tipo === "cancelado") {
    const semImprimir = trechoDosLotes(
      lotesSemImprimir(cartao),
      { um: "modo.semImprimir.um", varios: "modo.semImprimir.varios" },
      idioma,
    );
    return { titulo, detalhe: null, semImprimir };
  }
  if (titulo === null) return { titulo: todos, detalhe: null, semImprimir: null };
  const depois = lotes.length - cartao.lote;
  const detalhe =
    depois > 0
      ? traduzir(idioma, "modo.lote.restam", { n: String(depois) })
      : traduzir(idioma, "modo.lote.ultimo");
  return { titulo, detalhe, semImprimir: null };
}

/** A linha de dados fora da divisao. */
export function linhaDoLote(cartao: Cartao, idioma: Idioma = idiomaAtual): string | null {
  return cartao.total > 0
    ? traduzir(idioma, "cartao.lote", { n: String(cartao.lote), total: String(cartao.total) })
    : null;
}

export function nomeDoCartao(cartao: Cartao, idioma: Idioma = idiomaAtual): string {
  if (!cartao.simulada) return cartao.nome;
  return traduzir(
    idioma,
    cartao.impressora === "simulada:gaveta"
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
const ORIENTACOES: ReadonlyMap<string, Readonly<Record<Passada, OrientacaoDoAviso>>> = new Map([
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
  ["documento_mudou", "modo.motivo.documentoMudou"],
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
  const colocacao = traduzir(idioma, recolocar.lado === "cima" ? "cartao.gesto.cima" : "cartao.gesto.baixo");
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
  const numeros = faces
    .map((face) => (face.numero === null ? traduzir(idioma, "cartao.sequencia.branca") : face.numero))
    .join(", ");
  const girada = faces.length > 0 && faces.every((face) => face.girada);
  return girada ? `${numeros} ${traduzir(idioma, "cartao.sequencia.girada")}` : numeros;
}

/** So a simulacao mostra a ordem: e o que ela existe para conferir. */
export function sequenciaDoCartao(cartao: Cartao, idioma: Idioma = idiomaAtual): string[] {
  if (cartao.sequencia === null) return [];
  return [
    traduzir(idioma, "cartao.sequencia.frente", {
      lista: listaDeFaces(cartao.sequencia.frente, idioma),
    }),
    traduzir(idioma, "cartao.sequencia.verso", {
      lista: listaDeFaces(cartao.sequencia.verso, idioma),
    }),
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
  switch (estado.tipo) {
    case "preparando":
      return [linha(em("cartao.preparando"), "contexto")];
    case "imprimindo":
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
      if (cartao.simulada) {
        return [
          linha(em("cartao.frente.simulada"), "contexto"),
          linha(em("cartao.simulada.proximo"), "contexto"),
          ...gesto(),
          linha(em("cartao.simulada.clique"), "acao"),
        ];
      }
      return [
        linha(em("cartao.frente.confirmada"), "ressalva"),
        linha(em("cartao.esperar"), "contexto"),
        ...gesto(),
        linha(em("cartao.verso.clique"), "acao"),
      ];
    }
    case "proximo_lote": {
      const linhas = [
        linha(em(cartao.simulada ? "cartao.verso.simulado" : "cartao.verso.confirmado"), "ressalva"),
      ];
      const retirar = fraseDeRetirar(cartao, idioma);
      if (retirar !== null) linhas.push(linha(retirar, "contexto"));
      if (cartao.trocaDePapel) {
        linhas.push(
          linha(em("cartao.trocaPapel", { papel: nomeDoPapel(cartao.trocaDePapel, idioma) }), "contexto"),
        );
      } else if (retirar !== null) {
        linhas.push(linha(em("modo.proximo.depois"), "contexto"));
      } else if (!cartao.simulada) {
        linhas.push(linha(em("cartao.proximo.semTroca"), "contexto"));
      }
      linhas.push(
        linha(em(cartao.simulada ? "cartao.proximo.cliqueSimulado" : "cartao.proximo.clique"), "acao"),
      );
      return linhas;
    }
    case "aviso": {
      const orientacao = ORIENTACOES.get(estado.motivo.codigo)?.[estado.passada];
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
      return [linha(textoDoMotivo(estado.motivo, idioma), "contexto")];
    case "concluido": {
      const linhas = [
        linha(em(cartao.simulada ? "cartao.concluido.simulado" : "cartao.concluido"), "contexto"),
      ];
      if (cartao.viraParaLer) linhas.push(linha(em("cartao.virarParaLer"), "acao"));
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
    verso: {
      visivel: emFrente || emVerso || estado.tipo === "aguardando",
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
  return traduzir(idioma, cartao.simulada ? "cartao.simular.frente" : "imprimir.frente");
}

export interface ResumoDoEnvio {
  paginas: number;
  lotes: number;
  papeis: PapelDoCartao[];
  paresMistos: [number, number][];
  assinado: boolean;
}

/** As linhas da confirmacao: quem recebe, quantas paginas e em que papel. */
export function linhasDoEnvio(
  destinos: readonly Impressora[],
  resumo: ResumoDoEnvio,
  idioma: Idioma = idiomaAtual,
  filasComAjuste: ReadonlySet<string> = filasComAjusteAgora(),
): { linhas: string[]; notas: string[] } {
  const reais = destinos.filter((impressora) => impressora.tipo === "fila");
  const simuladas = destinos.length - reais.length;
  const linhas = [
    ...reais.map((impressora) => nomeDeExibicao(impressora, idioma)),
    traduzir(idioma, resumo.lotes === 1 ? "cartao.envio.paginasUmLote" : "cartao.envio.paginas", {
      paginas: String(resumo.paginas),
      lotes: String(resumo.lotes),
    }),
    traduzir(idioma, "cartao.envio.papel", {
      lista: resumo.papeis.map((papel) => nomeDoPapel(papel, idioma)).join(", "),
    }),
  ];
  const doAjuste = linhaDoAjusteNoEnvio(reais, filasComAjuste, idioma);
  if (doAjuste !== null) linhas.push(doAjuste);
  const notas: string[] = [];
  if (resumo.papeis.length > 1) notas.push(traduzir(idioma, "cartao.envio.variosPapeis"));
  for (const [a, b] of resumo.paresMistos) {
    notas.push(traduzir(idioma, "cartao.envio.misto", { a: String(a), b: String(b) }));
  }
  if (resumo.assinado) notas.push(traduzir(idioma, "cartao.envio.assinado"));
  if (reais.some((impressora) => impressora.modo === "jato" || impressora.modo === "laser")) {
    notas.push(traduzir(idioma, "modo.seletor.nota"));
  }
  if (simuladas > 0) notas.push(traduzir(idioma, "cartao.envio.simuladas"));
  return { linhas, notas };
}
