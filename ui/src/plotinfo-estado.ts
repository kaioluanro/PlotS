import {
  lerCopiasDoPainel,
  totalDePaginas,
  type ConfiguracaoDeTinta,
  type PerfilDeTinta,
  type ValoresPorCanal,
} from "./analise-tinta";

/** Os dois campos que pertencem so a gaveta. O estado das abas nao aparece aqui de proposito. */
export interface EstadoDoPainelDeTinta {
  paginasTexto: string;
  copiasTexto: string;
}

export const ESTADO_INICIAL_DO_PAINEL: EstadoDoPainelDeTinta = {
  paginasTexto: "",
  copiasTexto: "1",
};

export type AcaoDoPainelDeTinta =
  | { tipo: "paginas"; valor: string }
  | { tipo: "copias"; valor: string }
  | { tipo: "reiniciar" };

export function reduzirPainelDeTinta(
  estado: EstadoDoPainelDeTinta,
  acao: AcaoDoPainelDeTinta,
): EstadoDoPainelDeTinta {
  switch (acao.tipo) {
    case "paginas":
      return estado.paginasTexto === acao.valor
        ? estado
        : { ...estado, paginasTexto: acao.valor };
    case "copias":
      return estado.copiasTexto === acao.valor
        ? estado
        : { ...estado, copiasTexto: acao.valor };
    case "reiniciar":
      return ESTADO_INICIAL_DO_PAINEL;
  }
}

interface LeituraDeNumero {
  valor: number | null;
  vazio: boolean;
}

function lerNumero(texto: string): LeituraDeNumero {
  const limpo = texto.trim();
  if (limpo === "") return { valor: null, vazio: true };
  if (!/^[+-]?(\d+([.,]\d*)?|[.,]\d+)$/.test(limpo)) return { valor: null, vazio: false };
  const normalizado = limpo.replace(",", ".");
  const valor = Number(normalizado);
  return Number.isFinite(valor) ? { valor, vazio: false } : { valor: null, vazio: false };
}

function valorValido(
  texto: string,
  conferir: (valor: number) => boolean,
): boolean {
  const lido = lerNumero(texto);
  return !lido.vazio && lido.valor !== null && conferir(lido.valor);
}

function formatarValor(valor: number): string {
  return Number.isFinite(valor) ? String(valor).replace(".", ",") : "";
}

export interface RascunhoDoPerfil {
  id: string;
  nome: string;
  moeda: string;
  volumes_ml: readonly [string, string, string, string];
  rendimentos_paginas: readonly [string, string, string, string];
  precos_ml: readonly [string, string, string, string];
  fatores_qualidade: readonly [string, string, string];
}

export interface ErrosDoRascunho {
  id: boolean;
  nome: boolean;
  moeda: boolean;
  volumes: readonly boolean[];
  rendimentos: readonly boolean[];
  precos: readonly boolean[];
  fatores: readonly boolean[];
  algum: boolean;
}

function textosDosCanais(valores: ValoresPorCanal): [string, string, string, string] {
  return [formatarValor(valores[0]), formatarValor(valores[1]), formatarValor(valores[2]), formatarValor(valores[3])];
}

export function rascunhoDoPerfil(perfil: PerfilDeTinta): RascunhoDoPerfil {
  return {
    id: perfil.id,
    nome: perfil.nome,
    moeda: perfil.moeda,
    volumes_ml: textosDosCanais(perfil.volumes_ml),
    rendimentos_paginas: [
      String(perfil.rendimentos_paginas[0]),
      String(perfil.rendimentos_paginas[1]),
      String(perfil.rendimentos_paginas[2]),
      String(perfil.rendimentos_paginas[3]),
    ],
    precos_ml: textosDosCanais(perfil.precos_ml),
    fatores_qualidade: [
      formatarValor(perfil.fatores_qualidade[0]),
      formatarValor(perfil.fatores_qualidade[1]),
      formatarValor(perfil.fatores_qualidade[2]),
    ],
  };
}

function identificadorDe(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/** Identificador estavel a partir do nome, sem repetir um perfil que ja existe. */
export function idDePerfil(nome: string, existentes: readonly string[]): string {
  const base = identificadorDe(nome).slice(0, 48) || "perfil";
  if (!existentes.includes(base)) return base;
  for (let sequencia = 2; sequencia <= 9_999; sequencia += 1) {
    const sufixo = `-${sequencia}`;
    const candidato = base.slice(0, 64 - sufixo.length) + sufixo;
    if (!existentes.includes(candidato)) return candidato;
  }
  return `${base.slice(0, 55)}-${existentes.length + 1}`;
}

function rascunhoBase(nome: string, existentes: readonly string[]): RascunhoDoPerfil {
  return {
    id: idDePerfil(nome, existentes),
    nome,
    moeda: "BRL",
    volumes_ml: ["50", "50", "50", "50"],
    rendimentos_paginas: ["5000", "5000", "5000", "5000"],
    precos_ml: ["1", "1", "1", "1"],
    fatores_qualidade: ["0,6", "1", "1,3"],
  };
}

export function rascunhoNovo(nome: string, existentes: readonly string[]): RascunhoDoPerfil {
  return rascunhoBase(nome, existentes);
}

export function rascunhoDuplicado(
  perfil: PerfilDeTinta,
  existentes: readonly string[],
  nomeDaCopia: string,
): RascunhoDoPerfil {
  return { ...rascunhoDoPerfil(perfil), id: idDePerfil(nomeDaCopia, existentes), nome: nomeDaCopia };
}

export function errosDoRascunho(
  rascunho: RascunhoDoPerfil,
  existentes: readonly string[],
  idOriginal: string | null,
): ErrosDoRascunho {
  const idRepetido =
    rascunho.id !== idOriginal && existentes.includes(rascunho.id.trim());
  const id = rascunho.id.trim() === "" || rascunho.id.trim().length > 64 || idRepetido;
  const nome = rascunho.nome.trim() === "" || rascunho.nome.length > 200;
  const moeda = !/^[A-Z]{3}$/.test(rascunho.moeda);
  const volumes = rascunho.volumes_ml.map(
    (valor) => !valorValido(valor, (lido) => lido > 0 && lido <= 10_000),
  );
  const rendimentos = rascunho.rendimentos_paginas.map(
    (valor) => !valorValido(valor, (lido) => Number.isInteger(lido) && lido >= 1 && lido <= 10_000_000),
  );
  const precos = rascunho.precos_ml.map(
    (valor) => !valorValido(valor, (lido) => lido >= 0 && lido <= 10_000),
  );
  const fatores = rascunho.fatores_qualidade.map(
    (valor) => !valorValido(valor, (lido) => lido > 0 && lido <= 10),
  );
  const algum =
    id ||
    nome ||
    moeda ||
    volumes.some(Boolean) ||
    rendimentos.some(Boolean) ||
    precos.some(Boolean) ||
    fatores.some(Boolean);
  return { id, nome, moeda, volumes, rendimentos, precos, fatores, algum };
}

function numeros(
  textos: readonly string[],
  conferir: (valor: number) => boolean,
): number[] | null {
  const lidos: number[] = [];
  for (const texto of textos) {
    const lido = lerNumero(texto);
    if (lido.valor === null || !conferir(lido.valor)) return null;
    lidos.push(lido.valor);
  }
  return lidos;
}

/** Converte o formulario em perfil; devolve `null` enquanto houver qualquer campo invalido. */
export function perfilDoRascunho(
  rascunho: RascunhoDoPerfil,
  existentes: readonly string[],
  idOriginal: string | null,
): PerfilDeTinta | null {
  if (errosDoRascunho(rascunho, existentes, idOriginal).algum) return null;
  const volumes = numeros(rascunho.volumes_ml, (valor) => valor > 0 && valor <= 10_000);
  const rendimentos = numeros(
    rascunho.rendimentos_paginas,
    (valor) => Number.isInteger(valor) && valor >= 1 && valor <= 10_000_000,
  );
  const precos = numeros(rascunho.precos_ml, (valor) => valor >= 0 && valor <= 10_000);
  const fatores = numeros(rascunho.fatores_qualidade, (valor) => valor > 0 && valor <= 10);
  if (volumes === null || rendimentos === null || precos === null || fatores === null) return null;
  const [v0, v1, v2, v3] = volumes;
  const [r0, r1, r2, r3] = rendimentos;
  const [p0, p1, p2, p3] = precos;
  const [f0, f1, f2] = fatores;
  if (
    v0 === undefined ||
    v1 === undefined ||
    v2 === undefined ||
    v3 === undefined ||
    r0 === undefined ||
    r1 === undefined ||
    r2 === undefined ||
    r3 === undefined ||
    p0 === undefined ||
    p1 === undefined ||
    p2 === undefined ||
    p3 === undefined ||
    f0 === undefined ||
    f1 === undefined ||
    f2 === undefined
  ) {
    return null;
  }
  return {
    id: rascunho.id.trim(),
    nome: rascunho.nome.trim(),
    moeda: rascunho.moeda,
    volumes_ml: [v0, v1, v2, v3],
    rendimentos_paginas: [r0, r1, r2, r3],
    precos_ml: [p0, p1, p2, p3],
    fatores_qualidade: [f0, f1, f2],
  };
}

export function configuracaoValida(
  configuracao: ConfiguracaoDeTinta,
  perfis: readonly PerfilDeTinta[],
): boolean {
  return (
    configuracao.perfil.trim() !== "" &&
    perfis.some((perfil) => perfil.id === configuracao.perfil) &&
    (configuracao.dpi === 150 || configuracao.dpi === 300)
  );
}

export const QUALIDADES = ["rascunho", "normal", "alta"] as const;
export const DPIS = [150, 300] as const;
export const MODOS = ["simples", "cmyk"] as const;
export const TAMANHOS = ["a4", "dpi"] as const;

/** Quantidade de paginas de uma copia; a selecao e do backend, a multiplicacao e nossa. */
export function totalDoPainel(paginas: number, copiasTexto: string): number | null {
  const copias = lerCopiasDoPainel(copiasTexto);
  return copias === null ? null : totalDePaginas(paginas, copias);
}