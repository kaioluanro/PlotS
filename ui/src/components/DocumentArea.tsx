import { CircleAlert, FileUp, X } from "lucide-react";
import { t } from "../i18n";
import type { Documento } from "../tabs";
import { Visualizador } from "../visualizador/Visualizador";
import botoes from "./Botao.module.css";
import styles from "./DocumentArea.module.css";
import { EmptyState } from "./EmptyState";
import { idDaAba } from "./TitleBar";

export const ID_AREA_DOCUMENTO = "area-documento";

interface Props {
  documento: Documento | null;
  idAbaAtiva: number | null;
  erro: string | null;
  /** Motivo de o PDF desta aba ter sido recusado; a aba fica aberta para mostrar o motivo. */
  recusa: string | null;
  arrastando: boolean;
  aoDispensarErro: () => void;
  aoAbrir: () => void;
}

export function DocumentArea({
  documento,
  idAbaAtiva,
  erro,
  recusa,
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
      {documento && recusa === null ? (
        // A chave separa as abas: trocar de aba solta o PDF anterior, mesmo com o mesmo arquivo.
        <Visualizador key={idAbaAtiva} documento={documento} />
      ) : (
        <div className={styles.centro}>
          {documento ? (
            <div role="alert">
              <EmptyState icone={<CircleAlert size={18} />} titulo={documento.nome} texto={recusa ?? ""}>
                <button
                  type="button"
                  className={`${botoes.botao} ${botoes.secundario}`}
                  onClick={aoAbrir}
                >
                  {t("documento.abrir")}
                </button>
              </EmptyState>
            </div>
          ) : (
            <EmptyState
              icone={<FileUp size={18} />}
              titulo={t("documento.vazioTitulo")}
              texto={t("documento.vazioTexto")}
            >
              <button
                type="button"
                className={`${botoes.botao} ${botoes.secundario}`}
                onClick={aoAbrir}
              >
                {t("documento.abrir")}
                <kbd className={styles.atalho}>{t("documento.atalhoAbrir")}</kbd>
              </button>
            </EmptyState>
          )}
        </div>
      )}
      {arrastando && (
        <div className={styles.soltar} role="status">
          {t("documento.soltar")}
        </div>
      )}
    </main>
  );
}
