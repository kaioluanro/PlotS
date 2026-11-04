import { CircleAlert } from "lucide-react";
import { useId, type FocusEventHandler, type ReactNode, type RefObject } from "react";
import type { EstadoDoEnvio } from "../cartoes";
import { t } from "../i18n";
import botoes from "./Botao.module.css";
import { ID_ERRO_DA_QUANTIDADE } from "./CampoCopias";
import styles from "./Sidebar.module.css";

interface Props {
  blocoFace: ReactNode;
  campoCopias: ReactNode;
  botao: EstadoDoEnvio;
  /** Falso enquanto o texto da quantidade nao e um numero de 1 a 999: o envio espera a correcao. */
  copiasValidas: boolean;
  botaoRef?: RefObject<HTMLButtonElement | null>;
  aoFocarEnvio?: FocusEventHandler<HTMLButtonElement>;
  aoImprimir: () => void;
}

/**
 * O que se escolhe antes do envio e o proprio envio, com o motivo embaixo. A quantidade invalida
 * troca o motivo pelo erro, e trava o botao: o envio nunca sai com um numero que nao esta na tela.
 */
export function GrupoDeEnvio({
  blocoFace,
  campoCopias,
  botao,
  copiasValidas,
  botaoRef,
  aoFocarEnvio,
  aoImprimir,
}: Props) {
  const idMotivo = useId();
  const erroDeCopias = !copiasValidas;
  const mensagem = erroDeCopias ? t("copias.invalida") : botao.motivo;
  const idDaMensagem = erroDeCopias ? ID_ERRO_DA_QUANTIDADE : idMotivo;
  return (
    <div className={styles.grupoEnvio}>
      {blocoFace}
      {campoCopias}
      <button
        ref={botaoRef}
        type="button"
        onFocus={aoFocarEnvio}
        className={`${botoes.botao} ${botoes.primario} ${styles.imprimir}`}
        disabled={botao.desabilitado || erroDeCopias}
        title={mensagem ?? undefined}
        aria-label={botao.rotuloAcessivel}
        aria-describedby={mensagem === null ? undefined : idDaMensagem}
        onClick={aoImprimir}
      >
        {botao.rotulo}
      </button>
      {mensagem !== null &&
        (erroDeCopias ? (
          <p id={idDaMensagem} role="alert" className={styles.recusado}>
            <CircleAlert size={12} aria-hidden="true" />
            {mensagem}
          </p>
        ) : (
          <p id={idDaMensagem} className={styles.dica}>
            {mensagem}
          </p>
        ))}
    </div>
  );
}
