import { Check, ChevronDown, CircleAlert, Info } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import { trocaPoeCalibracaoEmDuvida } from "../calibracao";
import { t } from "../i18n";
import type { Impressora } from "../impressoras";
import { nomeDoPapelDeAjuste, PAPEIS_DE_AJUSTE } from "../opcoes-envio";
import type { Aba, PapelDeAjuste } from "../tabs";
import { definirInversao } from "../trabalhos";
import opcoes from "./HistoricoDialog.module.css";
import styles from "./Sidebar.module.css";

/** Uma opcao nao foi gravada. Quem clicou precisa ver a frase, mesmo no fim de uma lista que rola. */
export function FalhaDaOpcao() {
  const frase = useRef<HTMLParagraphElement>(null);
  useEffect(() => {
    frase.current?.scrollIntoView({ block: "nearest" });
  }, []);
  return (
    <p ref={frase} role="alert" className={opcoes.falhaOpcao}>
      <CircleAlert size={12} className={opcoes.iconeFalha} aria-hidden="true" />
      <span>{t("inversao.falha")}</span>
    </p>
  );
}

/** As duas opcoes por aba do rodape: papel e folha de controle. Cedem antes do resto na janela curta. */
export function BlocoAjuste({
  aba,
  semMarca = false,
  aoDefinirPapel,
  aoDefinirControle,
}: {
  aba: Aba | null;
  /** A opcao esta ligada, mas tudo neste envio sai pela rota que nao usa marca. */
  semMarca?: boolean;
  aoDefinirPapel: (papel: PapelDeAjuste | null) => void;
  aoDefinirControle: (ligado: boolean) => void;
}) {
  const id = useId();
  const idDica = useId();
  const idAviso = useId();
  const ligado = aba?.controle ?? false;
  return (
    <div className={opcoes.opcoes} data-opcoes-do-envio="">
      <div className={opcoes.papel}>
        <label htmlFor={id} className={styles.rotuloLote}>
          {t("ajuste.rotulo")}
        </label>
        <span className={styles.caixaAjuste}>
          <select
            id={id}
            className={styles.campoAjuste}
            disabled={aba === null}
            value={aba?.papelDeAjuste ?? ""}
            onChange={(evento) => {
              const valor = evento.currentTarget.value;
              aoDefinirPapel(PAPEIS_DE_AJUSTE.find((papel) => papel === valor) ?? null);
            }}
          >
            <option value="">{t("ajuste.original")}</option>
            {PAPEIS_DE_AJUSTE.map((papel) => (
              <option key={papel} value={papel}>
                {nomeDoPapelDeAjuste(papel)}
              </option>
            ))}
          </select>
          <ChevronDown size={10} className={`${styles.setaSeletor} ${styles.setaAjuste}`} aria-hidden="true" />
        </span>
      </div>
      <div>
        <label className={opcoes.controle}>
          <span>{t("controle.rotulo")}</span>
          <span className={opcoes.alvoCaixa}>
            <input
              type="checkbox"
              className={opcoes.caixa}
              checked={ligado}
              disabled={aba === null}
              aria-describedby={semMarca ? `${idDica} ${idAviso}` : idDica}
              onChange={(evento) => aoDefinirControle(evento.currentTarget.checked)}
            />
            {ligado && <Check size={12} strokeWidth={3} className={opcoes.glifo} aria-hidden="true" />}
          </span>
        </label>
        <p id={idDica} className={opcoes.dicaOpcao} data-dica-opcao="">
          {t("controle.titulo")}
          {ligado && !semMarca && ` ${t("controle.canto")}`}
        </p>
        {semMarca && (
          <p id={idAviso} role="status" className={opcoes.avisoOpcao}>
            <Info size={12} className={opcoes.iconeAviso} aria-hidden="true" />
            <span>{t("controle.semMarca")}</span>
          </p>
        )}
      </div>
    </div>
  );
}

/** Na linha da impressora marcada: guarda no perfil da fila que o driver dela inverte a ordem. */
export function OpcaoInversao({
  impressora,
  aoMudar,
}: {
  impressora: Impressora;
  aoMudar?: () => void;
}) {
  const gravado = impressora.driverInverte ?? false;
  const [ligado, setLigado] = useState(gravado);
  const [falhou, setFalhou] = useState(false);
  const idDica = useId();
  const avisoDaCalibracao = useRef<HTMLParagraphElement>(null);
  // Compara com o valor gravado na calibracao, e nao com o que a tela lembrava ao abrir: remarcar a
  // impressora nao pode apagar o aviso.
  const emDuvida = trocaPoeCalibracaoEmDuvida(impressora.modo, impressora.driverNaCalibracao, ligado);
  useEffect(() => setLigado(gravado), [gravado]);
  // A linha cresce e a lista rola: o aviso nao pode nascer fora da vista.
  useEffect(() => {
    if (emDuvida) avisoDaCalibracao.current?.scrollIntoView?.({ block: "nearest" });
  }, [emDuvida]);
  return (
    <div>
      <label className={opcoes.linhaInversao} title={t("inversao.titulo")}>
        <span className={opcoes.alvoCaixa}>
          <input
            type="checkbox"
            className={opcoes.caixa}
            checked={ligado}
            aria-describedby={idDica}
            onChange={(evento) => {
              const pedido = evento.currentTarget.checked;
              setLigado(pedido);
              setFalhou(false);
              definirInversao(impressora.id, pedido)
                .then(() => aoMudar?.())
                .catch(() => {
                  setLigado(gravado);
                  setFalhou(true);
                });
            }}
          />
          {ligado && <Check size={12} strokeWidth={3} className={opcoes.glifo} aria-hidden="true" />}
        </span>
        <span>{t("inversao.rotulo")}</span>
      </label>
      <p id={idDica} className={`${opcoes.dicaOpcao} ${opcoes.dicaInversao}`} data-dica-opcao="">
        {t("inversao.dica")}
      </p>
      {emDuvida && (
        <p ref={avisoDaCalibracao} role="status" className={`${opcoes.avisoOpcao} ${opcoes.avisoDriver}`}>
          <CircleAlert size={14} className={opcoes.iconeAvisoDriver} aria-hidden="true" />
          <span>{t("calibracao.driverTrocado")}</span>
        </p>
      )}
      {falhou && <FalhaDaOpcao />}
    </div>
  );
}
