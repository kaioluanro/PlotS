import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Cartao, Comando } from "./cartoes";
import type { ResumoComDivisao } from "./confirmacao";
import type { TrechoDoEnvio } from "./intervalo";
import type { ModoDistribuicao } from "./tabs";

/** O backend publica cada mudanca de um trabalho por este evento. */
export function ouvirCartoes(aoReceber: (cartao: Cartao) => void): Promise<() => void> {
  return listen<Cartao>("trabalho", (evento) => aoReceber(evento.payload));
}

/** Le o PDF para a confirmacao, contando so o trecho, e diz o que cabe a cada impressora; nada e enviado. */
export function resumoDoEnvio(
  caminho: string,
  folhas: number,
  intervalo: TrechoDoEnvio | null,
  modo: ModoDistribuicao,
  impressoras: readonly string[],
): Promise<ResumoComDivisao> {
  return invoke<ResumoComDivisao>("resumo_do_envio", {
    caminho,
    folhas,
    intervalo,
    modo,
    impressoras,
  });
}

/** Inicia um trabalho por impressora, so com o trecho, e devolve quem aceitou. */
export function iniciarImpressao(
  aba: number,
  caminho: string,
  folhas: number,
  impressoras: readonly string[],
  intervalo: TrechoDoEnvio | null,
  modo: ModoDistribuicao,
  lotes: number,
  paginas: number,
): Promise<string[]> {
  return invoke<string[]>("iniciar_impressao", {
    aba,
    caminho,
    folhas,
    impressoras,
    intervalo,
    modo,
    lotes,
    paginas,
  });
}

/** A recusa quer dizer que nenhuma thread vai atender o pedido. */
export function comandarTrabalho(impressora: string, comando: Comando): Promise<void> {
  return invoke<void>("comandar_trabalho", { impressora, comando });
}

/** Para de acompanhar os trabalhos do documento; nada sai da fila. */
export function soltarTrabalhos(aba: number): Promise<void> {
  return invoke<void>("soltar_trabalhos", { aba });
}
