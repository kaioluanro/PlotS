// Lógica da tela, sem React: o que cada estado diz, o que cada botão pode e como a folha é desenhada.
import type { Idioma } from "@plots/i18n";
import { existeChave, idiomaAtual, t, type Chave } from "./i18n";

export type EstadoFila =
  | "ociosa"
  | "recebendo"
  | "aguardando_verso"
  | "com_aviso"
  | "sem_modo"
  | "fora_do_windows"
  | "diferente";

export type ModoFila =
  | "jato"
  | "laser"
  | "traseira_face_para_baixo"
  | "gaveta_face_para_cima"
  | "sem_modo";

export interface FilaTela {
  nome: string;
  modo: ModoFila;
  estado: EstadoFila;
  historico: string;
  porta: string;
}

export interface CamadaTela {
  pagina: number;
  numero: number | null;
  /** Página em branco: não tem marca de propósito, e a tela não a trata como sem marca. */
  branca: boolean;
  graus: number;
  atras: boolean;
  larguraMm: number;
  alturaMm: number;
}

export type SituacaoFolha = "em_andamento" | "certa" | "errada" | "sem_marca";

export interface FolhaTela {
  numero: number;
  situacao: SituacaoFolha;
  motivos: string[];
  frente: CamadaTela[];
  verso: CamadaTela[];
}

export type Etapa = "vazia" | "aguardando_gesto" | "na_entrada" | "conferida";
export type Operacao =
  | "virar_pela_borda_curta"
  | "virar_pela_borda_longa"
  | "girar"
  | "inverter_ordem";

export interface ErroTela {
  codigo: string;
  detalhe: string | null;
}

export interface BandejaTela {
  etapa: Etapa;
  recebendo: boolean;
  esperando: number;
  folhas: FolhaTela[];
  corretas: number;
  semMarca: number;
  conferidas: number;
  gerais: string[];
  leitura: "como_saiu" | "virando_o_maco" | null;
  /** A forma de pegar veio da bandeja, porque o PlotS não mandou instrução. */
  semInstrucao: boolean;
  operacoes: Operacao[];
  paginasSemFolha: number;
  folhasSemVerso: number;
  aviso: ErroTela | null;
  /** O trabalho não é um documento inteiro que começa na 1: cada folha se confere sozinha. */
  conferenciaPorFolha: boolean;
}

export const PREFIXO_FILA = "PlotSim — ";

export function nomeDaFila(curto: string): string {
  return `${PREFIXO_FILA}${curto}`;
}

/** Fila sem modo, fora do Windows ou de outra configuração não tem bandeja nenhuma para consultar. */
export function temBandeja(fila: FilaTela | null): boolean {
  return fila !== null && !["sem_modo", "fora_do_windows", "diferente"].includes(fila.estado);
}

/** Estes detalhes são textos do núcleo, escritos só em português. Em outro idioma, fica a mensagem. */
const DETALHE_SO_EM_PORTUGUES: ReadonlySet<string> = new Set([
  "pdf_ilegivel",
  "erro_de_fila",
  "registro",
  "pasta",
  "processo",
  "leitura_filas",
  "historico",
  "porta_presa",
]);

function detalheVisivel(erro: ErroTela, idioma: Idioma): string | null {
  return idioma !== "pt-BR" && DETALHE_SO_EM_PORTUGUES.has(erro.codigo) ? null : erro.detalhe;
}

/** O texto do erro, com o detalhe que veio de fora quando a mensagem não o usa. */
export function textoDoErro(
  erro: unknown,
  idioma: Idioma = idiomaAtual,
): { texto: string; detalhe: string | null } {
  const dados = comoErroTela(erro);
  const chave = `erro.${dados.codigo}`;
  if (!existeChave(chave)) return { texto: t("erro.interno"), detalhe: detalheVisivel(dados, idioma) };
  const usaDetalhe = chave === "erro.nome_caractere" || chave === "erro.fila_alheia";
  return {
    texto: t(chave, dados.detalhe === null ? undefined : { detalhe: dados.detalhe }),
    detalhe: usaDetalhe ? null : detalheVisivel(dados, idioma),
  };
}

export function comoErroTela(erro: unknown): ErroTela {
  if (typeof erro === "object" && erro !== null && "codigo" in erro) {
    const { codigo, detalhe } = erro as { codigo: unknown; detalhe?: unknown };
    if (typeof codigo === "string") {
      return { codigo, detalhe: typeof detalhe === "string" ? detalhe : null };
    }
  }
  return { codigo: "interno", detalhe: typeof erro === "string" ? erro : null };
}

export function textoDoAviso(
  aviso: ErroTela,
  idioma: Idioma = idiomaAtual,
): { texto: string; detalhe: string | null } {
  const chave = `aviso.${aviso.codigo}`;
  return {
    texto: t(existeChave(chave) ? chave : "aviso.padrao"),
    detalhe: detalheVisivel(aviso, idioma),
  };
}

/** A forma do ponto muda com o estado: nada aqui depende só da cor. */
export type FormaDoPonto = "vazado" | "cheio" | "quadrado" | "discreto";

export function pontoDoEstado(estado: EstadoFila): { forma: FormaDoPonto; cor: string } {
  switch (estado) {
    case "recebendo":
      return { forma: "vazado", cor: "trabalhando" };
    case "aguardando_verso":
      return { forma: "cheio", cor: "esperando" };
    case "com_aviso":
    case "fora_do_windows":
    case "diferente":
    case "sem_modo":
      return { forma: "quadrado", cor: "problema" };
    case "ociosa":
      return { forma: "discreto", cor: "parado" };
  }
}

/** Escolhe a forma do texto pelo número: "1 folha", "2 folhas". */
export function textoComNumero(
  n: number,
  singular: Chave,
  plural: Chave,
  parametros: Record<string, string> = {},
): string {
  return n === 1 ? t(singular, parametros) : t(plural, { ...parametros, n: String(n) });
}

export function resumoDaBandeja(bandeja: BandejaTela | null): string {
  if (bandeja === null || bandeja.folhas.length === 0) return t("rodape.vazia");
  if (bandeja.conferidas > 0 || bandeja.leitura !== null) {
    const quantas = Math.max(bandeja.conferidas, bandeja.corretas + bandeja.semMarca);
    const total = String(quantas);
    const certas = String(bandeja.corretas);
    const semMarca = String(bandeja.semMarca);
    if (bandeja.conferenciaPorFolha) {
      // Cada folha foi conferida pelas marcas dela: o maço inteiro, como conjunto, nao foi.
      if (bandeja.semMarca > 0) {
        return quantas === 1
          ? t("rodape.porFolha.um.semMarca", { certas, semMarca })
          : t("rodape.porFolha.semMarca", { certas, total, semMarca });
      }
      return quantas === 1
        ? t("rodape.porFolha.um", { certas })
        : t("rodape.porFolha", { certas, total });
    }
    if (bandeja.semMarca > 0) {
      return quantas === 1
        ? t("rodape.resumo.um.semMarca", { certas, semMarca })
        : t("rodape.resumo.semMarca", { certas, total, semMarca });
    }
    return quantas === 1 ? t("rodape.resumo.um", { certas }) : t("rodape.resumo", { certas, total });
  }
  return textoComNumero(bandeja.folhas.length, "rodape.andamento.um", "rodape.andamento");
}

export interface Acao {
  ativa: boolean;
  /** Por que está desligada; sem motivo quando está ligada. */
  motivo: Chave | null;
}

const ligada: Acao = { ativa: true, motivo: null };
const desligada = (motivo: Chave): Acao => ({ ativa: false, motivo });

export function acaoVirar(bandeja: BandejaTela | null, selecionada: number | null): Acao {
  if (bandeja === null || bandeja.folhas.length === 0) return desligada("rodape.motivo.semFolhas");
  if (selecionada === null || !bandeja.folhas.some((f) => f.numero === selecionada)) {
    return desligada("rodape.motivo.semSelecao");
  }
  return ligada;
}

export function acaoRecolocar(bandeja: BandejaTela | null): Acao {
  switch (bandeja?.etapa ?? "vazia") {
    case "aguardando_gesto":
      return ligada;
    case "na_entrada":
      return desligada("rodape.motivo.jaNaEntrada");
    case "conferida":
      return desligada("rodape.motivo.loteCompleto");
    case "vazia":
      return desligada("rodape.motivo.semPrimeira");
  }
}

export function acaoLimpar(bandeja: BandejaTela | null): Acao {
  const haAlgo =
    bandeja !== null &&
    (bandeja.folhas.length > 0 || bandeja.esperando > 0 || bandeja.aviso !== null);
  return haAlgo ? ligada : desligada("rodape.motivo.nadaParaLimpar");
}

/** Mantém a folha escolhida enquanto ela existir; senão, fica com a primeira. */
export function folhaEscolhida(folhas: readonly FolhaTela[], atual: number | null): number | null {
  if (atual !== null && folhas.some((folha) => folha.numero === atual)) return atual;
  return folhas[0]?.numero ?? null;
}

export function camadasVisiveis(folha: FolhaTela, virada: boolean): CamadaTela[] {
  return virada ? folha.verso : folha.frente;
}

const PROPORCAO_A4 = 297 / 210;

/** Altura sobre largura da folha, medida na página que está à frente. */
export function proporcaoDaFolha(camadas: readonly CamadaTela[]): number {
  const base = camadas.find((c) => !c.atras) ?? camadas[0];
  if (!base || !(base.larguraMm > 0) || !(base.alturaMm > 0)) return PROPORCAO_A4;
  // Página muito fora do comum não deforma a grade inteira.
  return Math.min(2, Math.max(0.5, base.alturaMm / base.larguraMm));
}

export function deitada(graus: number): boolean {
  return graus === 90 || graus === 270;
}

/** Transformação do desenho: o espelho vem antes do giro, para o topo cair onde a física disse. */
export function transformacaoDaCamada(camada: Pick<CamadaTela, "graus" | "atras">): string {
  const giro = `rotate(${camada.graus}deg)`;
  return camada.atras ? `${giro} scaleX(-1)` : giro;
}

export function estiloDaCamada(
  camada: Pick<CamadaTela, "graus" | "atras">,
  proporcao: number,
): { transform: string; width?: string; height?: string; left?: string; top?: string } {
  const transform = transformacaoDaCamada(camada);
  if (!deitada(camada.graus)) return { transform };
  // Deitado, o desenho troca largura e altura para continuar ocupando a folha.
  const largura = proporcao * 100;
  const altura = (1 / proporcao) * 100;
  return {
    transform,
    width: `${largura}%`,
    height: `${altura}%`,
    left: `${(100 - largura) / 2}%`,
    top: `${(100 - altura) / 2}%`,
  };
}

function rotuloDaPagina(camada: CamadaTela | undefined): string {
  if (!camada || camada.branca) return t("folha.paginaEmBranco");
  return camada.numero === null
    ? t("folha.paginaSemMarca")
    : t("folha.paginaLida", { n: String(camada.numero) });
}

/** "frente p. 1 · verso p. 2", sempre pelo lado que está à vista. */
export function textoDasPaginas(camadas: readonly CamadaTela[]): string {
  return t("folha.paginas", {
    frente: rotuloDaPagina(camadas.find((c) => !c.atras)),
    verso: rotuloDaPagina(camadas.find((c) => c.atras)),
  });
}

/** Desenho de uma página que já chegou à tela, ou que falhou. Sem nenhum dos dois, ainda está a caminho. */
export type SituacaoDoDesenho = "pronta" | "falhou";

/** Alguma camada visível ainda está sem desenho? Página em branco não tem o que desenhar. */
export function pendenteDoPapel(
  camadas: readonly CamadaTela[],
  situacoes: ReadonlyMap<number, SituacaoDoDesenho>,
): boolean {
  return camadas.some((camada) => !camada.branca && !situacoes.has(camada.pagina));
}

/**
 * Sem desenho, o verso e a frente dizem isso com palavras diferentes, para não se confundirem.
 * Virada a folha, o lado à vista é o oposto das camadas: a página da frente passa a ser o verso.
 */
export function chaveDoDesenhoFalho(camada: CamadaTela, virada: boolean): Chave {
  return camada.atras !== virada ? "folha.semDesenhoVerso" : "folha.semDesenho";
}

/**
 * O ícone do resumo: certo só quando todas as folhas conferidas estão certas. Sem a instrução
 * do PlotS, o certo não ganha a cor de veredito e fica neutro.
 */
export function iconeDoResumo(
  bandeja: BandejaTela | null,
): "certo" | "com_erro" | "neutro" | null {
  if (bandeja === null || bandeja.folhas.length === 0) return null;
  if (bandeja.conferidas === 0 && bandeja.leitura === null) return null;
  const total = Math.max(bandeja.conferidas, bandeja.corretas + bandeja.semMarca);
  if (!(total > 0 && bandeja.corretas === total)) return "com_erro";
  return bandeja.semInstrucao ? "neutro" : "certo";
}

/** O botão de virar diz a folha escolhida; sem folha escolhida, diz só o verbo. */
export function rotuloDeVirar(
  bandeja: BandejaTela | null,
  selecionada: number | null,
): { chave: Chave; parametros: Record<string, string> } {
  const escolhida = selecionada !== null && bandeja?.folhas.some((f) => f.numero === selecionada);
  if (!escolhida || selecionada === null) return { chave: "rodape.virar", parametros: {} };
  return { chave: "rodape.virar.folha", parametros: { n: String(selecionada) } };
}

export function chaveDaSituacao(situacao: SituacaoFolha): Chave {
  const chaves: Record<SituacaoFolha, Chave> = {
    em_andamento: "folha.em_andamento",
    certa: "folha.certa",
    errada: "folha.errada",
    sem_marca: "folha.sem_marca",
  };
  return chaves[situacao];
}

export function textoDasOperacoes(operacoes: readonly Operacao[]): string {
  if (operacoes.length === 0) return t("centro.operacoes.nenhuma");
  const lista = operacoes.map((operacao) => t(`operacao.${operacao}`)).join(", ");
  return t("centro.operacoes", { lista });
}

export function formatarTamanho(bytes: number): string {
  const seguro = Number.isFinite(bytes) && bytes > 0 ? bytes : 0;
  if (seguro < 1024 * 1024) return `${Math.ceil(seguro / 1024)} KB`;
  const megas = (seguro / (1024 * 1024)).toLocaleString(idiomaAtual, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  });
  return `${megas} MB`;
}

/* --- O gesto escolhido à mão --- */

export type EntradaGesto = "traseira" | "gaveta" | "manual";
export type FaceGesto = "para_frente" | "para_tras" | "para_cima" | "para_baixo";
export type BordaGesto = "topo" | "base";

export interface EscolhaDoGesto {
  entrada: EntradaGesto | null;
  faceImpressa: FaceGesto | null;
  bordaQueEntra: BordaGesto | null;
  inverterOrdem: boolean | null;
}

export interface GestoPedido {
  entrada: EntradaGesto;
  faceImpressa: FaceGesto;
  bordaQueEntra: BordaGesto;
  inverterOrdem: boolean;
}

export const ESCOLHA_VAZIA: EscolhaDoGesto = {
  entrada: null,
  faceImpressa: null,
  bordaQueEntra: null,
  inverterOrdem: null,
};

/** Na traseira a folha fica de pé; deitada, nas outras. A face só existe conforme a entrada. */
export function facesDaEntrada(entrada: EntradaGesto | null): readonly FaceGesto[] {
  if (entrada === null) return [];
  return entrada === "traseira" ? ["para_frente", "para_tras"] : ["para_cima", "para_baixo"];
}

export function escolherEntrada(escolha: EscolhaDoGesto, entrada: EntradaGesto): EscolhaDoGesto {
  const faces = facesDaEntrada(entrada);
  const face = escolha.faceImpressa;
  return { ...escolha, entrada, faceImpressa: face !== null && faces.includes(face) ? face : null };
}

/** Só vira pedido com as quatro escolhas feitas: nada de valor padrão escondendo uma instrução mal lida. */
export function gestoCompleto(escolha: EscolhaDoGesto): GestoPedido | null {
  const { entrada, faceImpressa, bordaQueEntra, inverterOrdem } = escolha;
  if (entrada === null || faceImpressa === null || bordaQueEntra === null || inverterOrdem === null) {
    return null;
  }
  if (!facesDaEntrada(entrada).includes(faceImpressa)) return null;
  return { entrada, faceImpressa, bordaQueEntra, inverterOrdem };
}

/* --- Guarda dos desenhos das páginas --- */

/** Guarda só os mais recentes: uma bandeja longa não pode crescer sem limite na memória. */
export class GuardaDeDesenhos {
  private readonly itens = new Map<string, string>();

  constructor(private readonly limite: number) {}

  ler(chave: string): string | undefined {
    const valor = this.itens.get(chave);
    if (valor !== undefined) {
      this.itens.delete(chave);
      this.itens.set(chave, valor);
    }
    return valor;
  }

  guardar(chave: string, valor: string): void {
    this.itens.delete(chave);
    this.itens.set(chave, valor);
    while (this.itens.size > this.limite) {
      const antiga = this.itens.keys().next().value;
      if (antiga === undefined) return;
      this.itens.delete(antiga);
    }
  }

  esquecerTudo(): void {
    this.itens.clear();
  }

  get tamanho(): number {
    return this.itens.size;
  }
}
