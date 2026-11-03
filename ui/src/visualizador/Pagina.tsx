import { CircleAlert, LoaderCircle } from "lucide-react";
import type { PDFDocumentProxy } from "pdfjs-dist";
import { memo, useEffect } from "react";
import { t } from "../i18n";
import { chaveDoRotuloDaPagina } from "./contas";
import { PRIORIDADE_PAGINA, type FilaDeDesenho } from "./fila";
import { useDesenho } from "./useDesenho";
import styles from "./Visualizador.module.css";

interface Props {
  documento: PDFDocumentProxy;
  fila: FilaDeDesenho;
  numero: number;
  topo: number;
  esquerda: number;
  largura: number;
  altura: number;
  pixelsPorPonto: number;
  aoDescartar: () => void;
}

const ATRASO_DA_PAGINA_MS = 80;

export const Pagina = memo(function Pagina({
  documento,
  fila,
  numero,
  topo,
  esquerda,
  largura,
  altura,
  pixelsPorPonto,
  aoDescartar,
}: Props) {
  const { caixa, estado } = useDesenho({
    documento,
    fila,
    numero,
    pixelsPorPonto,
    prioridade: PRIORIDADE_PAGINA,
    atrasoMs: ATRASO_DA_PAGINA_MS,
  });
  useEffect(() => aoDescartar, [aoDescartar]);
  const n = String(numero);

  return (
    <div
      className={styles.pagina}
      style={{ top: topo, left: esquerda, width: largura, height: altura }}
      role="img"
      aria-label={t(chaveDoRotuloDaPagina(estado), { n })}
      aria-busy={estado === "carregando"}
    >
      <span ref={caixa} className={styles.tela} />
      {estado === "carregando" && (
        <div className={styles.aviso} title={t("visualizador.pagina.carregando", { n })}>
          <LoaderCircle size={14} className={styles.girando} aria-hidden="true" />
          <span className={styles.numero}>{n}</span>
        </div>
      )}
      {estado === "erro" && (
        <div className={styles.aviso}>
          <CircleAlert size={14} className={styles.iconeErro} aria-hidden="true" />
          <span className={styles.textoErroDaPagina} title={t("visualizador.pagina.erro", { n })}>
            {t("visualizador.pagina.erro", { n })}
          </span>
        </div>
      )}
    </div>
  );
});
