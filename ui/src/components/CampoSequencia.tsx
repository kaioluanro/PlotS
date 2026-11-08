import { useId } from "react";
import { t } from "../i18n";
import type { FaceDoEnvio } from "../tabs";
import styles from "./CampoSequencia.module.css";

/**
 * A escolha de mandar as frentes uma cópia por vez. Fica acima da quantidade e vale só para esta
 * aba; a dica muda no verso, onde as cópias continuam saindo juntas.
 */
export function CampoSequencia({
  face,
  ligada,
  aoDefinir,
}: {
  face: FaceDoEnvio;
  ligada: boolean;
  aoDefinir: (ligada: boolean) => void;
}) {
  const id = useId();
  const idDica = useId();
  return (
    <div className={styles.campo} data-campo-sequencia="" data-face={face}>
      <label htmlFor={id} className={styles.rotulo}>
        <input
          id={id}
          type="checkbox"
          checked={ligada}
          aria-describedby={idDica}
          onChange={(evento) => aoDefinir(evento.target.checked)}
        />
        <span>{t("sequencia.rotulo")}</span>
      </label>
      <p id={idDica} className={styles.dica}>
        {t(ligada ? "sequencia.dica" : "sequencia.dica.desligada")}
      </p>
    </div>
  );
}