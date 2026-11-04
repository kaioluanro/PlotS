import type { EstadoDoEnvio } from "./cartoes";
import { idiomaAtual, traduzir, type Idioma } from "./i18n";
import type { Aba, PaginasDaAba } from "./tabs";

/** O que esta escrito nos dois campos. Fica como texto: o usuario pode estar no meio da digitacao. */
export interface RascunhoIntervalo {
  inicio: string;
  fim: string;
}

export type CampoIntervalo = keyof RascunhoIntervalo;

/** O trecho como o backend recebe: `null` quer dizer o documento inteiro. */
export interface TrechoDoEnvio {
  inicio: number;
  fim: number;
}

export type ErroIntervalo = "vazio" | "naoNumero" | "fora" | "invertido";

export type LeituraIntervalo =
  | { estado: "indisponivel" }
  | { estado: "inteiro"; total: number }
  | { estado: "trecho"; inicio: number; fim: number; total: number }
  | { estado: "invalido"; erro: ErroIntervalo; campos: readonly CampoIntervalo[]; total: number };

export type LeituraInvalida = Extract<LeituraIntervalo, { estado: "invalido" }>;

type Campo = { tipo: "vazio" } | { tipo: "naoNumero" } | { tipo: "numero"; valor: number };

/** Numero com mais digitos que isto ja passou de qualquer total aceito, e nao perde precisao. */
const DIGITOS_CONFIAVEIS = 15;

function lerCampo(texto: string): Campo {
  const limpo = texto.trim();
  if (limpo === "") return { tipo: "vazio" };
  if (!/^[0-9]+$/.test(limpo)) return { tipo: "naoNumero" };
  const semZeros = limpo.replace(/^0+(?=[0-9])/, "");
  const valor = semZeros.length > DIGITOS_CONFIAVEIS ? Number.POSITIVE_INFINITY : Number(semZeros);
  return { tipo: "numero", valor };
}

function camposCom(
  inicio: Campo,
  fim: Campo,
  aceita: (campo: Campo) => boolean,
): readonly CampoIntervalo[] {
  const campos: CampoIntervalo[] = [];
  if (aceita(inicio)) campos.push("inicio");
  if (aceita(fim)) campos.push("fim");
  return campos;
}

/** Decide o que o rascunho quer dizer diante do total de paginas da aba. */
export function lerIntervalo(
  paginas: PaginasDaAba | null,
  rascunho: RascunhoIntervalo | null,
): LeituraIntervalo {
  if (paginas?.estado !== "conhecido") return { estado: "indisponivel" };
  const { total } = paginas;
  if (rascunho === null) return { estado: "inteiro", total };
  const inicio = lerCampo(rascunho.inicio);
  const fim = lerCampo(rascunho.fim);
  const invalido = (erro: ErroIntervalo, campos: readonly CampoIntervalo[]): LeituraIntervalo => ({
    estado: "invalido",
    erro,
    campos,
    total,
  });
  if (inicio.tipo === "vazio" || fim.tipo === "vazio") {
    return invalido(
      "vazio",
      camposCom(inicio, fim, (campo) => campo.tipo === "vazio"),
    );
  }
  if (inicio.tipo === "naoNumero" || fim.tipo === "naoNumero") {
    return invalido(
      "naoNumero",
      camposCom(inicio, fim, (campo) => campo.tipo === "naoNumero"),
    );
  }
  const fora = (campo: Campo) =>
    campo.tipo === "numero" && (campo.valor < 1 || campo.valor > total);
  if (fora(inicio) || fora(fim)) return invalido("fora", camposCom(inicio, fim, fora));
  if (inicio.valor > fim.valor) return invalido("invertido", ["inicio", "fim"]);
  if (inicio.valor === 1 && fim.valor === total) return { estado: "inteiro", total };
  return { estado: "trecho", inicio: inicio.valor, fim: fim.valor, total };
}

export function lerIntervaloDaAba(aba: Aba | null): LeituraIntervalo {
  return lerIntervalo(aba?.paginas ?? null, aba?.intervalo ?? null);
}

/**
 * A leitura de um trecho que ja foi enviado. A confirmacao do verso nao herda o que a aba mostra
 * agora: o lote planejado e que diz de onde saem as paginas.
 */
export function leituraDoTrecho(
  paginas: PaginasDaAba,
  trecho: TrechoDoEnvio | null,
): LeituraIntervalo {
  if (paginas.estado !== "conhecido") return { estado: "indisponivel" };
  const { total } = paginas;
  if (trecho === null || (trecho.inicio === 1 && trecho.fim === total)) {
    return { estado: "inteiro", total };
  }
  return { estado: "trecho", inicio: trecho.inicio, fim: trecho.fim, total };
}

/**
 * Enquanto se digita, o erro espera para nao gritar a cada tecla. `fixado` guarda o erro que ja
 * estava na tela: ele fica como estava, com o mesmo texto e os mesmos campos marcados, ate a
 * digitacao acabar, e os campos nao pulam a cada tecla.
 */
export interface EstadoDaEspera {
  digitando: boolean;
  fixado: LeituraInvalida | null;
}

export const ESPERA_PARADA: EstadoDaEspera = { digitando: false, fixado: null };

/** O erro que a tela mostra agora, ou `null`. Texto valido nunca mostra erro, nem por um instante. */
export function leituraDoErro(
  leitura: LeituraIntervalo,
  espera: EstadoDaEspera,
): LeituraInvalida | null {
  if (leitura.estado !== "invalido") return null;
  return espera.digitando ? espera.fixado : leitura;
}

/** O estado depois de uma tecla. So fica fixado o erro que estava na tela e continua valendo. */
export function proximaEspera(
  espera: EstadoDaEspera,
  antes: LeituraIntervalo,
  depois: LeituraIntervalo,
): EstadoDaEspera {
  return {
    digitando: true,
    fixado: depois.estado === "invalido" ? leituraDoErro(antes, espera) : null,
  };
}

/** O que o envio pede ao backend. Um rascunho invalido nunca chega aqui: o botao fica desabilitado. */
export function trechoDoEnvio(leitura: LeituraIntervalo): TrechoDoEnvio | null {
  return leitura.estado === "trecho" ? { inicio: leitura.inicio, fim: leitura.fim } : null;
}

/** Os textos dos campos: o rascunho, ou o documento inteiro; sem total conhecido, nada. */
export function valoresDosCampos(aba: Aba | null): RascunhoIntervalo {
  if (aba?.intervalo) return aba.intervalo;
  if (aba?.paginas.estado === "conhecido") {
    return { inicio: "1", fim: String(aba.paginas.total) };
  }
  return { inicio: "", fim: "" };
}

/** O rascunho que vale para a aba: o documento inteiro escrito por extenso volta a ser "nenhum". */
export function normalizarRascunho(
  rascunho: RascunhoIntervalo | null,
  paginas: PaginasDaAba,
): RascunhoIntervalo | null {
  if (rascunho === null) return null;
  return lerIntervalo(paginas, rascunho).estado === "inteiro" ? null : rascunho;
}

/** O que o campo diz ao leitor de tela: sem numero, le o texto; sem total, nao anuncia maximo. */
export function ariaDoCampo(
  texto: string,
  total: number,
): {
  "aria-valuemax": number | undefined;
  "aria-valuenow": number | undefined;
  "aria-valuetext": string | undefined;
} {
  const campo = lerCampo(texto);
  const numero = campo.tipo === "numero" && Number.isSafeInteger(campo.valor) ? campo.valor : null;
  return {
    "aria-valuemax": total > 0 ? total : undefined,
    "aria-valuenow": numero ?? undefined,
    "aria-valuetext": numero === null ? texto : undefined,
  };
}

/** Ao sair do campo: tira espacos e zeros a esquerda do que ja e numero; o resto fica para corrigir. */
export function limparCampo(texto: string): string {
  const campo = lerCampo(texto);
  return campo.tipo === "numero" ? texto.trim().replace(/^0+(?=[0-9])/, "") : texto;
}

/** As setas andam dentro do documento, a partir do que esta escrito ou da ponta daquele campo. */
export function passoNoCampo(
  valores: RascunhoIntervalo,
  campo: CampoIntervalo,
  delta: number,
  total: number,
): RascunhoIntervalo {
  const atual = lerCampo(valores[campo]);
  const base = atual.tipo === "numero" ? atual.valor : campo === "inicio" ? 1 : total;
  const proximo = Math.min(Math.max(base + delta, 1), total);
  return { ...valores, [campo]: String(proximo) };
}

/** A explicacao abaixo dos campos, ou `null` quando nao ha o que corrigir. */
export function textoDoErro(leitura: LeituraIntervalo, idioma: Idioma = idiomaAtual): string | null {
  if (leitura.estado !== "invalido") return null;
  switch (leitura.erro) {
    case "vazio":
      return traduzir(idioma, "intervalo.erro.vazio");
    case "naoNumero":
      return traduzir(idioma, "intervalo.erro.naoNumero");
    case "invertido":
      return traduzir(idioma, "intervalo.erro.invertido");
    case "fora":
      return leitura.total === 1
        ? traduzir(idioma, "intervalo.erro.foraUma")
        : traduzir(idioma, "intervalo.erro.fora", { total: String(leitura.total) });
  }
}

/** Por que os campos estao desabilitados, ou `null` quando estao abertos. */
export function motivoDosCamposFechados(
  aba: Aba | null,
  idioma: Idioma = idiomaAtual,
): string | null {
  if (aba === null) return traduzir(idioma, "intervalo.fechado.semDocumento");
  if (aba.paginas.estado === "contando") return traduzir(idioma, "intervalo.fechado.contando");
  if (aba.paginas.estado === "recusado") return traduzir(idioma, "intervalo.fechado.recusado");
  return null;
}

/**
 * Um intervalo por corrigir desabilita o envio na hora, e o motivo so aparece junto com o erro.
 * Texto valido nao muda nada: quem clica no botao tira o foco do campo e envia o que esta nele.
 */
export function envioComIntervalo(
  envio: EstadoDoEnvio,
  aba: Aba | null,
  espera: EstadoDaEspera = ESPERA_PARADA,
  idioma: Idioma = idiomaAtual,
): EstadoDoEnvio {
  const leitura = lerIntervaloDaAba(aba);
  if (leitura.estado !== "invalido") return envio;
  return leituraDoErro(leitura, espera) === null
    ? { ...envio, desabilitado: true }
    : { ...envio, desabilitado: true, motivo: traduzir(idioma, "intervalo.botao.invalido") };
}

/** Quantas paginas o resumo do lote conta; `null` enquanto o intervalo esta por corrigir. */
export function paginasDoResumo(leitura: LeituraIntervalo): number | null {
  if (leitura.estado === "inteiro") return leitura.total;
  if (leitura.estado === "trecho") return leitura.fim - leitura.inicio + 1;
  return null;
}

/** Na confirmacao, o trecho vem antes da contagem: diz de onde saem as paginas que ela conta. */
export function linhasComTrecho(
  linhas: readonly string[],
  leitura: LeituraIntervalo,
  idioma: Idioma = idiomaAtual,
): string[] {
  if (leitura.estado !== "trecho") return [...linhas];
  const { inicio, fim, total } = leitura;
  const linha =
    inicio === fim
      ? traduzir(idioma, "intervalo.envio.uma", { inicio: String(inicio), total: String(total) })
      : traduzir(idioma, "intervalo.envio.varias", {
          inicio: String(inicio),
          fim: String(fim),
          total: String(total),
        });
  // As duas ultimas linhas sao a contagem e o papel; antes delas vem quem recebe.
  const posicao = Math.max(linhas.length - 2, 0);
  return [...linhas.slice(0, posicao), linha, ...linhas.slice(posicao)];
}
