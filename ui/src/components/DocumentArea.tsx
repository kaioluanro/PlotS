import { CircleAlert, FileText, FileUp, X } from "lucide-react";
import { t } from "../i18n";
import type { Documento } from "../tabs";
import botoes from "./Botao.module.css";
import styles from "./DocumentArea.module.css";
import { EmptyState } from "./EmptyState";
import { idDaAba } from "./TitleBar";

export const ID_AREA_DOCUMENTO = "area-documento";

interface Props {
  documento: Documento | null;
  idAbaAtiva: number | null;
  erro: string | null;
  arrastando: boolean;
  aoDispensarErro: () => void;
  aoAbrir: () => void;
}

export function DocumentArea({
  documento,
  idAbaAtiva,
  erro,
  arrastando,
  aoDispensarErro,
  aoAbrir,
}: Props) {
  return (
    <main
      id={ID_AREA_DOCUMENTO}
      className={styles.area}
      {...(idAbaAtiva !== null
        ? { role: "tabpanel", "aria-labelledby": idDaAba(idAbaAtiva) }
        : {})}
    >
      {erro !== null && (
        <div className={styles.erro} role="alert">
          <CircleAlert size={14} className={styles.iconeErro} aria-hidden="true" />
          <p className={styles.textoErro}>{erro}</p>
          <button
            type="button"
            className={styles.dispensar}
            aria-label={t("erro.dispensar")}
            title={t("erro.dispensar")}
            onClick={aoDispensarErro}
          >
            <X size={12} />
          </button>
        </div>
      )}
      <div className={styles.centro}>
        {documento ? (
          <EmptyState
            icone={<FileText size={18} />}
            titulo={documento.nome}
            texto={t("documento.semVisualizacao")}
          />
        ) : (
          <EmptyState
            icone={<FileUp size={18} />}
            titulo={t("documento.vazioTitulo")}
            texto={t("documento.vazioTexto")}
          >
            <button type="button" className={`${botoes.botao} ${botoes.secundario}`} onClick={aoAbrir}>
              {t("documento.abrir")}
              <kbd className={styles.atalho}>{t("documento.atalhoAbrir")}</kbd>
            </button>
          </EmptyState>
        )}
      </div>
      {arrastando && (
        <div className={styles.soltar} role="status">
          {t("documento.soltar")}
        </div>
      )}
    </main>
  );
}
