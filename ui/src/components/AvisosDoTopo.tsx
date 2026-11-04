import { Check, CircleAlert, Info, LoaderCircle, X } from "lucide-react";
import { useEffect, useRef, useState, type FocusEvent } from "react";
import {
  avisosNoTopo,
  esperaAteSair,
  frasesDeAcaoDoAviso,
  nomeDoAvisoNoTopo,
  tomDoAviso,
  type AvisoDoTrabalho,
  type PausaDoTopo,
} from "../avisos";
import { nomeDoCartao, textoDoEstadoDoCartao, type Cartao } from "../cartoes";
import { t } from "../i18n";
import styles from "./AvisosDoTopo.module.css";

export const ICONE_DO_TOM = {
  ativo: Check,
  espera: Info,
  problema: CircleAlert,
  parado: Info,
} as const;

export const ICONE_QUE_GIRA = LoaderCircle;

/** Preparando, enviando e cancelando andam: o icone diz isso, e nao um "feito". */
export function iconeDoCartao(cartao: Cartao): typeof Check {
  const { tipo } = cartao.estado;
  if (tipo === "preparando" || tipo === "imprimindo" || tipo === "cancelando") return LoaderCircle;
  return ICONE_DO_TOM[tomDoAviso(cartao)];
}

interface Props {
  itens: readonly AvisoDoTrabalho[];
  /** Com um dialogo aberto, a faixa sai da frente: o dialogo e o que a pessoa esta lendo. */
  oculto?: boolean;
  aoFechar: (id: number) => void;
}

/**
 * Faixa logo abaixo da barra de titulo. Nao ocupa altura no fluxo, para a lista e o documento nao
 * descerem quando um aviso aparece, e nunca pega o foco.
 */
export function AvisosDoTopo({ itens, oculto = false, aoFechar }: Props) {
  const [agora, setAgora] = useState(() => Date.now());
  // O ponteiro ou o foco sobre a faixa para o prazo dos avisos passageiros: quem le nao perde o texto.
  const [pausas, setPausas] = useState<readonly PausaDoTopo[]>([]);
  const [ponteiro, setPonteiro] = useState(false);
  const [foco, setFoco] = useState(false);
  const pausado = ponteiro || foco;
  const estavaPausado = useRef(false);

  useEffect(() => {
    if (pausado === estavaPausado.current) return;
    estavaPausado.current = pausado;
    const agoraExato = Date.now();
    setPausas((atuais) =>
      pausado
        ? [...atuais, { de: agoraExato, ate: null }]
        : atuais.map((pausa) => (pausa.ate === null ? { ...pausa, ate: agoraExato } : pausa)),
    );
    setAgora(agoraExato);
  }, [pausado]);

  // Sempre que a lista muda, o relogio do topo e lido de novo.
  useEffect(() => {
    setAgora(Date.now());
  }, [itens]);

  // Um aviso passageiro sai quando a espera acaba: acorda a faixa so nessa hora.
  useEffect(() => {
    const espera = esperaAteSair(itens, Date.now(), pausas);
    if (espera === null || pausado) return undefined;
    const prazo = window.setTimeout(() => setAgora(Date.now()), espera + 1);
    return () => window.clearTimeout(prazo);
  }, [itens, agora, pausas, pausado]);

  const visiveis = oculto ? [] : avisosNoTopo(itens, agora, pausas);
  const aoSairDoFoco = (evento: FocusEvent<HTMLDivElement>) => {
    if (!evento.currentTarget.contains(evento.relatedTarget as Node | null)) setFoco(false);
  };
  // A faixa e sempre a mesma regiao: so assim o leitor de tela anuncia o que chega depois.
  return (
    <div className={styles.faixa} aria-live="polite" aria-relevant="additions">
      <div
        className={styles.pilha}
        onMouseEnter={() => setPonteiro(true)}
        onMouseLeave={() => setPonteiro(false)}
        onFocus={() => setFoco(true)}
        onBlur={aoSairDoFoco}
      >
        {visiveis.map((aviso) => {
          const { cartao } = aviso;
          const tom = tomDoAviso(cartao);
          const Icone = iconeDoCartao(cartao);
          const nome = nomeDoAvisoNoTopo(aviso);
          const fila = nomeDoCartao(cartao);
          const documento = aviso.documento ?? "";
          const erro = tom === "problema";
          return (
            <div
              key={aviso.id}
              className={styles.aviso}
              role={erro ? "alert" : undefined}
              data-tom={tom}
            >
              <Icone
                size={14}
                className={styles.icone}
                data-gira={Icone === ICONE_QUE_GIRA ? "" : undefined}
                aria-hidden="true"
              />
              <div className={styles.texto}>
                <p className={styles.titulo}>
                  {documento !== "" && (
                    <>
                      <span className={styles.documento} title={documento}>
                        {documento}
                      </span>
                      <span className={styles.separador} aria-hidden="true">
                        ·
                      </span>
                    </>
                  )}
                  <span className={styles.fila} title={fila}>
                    {fila}
                  </span>
                  <span className={styles.estado}>{textoDoEstadoDoCartao(cartao)}</span>
                </p>
                {frasesDeAcaoDoAviso(cartao).map((frase, indice) => (
                  <p key={`${indice}-${frase}`} className={styles.frase}>
                    {frase}
                  </p>
                ))}
              </div>
              <button
                type="button"
                className={styles.fechar}
                aria-label={t("avisos.fechar", { nome })}
                title={t("avisos.fechar", { nome })}
                onClick={() => aoFechar(aviso.id)}
              >
                <X size={12} aria-hidden="true" />
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
