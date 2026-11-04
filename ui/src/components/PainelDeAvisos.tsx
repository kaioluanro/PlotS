import * as Dialog from "@radix-ui/react-dialog";
import { useRef } from "react";
import { entradasDoPainel, LIMITE_DE_AVISOS, type AvisoDoTrabalho } from "../avisos";
import type { Cartao, Comando } from "../cartoes";
import { origemDoDialogo, rolarPeloTeclado } from "../dialogo";
import { idiomaAtual, t } from "../i18n";
import botoes from "./Botao.module.css";
import dialogo from "./ConfirmDialog.module.css";
import { ICONE_QUE_GIRA, iconeDoCartao } from "./AvisosDoTopo";
import { CartaoImpressao } from "./CartaoImpressao";
import styles from "./PainelDeAvisos.module.css";

interface Props {
  aberto: boolean;
  itens: readonly AvisoDoTrabalho[];
  /** Os cartoes que ainda estao na janela, em todas as abas: so eles recebem acoes. */
  cartoesVivos: readonly Cartao[];
  aoFechar: () => void;
  aoComandar: (cartao: Cartao, comando: Comando) => void;
  aoPedirCancelar: (cartao: Cartao) => void;
  aoPedirReimprimir: (cartao: Cartao) => void;
}

function horaDoAviso(quando: number): string {
  return new Intl.DateTimeFormat(idiomaAtual === "en" ? "en-US" : "pt-BR", {
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(quando));
}

const CLASSE_DO_BOTAO = `${botoes.botao} ${botoes.secundario}`;

/** Onde a pessoa clica para a face que falta: o painel so diz, o envio fica no rodape. */
function dicaDoPainel(cartao: Cartao): string | null {
  switch (cartao.estado.tipo) {
    case "frentes_prontas":
      return t("avisos.dicaVersos");
    case "aguardando":
      return t(cartao.simulada ? "cartao.verso.cliqueSimulado" : "cartao.verso.clique");
    case "proximo_lote":
      return t(cartao.simulada ? "cartao.proximo.cliqueSimulado" : "cartao.proximo.clique");
    default:
      return null;
  }
}

/**
 * Lista da sessao, do mais novo para o mais antigo. O texto e o gesto vem do registro do momento; as
 * acoes so existem no cartao que ainda esta na janela.
 */
export function PainelDeAvisos({
  aberto,
  itens,
  cartoesVivos,
  aoFechar,
  aoComandar,
  aoPedirCancelar,
  aoPedirReimprimir,
}: Props) {
  /** O cartao vivo do trabalho da entrada: as acoes valem para ele, nunca para o registro guardado. */
  const vivoDe = (aviso: AvisoDoTrabalho): Cartao | undefined =>
    cartoesVivos.find(
      (atual) => atual.aba === aviso.cartao.aba && atual.impressora === aviso.cartao.impressora,
    );
  const aoComandarVivo = (aviso: AvisoDoTrabalho, comando: Comando) => {
    const vivo = vivoDe(aviso);
    if (vivo !== undefined) aoComandar(vivo, comando);
  };
  const aoPedirCancelarVivo = (aviso: AvisoDoTrabalho) => {
    const vivo = vivoDe(aviso);
    if (vivo !== undefined) aoPedirCancelar(vivo);
  };
  const aoPedirReimprimirVivo = (aviso: AvisoDoTrabalho) => {
    const vivo = vivoDe(aviso);
    if (vivo !== undefined) aoPedirReimprimir(vivo);
  };
  const botaoFechar = useRef<HTMLButtonElement>(null);
  const origem = useRef<HTMLElement | null>(null);
  // Fechado de vez (Fechar, Esc): so ai a origem do foco e esquecida. Fechar para abrir o dialogo de
  // cancelar ou de reimprimir nao conta: o painel volta, e o foco tem de voltar ao sino.
  const fechadoDeVez = useRef(false);
  const corpo = useRef<HTMLDivElement>(null);

  return (
    <Dialog.Root
      open={aberto}
      onOpenChange={(abrir) => {
        if (!abrir) {
          fechadoDeVez.current = true;
          aoFechar();
        }
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className={dialogo.veu} />
        <Dialog.Content
          className={`${dialogo.conteudo} ${styles.conteudo}`}
          onKeyDown={(evento) => {
            if (rolarPeloTeclado(evento.key, corpo.current)) evento.preventDefault();
          }}
          onOpenAutoFocus={(evento) => {
            origem.current = origemDoDialogo(
              origem.current,
              document.activeElement instanceof HTMLElement ? document.activeElement : null,
            );
            evento.preventDefault();
            botaoFechar.current?.focus();
          }}
          onCloseAutoFocus={(evento) => {
            evento.preventDefault();
            if (origem.current?.isConnected) origem.current.focus();
            if (fechadoDeVez.current) {
              fechadoDeVez.current = false;
              origem.current = null;
            }
          }}
        >
          <Dialog.Title className={dialogo.titulo}>{t("avisos.titulo")}</Dialog.Title>
          <Dialog.Description asChild>
            <div ref={corpo} className={dialogo.corpo}>
              <p className={dialogo.nota}>{t("avisos.nota")}</p>
              {itens.length === 0 && <p className={dialogo.texto}>{t("avisos.vazio")}</p>}
              {itens.length > 0 && (
                <ol className={styles.lista}>
                  {entradasDoPainel(itens, cartoesVivos).map(({ aviso, acoes, dicaDe, semTrabalho }) => {
                    const { cartao } = aviso;
                    const Icone = iconeDoCartao(cartao);
                    // Frente e verso saem so pelo rodape: aqui a dica diz onde ir, sem botao de envio.
                    const dica = dicaDe === null ? null : dicaDoPainel(dicaDe);
                    const semAcaoNenhuma =
                      acoes !== null &&
                      !acoes.continuar &&
                      !acoes.reimprimir &&
                      acoes.cancelar === null;
                    return (
                      <li key={aviso.id} className={styles.item}>
                        <span className={styles.cabeca}>
                          <Icone
                            size={12}
                            className={styles.icone}
                            data-gira={Icone === ICONE_QUE_GIRA ? "" : undefined}
                            aria-hidden="true"
                          />
                          <time className={styles.hora} dateTime={new Date(aviso.quando).toISOString()}>
                            {horaDoAviso(aviso.quando)}
                          </time>
                          {aviso.documento !== null && aviso.documento !== "" && (
                            <span className={styles.documento} title={aviso.documento}>
                              {aviso.documento}
                            </span>
                          )}
                        </span>
                        <CartaoImpressao cartao={cartao} registro />
                        {semTrabalho && <p className={styles.semAcao}>{t("avisos.semTrabalho")}</p>}
                        {acoes !== null && (
                          <div className={styles.acoes}>
                            {acoes.continuar && (
                              <button
                                type="button"
                                className={CLASSE_DO_BOTAO}
                                onClick={() => aoComandarVivo(aviso, "continuar_esperando")}
                              >
                                {t("cartao.continuar")}
                              </button>
                            )}
                            {acoes.reimprimir && (
                              <button
                                type="button"
                                className={CLASSE_DO_BOTAO}
                                onClick={() => aoPedirReimprimirVivo(aviso)}
                              >
                                {t("cartao.reimprimir")}
                              </button>
                            )}
                            {acoes.cancelar !== null && (
                              <button
                                type="button"
                                className={CLASSE_DO_BOTAO}
                                disabled={acoes.cancelar.desabilitado}
                                onClick={() => aoPedirCancelarVivo(aviso)}
                              >
                                {t("cartao.cancelar")}
                              </button>
                            )}
                            {dica !== null && <p className={styles.semAcao}>{dica}</p>}
                            {dica === null && semAcaoNenhuma && (
                              <p className={styles.semAcao}>{t("avisos.semAcao")}</p>
                            )}
                          </div>
                        )}
                      </li>
                    );
                  })}
                </ol>
              )}
              {itens.length >= LIMITE_DE_AVISOS && (
                <p className={dialogo.nota}>{t("avisos.limite", { n: String(LIMITE_DE_AVISOS) })}</p>
              )}
            </div>
          </Dialog.Description>
          <div className={dialogo.rodape}>
            <Dialog.Close ref={botaoFechar} className={`${botoes.botao} ${botoes.primario}`}>
              {t("historico.fechar")}
            </Dialog.Close>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
