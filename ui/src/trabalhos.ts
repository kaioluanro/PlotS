import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Cartao, Comando, ResumoDoEnvio } from "./cartoes";

/** O backend publica cada mudanca de um trabalho por este evento. */
export function ouvirCartoes(aoReceber: (cartao: Cartao) => void): Promise<() => void> {
  return listen<Cartao>("trabalho", (evento) => aoReceber(evento.payload));
}

/** Le o PDF para a confirmacao; nada e enviado. */
export function resumoDoEnvio(caminho: string, folhas: number): Promise<ResumoDoEnvio> {
  return invoke<ResumoDoEnvio>("resumo_do_envio", { caminho, folhas });
}

/** Inicia um trabalho por impressora e devolve quem aceitou. */
export function iniciarImpressao(
  aba: number,
  caminho: string,
  folhas: number,
  impressoras: readonly string[],
): Promise<string[]> {
  return invoke<string[]>("iniciar_impressao", { aba, caminho, folhas, impressoras });
}

/** A recusa quer dizer que nenhuma thread vai atender o pedido. */
export function comandarTrabalho(impressora: string, comando: Comando): Promise<void> {
  return invoke<void>("comandar_trabalho", { impressora, comando });
}

/** Para de acompanhar os trabalhos do documento; nada sai da fila. */
export function soltarTrabalhos(aba: number): Promise<void> {
  return invoke<void>("soltar_trabalhos", { aba });
}
