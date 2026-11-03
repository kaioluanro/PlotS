import { idiomaAtual, traduzir, type Chave, type Idioma } from "./i18n";
import { nomeDeExibicao, textoDoEstado, type Impressora } from "./impressoras";
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
  | { tipo: "aviso"; passada: Passada; motivo: Motivo; respondido: boolean }
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
}

export type Comando =
  | "enviar_frente"
  | "enviar_verso"
  | "reimprimir"
  | "cancelar"
  | "continuar_esperando";

/** Motivos de erro que encerram o cartao sem nenhum trabalho na fila para acompanhar. */
const MOTIVOS_SEM_TRABALHO: ReadonlySet<string> = new Set(["interrompido", "ocupada"]);

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

function motivoDeFora(
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
}

const MAXIMO_DE_NOMES_NO_MOTIVO = 2;

function listaDeFora(motivos: readonly string[], idioma: Idioma): string {
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

export function textoDoEstadoDoCartao(cartao: Cartao, idioma: Idioma = idiomaAtual): string {
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
  ["a impressora está offline", "the printer is offline"],
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
 * que ela nao garante; `contexto` explica o resto.
 */
export type PapelDaLinha = "acao" | "ressalva" | "contexto";

export interface LinhaDoCartao {
  texto: string;
  papel: PapelDaLinha;
}

/** O que o cartao explica em cada estado, com os termos de quem so sabe o que a fila diz. */
export function linhasDoCartao(cartao: Cartao, idioma: Idioma = idiomaAtual): LinhaDoCartao[] {
  const { estado } = cartao;
  const em = (chave: Chave, parametros?: Record<string, string>) =>
    traduzir(idioma, chave, parametros);
  const linha = (texto: string, papel: PapelDaLinha): LinhaDoCartao => ({ texto, papel });
  const gesto = (): LinhaDoCartao[] =>
    cartao.recolocar
      ? frasesDoGesto(cartao.recolocar, idioma).map((texto) => linha(texto, "contexto"))
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
      if (cartao.trocaDePapel) {
        linhas.push(
          linha(em("cartao.trocaPapel", { papel: nomeDoPapel(cartao.trocaDePapel, idioma) }), "contexto"),
        );
      } else if (!cartao.simulada) {
        linhas.push(linha(em("cartao.proximo.semTroca"), "contexto"));
      }
      linhas.push(
        linha(em(cartao.simulada ? "cartao.proximo.cliqueSimulado" : "cartao.proximo.clique"), "acao"),
      );
      return linhas;
    }
    case "aviso":
      return estado.respondido
        ? [
            linha(textoDoMotivo(estado.motivo, idioma), "contexto"),
            linha(em("cartao.aviso.seguindo"), "contexto"),
          ]
        : [linha(textoDoMotivo(estado.motivo, idioma), "contexto")];
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
  const emVerso = estado.tipo === "imprimindo" && estado.passada === "verso";
  const semTrabalho = estado.tipo === "erro" && !cartaoEmAndamento(cartao);
  return {
    frente: estado.tipo === "proximo_lote",
    verso: {
      visivel: emFrente || emVerso || estado.tipo === "aguardando",
      liberado: estado.tipo === "aguardando",
    },
    // Sem lote ainda, a preparacao falhou: nao ha o que refazer sem pedir de novo. Thread morta
    // ou recusa ao iniciar tambem nao tem lote para refazer: o caminho e dispensar e imprimir de novo.
    reimprimir: estado.tipo === "erro" && cartao.total > 0 && !semTrabalho,
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
