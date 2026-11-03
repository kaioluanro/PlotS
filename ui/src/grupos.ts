import { invoke } from "@tauri-apps/api/core";
import { traduzir, idiomaAtual, type Chave, type Idioma } from "./i18n";
import type { ModoDistribuicao } from "./tabs";

/** Uma selecao de impressoras e um modo, com um nome; fica no arquivo de perfis. */
export interface GrupoSalvo {
  nome: string;
  impressoras: string[];
  modo: ModoDistribuicao;
}

/** O grupo pronto para marcar: quem ainda existe, e os nomes de quem sumiu. */
export interface GrupoAplicado {
  impressoras: string[];
  modo: ModoDistribuicao;
  ausentes: string[];
}

export const NOME_DE_GRUPO_MAXIMO = 60;

/** O que o arquivo tem; `ignorados` conta o que estava la e nao vale como grupo. */
export interface ListaDeGrupos {
  grupos: GrupoSalvo[];
  ignorados: number;
}

export function listarGrupos(): Promise<ListaDeGrupos> {
  return invoke<ListaDeGrupos>("listar_grupos");
}

export function salvarGrupo(
  nome: string,
  impressoras: readonly string[],
  modo: ModoDistribuicao,
): Promise<GrupoSalvo[]> {
  return invoke<GrupoSalvo[]>("salvar_grupo", { nome, impressoras, modo });
}

export function apagarGrupo(nome: string): Promise<GrupoSalvo[]> {
  return invoke<GrupoSalvo[]>("apagar_grupo", { nome });
}

export function aplicarGrupo(nome: string): Promise<GrupoAplicado> {
  return invoke<GrupoAplicado>("aplicar_grupo", { nome });
}

const CHAVES_DA_FALHA: ReadonlyMap<unknown, Chave> = new Map<unknown, Chave>([
  ["nome_vazio", "grupo.erro.nomeVazio"],
  ["nome_grande", "grupo.erro.nomeGrande"],
  ["nome_invalido", "grupo.erro.nomeInvalido"],
  ["nome_repetido", "grupo.erro.nomeRepetido"],
  ["sem_impressoras", "grupo.erro.semImpressoras"],
  ["muitas_impressoras", "grupo.erro.muitasImpressoras"],
  ["muitos_grupos", "grupo.erro.muitosGrupos"],
  ["inexistente", "grupo.erro.inexistente"],
  ["leitura", "grupo.erro.leitura"],
  ["gravacao", "grupo.erro.gravacao"],
]);

/** A frase da recusa. Nada que venha de fora aparece como veio: so o codigo conhecido escolhe. */
export function textoDaFalhaDoGrupo(falha: unknown, idioma: Idioma = idiomaAtual): string {
  return traduzir(idioma, CHAVES_DA_FALHA.get(falha) ?? "grupo.erro.gravacao");
}

/** O aviso de grupos ignorados: so o numero, porque o nome veio de um arquivo que nao passou. */
export function textoDosIgnorados(n: number, idioma: Idioma = idiomaAtual): string | null {
  if (n <= 0) return null;
  return n === 1
    ? traduzir(idioma, "grupo.ignorados.um")
    : traduzir(idioma, "grupo.ignorados.varios", { n: String(n) });
}

/** Depois de aplicar, a interface diz quais impressoras do grupo nao existem mais. */
export function textoDasAusentes(ausentes: readonly string[], idioma: Idioma = idiomaAtual): string | null {
  if (ausentes.length === 0) return null;
  return traduzir(idioma, "grupo.aplicado.ignoradas", { lista: ausentes.join(", ") });
}

/** Uma nota da linha de grupos, com a selecao de que ela fala. */
export interface NotaDaSelecao {
  texto: string;
  aba: number;
  impressoras: readonly string[];
}

/** Outra aba ou outra marcacao ja nao e aquela selecao: a nota sai. */
export function notaValeNaSelecao(
  nota: NotaDaSelecao | null,
  aba: number,
  marcadas: readonly string[],
): nota is NotaDaSelecao {
  return (
    nota !== null &&
    nota.aba === aba &&
    nota.impressoras.length === marcadas.length &&
    nota.impressoras.every((impressora) => marcadas.includes(impressora))
  );
}

/** O grupo so continua escolhido na lista enquanto a selecao for a dele. */
export function selecaoCoincide(
  aplicado: { impressoras: readonly string[]; modo: ModoDistribuicao },
  marcadas: readonly string[],
  modo: ModoDistribuicao,
): boolean {
  return (
    aplicado.modo === modo &&
    aplicado.impressoras.length === marcadas.length &&
    aplicado.impressoras.every((impressora) => marcadas.includes(impressora))
  );
}
