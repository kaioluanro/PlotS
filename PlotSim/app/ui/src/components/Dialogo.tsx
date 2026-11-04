import styles from "@plots/components/ConfirmDialog.module.css";
import { rolarPeloTeclado } from "@plots/dialogo";
import * as Dialog from "@radix-ui/react-dialog";
import { CircleAlert } from "lucide-react";
import { useRef, type ReactNode } from "react";
import { textoDoErro } from "../tela";
import proprios from "./Dialogo.module.css";

interface Props {
  titulo: string;
  aoFechar: () => void;
  children: ReactNode;
  rodape: ReactNode;
  /** Com rádios no corpo, as setas são deles; sem isso, as setas rolam o corpo. */
  setasRolam?: boolean;
  largura?: "normal" | "larga";
}

/** O diálogo do PlotS, com o CSS dele e o texto daqui. */
export function Dialogo({ titulo, aoFechar, children, rodape, setasRolam = true, largura }: Props) {
  const origem = useRef<HTMLElement | null>(null);
  const corpo = useRef<HTMLDivElement>(null);
  return (
    <Dialog.Root
      open
      onOpenChange={(aberto) => {
        if (!aberto) aoFechar();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className={styles.veu} />
        <Dialog.Content
          className={`${styles.conteudo} ${largura === "larga" ? proprios.larga : ""}`}
          aria-describedby={undefined}
          onKeyDown={(evento) => {
            if (setasRolam && rolarPeloTeclado(evento.key, corpo.current)) evento.preventDefault();
          }}
          onOpenAutoFocus={() => {
            origem.current =
              document.activeElement instanceof HTMLElement ? document.activeElement : null;
          }}
          onCloseAutoFocus={(evento) => {
            evento.preventDefault();
            // A origem pode ter sumido, como ao remover a impressora que a abriu.
            if (origem.current?.isConnected) origem.current.focus();
          }}
        >
          <Dialog.Title className={styles.titulo}>{titulo}</Dialog.Title>
          <div ref={corpo} className={styles.corpo}>
            {children}
          </div>
          <div className={styles.rodape}>{rodape}</div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

export function ErroDoDialogo({ erro }: { erro: unknown }) {
  const { texto, detalhe } = textoDoErro(erro);
  return (
    <div role="alert" className={proprios.erro}>
      <CircleAlert size={14} aria-hidden="true" className={proprios.erroIcone} />
      <div className={proprios.erroTexto}>
        <p>{texto}</p>
        {detalhe !== null && <p className={proprios.erroDetalhe}>{detalhe}</p>}
      </div>
    </div>
  );
}
