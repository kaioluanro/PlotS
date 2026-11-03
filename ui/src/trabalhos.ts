import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Cartao, Comando } from "./cartoes";
import type { Idioma } from "./i18n";
import type { ResumoComDivisao } from "./confirmacao";
import type { TrechoDoEnvio } from "./intervalo";
import type { ModoDistribuicao, PapelDeAjuste } from "./tabs";

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
  papelDeAjuste: PapelDeAjuste | null = null,
): Promise<ResumoComDivisao> {
  return invoke<ResumoComDivisao>("resumo_do_envio", {
    caminho,
    folhas,
    intervalo,
    modo,
    impressoras,
    papelDeAjuste,
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
  papelDeAjuste: PapelDeAjuste | null = null,
  marcaControle: Idioma | null = null,
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
    papelDeAjuste,
    marcaControle,
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
