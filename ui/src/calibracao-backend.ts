import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AvaliacaoDaRodada,
  ConclusaoAutomatica,
  ModeloInicial,
  PapelDoTeste,
  PerfilDaRodada,
  SaidaSimulada,
} from "./calibracao";
import type { Cartao, Recolocar } from "./cartoes";

/** Os cartoes do teste tem evento proprio: nao aparecem na lista de trabalhos do documento. */
export function ouvirCartoesDaCalibracao(aoReceber: (cartao: Cartao) => void): Promise<() => void> {
  return listen<Cartao>("calibracao", (evento) => aoReceber(evento.payload));
}

export function perfilInicial(modelo: ModeloInicial, driverInverte: boolean): Promise<PerfilDaRodada> {
  return invoke<PerfilDaRodada>("calibracao_perfil_inicial", { modelo, driverInverte });
}

/** Devolve `false` se a impressora ja tem um trabalho; nesse caso, nada foi enviado. */
export function enviarRodada(impressora: string, perfil: PerfilDaRodada, papel: PapelDoTeste): Promise<boolean> {
  return invoke<boolean>("calibracao_enviar", { impressora, perfil, papel });
}

export function encerrarCalibracao(impressora: string, cancelar: boolean): Promise<void> {
  return invoke<void>("calibracao_encerrar", { impressora, cancelar });
}

export function recolocarDoPerfil(perfil: PerfilDaRodada, saiuParaCima: boolean): Promise<Recolocar> {
  return invoke<Recolocar>("calibracao_recolocar", { perfil, saiuParaCima });
}

export function avaliarRodada(perfil: PerfilDaRodada, respostas: unknown): Promise<AvaliacaoDaRodada> {
  return invoke<AvaliacaoDaRodada>("calibracao_avaliar", { perfil, respostas });
}

export function conferirAutomatica(
  parCerto: boolean,
  setasIguais: boolean,
  leitura: string,
): Promise<ConclusaoAutomatica> {
  return invoke<ConclusaoAutomatica>("calibracao_conferir_automatica", { parCerto, setasIguais, leitura });
}

export function gravarCalibracao(
  impressora: string,
  perfil: PerfilDaRodada,
  saiuParaCima: boolean,
  leitura: string,
): Promise<void> {
  return invoke<void>("calibracao_gravar", { impressora, perfil, saiuParaCima, leitura });
}

export function saidaSimulada(impressora: string, perfil: PerfilDaRodada): Promise<SaidaSimulada> {
  return invoke<SaidaSimulada>("calibracao_saida_simulada", { impressora, perfil });
}
