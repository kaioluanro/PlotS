import { Printer } from "lucide-react";
import { t } from "../i18n";
import { EmptyState } from "./EmptyState";
import styles from "./Sidebar.module.css";

export const ID_LATERAL = "barra-lateral";

export function Sidebar() {
  return (
    <aside id={ID_LATERAL} className={styles.lateral} aria-labelledby="titulo-lateral">
      <div className={styles.cabecalho}>
        <h2 id="titulo-lateral" className={styles.titulo}>
          {t("lateral.titulo")}
        </h2>
      </div>
      <div className={styles.corpo}>
        <EmptyState
          discreto
          icone={<Printer size={18} />}
          titulo={t("lateral.vazioTitulo")}
          texto={t("lateral.vazioTexto")}
        />
      </div>
    </aside>
  );
}
