import { useCallback, useEffect, useState } from "react";
import { assinar } from "./backend";
import type { Impressora } from "./impressoras";
import {
  abrirPreferencias,
  avisosDepoisDaLeitura,
  codigoDaFalha,
  comAviso,
  lerEstadoDasPreferencias,
  ouvirRecusaDoAjuste,
  registrarFilasComAjuste,
  semAviso,
  semFilasQueSairam,
  voltarAoPadraoDaFila,
  type CodigoAvisoDoAjuste,
} from "./preferencias";

export interface Preferencias {
  salvas: ReadonlySet<string>;
  duasFaces: ReadonlySet<string>;
  /** Filas com a janela do driver aberta agora. */
  abertas: ReadonlySet<string>;
  avisos: ReadonlyMap<string, CodigoAvisoDoAjuste>;
  abrir: (fila: string) => void;
  voltarAoPadrao: (fila: string) => void;
  dispensarAviso: (fila: string) => void;
  /** Relê quem tem ajuste salvo, depois de aplicar ou voltar ao padrão pelo painel. */
  recarregar: () => void;
}

const NENHUMA: ReadonlySet<string> = new Set();
const SEM_AVISOS: ReadonlyMap<string, CodigoAvisoDoAjuste> = new Map();

/** Vale ate o fim da sessao, mesmo com a lateral recolhida e aberta de novo. */
const DISPENSADAS = new Set<string>();

function comFila(filas: ReadonlySet<string>, fila: string): ReadonlySet<string> {
  return new Set(filas).add(fila);
}

function semFila(filas: ReadonlySet<string>, fila: string): ReadonlySet<string> {
  const novas = new Set(filas);
  novas.delete(fila);
  return novas;
}

/** Quem tem ajuste salvo e o que aconteceu com ele. Rele junto com a lista de impressoras. */
export function usePreferencias(lendoLista: boolean, impressoras: readonly Impressora[]): Preferencias {
  const [salvas, setSalvas] = useState(NENHUMA);
  const [duasFaces, setDuasFaces] = useState(NENHUMA);
  const [abertas, setAbertas] = useState(NENHUMA);
  const [avisos, setAvisos] = useState(SEM_AVISOS);

  const ler = useCallback(() => {
    lerEstadoDasPreferencias()
      .then(({ salvas: lidas, ignoradas, duasFaces: lidasEmDuasFaces }) => {
        const filas = new Set(lidas);
        registrarFilasComAjuste(filas);
        setSalvas(filas);
        setDuasFaces(new Set(lidasEmDuasFaces));
        setAvisos((atuais) => avisosDepoisDaLeitura(atuais, ignoradas, DISPENSADAS));
      })
      .catch(() => {
        // Sem a leitura, a linha so nao mostra o ajuste; a lista avisa se os perfis nao abrem.
      });
  }, []);

  useEffect(() => {
    if (!lendoLista) ler();
  }, [lendoLista, ler]);

  useEffect(() => {
    if (!lendoLista) setAvisos((atuais) => semFilasQueSairam(atuais, impressoras));
  }, [lendoLista, impressoras]);

  useEffect(
    () =>
      assinar(
        ouvirRecusaDoAjuste((fila) => {
          setAvisos((atuais) => comAviso(atuais, fila, "recusado"));
          ler();
        }),
      ),
    [ler],
  );

  const abrir = useCallback(
    (fila: string) => {
      setAvisos((atuais) => semAviso(atuais, fila));
      setAbertas((atuais) => comFila(atuais, fila));
      abrirPreferencias(fila)
        .then((fim) => {
          if (fim === "salvo") ler();
          // Outro clique: a janela do primeiro continua aberta, e ele e quem encerra o estado.
          if (fim !== "ja_aberta") setAbertas((atuais) => semFila(atuais, fila));
        })
        .catch((erro: unknown) => {
          setAbertas((atuais) => semFila(atuais, fila));
          setAvisos((atuais) => comAviso(atuais, fila, codigoDaFalha(erro)));
        });
    },
    [ler],
  );

  const voltarAoPadrao = useCallback(
    (fila: string) => {
      setAvisos((atuais) => semAviso(atuais, fila));
      voltarAoPadraoDaFila(fila)
        .then(ler)
        .catch((erro: unknown) => {
          setAvisos((atuais) => comAviso(atuais, fila, codigoDaFalha(erro)));
        });
    },
    [ler],
  );

  const dispensarAviso = useCallback((fila: string) => {
    DISPENSADAS.add(fila);
    setAvisos((atuais) => semAviso(atuais, fila));
  }, []);

  return { salvas, duasFaces, abertas, avisos, abrir, voltarAoPadrao, dispensarAviso, recarregar: ler };
}
