import { useEffect, useId, useState } from "react";
import { t } from "../i18n";
import { lerQuantidadeDeCopias, type FaceDoEnvio } from "../tabs";
import styles from "./BlocoGrupos.module.css";
import campo from "./CampoCopias.module.css";
import lote from "./Sidebar.module.css";

/** O erro da quantidade mora no rodape do envio: o campo so aponta para ele. */
export const ID_ERRO_DA_QUANTIDADE = "erro-quantidade";

/**
 * Quantas copias do documento sai da face escolhida. Um valor fora de 1 a 999 nao muda o numero que
 * vale: o texto digitado fica na tela, o envio espera a correcao, e o motivo aparece no rodape.
 */
export function CampoCopias({
  face,
  valor,
  aoDefinir,
  aoMudarValidade,
}: {
  face: FaceDoEnvio;
  valor: number;
  aoDefinir: (valor: number) => void;
  /** Avisa quem monta o rodape se o texto digitado e um numero aceito. */
  aoMudarValidade: (valido: boolean) => void;
}) {
  const idCampo = useId();
  const idDica = useId();
  const [texto, setTexto] = useState(String(valor));
  const [invalido, setInvalido] = useState(false);

  // Trocar de face ou um numero vindo da janela mostra o valor da face, sem texto antigo.
  useEffect(() => {
    setTexto(String(valor));
    setInvalido(false);
  }, [valor, face]);

  useEffect(() => {
    aoMudarValidade(!invalido);
  }, [invalido, aoMudarValidade]);

  // Se o campo some (outra aba, janela fechada), o rodape nao pode ficar preso num erro antigo.
  useEffect(() => () => aoMudarValidade(true), [aoMudarValidade]);

  const aoMudar = (novo: string) => {
    setTexto(novo);
    const numero = lerQuantidadeDeCopias(novo);
    if (numero === null) {
      setInvalido(true);
    } else {
      setInvalido(false);
      aoDefinir(numero);
    }
  };

  return (
    <div className={campo.campo} data-campo-copias="" data-face={face}>
      <div className={lote.loteLinha}>
        <label htmlFor={idCampo} className={styles.rotulo}>
          {t("copias.rotulo")}
        </label>
        <input
          id={idCampo}
          type="text"
          inputMode="numeric"
          autoComplete="off"
          maxLength={4}
          value={texto}
          aria-invalid={invalido}
          aria-describedby={invalido ? `${idDica} ${ID_ERRO_DA_QUANTIDADE}` : idDica}
          className={invalido ? `${lote.campoLote} ${lote.campoInvalido}` : lote.campoLote}
          onChange={(evento) => aoMudar(evento.target.value)}
        />
      </div>
      <p id={idDica} className={campo.dica}>
        {t("copias.dica")}
      </p>
    </div>
  );
}
