import * as Dialog from "@radix-ui/react-dialog";
import { useRef } from "react";
import type { ConteudoConfirmacao } from "../confirmacao";
import { t } from "../i18n";
import botoes from "./Botao.module.css";
import styles from "./ConfirmDialog.module.css";
import { focarAbaAtiva } from "./TitleBar";

interface Props {
  /** O que o dialogo diz; `null` o mantem fechado. */
  conteudo: ConteudoConfirmacao | null;
  aoCancelar: () => void;
  aoConfirmar: () => void;
}

export function ConfirmDialog({ conteudo, aoCancelar, aoConfirmar }: Props) {
  // O conteudo fica guardado para o texto nao sumir durante a animacao de saida.
  const ultimo = useRef<ConteudoConfirmacao | null>(null);
  if (conteudo !== null) ultimo.current = conteudo;
  const visivel = ultimo.current;
  const confirmado = useRef(false);

  return (
    <Dialog.Root
      open={conteudo !== null}
      onOpenChange={(aberto) => {
        if (!aberto) aoCancelar();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className={styles.veu} />
        <Dialog.Content
          className={styles.conteudo}
          onCloseAutoFocus={(evento) => {
            // A aba que tinha o foco pode ter sumido; so nesse caso o foco vai para a aba ativa.
            if (confirmado.current) {
              confirmado.current = false;
              evento.preventDefault();
              focarAbaAtiva();
            }
          }}
        >
          {visivel && (
            <>
              <Dialog.Title className={styles.titulo}>{t(visivel.titulo)}</Dialog.Title>
              <Dialog.Description asChild>
                <div className={styles.corpo}>
                  <p className={styles.texto}>{t(visivel.texto, visivel.parametros)}</p>
                  <p className={styles.texto}>{t(visivel.consequencia)}</p>
                </div>
              </Dialog.Description>
              <div className={styles.rodape}>
                <Dialog.Close className={`${botoes.botao} ${botoes.secundario}`}>
                  {t(visivel.manter)}
                </Dialog.Close>
                <button
                  type="button"
                  className={`${botoes.botao} ${botoes.perigo}`}
                  onClick={() => {
                    confirmado.current = true;
                    aoConfirmar();
                  }}
                >
                  {t(visivel.fechar)}
                </button>
              </div>
            </>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
