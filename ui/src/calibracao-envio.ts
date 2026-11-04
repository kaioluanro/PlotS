import { enviarRodada } from "./calibracao-backend";
import type { Acao, PapelDoTeste, PerfilDaRodada } from "./calibracao";
import { comandarTrabalho } from "./trabalhos";

/** O que a tela esta fazendo agora: enquanto houver chamada, os botoes e a saida ficam travados. */
export type Ocupacao = "enviando" | "gravando" | "conferindo" | null;

/** Uma trava que vale no mesmo clique: o estado da tela so chega no proximo render. */
export interface Trava {
  atual(): Ocupacao;
  /** Roda a tarefa com a trava fechada. Devolve `false`, sem rodar nada, se ja houver chamada. */
  executar(ocupacao: Exclude<Ocupacao, null>, tarefa: () => Promise<unknown> | void): boolean;
}

export function criarTrava(aoMudar: (ocupacao: Ocupacao) => void): Trava {
  let ocupada: Ocupacao = null;
  return {
    atual: () => ocupada,
    executar(ocupacao, tarefa) {
      if (ocupada !== null) return false;
      ocupada = ocupacao;
      aoMudar(ocupacao);
      const soltar = () => {
        ocupada = null;
        aoMudar(null);
      };
      let execucao: Promise<unknown> | void;
      try {
        execucao = tarefa();
      } catch {
        execucao = undefined;
      }
      void Promise.resolve(execucao).catch(() => undefined).then(soltar);
      return true;
    },
  };
}

export interface PedidoDeRodada {
  trava: Trava;
  id: string;
  papel: PapelDoTeste;
  rodada: number;
  perfil: () => Promise<PerfilDaRodada>;
  despachar: (acao: Acao) => void;
}

export interface PedidoDoVerso {
  trava: Trava;
  id: string;
  despachar: (acao: Acao) => void;
}

/** Manda o verso. Enquanto o comando corre, nada da tela responde; uma falha vira a frase de verso. */
export function enviarVerso(pedido: PedidoDoVerso): boolean {
  return pedido.trava.executar("enviando", () =>
    comandarTrabalho(pedido.id, "enviar_verso").catch(() => {
      pedido.despachar({ tipo: "verso_falhou" });
    }),
  );
}

/** Manda o teste de uma rodada. Um segundo pedido enquanto o primeiro corre nao manda nada. */
export function iniciarRodada(pedido: PedidoDeRodada): boolean {
  return pedido.trava.executar("enviando", async () => {
    pedido.despachar({ tipo: "chamada_iniciada" });
    let perfil: PerfilDaRodada;
    try {
      perfil = await pedido.perfil();
    } catch {
      pedido.despachar({ tipo: "recusada", recusa: "falha_perfil" });
      return;
    }
    try {
      const aceita = await enviarRodada(pedido.id, perfil, pedido.papel);
      if (aceita) pedido.despachar({ tipo: "rodada_iniciada", perfil, rodada: pedido.rodada });
      else pedido.despachar({ tipo: "recusada", recusa: "ocupada" });
    } catch {
      pedido.despachar({ tipo: "recusada", recusa: "falha_envio" });
    }
  });
}
