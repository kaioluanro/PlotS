import { invoke } from "@tauri-apps/api/core";
import type { ResumoDoEnvio } from "./cartoes";
import type { TrechoDoEnvio } from "./intervalo";

/** Le o PDF para a confirmacao, contando so o trecho; nada e enviado. */
export function resumoDoTrecho(
  caminho: string,
  folhas: number,
  intervalo: TrechoDoEnvio | null,
): Promise<ResumoDoEnvio> {
  return invoke<ResumoDoEnvio>("resumo_do_envio", { caminho, folhas, intervalo });
}

/** Inicia um trabalho por impressora, so com o trecho, e devolve quem aceitou. */
export function iniciarImpressaoDoTrecho(
  aba: number,
  caminho: string,
  folhas: number,
  impressoras: readonly string[],
  intervalo: TrechoDoEnvio | null,
): Promise<string[]> {
  return invoke<string[]>("iniciar_impressao", { aba, caminho, folhas, impressoras, intervalo });
}
