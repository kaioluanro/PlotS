import type { Cartao, Recolocar } from "./cartoes";

export type LadoDaFace = "cima" | "baixo";
export type GestoDaFigura = "borda_de_cima" | "lateral" | "nao_virar" | "girar";

/** O que a figura precisa dizer: como o maco saiu, o que fazer com ele e como ele entra. */
export interface DesenhoDoGesto {
  saida: LadoDaFace;
  gesto: GestoDaFigura;
  entrada: LadoDaFace;
}

const OPOSTO: Readonly<Record<LadoDaFace, LadoDaFace>> = { cima: "baixo", baixo: "cima" };

/** Virar troca a face de lado; nao virar e girar no plano, nao. */
const TROCA_A_FACE = new Map<string, boolean>([
  ["borda_de_cima", true],
  ["lateral", true],
  ["nao_virar", false],
  ["girar", false],
]);

function ehLado(valor: string | null): valor is LadoDaFace {
  return valor === "cima" || valor === "baixo";
}

/** Sem dado, ou com um valor que esta versao nao conhece, nao ha figura: um desenho errado perde o lote. */
export function desenhoDoGesto(recolocar: Recolocar | null): DesenhoDoGesto | null {
  if (recolocar === null || !ehLado(recolocar.lado) || recolocar.virada === null) return null;
  const troca = TROCA_A_FACE.get(recolocar.virada);
  if (troca === undefined) return null;
  return {
    saida: troca ? OPOSTO[recolocar.lado] : recolocar.lado,
    gesto: recolocar.virada as GestoDaFigura,
    entrada: recolocar.lado,
  };
}

/** A figura so existe enquanto o cartao espera a recolocacao. */
export function desenhoDoCartao(cartao: Cartao): DesenhoDoGesto | null {
  return cartao.estado.tipo === "aguardando" || cartao.estado.tipo === "frentes_prontas"
    ? desenhoDoGesto(cartao.recolocar)
    : null;
}

/** O minimo que `rolarAteOCartao` usa da lista: um elemento real da pagina serve. */
export interface ListaQueRola {
  matches(seletor: string): boolean;
  addEventListener(tipo: "pointerleave" | "scroll", ouvinte: (evento?: SaidaDoPonteiro) => void): void;
  removeEventListener(tipo: "pointerleave" | "scroll", ouvinte: (evento?: SaidaDoPonteiro) => void): void;
}

/** Para onde o ponteiro foi ao sair da lista. */
export interface SaidaDoPonteiro {
  relatedTarget: unknown;
}

/** Marca a linha fixa na pagina: ir da lista para ela nao conta como sair da lista. */
export const ATRIBUTO_DA_LINHA = "data-linha-rolagem";

function foiParaALinha(evento: SaidaDoPonteiro | undefined): boolean {
  const destino = evento?.relatedTarget as { closest?: (seletor: string) => unknown } | null | undefined;
  return (destino?.closest?.(`[${ATRIBUTO_DA_LINHA}]`) ?? null) !== null;
}

export type PosicaoDoCartao = "acima" | "abaixo" | "em_vista";

interface Faixa {
  top: number;
  bottom: number;
}

/** Em vista quer dizer inteiro na lista; se ele for mais alto que ela, com o topo a vista. */
export function posicaoDoCartao(cartao: Faixa, lista: Faixa): PosicaoDoCartao {
  const folga = 1;
  if (cartao.top < lista.top - folga) return "acima";
  if (cartao.top >= lista.bottom) return "abaixo";
  if (cartao.bottom <= lista.bottom + folga) return "em_vista";
  return cartao.bottom - cartao.top > lista.bottom - lista.top ? "em_vista" : "abaixo";
}

/** O que a lista precisa saber do cartao que pediu para ser visto. */
export interface CartaoADescer {
  posicao(): PosicaoDoCartao;
  /** Leva o foco ao cartao, rolando so o necessario; se ele nao existir mais, leva a lista. */
  focar(): void;
}

interface Adiada {
  cartao: CartaoADescer;
  descer: () => void;
  ponteiroForaDaLista: () => boolean;
  naLinha: boolean;
  /** Depois de um clique na linha, a lista nao se move sozinha: este cartao espera o proprio clique. */
  soPelaLinha: boolean;
}

function focoEstaNaLinha(): boolean {
  const ativo = globalThis.document?.activeElement;
  return (ativo?.closest(`[${ATRIBUTO_DA_LINHA}]`) ?? null) !== null;
}

/** As descidas que estao esperando o ponteiro sair da lista, e quais a linha fixa ainda anuncia. */
export function criarRolagensAdiadas(linhaTemFoco: () => boolean = focoEstaNaLinha) {
  let adiadas: readonly Adiada[] = [];
  const ouvintes = new Set<() => void>();
  const avisar = () => ouvintes.forEach((ouvinte) => ouvinte());
  const primeira = () => adiadas.find((adiada) => adiada.naLinha);
  const quantas = (): number => adiadas.filter((adiada) => adiada.naLinha).length;
  // O botao ainda esta na pagina aqui: sem isto, o foco cairia no vazio quando ele sumisse.
  const saiuDaLinha = (adiada: Adiada) => {
    if (quantas() === 0 && linhaTemFoco()) adiada.cartao.focar();
  };
  const irParaAPrimeira = (): void => {
    const alvo = primeira();
    if (alvo === undefined) return;
    alvo.naLinha = false;
    for (const adiada of adiadas) adiada.soPelaLinha = adiada !== alvo;
    alvo.descer();
    alvo.cartao.focar();
    avisar();
  };
  return {
    assinar(ouvinte: () => void): () => void {
      ouvintes.add(ouvinte);
      return () => ouvintes.delete(ouvinte);
    },
    quantas,
    posicaoDaPrimeira: (): PosicaoDoCartao => primeira()?.cartao.posicao() ?? "em_vista",
    /** Desce ate o primeiro cartao, e o foco vai junto. Os outros continuam na linha. */
    irParaAPrimeira,
    /**
     * O clique de verdade, com o `detail` do evento: zero pelo teclado, um ou mais com o ponteiro.
     * A partir do segundo, e o resto de um clique duplo e nao aciona nada.
     */
    cliqueNaLinha(detalhe: number): void {
      if (detalhe <= 1) irParaAPrimeira();
    },
    /** O ponteiro saiu da linha: se tambem nao voltou para a lista, as descidas que esperavam acontecem. */
    ponteiroSaiuDaLinha(): void {
      for (const adiada of adiadas) {
        if (!adiada.soPelaLinha && adiada.ponteiroForaDaLista()) adiada.descer();
      }
    },
    incluir(adiada: Adiada): void {
      adiadas = [...adiadas, adiada];
      avisar();
    },
    excluir(adiada: Adiada): void {
      adiadas = adiadas.filter((outra) => outra !== adiada);
      if (adiada.naLinha) saiuDaLinha(adiada);
      avisar();
    },
    /** A lista rolou: a seta pode ter mudado, e o cartao que chegou em vista sai da linha. */
    rolou(adiada: Adiada): void {
      if (adiada.naLinha && adiada.cartao.posicao() === "em_vista") {
        adiada.naLinha = false;
        saiuDaLinha(adiada);
      }
      avisar();
    },
  };
}

/** O que a linha fixa sabia de si no ultimo aviso, enquanto ainda estava na pagina. */
export interface LeituraDaLinha {
  sobPonteiro: boolean;
  altura: number;
}

/**
 * Linha que some sob o ponteiro, por qualquer causa, deixa o lugar dela: a lista cresceria ate ali, e
 * o proximo clique cairia em um cartao. Devolve a altura do espaco, ou nada se ele nao deve ficar.
 */
export function espacoAoSumir(quantos: number, leitura: LeituraDaLinha): number | null {
  return quantos === 0 && leitura.sobPonteiro ? leitura.altura : null;
}

/** A lista e uma so: o cartao inclui as descidas dele aqui, e a linha fixa le daqui. */
export const rolagensAdiadas = criarRolagensAdiadas();

const CARTAO_SEM_MEDIDA: CartaoADescer = { posicao: () => "abaixo", focar: () => {} };

/**
 * Com o ponteiro sobre a lista, ela nao se move sob a mao de quem esta lendo ou mirando um botao: a
 * descida fica para quando o ponteiro sair, e a linha fixa anuncia o cartao ate ele estar em vista.
 * Devolve como desistir dela.
 */
export function rolarAteOCartao(
  lista: ListaQueRola | null,
  rolar: () => void,
  cartao: CartaoADescer = CARTAO_SEM_MEDIDA,
  registro: ReturnType<typeof criarRolagensAdiadas> = rolagensAdiadas,
): () => void {
  if (lista === null || !lista.matches(":hover")) {
    rolar();
    return () => {};
  }
  if (cartao.posicao() === "em_vista") return () => {};
  const parar = () => {
    lista.removeEventListener("pointerleave", aoSair);
    lista.removeEventListener("scroll", aoRolar);
    registro.excluir(adiada);
  };
  const descer = () => {
    parar();
    rolar();
  };
  // A caminho da linha, o ponteiro ainda nao largou a lista: descer aqui tiraria o botao de baixo dele.
  const aoSair = (evento?: SaidaDoPonteiro) => {
    if (!adiada.soPelaLinha && !foiParaALinha(evento)) descer();
  };
  // Mudanca de layout tambem gera rolagem: so sai da linha quando o cartao esta mesmo em vista.
  const aoRolar = () => registro.rolou(adiada);
  const adiada: Adiada = {
    cartao,
    descer,
    ponteiroForaDaLista: () => !lista.matches(":hover"),
    naLinha: true,
    soPelaLinha: false,
  };
  lista.addEventListener("pointerleave", aoSair);
  lista.addEventListener("scroll", aoRolar);
  registro.incluir(adiada);
  return parar;
}
