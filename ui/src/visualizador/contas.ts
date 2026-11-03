import type { Chave } from "../i18n";
/**
 * Contas do visualizador, sem DOM nem pdf.js. Tudo o que vem do PDF e tratado como hostil: tamanho
 * sem sentido, contagem absurda e valores que nao sao numeros caem em limites conhecidos.
 */

export const ZOOM_MINIMO = 0.25;
/** Menor zoom que o ajuste a largura aceita: abaixo do minimo dos botoes, a pagina ainda cabe. */
export const PISO_DO_AJUSTE = 0.02;
export const ZOOM_MAXIMO = 4;
const DEGRAUS_DE_ZOOM = [0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4];

/** 100% e o tamanho fisico da pagina numa tela de 96 dpi. */
export const PIXELS_POR_PONTO = 96 / 72;
export const ESPACO_ENTRE_PAGINAS = 8;
export const MARGEM_DA_ROLAGEM = 12;

/** Acima disso o navegador deixa de posicionar elementos com precisao. */
const ALTURA_MAXIMA_DA_ROLAGEM = 30_000_000;

/** Contagem maior que esta so aparece em PDF hostil e nao ganha tabelas em memoria. */
export const LIMITE_DE_PAGINAS = 100_000;

/** Um lado de pagina em pontos: o maximo da norma do PDF, multiplicado por uma UserUnit alta. */
const LADO_MAXIMO_EM_PONTOS = 144_000;
const PAGINA_PADRAO: TamanhoPagina = { largura: 595, altura: 842 };

/** Memoria de cada canvas: 16 milhoes de pixels ocupam cerca de 64 MB. */
const PIXELS_MAXIMOS_POR_CANVAS = 16_000_000;
const LADO_MAXIMO_DO_CANVAS = 8192;

export interface TamanhoPagina {
  largura: number;
  altura: number;
}

export interface Pilha {
  /** Topo de cada item, em pixels, dentro da area rolavel. */
  topos: readonly number[];
  alturas: readonly number[];
  alturaTotal: number;
}

export interface Faixa {
  /** Primeira pagina, contada a partir de 1. */
  inicio: number;
  /** Ultima pagina, inclusive. */
  fim: number;
}

/** Faixa vazia: nada a mostrar. */
export const FAIXA_VAZIA: Faixa = { inicio: 1, fim: 0 };

function finitoPositivo(valor: number): boolean {
  return Number.isFinite(valor) && valor > 0;
}

/** Tamanho aproveitavel de uma pagina. Um valor sem sentido nao pode quebrar as contas seguintes. */
export function tamanhoSeguro(largura: number, altura: number): TamanhoPagina {
  if (!finitoPositivo(largura) || !finitoPositivo(altura)) return PAGINA_PADRAO;
  return {
    largura: Math.min(Math.max(largura, 1), LADO_MAXIMO_EM_PONTOS),
    altura: Math.min(Math.max(altura, 1), LADO_MAXIMO_EM_PONTOS),
  };
}

/** Total de paginas aproveitavel, ou nulo quando o numero nao faz sentido para este documento. */
export function totalSeguro(total: number): number | null {
  return Number.isInteger(total) && total >= 1 && total <= LIMITE_DE_PAGINAS ? total : null;
}

/** Empilha itens de alturas conhecidas, com a margem nas pontas e o espaco entre eles. */
export function empilhar(alturas: readonly number[], espaco: number, margem: number): Pilha {
  const topos = new Array<number>(alturas.length);
  let cursor = margem;
  alturas.forEach((altura, indice) => {
    topos[indice] = cursor;
    cursor += altura + espaco;
  });
  const alturaTotal = alturas.length === 0 ? margem * 2 : cursor - espaco + margem;
  return { topos, alturas, alturaTotal };
}

/** Altura de cada pagina em pixels de tela, no zoom pedido. */
export function alturasNoZoom(tamanhos: readonly TamanhoPagina[], zoom: number): number[] {
  return tamanhos.map((tamanho) => tamanho.altura * zoom * PIXELS_POR_PONTO);
}

function primeiroVisivel(pilha: Pilha, rolagem: number): number {
  let baixo = 0;
  let alto = pilha.topos.length - 1;
  while (baixo < alto) {
    const meio = Math.floor((baixo + alto) / 2);
    const topo = pilha.topos[meio] ?? 0;
    const altura = pilha.alturas[meio] ?? 0;
    if (topo + altura <= rolagem) baixo = meio + 1;
    else alto = meio;
  }
  return baixo;
}

/** Paginas que tocam a janela de rolagem. */
export function faixaVisivel(pilha: Pilha, rolagem: number, alturaDaVisao: number): Faixa {
  const total = pilha.topos.length;
  if (total === 0) return FAIXA_VAZIA;
  const fundo = rolagem + Math.max(alturaDaVisao, 0);
  const primeiro = primeiroVisivel(pilha, rolagem);
  let ultimo = primeiro;
  while (ultimo + 1 < total && (pilha.topos[ultimo + 1] ?? Infinity) < fundo) ultimo += 1;
  return { inicio: primeiro + 1, fim: ultimo + 1 };
}

/**
 * Pagina atual: a que ocupa mais altura na janela; em empate, a de cima. Nas pontas da rolagem e a
 * primeira ou a ultima, porque uma pagina curta poderia nunca vencer a vizinha maior.
 */
export function paginaAtual(pilha: Pilha, rolagem: number, alturaDaVisao: number): number {
  const total = pilha.topos.length;
  if (total === 0 || rolagem <= 0) return 1;
  const rolagemMaxima = pilha.alturaTotal - alturaDaVisao;
  if (rolagemMaxima > 0 && rolagem >= rolagemMaxima - 1) return total;

  const faixa = faixaVisivel(pilha, rolagem, alturaDaVisao);
  const fundo = rolagem + alturaDaVisao;
  let melhor = faixa.inicio;
  let maiorAltura = -1;
  for (let pagina = faixa.inicio; pagina <= faixa.fim; pagina += 1) {
    const topo = pilha.topos[pagina - 1] ?? 0;
    const altura = pilha.alturas[pagina - 1] ?? 0;
    const visivel = Math.min(topo + altura, fundo) - Math.max(topo, rolagem);
    if (visivel > maiorAltura) {
      maiorAltura = visivel;
      melhor = pagina;
    }
  }
  return melhor;
}

/**
 * Paginas a manter desenhadas: as visiveis e uma margem de cada lado. O teto vale para documento com
 * paginas minusculas, em que centenas caberiam na tela; o que passa dele e descartado pelas pontas.
 */
export function janelaDePaginas(
  visiveis: Faixa,
  total: number,
  margem: number,
  teto: number,
): Faixa {
  if (total < 1 || teto < 1 || visiveis.fim < visiveis.inicio) return FAIXA_VAZIA;
  const inicio = Math.max(1, visiveis.inicio - Math.max(margem, 0));
  const fim = Math.min(total, visiveis.fim + Math.max(margem, 0));
  if (fim - inicio + 1 <= teto) return { inicio, fim };
  const centro = Math.floor((visiveis.inicio + visiveis.fim) / 2);
  const novoInicio = Math.max(inicio, centro - Math.floor((teto - 1) / 2));
  return { inicio: novoInicio, fim: Math.min(fim, novoInicio + teto - 1) };
}

/** Maior zoom em que o documento inteiro ainda cabe na altura que a rolagem aguenta. */
function tetoDoZoom(alturaBase: number): number {
  if (!finitoPositivo(alturaBase)) return ZOOM_MAXIMO;
  return Math.min(ZOOM_MAXIMO, ALTURA_MAXIMA_DA_ROLAGEM / alturaBase);
}

/**
 * Zoom aceito para este documento. `alturaBase` e a altura do documento em 100%; um documento com
 * dezenas de milhares de paginas pode reduzir o teto e, no extremo, o piso.
 */
export function limitarZoom(zoom: number, alturaBase = 0): number {
  const teto = tetoDoZoom(alturaBase);
  const piso = Math.min(ZOOM_MINIMO, teto);
  if (!Number.isFinite(zoom)) return Math.min(1, teto);
  return Math.min(Math.max(zoom, piso), teto);
}

/** Um degrau a menos de 1% do zoom atual nao muda nada na tela e por isso e pulado. */
const DIFERENCA_MINIMA_DE_DEGRAU = 0.01;

export function aumentarZoom(zoom: number, alturaBase = 0): number {
  const proximo = DEGRAUS_DE_ZOOM.find(
    (degrau) => degrau > zoom * (1 + DIFERENCA_MINIMA_DE_DEGRAU),
  );
  return limitarZoom(proximo ?? ZOOM_MAXIMO, alturaBase);
}

export function diminuirZoom(zoom: number, alturaBase = 0): number {
  // Abaixo do minimo dos botoes, diminuir nao faz nada: o zoom nao sobe de volta.
  if (zoom <= ZOOM_MINIMO + 0.001) return zoom;
  const anteriores = DEGRAUS_DE_ZOOM.filter(
    (degrau) => degrau < zoom * (1 - DIFERENCA_MINIMA_DE_DEGRAU),
  );
  return limitarZoom(anteriores[anteriores.length - 1] ?? ZOOM_MINIMO, alturaBase);
}

export function podeAumentar(zoom: number, alturaBase = 0): boolean {
  return limitarZoom(ZOOM_MAXIMO, alturaBase) > zoom + 0.001;
}

export function podeDiminuir(zoom: number, alturaBase = 0): boolean {
  return limitarZoom(ZOOM_MINIMO, alturaBase) < zoom - 0.001;
}

/**
 * Posicao da pagina na area rolavel: centrada na janela, para uma pagina larga em outro ponto do
 * documento nao tirar as demais do centro. A que nao cabe encosta na margem esquerda.
 */
export function esquerdaDaPagina(largura: number, larguraDaVisao: number): number {
  return Math.max((larguraDaVisao - largura) / 2, MARGEM_DA_ROLAGEM);
}

/**
 * Zoom em que a maior pagina ocupa a largura disponivel, descontadas as margens. Arredonda para
 * baixo, em pontos percentuais, para que o documento nunca passe da largura e nao surja barra.
 */
export function zoomParaLargura(
  larguraDaPagina: number,
  larguraDisponivel: number,
  alturaBase = 0,
): number {
  const util = larguraDisponivel - MARGEM_DA_ROLAGEM * 2;
  if (!finitoPositivo(larguraDaPagina) || !finitoPositivo(util)) return limitarZoom(1, alturaBase);
  const bruto = util / (larguraDaPagina * PIXELS_POR_PONTO);
  // O ajuste pode ficar abaixo do zoom minimo dos botoes: janela estreita com pagina enorme.
  const arredondado = Math.max(Math.floor(bruto * 100 + 1e-9) / 100, PISO_DO_AJUSTE);
  return Math.min(arredondado, tetoDoZoom(alturaBase));
}

/** Largura da pagina mais larga entre as medidas: a que define o ajuste a largura. */
export function maiorLargura(tamanhos: readonly TamanhoPagina[]): number {
  return tamanhos.reduce((maior, tamanho) => Math.max(maior, tamanho.largura), 0);
}

/**
 * Zoom depois de uma acao do teclado ou da barra, ou nulo quando nada muda (por exemplo, Ctrl+-
 * no limite). Quem recebe nulo nao sai do ajuste a largura.
 */
export function proximoZoom(
  acao: AcaoDeZoom,
  zoom: number,
  alturaBase = 0,
): number | null {
  if (acao === "ajustar") return null;
  const novo = acao === "aumentar" ? aumentarZoom(zoom, alturaBase) : diminuirZoom(zoom, alturaBase);
  return Math.abs(novo - zoom) < 0.001 ? null : novo;
}

export function percentualDoZoom(zoom: number): number {
  return Math.round(zoom * 100);
}

/**
 * Lugar na leitura: a pagina, quanto dela ja passou e, antes de ela comecar, a distancia ate o topo.
 * Sobrevive a mudanca de zoom e as medidas de pagina que chegam depois.
 */
export interface Ancora {
  pagina: number;
  fracao: number;
  folga: number;
}

export function ancoraDaRolagem(pilha: Pilha, rolagem: number): Ancora {
  if (pilha.topos.length === 0) return { pagina: 1, fracao: 0, folga: 0 };
  const indice = primeiroVisivel(pilha, rolagem);
  const topo = pilha.topos[indice] ?? 0;
  const altura = pilha.alturas[indice] ?? 0;
  const passou = rolagem - topo;
  return {
    pagina: indice + 1,
    fracao: altura > 0 ? Math.min(Math.max(passou / altura, 0), 1) : 0,
    folga: Math.max(-passou, 0),
  };
}

export function rolagemDaAncora(pilha: Pilha, ancora: Ancora): number {
  const indice = Math.min(Math.max(ancora.pagina - 1, 0), pilha.topos.length - 1);
  if (indice < 0) return 0;
  const topo = pilha.topos[indice] ?? 0;
  const altura = pilha.alturas[indice] ?? 0;
  return Math.max(topo + altura * ancora.fracao - ancora.folga, 0);
}

/** Rolagem que poe a pagina no topo da janela, com a margem acima dela. */
export function rolagemDaPagina(pilha: Pilha, pagina: number): number {
  return rolagemDaAncora(pilha, { pagina, fracao: 0, folga: MARGEM_DA_ROLAGEM });
}

/**
 * Pixels reais de uma pagina por pixel de tela. Fica abaixo do pedido quando o canvas passaria do
 * limite de memoria ou de lado: a pagina aparece menos nitida, mas nao derruba a janela.
 */
export function escalaDoCanvas(
  larguraNaTela: number,
  alturaNaTela: number,
  pixelsPorPonto: number,
): number {
  const pedida = finitoPositivo(pixelsPorPonto) ? pixelsPorPonto : 1;
  if (!finitoPositivo(larguraNaTela) || !finitoPositivo(alturaNaTela)) return pedida;
  const porArea = Math.sqrt(PIXELS_MAXIMOS_POR_CANVAS / (larguraNaTela * alturaNaTela));
  const porLado = LADO_MAXIMO_DO_CANVAS / Math.max(larguraNaTela, alturaNaTela);
  return Math.min(pedida, porArea, porLado);
}

export type AcaoDeZoom = "aumentar" | "diminuir" | "ajustar";

export interface TeclaDeZoom {
  key: string;
  ctrlKey: boolean;
  altKey: boolean;
}

/**
 * Ctrl com +, - e 0, inclusive no teclado numerico e no = sem Shift. Ctrl+Alt fica de fora: e assim
 * que o AltGr chega, e ele digita outros caracteres.
 */
export function acaoDeZoom(tecla: TeclaDeZoom): AcaoDeZoom | null {
  if (!tecla.ctrlKey || tecla.altKey) return null;
  switch (tecla.key) {
    case "+":
    case "=":
      return "aumentar";
    case "-":
      return "diminuir";
    case "0":
      return "ajustar";
    default:
      return null;
  }
}

/** Abaixo desta largura as miniaturas comecam recolhidas, para sobrar espaco para a pagina. */
const LARGURA_PARA_MINIATURAS = 560;

export const LARGURA_DA_MINIATURA = 112;
const ALTURA_MAXIMA_DA_MINIATURA = 150;
const ALTURA_DO_ROTULO = 18;
const ESPACO_ENTRE_MINIATURAS = 8;
const MARGEM_DAS_MINIATURAS = 8;

/** A preferencia do usuario, quando existe, vale mais que a largura da area. */
export function miniaturasAbertas(preferencia: boolean | null, larguraDaArea: number): boolean {
  return preferencia ?? larguraDaArea >= LARGURA_PARA_MINIATURAS;
}

/** Tamanho da imagem da miniatura, proporcional a pagina e dentro da caixa da lista. */
export function tamanhoDaMiniatura(tamanho: TamanhoPagina): TamanhoPagina {
  const escala = Math.min(
    LARGURA_DA_MINIATURA / tamanho.largura,
    ALTURA_MAXIMA_DA_MINIATURA / tamanho.altura,
  );
  return {
    largura: Math.max(Math.round(tamanho.largura * escala), 1),
    altura: Math.max(Math.round(tamanho.altura * escala), 1),
  };
}

/** Folga entre a moldura de uma miniatura e a seguinte, para o foco nao encostar no vizinho. */
const FOLGA_DO_ITEM_DE_MINIATURA = 8;

export function pilhaDeMiniaturas(tamanhos: readonly TamanhoPagina[]): Pilha {
  return empilhar(
    tamanhos.map(
      (tamanho) =>
        tamanhoDaMiniatura(tamanho).altura + ALTURA_DO_ROTULO + FOLGA_DO_ITEM_DE_MINIATURA,
    ),
    ESPACO_ENTRE_MINIATURAS,
    MARGEM_DAS_MINIATURAS,
  );
}

/**
 * Miniaturas presentes na lista: as que estao na janela, uma margem, e a que tem o foco do teclado,
 * que nao pode sumir enquanto a pessoa navega por ela.
 */
export function miniaturasAMontar(
  visiveis: Faixa,
  total: number,
  margem: number,
  foco: number | null,
): number[] {
  const janela = janelaDePaginas(visiveis, total, margem, 64);
  const paginas: number[] = [];
  for (let pagina = janela.inicio; pagina <= janela.fim; pagina += 1) paginas.push(pagina);
  if (foco !== null && foco >= 1 && foco <= total && !paginas.includes(foco)) {
    if (foco < janela.inicio) paginas.unshift(foco);
    else paginas.push(foco);
  }
  return paginas;
}

/** Proxima miniatura a receber o foco, ou nulo se a tecla nao navega pela lista. */
export function miniaturaPorTecla(atual: number, tecla: string, total: number): number | null {
  if (total < 1) return null;
  const limitar = (pagina: number) => Math.min(Math.max(pagina, 1), total);
  switch (tecla) {
    case "ArrowDown":
      return limitar(atual + 1);
    case "ArrowUp":
      return limitar(atual - 1);
    case "Home":
      return 1;
    case "End":
      return total;
    default:
      return null;
  }
}

/** Rolagem que deixa o item inteiro na janela, mexendo o minimo; sem mudanca devolve a atual. */
export function rolagemParaMostrar(
  pilha: Pilha,
  pagina: number,
  rolagem: number,
  alturaDaVisao: number,
): number {
  const topo = pilha.topos[pagina - 1];
  const altura = pilha.alturas[pagina - 1];
  if (topo === undefined || altura === undefined) return rolagem;
  if (topo < rolagem) return Math.max(topo - MARGEM_DAS_MINIATURAS, 0);
  if (topo + altura > rolagem + alturaDaVisao) {
    return Math.max(topo + altura + MARGEM_DAS_MINIATURAS - alturaDaVisao, 0);
  }
  return rolagem;
}

function nomeDoErro(erro: unknown): unknown {
  return typeof erro === "object" && erro !== null && "name" in erro ? erro.name : null;
}

/** O pdf.js nunca recebe senha: se ele pedir uma, a resposta e a mesma recusa do backend. */
export function pdfPediuSenha(erro: unknown): boolean {
  return nomeDoErro(erro) === "PasswordException";
}

/** Cancelar um desenho rejeita a promessa dele; isso nao e erro para a interface. */
export function foiCancelamento(erro: unknown): boolean {
  return nomeDoErro(erro) === "RenderingCancelledException";
}

/** Barra: as setas e Home/End mudam o botao que tem o foco; nas outras teclas nada acontece. */
export function proximoBotaoDaBarra(atual: number, tecla: string, total: number): number | null {
  if (total < 1) return null;
  switch (tecla) {
    case "ArrowRight":
      return (atual + 1) % total;
    case "ArrowLeft":
      return (atual - 1 + total) % total;
    case "Home":
      return 0;
    case "End":
      return total - 1;
    default:
      return null;
  }
}

/** Depois deste atraso o texto "Abrindo o PDF" aparece; antes, a abertura rapida nao pisca. */
export const ATRASO_DO_TEXTO_DE_CARGA_MS = 200;

/** Agenda o aparecimento do texto de carga; a funcao devolvida cancela a espera. */
export function aguardarTextoDeCarga(mostrar: () => void): () => void {
  const espera = setTimeout(mostrar, ATRASO_DO_TEXTO_DE_CARGA_MS);
  return () => clearTimeout(espera);
}

/** Estado de uma pagina na area e na miniatura: cada um tem o proprio rotulo para tecnologia assistiva. */
export type EstadoDeDesenho = "carregando" | "pronto" | "erro";

export function chaveDoRotuloDaPagina(estado: EstadoDeDesenho): Chave {
  if (estado === "carregando") return "visualizador.pagina.carregando";
  if (estado === "erro") return "visualizador.pagina.rotuloErro";
  return "visualizador.pagina.rotulo";
}

export function chaveDoRotuloDaMiniatura(estado: EstadoDeDesenho): Chave {
  return estado === "erro" ? "visualizador.miniaturas.irErro" : "visualizador.miniaturas.ir";
}
