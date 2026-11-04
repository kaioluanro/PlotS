import { useEffect, useState } from "react";
import { lerIntervaloDaAba, trechoDoEnvio } from "./intervalo";
import type { Aba } from "./tabs";
import { resumoDoEnvio } from "./trabalhos";

/** O que a lateral precisa saber do papel deste envio, antes de alguem pedir para enviar. */
export interface AnaliseDoDocumento {
  variosPapeis: boolean;
  temPapelManual: boolean;
  /** Ha papel lido e todo ele sai sem gesto numa impressora automatica. */
  soPapelAutomatico: boolean;
}

/**
 * Le os papeis do trecho que esta na tela, ja no papel de destino. Sem resposta, a lateral fica como
 * sem documento lido: mostra a conta de sempre e nao oferece a opcao do driver na fila automatica.
 */
export function useAnaliseDoDocumento(aba: Aba | null): AnaliseDoDocumento | null {
  const [analise, setAnalise] = useState<AnaliseDoDocumento | null>(null);
  const trecho = aba === null ? null : trechoDoEnvio(lerIntervaloDaAba(aba));
  const caminho = aba?.documento.caminho ?? null;
  const conhecido = aba?.paginas.estado === "conhecido";
  const papel = aba?.papelDeAjuste ?? null;
  const inicio = trecho?.inicio ?? null;
  const fim = trecho?.fim ?? null;
  useEffect(() => {
    if (caminho === null || !conhecido) {
      setAnalise(null);
      return undefined;
    }
    let valida = true;
    resumoDoEnvio(
      caminho,
      inicio === null || fim === null ? null : { inicio, fim },
      [],
      papel,
    )
      .then((resumo) => {
        if (!valida) return;
        const fluxos = resumo.fluxos ?? [];
        setAnalise({
          variosPapeis: resumo.papeis.length > 1,
          temPapelManual: fluxos.some((fluxo) => !fluxo.automatico),
          soPapelAutomatico: fluxos.length > 0 && fluxos.every((fluxo) => fluxo.automatico),
        });
      })
      .catch(() => {
        if (valida) setAnalise(null);
      });
    return () => {
      valida = false;
    };
  }, [caminho, conhecido, papel, inicio, fim]);
  return analise;
}
