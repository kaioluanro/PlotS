import type { ReactNode } from "react";
import styles from "./EmptyState.module.css";

interface Props {
  icone: ReactNode;
  titulo: string;
  texto?: string;
  children?: ReactNode;
}

export function EmptyState({ icone, titulo, texto, children }: Props) {
  return (
    <div className={styles.vazio}>
      <div className={styles.icone} aria-hidden="true">
        {icone}
      </div>
      <p className={styles.titulo}>{titulo}</p>
      {texto && <p className={styles.texto}>{texto}</p>}
      {children && <div className={styles.acao}>{children}</div>}
    </div>
  );
}
