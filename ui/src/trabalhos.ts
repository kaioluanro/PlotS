import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Cartao, Comando, ResumoDoEnvio } from "./cartoes";
import type { Idioma } from "./i18n";
import type { TrechoDoEnvio } from "./intervalo";
import type { FaceDoEnvio, PapelDeAjuste } from "./tabs";

/** O backend publica cada mudanca de um trabalho por este evento. */
export function ouvirCartoes(aoReceber: (cartao: Cartao) => void): Promise<() => void> {
  return listen<Cartao>("trabalho", (evento) => aoReceber(evento.payload));
}

/** Le o PDF para a confirmacao, contando so o trecho, e traz o gesto de cada impressora; nada e enviado. */
export function resumoDoEnvio(
  caminho: string,
  intervalo: TrechoDoEnvio | null,
  impressoras: readonly string[],
  papelDeAjuste: PapelDeAjuste | null = null,
  copias = 1,
): Promise<ResumoDoEnvio> {
  return invoke<ResumoDoEnvio>("resumo_do_envio", {
    caminho,
    intervalo,
    impressoras,
    papelDeAjuste,
    copias,
  });
}

/** Inicia um trabalho por impressora, com a mesma face e o mesmo trecho, e devolve quem aceitou. */
export function iniciarImpressao(
  aba: number,
  caminho: string,
  impressoras: readonly string[],
  intervalo: TrechoDoEnvio | null,
  face: FaceDoEnvio,
  papelDeAjuste: PapelDeAjuste | null = null,
  marcaControle: Idioma | null = null,
  copias = 1,
): Promise<string[]> {
  return invoke<string[]>("iniciar_impressao", {
    aba,
    caminho,
    impressoras,
    intervalo,
    face,
    papelDeAjuste,
    marcaControle,
    copias,
  });
}

/** Guarda se o driver desta impressora inverte a ordem; a recusa diz que nada mudou. */
export function definirInversao(impressora: string, inverte: boolean): Promise<void> {
  return invoke<void>("definir_inversao", { impressora, inverte });
}

/** A recusa quer dizer que nenhuma thread vai atender o pedido. */
export function comandarTrabalho(impressora: string, comando: Comando): Promise<void> {
  return invoke<void>("comandar_trabalho", { impressora, comando });
}

/** Para de acompanhar os trabalhos do documento; nada sai da fila. */
export function soltarTrabalhos(aba: number): Promise<void> {
  return invoke<void>("soltar_trabalhos", { aba });
}
