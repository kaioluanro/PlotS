import type { ReactNode } from "react";
import styles from "./EmptyState.module.css";

interface Props {
  icone: ReactNode;
  titulo: string;
  texto?: string;
  /** Versao de apoio, para uma regiao que nao e o foco da janela. */
  discreto?: boolean;
  children?: ReactNode;
}

export function EmptyState({ icone, titulo, texto, discreto = false, children }: Props) {
  return (
    <div className={discreto ? `${styles.vazio} ${styles.discreto}` : styles.vazio}>
      <div className={styles.icone} aria-hidden="true">
        {icone}
      </div>
      <p className={styles.titulo}>{titulo}</p>
      {texto && <p className={styles.texto}>{texto}</p>}
      {children && <div className={styles.acao}>{children}</div>}
    </div>
  );
}
