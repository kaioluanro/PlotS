import * as Dialog from "@radix-ui/react-dialog";
import { CircleAlert, TriangleAlert } from "lucide-react";
import { useRef } from "react";
import type { ConteudoConfirmacao } from "../confirmacao";
import { rolarPeloTeclado } from "../dialogo";
import { t } from "../i18n";
import botoes from "./Botao.module.css";
import styles from "./ConfirmDialog.module.css";
import { FiguraGesto } from "./FiguraGesto";
import { focarAbaAtiva } from "./TitleBar";

interface Props {
  /** O que o dialogo diz; `null` o mantem fechado. */
  conteudo: ConteudoConfirmacao | null;
  aoCancelar: () => void;
  aoConfirmar: () => void;
}

/** O titulo com o alerta antes do texto quando o passo seguinte gasta papel; o nome acessivel e so o texto. */
export function TituloDoDialogo({ conteudo }: { conteudo: ConteudoConfirmacao }) {
  return (
    <>
      {conteudo.atencao === true && (
        <TriangleAlert size={16} aria-hidden="true" className={styles.iconeDoTitulo} />
      )}
      <span className={styles.textoDoTitulo}>{t(conteudo.titulo, conteudo.parametros)}</span>
    </>
  );
}

export function ConfirmDialog({ conteudo, aoCancelar, aoConfirmar }: Props) {
  // O conteudo fica guardado para o texto nao sumir durante a animacao de saida.
  const ultimo = useRef<ConteudoConfirmacao | null>(null);
  if (conteudo !== null) ultimo.current = conteudo;
  const visivel = ultimo.current;
  const confirmado = useRef(false);
  const origem = useRef<HTMLElement | null>(null);
  const corpo = useRef<HTMLDivElement>(null);

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
          onKeyDown={(evento) => {
            if (rolarPeloTeclado(evento.key, corpo.current)) evento.preventDefault();
          }}
          onOpenAutoFocus={() => {
            origem.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
          }}
          onCloseAutoFocus={(evento) => {
            // Sem um gatilho do proprio dialogo, o foco cairia na pagina ao fechar.
            evento.preventDefault();
            const voltar =
              (!confirmado.current || ultimo.current?.focoNaOrigemAoConfirmar) && origem.current;
            confirmado.current = false;
            // A origem pode ter sumido ou nem ser um controle, como em um pedido vindo do sistema.
            if (voltar && voltar.isConnected && voltar !== document.body) voltar.focus();
            else focarAbaAtiva();
          }}
        >
          {visivel && (
            <>
              <Dialog.Title className={styles.titulo}>
                <TituloDoDialogo conteudo={visivel} />
              </Dialog.Title>
              <Dialog.Description asChild>
                <div ref={corpo} className={styles.corpo}>
                  <p className={styles.texto}>{t(visivel.texto, visivel.parametros)}</p>
                  {visivel.aviso !== undefined && (
                    <p className={styles.aviso}>
                      <CircleAlert size={14} aria-hidden="true" />
                      <span>{visivel.aviso}</span>
                    </p>
                  )}
                  {visivel.linhas && visivel.linhas.length > 0 && (
                    <ul className={styles.linhas}>
                      {visivel.linhas.map((linha, indice) => (
                        <li key={`${indice}-${linha}`}>{linha}</li>
                      ))}
                    </ul>
                  )}
                  {visivel.gestos?.map((gesto) => (
                    <div key={gesto.nome} className={styles.gesto}>
                      <p className={styles.nomeDoGesto}>{gesto.nome}</p>
                      {gesto.desenho !== null && <FiguraGesto desenho={gesto.desenho} />}
                      {gesto.frases.map((frase) => (
                        <p key={frase} className={styles.texto}>
                          {frase}
                        </p>
                      ))}
                    </div>
                  ))}
                  {visivel.consequencia !== null && (
                    <p className={styles.texto}>{t(visivel.consequencia)}</p>
                  )}
                  {visivel.notas?.map((nota, indice) => (
                    <p key={`${indice}-${nota}`} className={styles.nota}>
                      {nota}
                    </p>
                  ))}
                </div>
              </Dialog.Description>
              <div className={styles.rodape}>
                <Dialog.Close className={`${botoes.botao} ${botoes.secundario}`}>
                  {t(visivel.manter)}
                </Dialog.Close>
                {visivel.semConfirmar !== true && (
                <button
                  type="button"
                  className={`${botoes.botao} ${visivel.confirmaAvancando ? botoes.primario : botoes.perigo}`}
                  onClick={() => {
                    confirmado.current = true;
                    aoConfirmar();
                  }}
                >
                  {t(visivel.fechar)}
                </button>
                )}
              </div>
            </>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
