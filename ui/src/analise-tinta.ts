import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import { idiomaAtual, traduzir, type Chave, type Idioma } from "./i18n";

/**
 * Uma unica camada para a estimativa de tinta: os comandos, os eventos e os tipos que a gaveta
 * usa. Se o backend renomear um campo, a mudanca fica neste arquivo.
 *
 * Contrato esperado (snake_case nos corpos, camelCase nos argumentos do invoke):
 *
 *   iniciar_analise(caminho, paginas, copias, configuracao, pagina_atual) -> id
 *   recalcular_analise(id, configuracao, paginas, copias) -> ResultadoDaAnalise
 *   cancelar_analise(id) -> bool
 *   perfis_de_tinta() -> { perfis, aviso }
 *   salvar_perfil_de_tinta(perfil) -> { perfis, aviso }
 *   apagar_perfil_de_tinta(id) -> { perfis, aviso }
 *   configuracao_de_tinta() -> { configuracao, aviso }
 *   definir_configuracao_de_tinta(configuracao) -> { configuracao, aviso }
 *
 * O CSV sai por `exportar_csv`. O mutool entra no caminho CMYK quando disponivel; sem ele,
 * a analise cai no modo simples e `modoUsado` diz qual foi usado.
 *
 * Eventos (camelCase):
 *   plotinfo://progresso  { id, feitas, total, selecao: ResumoDaSelecao | null }
 *   plotinfo://pagina     { id, pagina, larguraMm, alturaMm, cobertura, tinta, custo }
 *   plotinfo://fim        { id, resultado: ResultadoDaAnalise }
 *   plotinfo://falha      { id, codigo, detalhe }
 *
 * O resultado da selecao traz `paginas` (os numeros que o backend interpretou), `quantidade`
 * e `totalComCopias`; a regra de 1, 2,5,7,9, 5-16 e vazio continua so no backend.
 */

export const EVENTOS_DA_TINTA = {
  progresso: "plotinfo://progresso",
  pagina: "plotinfo://pagina",
  fim: "plotinfo://fim",
  falha: "plotinfo://falha",
} as const;

export const CANAIS_DE_TINTA = ["ciano", "magenta", "amarelo", "preto"] as const;
export type CanalDeTinta = (typeof CANAIS_DE_TINTA)[number];
/** Quatro valores na ordem ciano, magenta, amarelo e preto. */
export type ValoresPorCanal = readonly [number, number, number, number];

export const LIMITE_PAGINAS_DO_PAINEL = 100_000;
export const LIMITE_COPIAS_DO_PAINEL = 999;

export type Qualidade = "rascunho" | "normal" | "alta";
export type ModoDeCor = "cmyk" | "simples";
export type TamanhoDaImagem = "a4" | "dpi";

export interface PerfilDeTinta {
  id: string;
  nome: string;
  moeda: string;
  volumes_ml: ValoresPorCanal;
  rendimentos_paginas: readonly [number, number, number, number];
  precos_ml: ValoresPorCanal;
  fatores_qualidade: readonly [number, number, number];
}

export interface ConfiguracaoDeTinta {
  perfil: string;
  qualidade: Qualidade;
  dpi: number;
  modo: ModoDeCor;
  cinzas_so_preto: boolean;
  tamanho_da_imagem: TamanhoDaImagem;
}

export interface ArquivoDaAnalise {
  nome: string;
  paginas: number;
  /** A medida vem da primeira pagina analisada; o resumo final nao a repete. */
  largura_mm: number | null;
  altura_mm: number | null;
  dpi: number;
  modo_usado: ModoDeCor;
}

export interface ResultadoDaPagina {
  numero: number;
  largura_mm: number;
  altura_mm: number;
  cobertura_pct: ValoresPorCanal;
  tinta_ml: ValoresPorCanal;
  custo: ValoresPorCanal;
  total_ml: number;
  total_custo: number;
  em_branco: boolean;
}

export interface CartuchoDaSelecao {
  /** Consumo do reservatorio que acaba primeiro, em percentual; pode passar de 100. */
  percentual_consumido: number;
  /** `null` quando nada e consumido; inteiros acima do seguro tambem viram `null`. */
  copias_ate_esvaziar: number | null;
  canal_que_acaba_primeiro: CanalDeTinta;
}

export interface ResumoDeTinta {
  /** Numeros das paginas escolhidas, como o backend os interpretou. */
  paginas: readonly number[];
  quantidade: number;
  total_com_copias: number;
  tinta: ValoresPorCanal;
  total_ml: number;
  custo: ValoresPorCanal;
  total_custo: number;
  cartucho: CartuchoDaSelecao;
}

export interface PaginaMaisCara {
  numero: number;
  tinta: ValoresPorCanal;
  custo: ValoresPorCanal;
}

export interface ResumoDoDocumento {
  tinta: ValoresPorCanal;
  total_ml: number;
  custo: ValoresPorCanal;
  total_custo: number;
  cobertura_media: ValoresPorCanal;
  ml_medio_por_pagina: number;
  pagina_mais_cara: PaginaMaisCara | null;
}

export interface ResultadoDaAnalise {
  id: string;
  arquivo: ArquivoDaAnalise;
  /** Identificador do perfil; o nome fica na lista de perfis carregada pela gaveta. */
  perfil: string;
  moeda: string;
  qualidade: Qualidade;
  copias: number;
  documento_inteiro: boolean;
  /** Codigo do motivo do modo simples, traduzido na gaveta; nulo quando nao ha aviso. */
  aviso: string | null;
  selecao: ResumoDeTinta;
  documento: ResumoDoDocumento;
}

export interface ProgressoDaAnalise {
  feitas: number;
  total: number;
  /** A selecao chega preenchida assim que as paginas dela ficam prontas. */
  selecao: ResumoDeTinta | null;
}

export interface EstadoDosPerfis {
  perfis: readonly PerfilDeTinta[];
  aviso: string | null;
}

export interface EstadoDaConfiguracao {
  configuracao: ConfiguracaoDeTinta;
  aviso: string | null;
}

type Objeto = Record<string, unknown>;

function objeto(valor: unknown): Objeto | null {
  return typeof valor === "object" && valor !== null && !Array.isArray(valor)
    ? (valor as Objeto)
    : null;
}

function texto(valor: unknown): string | null {
  return typeof valor === "string" && valor.trim() !== "" ? valor : null;
}

function numero(valor: unknown, minimo: number, maximo: number): number | null {
  return typeof valor === "number" &&
    Number.isFinite(valor) &&
    valor >= minimo &&
    valor <= maximo
    ? valor
    : null;
}

function inteiro(valor: unknown, minimo: number, maximo: number): number | null {
  const lido = numero(valor, minimo, maximo);
  return lido !== null && Number.isInteger(lido) ? lido : null;
}

function valores(
  valor: unknown,
  minimo: number,
  maximo: number,
  somenteInteiros = false,
): ValoresPorCanal | null {
  if (!Array.isArray(valor) || valor.length !== 4) return null;
  const [a, b, c, d] = valor;
  const ler = somenteInteiros ? inteiro : numero;
  const ca = ler(a, minimo, maximo);
  const cb = ler(b, minimo, maximo);
  const cc = ler(c, minimo, maximo);
  const cd = ler(d, minimo, maximo);
  if (ca === null || cb === null || cc === null || cd === null) return null;
  return [ca, cb, cc, cd];
}



function canalPorNome(valor: unknown): CanalDeTinta | null {
  if (typeof valor === "number" && Number.isInteger(valor) && valor >= 0 && valor < 4) {
    return CANAIS_DE_TINTA[valor] ?? null;
  }
  if (typeof valor === "string") {
    return CANAIS_DE_TINTA.find((canal) => canal === valor) ?? null;
  }
  return null;
}

function qualidade(valor: unknown): Qualidade | null {
  return valor === "rascunho" || valor === "normal" || valor === "alta" ? valor : null;
}

function modo(valor: unknown): ModoDeCor | null {
  return valor === "cmyk" || valor === "simples" ? valor : null;
}

function tamanhoDaImagem(valor: unknown): TamanhoDaImagem | null {
  return valor === "a4" || valor === "dpi" ? valor : null;
}

function moeda(valor: unknown): string | null {
  return typeof valor === "string" && /^[A-Z]{3}$/.test(valor) ? valor : null;
}

/** Le um perfil vindo do backend; qualquer campo fora da forma esperada recusa o perfil. */
export function normalizarPerfil(valor: unknown): PerfilDeTinta | null {
  const bruto = objeto(valor);
  if (bruto === null) return null;
  const id = texto(bruto.id);
  const nome = texto(bruto.nome);
  const moedaLida = moeda(bruto.moeda);
  const volumes = valores(bruto.volumes_ml, 0.000_001, 10_000);
  const rendimentos = valores(bruto.rendimentos_paginas, 1, 10_000_000, true);
  const precos = valores(bruto.precos_ml, 0, 10_000);
  const fatores = Array.isArray(bruto.fatores_qualidade) && bruto.fatores_qualidade.length === 3
    ? bruto.fatores_qualidade.map((item) => numero(item, 0.000_001, 10))
    : null;
  if (id === null || nome === null || moedaLida === null || volumes === null) return null;
  if (rendimentos === null || precos === null || fatores === null) return null;
  if (id.trim().length > 64 || nome.length > 200) return null;
  const [a, b, c] = fatores;
  if (a == null || b == null || c == null) return null;
  return {
    id,
    nome,
    moeda: moedaLida,
    volumes_ml: volumes,
    rendimentos_paginas: rendimentos,
    precos_ml: precos,
    fatores_qualidade: [a, b, c],
  };
}

export function normalizarConfiguracao(valor: unknown): ConfiguracaoDeTinta | null {
  const bruto = objeto(valor);
  if (bruto === null) return null;
  const perfil = texto(bruto.perfil);
  const q = qualidade(bruto.qualidade);
  const dpi = inteiro(bruto.dpi, 1, 2_400);
  const m = modo(bruto.modo);
  const cinzas = typeof bruto.cinzas_so_preto === "boolean" ? bruto.cinzas_so_preto : null;
  const tamanho = tamanhoDaImagem(bruto.tamanho_da_imagem);
  if (perfil === null || q === null || (dpi !== 150 && dpi !== 300) || m === null) return null;
  if (cinzas === null || tamanho === null) return null;
  return {
    perfil,
    qualidade: q,
    dpi,
    modo: m,
    cinzas_so_preto: cinzas,
    tamanho_da_imagem: tamanho,
  };
}

interface CanaisLidos {
  valores: ValoresPorCanal;
  total: number;
}

/** Le o objeto `{ c, m, y, k, total }` que o backend manda em cobertura, tinta e custo. */
function canais(valor: unknown, minimo: number, maximo: number): CanaisLidos | null {
  const bruto = objeto(valor);
  if (bruto === null) return null;
  const c = numero(bruto.c, minimo, maximo);
  const m = numero(bruto.m, minimo, maximo);
  const y = numero(bruto.y, minimo, maximo);
  const k = numero(bruto.k, minimo, maximo);
  if (c === null || m === null || y === null || k === null) return null;
  const total = numero(bruto.total, minimo, maximo) ?? c + m + y + k;
  if (!Number.isFinite(total)) return null;
  return { valores: [c, m, y, k], total };
}

function listaDePaginas(valor: unknown): readonly number[] | null {
  if (!Array.isArray(valor) || valor.length > LIMITE_PAGINAS_DO_PAINEL) return null;
  const paginas: number[] = [];
  for (const item of valor) {
    const numeroDaPagina = inteiro(item, 1, LIMITE_PAGINAS_DO_PAINEL);
    if (numeroDaPagina === null) return null;
    paginas.push(numeroDaPagina);
  }
  return paginas;
}

/** A pagina chega plana: `pagina`, `larguraMm`, `alturaMm`, `cobertura`, `tinta` e `custo`. */
export function normalizarPagina(valor: unknown): ResultadoDaPagina | null {
  const bruto = objeto(valor);
  if (bruto === null) return null;
  const numeroLido = inteiro(bruto.pagina, 1, LIMITE_PAGINAS_DO_PAINEL);
  const largura = numero(bruto.larguraMm, 0.001, 100_000);
  const altura = numero(bruto.alturaMm, 0.001, 100_000);
  const cobertura = canais(bruto.cobertura, 0, 100);
  const tinta = canais(bruto.tinta, 0, 100_000_000);
  const custo = canais(bruto.custo, 0, 100_000_000);
  if (numeroLido === null || largura === null || altura === null) return null;
  if (cobertura === null || tinta === null || custo === null) return null;
  return {
    numero: numeroLido,
    largura_mm: largura,
    altura_mm: altura,
    cobertura_pct: cobertura.valores,
    tinta_ml: tinta.valores,
    custo: custo.valores,
    total_ml: tinta.total,
    total_custo: custo.total,
    em_branco: cobertura.valores.every((valorDoCanal) => valorDoCanal === 0),
  };
}

function normalizarCartucho(valor: unknown): CartuchoDaSelecao | null {
  const bruto = objeto(valor);
  if (bruto === null) return null;
  const percentual = numero(
    bruto.percentualConsumido ?? bruto.percentual_consumido,
    0,
    1_000_000,
  );
  const primeiro = canalPorNome(
    bruto.canalQueAcabaPrimeiro ?? bruto.canal_que_acaba_primeiro,
  );
  if (percentual === null || primeiro === null) return null;
  const brutoCopias = bruto.copiasAteEsvaziar ?? bruto.copias_ate_esvaziar;
  let copias: number | null = null;
  if (typeof brutoCopias === "number" && Number.isFinite(brutoCopias) && brutoCopias >= 0) {
    copias = Number.isSafeInteger(brutoCopias) ? brutoCopias : null;
  }
  return {
    percentual_consumido: percentual,
    copias_ate_esvaziar: copias,
    canal_que_acaba_primeiro: primeiro,
  };
}

function normalizarSelecao(valor: unknown): ResumoDeTinta | null {
  const bruto = objeto(valor);
  if (bruto === null) return null;
  const paginas = listaDePaginas(bruto.paginas);
  const tinta = canais(bruto.tinta, 0, 100_000_000);
  const custo = canais(bruto.custo, 0, 100_000_000);
  const cartucho = normalizarCartucho(bruto.cartucho);
  if (paginas === null || tinta === null || custo === null || cartucho === null) return null;
  const quantidade = inteiro(bruto.quantidade, 0, LIMITE_PAGINAS_DO_PAINEL) ?? paginas.length;
  const total = inteiro(
    bruto.totalComCopias ?? bruto.total_com_copias,
    0,
    LIMITE_PAGINAS_DO_PAINEL * LIMITE_COPIAS_DO_PAINEL,
  ) ?? quantidade;
  return {
    paginas,
    quantidade,
    total_com_copias: total,
    tinta: tinta.valores,
    total_ml: tinta.total,
    custo: custo.valores,
    total_custo: custo.total,
    cartucho,
  };
}

function normalizarPaginaMaisCara(valor: unknown): PaginaMaisCara | null {
  if (valor === null || valor === undefined) return null;
  const bruto = objeto(valor);
  if (bruto === null) return null;
  const numeroLido = inteiro(bruto.numero, 1, LIMITE_PAGINAS_DO_PAINEL);
  const tinta = canais(bruto.tinta, 0, 100_000_000);
  const custo = canais(bruto.custo, 0, 100_000_000);
  if (numeroLido === null || tinta === null || custo === null) return null;
  return { numero: numeroLido, tinta: tinta.valores, custo: custo.valores };
}

function normalizarDocumento(
  valor: unknown,
  paginasDoArquivo: number,
): ResumoDoDocumento | null {
  const bruto = objeto(valor);
  if (bruto === null) return null;
  const tinta = canais(bruto.tinta, 0, 100_000_000);
  const custo = canais(bruto.custo, 0, 100_000_000);
  const cobertura = canais(bruto.coberturaMedia ?? bruto.cobertura_media, 0, 100);
  if (tinta === null || custo === null || cobertura === null) return null;
  const media =
    numero(bruto.mlMedioPorPagina ?? bruto.ml_medio_por_pagina, 0, 100_000_000) ??
    (paginasDoArquivo > 0 ? tinta.total / paginasDoArquivo : 0);
  if (!Number.isFinite(media)) return null;
  return {
    tinta: tinta.valores,
    total_ml: tinta.total,
    custo: custo.valores,
    total_custo: custo.total,
    cobertura_media: cobertura.valores,
    ml_medio_por_pagina: media,
    pagina_mais_cara: normalizarPaginaMaisCara(bruto.paginaMaisCara),
  };
}

/** Le o resumo final. Sem os campos obrigatorios, a gaveta mostra a falha em vez de numeros tortos. */
export function normalizarResultado(
  valor: unknown,
  id: string,
  nomeDeReserva = "",
): ResultadoDaAnalise | null {
  const bruto = objeto(valor);
  if (bruto === null) return null;
  const nome = texto(nomeDeReserva) ?? "—";
  const paginasDoArquivo = inteiro(
    bruto.totalDePaginas ?? bruto.total_de_paginas,
    1,
    LIMITE_PAGINAS_DO_PAINEL,
  );
  const modoUsado = modo(bruto.modoUsado ?? bruto.modo_usado);
  const dpi = inteiro(bruto.dpi, 1, 2_400);
  const perfil = texto(bruto.perfil);
  const moedaLida = moeda(bruto.moeda);
  const q = qualidade(bruto.qualidade);
  const copias = inteiro(bruto.copias, 1, LIMITE_COPIAS_DO_PAINEL);
  const documentoInteiro =
    typeof bruto.documentoInteiro === "boolean"
      ? bruto.documentoInteiro
      : typeof bruto.documento_inteiro === "boolean"
        ? bruto.documento_inteiro
        : null;
  if (paginasDoArquivo === null || modoUsado === null || dpi === null) return null;
  if (perfil === null || moedaLida === null || q === null || copias === null) return null;
  if (documentoInteiro === null) return null;
  const selecao = normalizarSelecao(bruto.selecao);
  const documento = normalizarDocumento(bruto.documento, paginasDoArquivo);
  if (selecao === null || documento === null) return null;
  return {
    id,
    arquivo: {
      nome,
      paginas: paginasDoArquivo,
      largura_mm: null,
      altura_mm: null,
      dpi,
      modo_usado: modoUsado,
    },
    perfil,
    moeda: moedaLida,
    qualidade: q,
    copias,
    documento_inteiro: documentoInteiro,
    aviso: texto(bruto.aviso),
    selecao,
    documento,
  };
}

export function normalizarProgresso(valor: unknown): ProgressoDaAnalise | null {
  const bruto = objeto(valor);
  if (bruto === null) return null;
  const total = inteiro(bruto.total, 1, LIMITE_PAGINAS_DO_PAINEL);
  const feitas = inteiro(bruto.feitas, 0, LIMITE_PAGINAS_DO_PAINEL);
  if (total === null || feitas === null || feitas > total) return null;
  const selecao =
    bruto.selecao === null || bruto.selecao === undefined
      ? null
      : normalizarSelecao(bruto.selecao);
  return { feitas, total, selecao };
}

/** A recusa chega como objeto com motivo/detalhe ou como texto; a mensagem final e nossa. */
export function motivoDaFalha(erro: unknown, idioma: Idioma = idiomaAtual): string {
  if (typeof erro === "string" && erro.trim() !== "") return erro;
  const bruto = objeto(erro);
  const detalhe = texto(bruto?.motivo) ?? texto(bruto?.detalhe) ?? texto(bruto?.mensagem);
  if (detalhe !== null) return detalhe;
  const codigo = texto(bruto?.codigo);
  const chave = codigo === null ? null : CHAVES_DE_FALHA[codigo];
  return traduzir(idioma, chave ?? "plotinfo.erro.generico");
}

const CHAVES_DE_AVISO: Readonly<Record<string, Chave>> = {
  pasta_indisponivel: "plotinfo.aviso.pastaIndisponivel",
  arquivo_ilegivel: "plotinfo.aviso.arquivoIlegivel",
  arquivo_grande: "plotinfo.aviso.arquivoGrande",
  arquivo_corrompido: "plotinfo.aviso.arquivoCorrompido",
  versao_desconhecida: "plotinfo.aviso.versaoDesconhecida",
  perfil_ignorado: "plotinfo.aviso.perfilIgnorado",
  configuracao_ignorada: "plotinfo.aviso.configuracaoIgnorada",
  sem_conversor: "plotinfo.aviso.semConversor",
  falha_do_conversor: "plotinfo.aviso.falhaDoConversor",
  modo_simples: "plotinfo.aviso.modoSimples",
};

/** O aviso do arquivo de perfis e um codigo; a interface mostra a frase inteira. */
export function mensagemDeAviso(
  codigo: string | null,
  idioma: Idioma = idiomaAtual,
): string | null {
  if (codigo === null || codigo.trim() === "") return null;
  return traduzir(idioma, CHAVES_DE_AVISO[codigo] ?? "plotinfo.aviso.generico", {
    aviso: codigo,
  });
}

const CHAVES_DE_FALHA: Readonly<Record<string, Chave>> = {
  documento: "plotinfo.erro.documento",
  configuracao_invalida: "plotinfo.erro.configuracao",
  selecao_invalida: "plotinfo.erro.selecao",
  copias_invalidas: "plotinfo.erro.copias",
  perfil_nao_encontrado: "plotinfo.erro.perfil",
  renderizacao: "plotinfo.erro.renderizacao",
  biblioteca: "plotinfo.erro.biblioteca",
  calculo: "plotinfo.erro.calculo",
  analise_desconhecida: "plotinfo.erro.generico",
  cache_ausente: "plotinfo.erro.cacheAusente",
  ainda_nao_analisada: "plotinfo.erro.aindaNaoAnalisada",
  cancelado: "plotinfo.erro.cancelado",
  interrompido: "plotinfo.erro.interrompido",
  iniciar_trabalho: "plotinfo.erro.iniciar",
};

function idConfere(valor: unknown, idAtual: string | null): boolean {
  if (idAtual === null) return true;
  return typeof valor === "string" ? valor === idAtual : true;
}

interface OuvintesDaAnalise {
  aoProgresso: (progresso: ProgressoDaAnalise) => void;
  aoPagina: (pagina: ResultadoDaPagina) => void;
  aoFim: (resultado: ResultadoDaAnalise) => void;
  aoFalha: (motivo: string) => void;
}

/**
 * Assina os quatro eventos e espera as quatro assinaturas antes de devolver o cancelamento.
 * Quem chama so dispara a analise depois disso, para nao perder as primeiras paginas.
 */
export async function ouvirAnaliseDeTinta(
  ouvintes: OuvintesDaAnalise,
  idAtual: () => string | null,
  nomeDeReserva: string,
): Promise<() => void> {
  const cancelar: UnlistenFn[] = [];
  let ativo = true;
  const registrar = async (
    evento: string,
    tratar: (payload: unknown) => void,
  ): Promise<void> => {
    try {
      const cancelarEvento = await listen<unknown>(evento, (mensagem) => {
        if (ativo) tratar(mensagem.payload);
      });
      if (ativo) cancelar.push(cancelarEvento);
      else cancelarEvento();
    } catch {
      // Sem este evento, os outros caminhos da gaveta continuam.
    }
  };

  await Promise.all([
    registrar(EVENTOS_DA_TINTA.progresso, (payload) => {
      const bruto = objeto(payload);
      if (bruto === null || !idConfere(bruto.id, idAtual())) return;
      const progresso = normalizarProgresso(bruto);
      if (progresso !== null) ouvintes.aoProgresso(progresso);
    }),
    registrar(EVENTOS_DA_TINTA.pagina, (payload) => {
      const bruto = objeto(payload);
      if (bruto === null || !idConfere(bruto.id, idAtual())) return;
      const pagina = normalizarPagina(bruto);
      if (pagina !== null) ouvintes.aoPagina(pagina);
    }),
    registrar(EVENTOS_DA_TINTA.fim, (payload) => {
      const bruto = objeto(payload);
      if (bruto === null || !idConfere(bruto.id, idAtual())) return;
      const id = typeof bruto.id === "string" ? bruto.id : (idAtual() ?? "");
      const resultado = normalizarResultado(bruto.resultado, id, nomeDeReserva);
      if (resultado !== null) {
        ouvintes.aoFim(resultado);
      } else {
        ouvintes.aoFalha(traduzir(idiomaAtual, "plotinfo.erro.resposta"));
      }
    }),
    registrar(EVENTOS_DA_TINTA.falha, (payload) => {
      const bruto = objeto(payload);
      if (bruto === null || !idConfere(bruto.id, idAtual())) return;
      ouvintes.aoFalha(motivoDaFalha(bruto));
    }),
  ]);

  return () => {
    ativo = false;
    for (const cancelarEvento of cancelar) cancelarEvento();
    cancelar.length = 0;
  };
}

/** `null` significa o documento inteiro; a interpretacao do texto e do backend. */
export function textoDePaginas(valor: string): string | null {
  const limpo = valor.trim();
  return limpo === "" ? null : limpo;
}

export function lerCopiasDoPainel(valor: string): number | null {
  const limpo = valor.trim();
  if (!/^\d{1,4}$/.test(limpo)) return null;
  const numeroLido = Number(limpo);
  return Number.isInteger(numeroLido) &&
    numeroLido >= 1 &&
    numeroLido <= LIMITE_COPIAS_DO_PAINEL
    ? numeroLido
    : null;
}

/** `paginas` e a contagem que o backend interpretou; `copias` e o campo independente da gaveta. */
export function totalDePaginas(paginas: number, copias: number): number | null {
  if (!Number.isSafeInteger(paginas) || paginas < 0 || paginas > LIMITE_PAGINAS_DO_PAINEL) {
    return null;
  }
  if (!Number.isSafeInteger(copias) || copias < 1 || copias > LIMITE_COPIAS_DO_PAINEL) {
    return null;
  }
  const total = paginas * copias;
  return Number.isSafeInteger(total) ? total : null;
}

export function configuracaoPadrao(perfil: string): ConfiguracaoDeTinta {
  return {
    perfil,
    qualidade: "normal",
    dpi: 150,
    modo: "cmyk",
    cinzas_so_preto: true,
    tamanho_da_imagem: "a4",
  };
}

export function iniciarAnalise(
  caminho: string,
  paginas: string | null,
  copias: number,
  configuracao: ConfiguracaoDeTinta,
  paginaAtual: number | null,
): Promise<string> {
  return invoke<string>("iniciar_analise", {
    caminho,
    paginas,
    copias,
    configuracao,
    paginaAtual,
  });
}

/**
 * Reaproveita as coberturas em cache. Quando o backend devolve o resumo novo, a gaveta o usa;
 * quando devolve vazio, o resumo chega em `plotinfo://fim`.
 */
export function recalcularAnalise(
  id: string,
  configuracao: ConfiguracaoDeTinta,
  paginas: string | null,
  copias: number,
): Promise<unknown> {
  return invoke<unknown>("recalcular_analise", { id, configuracao, paginas, copias });
}

export function cancelarAnalise(id: string): Promise<boolean> {
  return invoke<boolean>("cancelar_analise", { id });
}

export async function perfisDeTinta(): Promise<EstadoDosPerfis> {
  const bruto = await invoke<unknown>("perfis_de_tinta");
  return normalizarEstadoDosPerfis(bruto);
}

export function normalizarEstadoDosPerfis(valor: unknown): EstadoDosPerfis {
  const bruto = objeto(valor);
  const lista = Array.isArray(bruto?.perfis) ? bruto.perfis : [];
  const perfis: PerfilDeTinta[] = [];
  for (const item of lista) {
    const perfil = normalizarPerfil(item);
    if (perfil !== null) perfis.push(perfil);
  }
  return { perfis, aviso: texto(bruto?.aviso) };
}

export async function salvarPerfilDeTinta(perfil: PerfilDeTinta): Promise<EstadoDosPerfis> {
  const bruto = await invoke<unknown>("salvar_perfil_de_tinta", { perfil });
  return normalizarEstadoDosPerfis(bruto);
}

export async function apagarPerfilDeTinta(id: string): Promise<EstadoDosPerfis> {
  const bruto = await invoke<unknown>("apagar_perfil_de_tinta", { id });
  return normalizarEstadoDosPerfis(bruto);
}

export async function configuracaoDeTinta(): Promise<EstadoDaConfiguracao> {
  const bruto = await invoke<unknown>("configuracao_de_tinta");
  return normalizarEstadoDaConfiguracao(bruto);
}

export function normalizarEstadoDaConfiguracao(valor: unknown): EstadoDaConfiguracao {
  const bruto = objeto(valor);
  const configuracao = normalizarConfiguracao(bruto?.configuracao) ?? configuracaoPadrao("");
  return { configuracao, aviso: texto(bruto?.aviso) };
}

export async function definirConfiguracaoDeTinta(
  configuracao: ConfiguracaoDeTinta,
): Promise<EstadoDaConfiguracao> {
  const bruto = await invoke<unknown>("definir_configuracao_de_tinta", { configuracao });
  return normalizarEstadoDaConfiguracao(bruto);
}

export function exportarCsv(id: string, caminho: string): Promise<void> {
  return invoke<void>("exportar_csv", { id, caminho });
}

/** Nome sugerido sem separadores de caminho e sempre com a extensao do CSV. */
export function nomeDoCsv(nomeDoDocumento: string): string {
  const base = nomeDoDocumento.trim().replace(/[\\/:*?"<>|]/g, "_");
  const semExtensao = base.replace(/\.[^.]+$/, "");
  return `tinta-${semExtensao === "" ? "documento" : semExtensao}.csv`;
}

export function escolherCaminhoCsv(
  nomeDoDocumento: string,
  idioma: Idioma = idiomaAtual,
): Promise<string | null> {
  return save({
    title: traduzir(idioma, "plotinfo.csv.titulo"),
    defaultPath: nomeDoCsv(nomeDoDocumento),
    filters: [{ name: traduzir(idioma, "plotinfo.csv.filtro"), extensions: ["csv"] }],
  });
}

export function formatarNumero(
  valor: number,
  idioma: Idioma = idiomaAtual,
  maximoDeCasas = 2,
): string {
  if (!Number.isFinite(valor)) return "—";
  return new Intl.NumberFormat(idioma, {
    maximumFractionDigits: maximoDeCasas,
    minimumFractionDigits: 0,
  }).format(valor);
}

export function formatarMl(valor: number, idioma: Idioma = idiomaAtual): string {
  return `${formatarNumero(valor, idioma, 2)} ${traduzir(idioma, "plotinfo.ml")}`;
}

export function formatarPercentual(valor: number, idioma: Idioma = idiomaAtual): string {
  return `${formatarNumero(valor, idioma, 1)}%`;
}

export function formatarMoeda(
  valor: number,
  moedaDoPerfil: string,
  idioma: Idioma = idiomaAtual,
): string {
  if (!Number.isFinite(valor)) return "—";
  try {
    return new Intl.NumberFormat(idioma, {
      style: "currency",
      currency: moedaDoPerfil,
      maximumFractionDigits: 2,
    }).format(valor);
  } catch {
    return `${moedaDoPerfil} ${formatarNumero(valor, idioma)}`;
  }
}

export interface ResumoDaBarra {
  totalPaginas: number;
  ml: number;
  valor: number;
  moeda: string;
}

export function resumoDaBarra(resultado: ResultadoDaAnalise): ResumoDaBarra {
  return {
    totalPaginas: resultado.selecao.total_com_copias,
    ml: resultado.selecao.total_ml,
    valor: resultado.selecao.total_custo,
    moeda: resultado.moeda,
  };
}

const PAPEIS: ReadonlyArray<{ chave: Chave; largura: number; altura: number }> = [
  { chave: "plotinfo.papel.a4", largura: 210, altura: 297 },
  { chave: "plotinfo.papel.a3", largura: 297, altura: 420 },
  { chave: "plotinfo.papel.a5", largura: 148, altura: 210 },
  { chave: "plotinfo.papel.carta", largura: 215.9, altura: 279.4 },
  { chave: "plotinfo.papel.oficio", largura: 215.9, altura: 355.6 },
];

/** Nome do papel pela medida, nas duas orientacoes; fora da tabela, mostra a medida. */
export function rotuloDoPapel(largura: number, altura: number, idioma: Idioma = idiomaAtual): string {
  for (const papel of PAPEIS) {
    const mesma =
      Math.abs(largura - papel.largura) <= 2 && Math.abs(altura - papel.altura) <= 2;
    const girada =
      Math.abs(largura - papel.altura) <= 2 && Math.abs(altura - papel.largura) <= 2;
    if (mesma || girada) return traduzir(idioma, papel.chave);
  }
  return traduzir(idioma, "plotinfo.papel.medida", {
    largura: formatarNumero(largura, idioma, 0),
    altura: formatarNumero(altura, idioma, 0),
  });
}

export const CHAVE_DO_CANAL: Readonly<Record<CanalDeTinta, Chave>> = {
  ciano: "plotinfo.canal.ciano",
  magenta: "plotinfo.canal.magenta",
  amarelo: "plotinfo.canal.amarelo",
  preto: "plotinfo.canal.preto",
};

export const CHAVE_DA_QUALIDADE: Readonly<Record<Qualidade, Chave>> = {
  rascunho: "plotinfo.qualidade.rascunho",
  normal: "plotinfo.qualidade.normal",
  alta: "plotinfo.qualidade.alta",
};

export const CHAVE_DO_MODO: Readonly<Record<ModoDeCor, Chave>> = {
  cmyk: "plotinfo.modo.cmyk",
  simples: "plotinfo.modo.simples",
};

export const CHAVE_DO_TAMANHO: Readonly<Record<TamanhoDaImagem, Chave>> = {
  a4: "plotinfo.tamanho.a4",
  dpi: "plotinfo.tamanho.dpi",
};