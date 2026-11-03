import { useEffect, useMemo, useRef, useState } from "react";
import {
  ESPERA_PARADA,
  lerIntervalo,
  proximaEspera,
  type EstadoDaEspera,
  type LeituraIntervalo,
  type RascunhoIntervalo,
} from "./intervalo";
import type { Aba } from "./tabs";

/** Quanto tempo sem digitar ate o erro aparecer. */
export const ESPERA_DO_ERRO_MS = 600;

export interface EsperaDoErro {
  /** Uma tecla: recomeca a contagem. Recebe o que o texto dizia antes e o que diz agora. */
  digitar: (antes: LeituraIntervalo, depois: LeituraIntervalo) => void;
  /** Saiu do campo ou apertou Enter: a digitacao acabou, sem esperar a contagem. */
  concluir: () => void;
  /** O componente saiu da tela: nenhuma contagem fica rodando. */
  encerrar: () => void;
}

/** A contagem, sem nada de React: quem usa recebe cada mudanca de estado em `aoMudar`. */
export function criarEsperaDoErro(aoMudar: (estado: EstadoDaEspera) => void): EsperaDoErro {
  let estado = ESPERA_PARADA;
  let relogio: ReturnType<typeof setTimeout> | null = null;
  const pararRelogio = () => {
    if (relogio !== null) clearTimeout(relogio);
    relogio = null;
  };
  const concluir = () => {
    pararRelogio();
    estado = ESPERA_PARADA;
    aoMudar(estado);
  };
  return {
    digitar(antes, depois) {
      pararRelogio();
      estado = proximaEspera(estado, antes, depois);
      aoMudar(estado);
      relogio = setTimeout(concluir, ESPERA_DO_ERRO_MS);
    },
    concluir,
    encerrar: pararRelogio,
  };
}

export interface ControleDaEspera {
  estado: EstadoDaEspera;
  aoDigitar: (proximo: RascunhoIntervalo | null) => void;
  aoConcluir: () => void;
}

/** A espera pertence a aba em que se digitou: as outras abas nao herdam a digitacao. */
export function useEsperaDoErro(aba: Aba | null): ControleDaEspera {
  const [registro, setRegistro] = useState<{ dono: number | null; estado: EstadoDaEspera }>({
    dono: null,
    estado: ESPERA_PARADA,
  });
  const idDaAba = aba?.id ?? null;
  const dono = useRef(idDaAba);
  const espera = useMemo(
    () => criarEsperaDoErro((estado) => setRegistro({ dono: dono.current, estado })),
    [],
  );
  useEffect(() => () => espera.encerrar(), [espera]);

  return {
    estado: registro.dono === idDaAba ? registro.estado : ESPERA_PARADA,
    aoDigitar(proximo) {
      if (aba === null) return;
      // Digitar em outra aba comeca do zero: o que estava na tela era de outro texto.
      if (dono.current !== idDaAba) {
        dono.current = idDaAba;
        espera.concluir();
      }
      espera.digitar(lerIntervalo(aba.paginas, aba.intervalo), lerIntervalo(aba.paginas, proximo));
    },
    aoConcluir: () => espera.concluir(),
  };
}
