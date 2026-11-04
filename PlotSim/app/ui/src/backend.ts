import { invoke as invocar } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { BandejaTela, EstadoFila, FilaTela, GestoPedido } from "./tela";
import { idiomaAtual } from "./i18n";

/** Respostas forçadas por comando, só na versão de verificação de telas. */
type Forcada = (argumentos: unknown) => unknown;
const forcadas = new Map<string, Forcada>();

if (import.meta.env.MODE === "e2e") {
  (window as unknown as { __plotsimE2e: unknown }).__plotsimE2e = {
    forcarErro(comando: string, codigo: string, detalhe: string | null = null) {
      forcadas.set(comando, () => {
        throw { codigo, detalhe };
      });
    },
    forcarDemora(comando: string) {
      forcadas.set(comando, () => new Promise(() => {}));
    },
    forcarResposta(comando: string, resposta: unknown) {
      forcadas.set(comando, () => resposta);
    },
    soltar(comando: string) {
      forcadas.delete(comando);
    },
  };
}

async function invoke<T>(comando: string, argumentos?: Record<string, unknown>): Promise<T> {
  const forcada = forcadas.get(comando);
  if (forcada) return (await forcada(argumentos)) as T;
  return invocar<T>(comando, argumentos);
}

export function listarFilas(): Promise<FilaTela[]> {
  return invoke("listar_filas");
}

export function criarFila(nome: string, jato: boolean): Promise<FilaTela[]> {
  return invoke("criar_fila", { nome, jato });
}

export function removerFila(nome: string, apagarHistorico: boolean): Promise<FilaTela[]> {
  return invoke("remover_fila", { nome, apagarHistorico });
}

export function verBandeja(nome: string): Promise<BandejaTela> {
  return invoke("ver_bandeja", { nome, idioma: idiomaAtual });
}

/** Sem tocar no sistema: é a leitura da memória do programa. */
export function estadosDasFilas(): Promise<Record<string, EstadoFila>> {
  return invoke("estados_das_filas");
}

export function recolocar(nome: string, gesto: GestoPedido): Promise<void> {
  return invoke("recolocar", { nome, gesto });
}

export function limparBandeja(nome: string): Promise<void> {
  return invoke("limpar_bandeja", { nome });
}

/** Se o canal com o PlotS não abriu: o gesto do modo de teste não chega à janela. */
export function canalFechado(): Promise<boolean> {
  return invoke("canal_fechado");
}

export function resumoDoHistorico(nome: string): Promise<{ arquivos: number; bytes: number }> {
  return invoke("resumo_do_historico", { nome });
}

export function limparHistorico(nome: string): Promise<number> {
  return invoke("limpar_historico", { nome });
}

export function desenharPagina(nome: string, pagina: number): Promise<string> {
  return invoke("desenhar_pagina", { nome, pagina });
}

export function interfaceMontou(): Promise<void> {
  return invoke("interface_montou");
}

export function aoMudar(acao: () => void): Promise<() => void> {
  return listen("plotsim-mudou", acao);
}
