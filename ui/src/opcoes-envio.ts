import { nomeDoPapel } from "./cartoes";
import { idiomaAtual, traduzir, type Idioma } from "./i18n";
import { nomeDeExibicao, type Impressora } from "./impressoras";
import type { ModoDistribuicao, PapelDeAjuste } from "./tabs";

export const PAPEIS_DE_AJUSTE: readonly PapelDeAjuste[] = ["a4", "a5", "carta", "oficio", "legal"];

export function nomeDoPapelDeAjuste(papel: PapelDeAjuste, idioma: Idioma = idiomaAtual): string {
  return nomeDoPapel({ formato: papel, larguraMm: 0, alturaMm: 0 }, idioma);
}

/** A menor e a maior escala do trecho; iguais depois de arredondar, viram um numero so. */
export function textoDaEscala(escalas: readonly [number, number]): string {
  const [menor, maior] = escalas.map((valor) => Math.round(valor * 100));
  return menor === maior ? `${menor}%` : `${menor}–${maior}%`;
}

/** Quantas vezes mais rapida, com uma casa so quando ela diz alguma coisa: "2x", "1,3x". */
export function textoDoFator(razao: number, idioma: Idioma = idiomaAtual): string {
  const arredondado = Math.round(razao * 10) / 10;
  return new Intl.NumberFormat(idioma, { maximumFractionDigits: 1 }).format(arredondado);
}

export interface ComparacaoDoEnvio {
  rapida: string;
  lenta: string;
  razao: number;
}

/** O papel de destino com a escala, para a linha do papel. Sem ajuste, nada a acrescentar. */
export function textoDoPapelAjustado(
  papel: PapelDeAjuste | null | undefined,
  escalas: readonly [number, number] | null | undefined,
  idioma: Idioma = idiomaAtual,
): string | null {
  if (!papel || !escalas) return null;
  return traduzir(idioma, "ajuste.escala", {
    papel: nomeDoPapelDeAjuste(papel, idioma),
    escala: textoDaEscala(escalas),
  });
}

interface TrechoDoFluxo {
  automatico: boolean;
  inicio: number;
  fim: number;
}

interface ParteDaNota {
  impressora: string;
  faixas?: readonly { inicio: number; fim: number }[];
}

export interface OpcoesDaNota {
  controle?: boolean;
  comparacao?: ComparacaoDoEnvio | null;
  modo: ModoDistribuicao;
  destinos: readonly Impressora[];
  /** Por que rota cada trecho do documento sai numa impressora automatica. */
  fluxos?: readonly TrechoDoFluxo[];
  /** Na divisao, o que cada impressora recebe. */
  partes?: readonly ParteDaNota[];
}

function nomeDoDestino(id: string, destinos: readonly Impressora[], idioma: Idioma): string {
  const destino = destinos.find((impressora) => impressora.id === id);
  return destino ? nomeDeExibicao(destino, idioma) : id;
}

/** Quem nao e automatica so tem a rota manual. A automatica, so se recebe papel que nao sai sozinho. */
function passaPelaRotaManual(
  destino: Impressora,
  fluxos: readonly TrechoDoFluxo[],
  partes: readonly ParteDaNota[] | undefined,
): boolean {
  if (destino.modo !== "automatico") return true;
  const manuais = fluxos.filter((fluxo) => !fluxo.automatico);
  if (partes === undefined) return manuais.length > 0;
  const faixas = partes.find((parte) => parte.impressora === destino.id)?.faixas ?? [];
  return manuais.some((fluxo) =>
    faixas.some((faixa) => faixa.inicio <= fluxo.fim && fluxo.inicio <= faixa.fim),
  );
}

/** A automatica passa pela rota sem gesto se recebe algum papel que sai sozinho. */
function passaPelaRotaAutomatica(
  destino: Impressora,
  fluxos: readonly TrechoDoFluxo[],
  partes: readonly ParteDaNota[] | undefined,
): boolean {
  if (destino.modo !== "automatico") return false;
  const automaticos = fluxos.filter((fluxo) => fluxo.automatico);
  if (partes === undefined) return automaticos.length > 0;
  const faixas = partes.find((parte) => parte.impressora === destino.id)?.faixas ?? [];
  return automaticos.some((fluxo) =>
    faixas.some((faixa) => faixa.inicio <= fluxo.fim && fluxo.inicio <= faixa.fim),
  );
}

/**
 * A folha de controle com a opcao ligada, antes de alguem pedir para enviar: so ha o que avisar quando
 * todas as marcadas sao automaticas e todo o papel lido sai sem gesto. Sem leitura, nada se afirma.
 */
export function envioSemMarca(
  controle: boolean,
  marcadas: readonly Impressora[],
  analise: { soPapelAutomatico: boolean } | null,
): boolean {
  return (
    controle &&
    analise?.soPapelAutomatico === true &&
    marcadas.length > 0 &&
    marcadas.every((impressora) => impressora.modo === "automatico")
  );
}

/** A opcao do driver na linha marcada: na automatica, so com papel da rota manual neste envio. */
export function mostraOpcaoDoDriver(impressora: Impressora, temPapelManual: boolean): boolean {
  if (impressora.podeInverter !== true) return false;
  return impressora.modo !== "automatico" || temPapelManual;
}

/** Quem ja compensa o driver. Com mais de um destino, a nota diz de qual deles se trata. */
function notaDaCompensacao(
  compensadas: readonly Impressora[],
  destinos: readonly Impressora[],
  idioma: Idioma,
): string | null {
  if (compensadas.length === 0) return null;
  if (destinos.length === 1) return traduzir(idioma, "inversao.compensada");
  return traduzir(idioma, "inversao.compensadaEm", {
    lista: compensadas.map((destino) => nomeDeExibicao(destino, idioma)).join(", "),
  });
}

/**
 * O que as opcoes acrescentam entre a rota e as instrucoes gerais, sempre na mesma ordem. A comparacao
 * de velocidade e so um aviso: os lotes continuam repartidos em rodizio. O que fala do driver vale so
 * para quem passa pela rota manual neste envio, e o aviso some na fila em que o PlotS ja compensa. A
 * folha de controle e citada porque na janela curta a opcao pode estar fora de vista, e so a rota manual
 * a usa: o que sai todo sem gesto nao tem marca nenhuma a citar.
 */
export function notasDasOpcoes(opcoes: OpcoesDaNota, idioma: Idioma = idiomaAtual): string[] {
  const notas: string[] = [];
  const { controle, comparacao, modo, destinos, fluxos = [], partes } = opcoes;
  const recebem =
    partes === undefined
      ? destinos
      : destinos.filter((destino) => partes.some((parte) => parte.impressora === destino.id));
  const naRotaManual = recebem.filter((destino) => passaPelaRotaManual(destino, fluxos, partes));
  if (controle && naRotaManual.length > 0) {
    const misto = recebem.some((destino) => passaPelaRotaAutomatica(destino, fluxos, partes));
    notas.push(traduzir(idioma, misto ? "controle.confirmacaoMista" : "controle.confirmacao"));
  }
  const compensacao = notaDaCompensacao(
    naRotaManual.filter((destino) => destino.driverInverte),
    recebem,
    idioma,
  );
  if (compensacao !== null) notas.push(compensacao);
  if (naRotaManual.some((destino) => destino.tipo === "fila" && !destino.driverInverte)) {
    notas.push(traduzir(idioma, "inversao.aviso"));
  }
  if (comparacao) {
    // "As duas" so vale para duas: com mais, a frase final nao conta quantas sao.
    const varias = recebem.length >= 3;
    const divisao = varias ? "historico.compara.divisaoVarias" : "historico.compara.divisao";
    const espelho = varias ? "historico.compara.espelhoVarias" : "historico.compara.espelho";
    notas.push(
      traduzir(idioma, modo === "divisao" ? divisao : espelho, {
        rapida: nomeDoDestino(comparacao.rapida, destinos, idioma),
        lenta: nomeDoDestino(comparacao.lenta, destinos, idioma),
        fator: textoDoFator(comparacao.razao, idioma),
      }),
    );
  }
  return notas;
}
