import type { OrdemDaPilha as Ordem } from "../confirmacao";
import { t } from "../i18n";
import styles from "./OrdemDaPilha.module.css";

/** Todos os lotes, de cima para baixo. Quem rola e a area em volta: nenhum lote fica escondido. */
export function OrdemDaPilha({ ordem }: { ordem: Ordem }) {
  return (
    <div className={styles.pilha}>
      <h4 className={styles.titulo}>{t("modo.pilha.titulo")}</h4>
      {ordem.cabecalhoDoGesto && <p className={styles.aoTerminar}>{t("modo.pilha.aoTerminar")}</p>}
      <ol className={ordem.comGesto ? `${styles.linhas} ${styles.comGesto}` : styles.linhas}>
        {ordem.linhas.map((linha) => (
          <li key={linha.lote} className={styles.linha}>
            <span className={styles.numero}>{linha.lote}</span>
            <span className={styles.paginas}>{linha.paginas}</span>
            <span className={styles.impressora} title={linha.impressora}>
              {linha.impressora}
            </span>
            {linha.gesto !== null && <span className={styles.gesto}>{linha.gesto}</span>}
          </li>
        ))}
      </ol>
      {ordem.frases.map((frase) => (
        <p key={frase} className={styles.frase}>
          {frase}
        </p>
      ))}
      <p className={styles.resumo}>{ordem.resumo}</p>
    </div>
  );
}
