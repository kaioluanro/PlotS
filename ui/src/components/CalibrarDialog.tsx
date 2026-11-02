import * as Dialog from "@radix-ui/react-dialog";
import { Check, CircleAlert } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { comandoDeCalibrar, PROGRAMA, SAIDA_NA_PASTA_ATUAL } from "../calibrar";
import { rolarPeloTeclado } from "../dialogo";
import { t } from "../i18n";
import botoes from "./Botao.module.css";
import styles from "./ConfirmDialog.module.css";

const DURACAO_DO_RETORNO_MS = 2500;

interface Props {
  /** Nome exato da fila; `null` mantem o dialogo fechado. */
  fila: string | null;
  aoFechar: () => void;
}

/** So explica como calibrar pelo terminal: nada e executado a partir daqui. */
export function CalibrarDialog({ fila, aoFechar }: Props) {
  // O nome fica guardado para o texto nao sumir durante a animacao de saida.
  const ultima = useRef("");
  if (fila !== null) ultima.current = fila;
  const nome = ultima.current;
  const gatilho = useRef<HTMLElement | null>(null);
  const comando = comandoDeCalibrar(nome);
  const [antesDoExecutavel, depoisDoExecutavel] = t("calibrar.instalacao").split(PROGRAMA);
  const corpo = useRef<HTMLDivElement>(null);

  return (
    <Dialog.Root
      open={fila !== null}
      onOpenChange={(aberto) => {
        if (!aberto) aoFechar();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className={styles.veu} />
        <Dialog.Content
          className={styles.conteudo}
          onKeyDown={(evento) => {
            if (rolarPeloTeclado(evento.key, corpo.current)) evento.preventDefault();
          }}
          onOpenAutoFocus={() => {
            gatilho.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
          }}
          onCloseAutoFocus={(evento) => {
            // Sem um gatilho do proprio dialogo, o foco cairia na pagina ao fechar.
            evento.preventDefault();
            gatilho.current?.focus();
          }}
        >
          <Dialog.Title className={styles.titulo}>{t("calibrar.titulo", { nome })}</Dialog.Title>
          <Dialog.Description asChild>
            <div ref={corpo} className={styles.corpo}>
              {comando === null ? (
                <p className={styles.texto}>{t("calibrar.semComando")}</p>
              ) : (
                <>
                  <p className={styles.texto}>{t("calibrar.intro")}</p>
                  <code className={styles.comando}>
                    {comando.slice(0, -SAIDA_NA_PASTA_ATUAL.length)}
                    <span className={styles.semQuebra}>{SAIDA_NA_PASTA_ATUAL}</span>
                  </code>
                  <p className={styles.texto}>
                    {antesDoExecutavel}
                    <code className={styles.termo}>{PROGRAMA}</code>
                    {depoisDoExecutavel}
                  </p>
                  <p className={styles.texto}>{t("calibrar.passos")}</p>
                  <p className={styles.texto}>{t("calibrar.conferir")}</p>
                </>
              )}
            </div>
          </Dialog.Description>
          <div className={styles.rodape}>
            {comando !== null && <CopiarComando comando={comando} />}
            <Dialog.Close className={`${botoes.botao} ${botoes.secundario}`}>
              {t("calibrar.fechar")}
            </Dialog.Close>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

type Retorno = "nenhum" | "copiado" | "falhou";

/** Fica dentro do conteudo do dialogo: ao fechar, o retorno some junto com ele. */
function CopiarComando({ comando }: { comando: string }) {
  const [retorno, setRetorno] = useState<Retorno>("nenhum");
  const prazo = useRef<number | undefined>(undefined);

  useEffect(() => () => window.clearTimeout(prazo.current), []);

  const copiar = () => {
    window.clearTimeout(prazo.current);
    // A area de transferencia pode faltar ou recusar: o comando continua selecionavel na tela.
    Promise.resolve()
      .then(() => navigator.clipboard.writeText(comando))
      .then(() => {
        setRetorno("copiado");
        prazo.current = window.setTimeout(() => setRetorno("nenhum"), DURACAO_DO_RETORNO_MS);
      })
      .catch(() => setRetorno("falhou"));
  };

  return (
    <>
      <p className={styles.retorno} aria-live="polite">
        {retorno === "copiado" && (
          <>
            <Check size={12} aria-hidden="true" />
            <span>{t("calibrar.copiado")}</span>
          </>
        )}
        {retorno === "falhou" && (
          <>
            <CircleAlert size={12} className={styles.retornoIcone} aria-hidden="true" />
            <span>{t("calibrar.copiaFalhou")}</span>
          </>
        )}
      </p>
      <button type="button" className={`${botoes.botao} ${botoes.secundario}`} onClick={copiar}>
        {t("calibrar.copiar")}
      </button>
    </>
  );
}
