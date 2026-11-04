import {
  cartaoEmAndamento,
  contagemPorExtenso,
  FOLHAS_QUE_CABEM,
  frasesDoGesto,
  linhasDoEnvio,
  listaDeFora,
  textoDasPaginas,
  textoDoTamanhoDoEnvio,
  type Cartao,
  type EstadoDoEnvio,
  type Recolocar,
  type ResumoDoEnvio,
} from "./cartoes";
import { desenhoDoGesto } from "./cartoes-gesto";
import { conteudoDoEnvio, type ConteudoConfirmacao } from "./confirmacao";
import { idiomaAtual, traduzir, type Idioma } from "./i18n";
import { nomeDeExibicao, type Impressora } from "./impressoras";
import {
  lerIntervaloDaAba,
  leituraDoTrecho,
  linhasComTrecho,
  trechoDoEnvio,
  type LeituraIntervalo,
  type TrechoDoEnvio,
} from "./intervalo";
import { passaPelaRotaManual } from "./opcoes-envio";
import { copiasDaFace, type Aba, type FaceDoEnvio, type FrenteEnviada, type PapelDeAjuste } from "./tabs";

/** O que a lateral sabe do papel deste envio; sem leitura, nada se afirma. */
interface PapelLido {
  temPapelManual: boolean;
}

/** Quem imprime as duas faces de uma vez nao tem frente nem verso a escolher. */
export function faceDisponivel(marcadas: readonly Impressora[]): boolean {
  return marcadas.some((impressora) => impressora.modo !== "automatico");
}

/** Com o seletor fora da tela, nada de um verso escolhido antes pode valer. */
export function faceDoEnvio(aba: Aba | null, marcadas: readonly Impressora[]): FaceDoEnvio {
  return aba !== null && faceDisponivel(marcadas) ? aba.face : "frente";
}

/** Uma automatica so tem verso a imprimir se o documento tem papel que ela nao faz sozinha. */
function recebeVerso(impressora: Impressora, documento: PapelLido | null): boolean {
  return impressora.modo !== "automatico" || documento?.temPapelManual === true;
}

/** As frentes ainda nao sairam nesta impressora: mandar os versos agora estragaria o maco. */
function frentePendente(aba: Aba, impressora: Impressora): boolean {
  const cartao = aba.cartoes.find((item) => item.impressora === impressora.id);
  return (
    cartao !== undefined &&
    cartaoEmAndamento(cartao) &&
    (cartao.alcance === "so_frentes" || cartao.estado.tipo === "preparando")
  );
}

/**
 * O botao diz o que fara e conta quem recebe. No verso, sai quem imprime as duas faces de uma vez, e
 * uma frente desta aba que a fila ainda nao confirmou recusa o envio, com o motivo.
 */
export function envioComFace(
  envio: EstadoDoEnvio,
  aba: Aba | null,
  lista: readonly Impressora[],
  documento: PapelLido | null,
  idioma: Idioma = idiomaAtual,
): EstadoDoEnvio {
  if (aba === null) return envio;
  const marcadas = lista.filter((impressora) => aba.impressorasMarcadas.includes(impressora.id));
  if (faceDoEnvio(aba, marcadas) !== "verso") return envio;
  const soSimuladas = marcadas.length > 0 && marcadas.every((item) => item.tipo === "simulada");
  const destinos = envio.destinos.filter((impressora) => recebeVerso(impressora, documento));
  const rotulo =
    destinos.length > 0
      ? traduzir(idioma, soSimuladas ? "face.simularVersoContagem" : "face.imprimirVersoContagem", {
          n: String(destinos.length),
        })
      : traduzir(idioma, soSimuladas ? "face.simularVerso" : "face.imprimirVerso");
  const rotuloAcessivel =
    destinos.length > 0
      ? contagemPorExtenso(
          soSimuladas ? "face.simularVersoContagem.aria" : "face.imprimirVersoContagem.aria",
          destinos.length,
          idioma,
        )
      : undefined;
  const pendentes = marcadas.filter((impressora) => frentePendente(aba, impressora));
  if (pendentes.length > 0) {
    return {
      rotulo,
      rotuloAcessivel,
      motivo: traduzir(idioma, "cartao.recusa.frentePendente", {
        lista: listaDeFora(
          pendentes.map((item) => nomeDeExibicao(item, idioma)),
          idioma,
        ),
      }),
      desabilitado: true,
      destinos: [],
    };
  }
  if (destinos.length === 0 && envio.destinos.length > 0) {
    return {
      rotulo,
      rotuloAcessivel,
      motivo: traduzir(idioma, "face.recusa.soAutomaticas"),
      desabilitado: true,
      destinos: [],
    };
  }
  return {
    ...envio,
    rotulo,
    rotuloAcessivel,
    destinos,
    desabilitado: envio.desabilitado || destinos.length === 0,
  };
}

/**
 * Diz o estado da face que a pessoa escolheu, sem mudar a escolha. Frente com frentes ja prontas nas
 * marcadas manda escolher Verso; verso com versos concluidos nas marcadas avisa que so repete se clicar.
 */
export function dicaDaFace(
  aba: Aba,
  marcadas: readonly Impressora[],
  idioma: Idioma = idiomaAtual,
): string | null {
  if (aba.face === "frente") {
    const prontas = marcadas.some((impressora) => aba.frentesProntas.includes(impressora.id));
    return prontas ? traduzir(idioma, "cartao.dica.frentesProntas") : null;
  }
  const concluidos = marcadas.some((impressora) =>
    aba.cartoes.some(
      (cartao) =>
        cartao.impressora === impressora.id &&
        cartao.estado.tipo === "concluido" &&
        (cartao.alcance === "so_versos" || cartao.alcance === "frente_e_verso"),
    ),
  );
  return concluidos ? traduzir(idioma, "cartao.dica.versosConcluidos") : null;
}

function mesmoTrecho(a: TrechoDoEnvio | null, b: TrechoDoEnvio | null): boolean {
  return a?.inicio === b?.inicio && a?.fim === b?.fim;
}

/** O verso precisa do mesmo trecho e do mesmo papel da frente. Sem frente nesta aba, nao ha o que comparar. */
export function mudouDesdeAFrente(
  frente: FrenteEnviada | null,
  trecho: TrechoDoEnvio | null,
  papel: PapelDeAjuste | null,
): boolean {
  return frente !== null && (!mesmoTrecho(frente.trecho, trecho) || frente.papel !== papel);
}

/** O que o verso em espera imprime: o lote planejado na frente, nao a aba como ela esta agora. */
export interface PlanoDoVerso {
  trecho: TrechoDoEnvio | null;
  leitura: LeituraIntervalo;
  papel: PapelDeAjuste | null;
  /** Copias do trabalho inteiro: o verso sai com o numero que a frente foi planejada para imprimir. */
  copias: number;
}

/**
 * Le o verso pelo que foi enviado: o trecho e o papel ficam na `frenteEnviada`, e as copias, no
 * cartao que espera. O comando `enviar_verso` so manda a passada 2 do lote ja planejado, entao a
 * confirmacao nunca troca os numeros do trabalho pelos da aba. Sem cartao em espera, o envio e
 * novo e a aba vale.
 */
export function planoDoVerso(aba: Aba, espera: readonly Cartao[]): PlanoDoVerso {
  if (espera.length === 0) {
    const leitura = lerIntervaloDaAba(aba);
    return {
      trecho: trechoDoEnvio(leitura),
      leitura,
      papel: aba.papelDeAjuste ?? null,
      copias: copiasDaFace(aba, "verso"),
    };
  }
  const frente = aba.frenteEnviada;
  // `null` e o documento inteiro, um valor do envio: nao quer dizer "sem frente".
  const trecho = frente === null ? trechoDoEnvio(lerIntervaloDaAba(aba)) : frente.trecho;
  return {
    trecho,
    leitura: leituraDoTrecho(aba.paginas, trecho),
    papel: frente === null ? (aba.papelDeAjuste ?? null) : frente.papel,
    // Sem o numero no cartao em espera, o trabalho saiu com uma copia: a aba de agora nao manda.
    copias: espera[0]?.copias ?? 1,
  };
}

/** Quem ja entregou as frentes desta aba e vai receber tudo de novo. */
export function quemJaSaiu(aba: Aba, destinos: readonly Impressora[]): readonly Impressora[] {
  return destinos.filter((destino) => aba.frentesProntas.includes(destino.id));
}

/** Quem tem nesta aba um trabalho em erro, que o envio novo vai substituir no cartao. */
export function quemDeuErro(aba: Aba, destinos: readonly Impressora[]): readonly Impressora[] {
  return destinos.filter((destino) =>
    aba.cartoes.some(
      (cartao) =>
        cartao.aba === aba.id && cartao.impressora === destino.id && cartao.estado.tipo === "erro",
    ),
  );
}

function nomes(impressoras: readonly Impressora[], idioma: Idioma): string {
  return new Intl.ListFormat(idioma, { style: "long", type: "conjunction" }).format(
    impressoras.map((impressora) => nomeDeExibicao(impressora, idioma)),
  );
}

/** O maco maior que a entrada so importa a quem o manuseia: o que sai sozinho nao volta para ela. */
export function notaDoMaco(
  resumo: ResumoDoEnvio,
  destinos: readonly Impressora[],
  idioma: Idioma = idiomaAtual,
): string | null {
  const alguemManual = destinos.some((destino) => passaPelaRotaManual(destino, resumo.fluxos ?? []));
  return alguemManual && (resumo.maiorMaco ?? 0) > FOLHAS_QUE_CABEM
    ? traduzir(idioma, "face.envio.maco")
    : null;
}

/** O que a confirmacao da frente acrescenta: papel gasto a toa, e quem imprime as duas faces de uma vez. */
export function notasDaFrente(
  aba: Aba,
  destinos: readonly Impressora[],
  resumo: ResumoDoEnvio,
  idioma: Idioma = idiomaAtual,
): { antes: string[]; depois: string[] } {
  const repetidas = quemJaSaiu(aba, destinos);
  // O trabalho em erro some do cartao, mas pode seguir na fila do Windows e imprimir alem do novo.
  const comErro = quemDeuErro(aba, destinos);
  const automaticas = destinos.filter((destino) => destino.modo === "automatico");
  const temAutomatico = (resumo.fluxos ?? []).some((fluxo) => fluxo.automatico);
  const manual = destinos.length > automaticas.length;
  const maco = notaDoMaco(resumo, destinos, idioma);
  return {
    antes: [
      ...(comErro.length > 0 ? [traduzir(idioma, "face.envio.erroAnterior")] : []),
      ...(repetidas.length > 0
        ? [traduzir(idioma, "face.envio.jaSaiu", { lista: nomes(repetidas, idioma) })]
        : []),
    ],
    depois: [
      ...(manual && automaticas.length > 0 && temAutomatico
        ? [traduzir(idioma, "face.envio.frenteInteira", { lista: nomes(automaticas, idioma) })]
        : []),
      ...(maco === null ? [] : [maco]),
    ],
  };
}

/** Frente e verso com numeros de copias diferentes: o maco do verso tem de ter o numero dele. */
function textoDasCopias(copias: { frente: number; verso: number }, idioma: Idioma): string {
  const palavra = (n: number) => traduzir(idioma, n === 1 ? "copias.um" : "copias.varias");
  return traduzir(idioma, "face.copias.diferentes", {
    frente: String(copias.frente),
    copiasFrente: palavra(copias.frente),
    verso: String(copias.verso),
    copiasVerso: palavra(copias.verso),
  });
}

/** Sem resposta do backend para esta impressora, vale o gesto da calibracao: nao se afirma lado. */
const SEM_GESTO: Recolocar = { lado: null, virada: null, textoDoPerfil: null };

/**
 * Antes dos versos, sempre: o maco errado estraga as folhas. Mostra o que vai sair, o gesto de cada
 * impressora e o que mudou desde a frente. Quem imprime as duas faces de uma vez nao entra.
 */
export function conteudoDoVerso(
  destinos: readonly Impressora[],
  foraDoVerso: readonly Impressora[],
  resumo: ResumoDoEnvio,
  leitura: LeituraIntervalo | null,
  papelAjustado: string | null,
  mudou: boolean,
  idioma: Idioma = idiomaAtual,
  copias?: { frente: number; verso: number },
  erroAnterior = false,
): ConteudoConfirmacao {
  const { linhaDoPapel } = linhasDoEnvio(destinos, resumo, idioma, new Set(), { papel: papelAjustado });
  const soSimuladas = destinos.every((destino) => destino.tipo === "simulada");
  const linhas = [
    textoDoTamanhoDoEnvio(resumo, idioma, copias?.verso),
    linhaDoPapel,
    ...(copias === undefined ? [] : [traduzir(idioma, "copias.linha", { n: String(copias.verso) })]),
  ];
  const primeira = resumo.faixas?.[0];
  const maisDeUmMaco = (resumo.faixas?.length ?? 0) > 1;
  const emPartes = (resumo.maiorMaco ?? 0) > FOLHAS_QUE_CABEM;
  // O aviso fica logo abaixo do texto: quem vai pegar o maco precisa ve-lo antes da lista.
  const aviso =
    copias !== undefined && copias.frente !== copias.verso ? textoDasCopias(copias, idioma) : undefined;
  return {
    ...conteudoDoEnvio([], [], false, soSimuladas),
    titulo: "face.envio.versoTitulo",
    texto: "face.envio.versoTexto",
    ...(aviso === undefined ? {} : { aviso }),
    fechar: soSimuladas ? "face.envio.versoSimular" : "face.envio.versoConfirmar",
    linhas: leitura === null ? linhas : linhasComTrecho(linhas, leitura, idioma),
    gestos: destinos.map((destino) => {
      const recolocar =
        resumo.gestos?.find((gesto) => gesto.impressora === destino.id)?.recolocar ?? SEM_GESTO;
      return {
        nome: nomeDeExibicao(destino, idioma),
        frases: frasesDoGesto(recolocar, idioma),
        desenho: desenhoDoGesto(recolocar),
      };
    }),
    notas: [
      // O trabalho anterior pode seguir na fila do Windows: vale antes da frente e antes do verso.
      ...(erroAnterior ? [traduzir(idioma, "face.envio.erroAnterior")] : []),
      ...(mudou ? [traduzir(idioma, "face.envio.mudou")] : []),
      ...(resumo.paresSeparados ?? []).map(([a, b]) =>
        traduzir(idioma, "cartao.envio.separado", { a: String(a), b: String(b) }),
      ),
      ...(maisDeUmMaco && primeira !== undefined
        ? [traduzir(idioma, "face.envio.comeca", { paginas: textoDasPaginas(primeira, idioma) })]
        : []),
      ...(emPartes ? [traduzir(idioma, "face.verso.emPartes")] : []),
      ...(foraDoVerso.length > 0
        ? [traduzir(idioma, "face.envio.deFora", { lista: nomes(foraDoVerso, idioma) })]
        : []),
    ],
  };
}
