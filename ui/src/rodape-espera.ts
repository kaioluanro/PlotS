import { contagemPorExtenso, listaDeFora, nomeDoPapel, type Cartao, type EstadoDoEnvio } from "./cartoes";
import { idiomaAtual, traduzir, type Idioma } from "./i18n";
import { nomeDeExibicao, type Impressora } from "./impressoras";
import type { Aba, FaceDoEnvio } from "./tabs";

/** Uma impressora cujo trabalho espera a pessoa, e o comando que o rodape manda para ela. */
export interface EsperaDoRodape {
  impressora: Impressora;
  cartao: Cartao;
  comando: "enviar_frente" | "enviar_verso";
}

export interface EsperasDoRodape {
  itens: readonly EsperaDoRodape[];
  /** Marcadas desta aba que a acao nao alcanca: seguem como estavam. */
  fora: readonly Impressora[];
}

/** A face escolhida no rodape diz qual espera ela alcanca: a frente continua o trecho, o verso manda o maco. */
const ESPERA_DA_FACE: Record<
  FaceDoEnvio,
  { estado: Cartao["estado"]["tipo"]; comando: EsperaDoRodape["comando"] }
> = {
  frente: { estado: "proximo_lote", comando: "enviar_frente" },
  verso: { estado: "aguardando", comando: "enviar_verso" },
};

/**
 * Quem, entre as marcadas desta aba, espera a face escolhida. So o trabalho desta aba conta: o verso de
 * um documento nunca sai pelo rodape de outro. Quem nao espera fica em `fora`.
 */
export function esperasDoRodape(
  aba: Aba | null,
  face: FaceDoEnvio,
  marcadas: readonly Impressora[],
): EsperasDoRodape {
  if (aba === null) return { itens: [], fora: [] };
  const alvo = ESPERA_DA_FACE[face];
  const itens: EsperaDoRodape[] = [];
  const fora: Impressora[] = [];
  for (const impressora of marcadas) {
    const cartao = aba.cartoes.find(
      (candidato) => candidato.aba === aba.id && candidato.impressora === impressora.id,
    );
    if (cartao !== undefined && cartao.estado.tipo === alvo.estado) {
      itens.push({ impressora, cartao, comando: alvo.comando });
    } else {
      fora.push(impressora);
    }
  }
  return { itens, fora };
}

/** O verso que espera sempre passa pela confirmacao: maco errado ou de cabeca para baixo estraga as folhas. */
export function esperaPedeConfirmacao(espera: EsperasDoRodape): boolean {
  return espera.itens.some((item) => item.comando === "enviar_verso");
}

/** O que o botao de envio faz com as esperas do rodape, antes de olhar o envio comum. */
export type AcaoDoEnvio =
  | { tipo: "confirmar"; itens: readonly EsperaDoRodape[] }
  | { tipo: "comandar"; itens: readonly EsperaDoRodape[] }
  | { tipo: "enviar" };

/**
 * A decisao do rodape, sem tela: para o verso a pessoa confirma o maco antes; para a frente do
 * proximo trecho, o comando sai direto; sem espera, o envio comum continua.
 */
export function acaoDoEnvio(espera: EsperasDoRodape): AcaoDoEnvio {
  if (espera.itens.length === 0) return { tipo: "enviar" };
  return esperaPedeConfirmacao(espera)
    ? { tipo: "confirmar", itens: espera.itens }
    : { tipo: "comandar", itens: espera.itens };
}

/** A dica que fica no botao: a troca de papel nao some quando o aviso do topo e fechado. */
function dicaDoProximoTrecho(itens: readonly EsperaDoRodape[], lista: string, idioma: Idioma): string {
  const papeis = [
    ...new Set(
      itens.flatMap((item) =>
        item.cartao.trocaDePapel === null ? [] : [nomeDoPapel(item.cartao.trocaDePapel, idioma)],
      ),
    ),
  ];
  if (papeis.length > 0) {
    return traduzir(idioma, "cartao.espera.frenteTroca", { papel: listaDeFora(papeis, idioma), lista });
  }
  const continua = traduzir(idioma, "cartao.espera.frente", { lista });
  const precisaRetirar = itens.some((item) => !item.cartao.simulada);
  return precisaRetirar ? `${traduzir(idioma, "cartao.proximo.semTroca")} ${continua}` : continua;
}

/**
 * Com alguem esperando a face escolhida, o botao de envio manda o comando para essas impressoras, sem
 * depender do envio comum (que trata o trabalho em andamento como ocupado). Sem espera, devolve o envio
 * como estava.
 */
export function envioComEspera(
  envio: EstadoDoEnvio,
  espera: EsperasDoRodape,
  idioma: Idioma = idiomaAtual,
): EstadoDoEnvio {
  if (espera.itens.length === 0) return envio;
  const n = String(espera.itens.length);
  const frente = espera.itens[0]?.comando === "enviar_frente";
  const soSimuladas = espera.itens.every((item) => item.cartao.simulada);
  const chaveDoRotulo = frente
    ? soSimuladas ? "cartao.continuar.simulacao" : "cartao.continuar.frentes"
    : soSimuladas ? "face.simularVersoContagem" : "face.imprimirVersoContagem";
  const chaveDoAria = frente
    ? soSimuladas ? "cartao.continuar.simulacao.aria" : "cartao.continuar.frentes.aria"
    : soSimuladas ? "face.simularVersoContagem.aria" : "face.imprimirVersoContagem.aria";
  const nomes = espera.itens.map((item) => nomeDeExibicao(item.impressora, idioma));
  const lista = listaDeFora(nomes, idioma);
  const acao = frente ? dicaDoProximoTrecho(espera.itens, lista, idioma) : traduzir(idioma, "cartao.espera.verso", { lista });
  const deFora = espera.fora.map((impressora) => nomeDeExibicao(impressora, idioma));
  const motivo =
    deFora.length === 0
      ? acao
      : `${acao} ${traduzir(idioma, "cartao.espera.foraDaAcao", { lista: listaDeFora(deFora, idioma) })}`;
  return {
    ...envio,
    rotulo: traduzir(idioma, chaveDoRotulo, { n }),
    rotuloAcessivel: contagemPorExtenso(chaveDoAria, espera.itens.length, idioma),
    motivo,
    desabilitado: false,
    destinos: [],
  };
}
