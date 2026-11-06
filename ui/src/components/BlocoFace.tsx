import { Check } from "lucide-react";
import { useId } from "react";
import { t } from "../i18n";
import type { FaceDoEnvio } from "../tabs";
import styles from "./BlocoGrupos.module.css";
import lateral from "./Sidebar.module.css";

const FACES: readonly { valor: FaceDoEnvio; rotulo: "face.frente" | "face.verso" }[] = [
  { valor: "frente", rotulo: "face.frente" },
  { valor: "verso", rotulo: "face.verso" },
];

/** O que o botao de envio vai imprimir, em todas as marcadas. So a pessoa muda esta escolha. */
export function BlocoFace({
  face,
  aoDefinir,
  dicaDoEstado = null,
  semVerso = false,
}: {
  face: FaceDoEnvio;
  aoDefinir: (face: FaceDoEnvio) => void;
  /** Diz o estado da face escolhida, sem trocar a escolha. Quando existe, substitui a dica do modo. */
  dicaDoEstado?: string | null;
  /** O trabalho não tem verso: só a frente pode ser escolhida, e o motivo fica à vista. */
  semVerso?: boolean;
}) {
  const nome = useId();
  const idDica = useId();
  const ativa = semVerso ? "frente" : face;
  return (
    <fieldset className={styles.modo} aria-describedby={idDica} data-bloco-face="">
      <legend className={styles.rotulo}>{t("face.rotulo")}</legend>
      <div className={styles.opcoes}>
        {FACES.map(({ valor, rotulo }) => {
          const indisponivel = semVerso && valor === "verso";
          const marcada = valor === ativa;
          return (
            <label
              key={valor}
              className={[
                styles.opcao,
                marcada ? styles.opcaoAtiva : "",
                indisponivel ? styles.opcaoIndisponivel : "",
              ]
                .filter(Boolean)
                .join(" ")}
              aria-disabled={indisponivel || undefined}
            >
              <input
                type="radio"
                className={lateral.ocultoVisual}
                name={nome}
                value={valor}
                checked={marcada}
                disabled={indisponivel}
                tabIndex={indisponivel ? -1 : undefined}
                aria-describedby={indisponivel ? idDica : undefined}
                onChange={() => aoDefinir(valor)}
              />
              {marcada && <Check size={12} strokeWidth={3} aria-hidden="true" />}
              <span>{t(rotulo)}</span>
            </label>
          );
        })}
      </div>
      {/* A dica do estado nao some em altura curta: ela diz o que fazer agora. */}
      <p id={idDica} className={styles.dica} data-dica-modo={dicaDoEstado === null && !semVerso ? "" : undefined}>
        {semVerso
          ? t("face.semVerso")
          : dicaDoEstado ?? t(face === "frente" ? "face.dica.frente" : "face.dica.verso")}
      </p>
    </fieldset>
  );
}
