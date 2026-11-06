import * as Dialog from "@radix-ui/react-dialog";
import { CircleAlert, LoaderCircle, RefreshCw, Settings } from "lucide-react";
import { useEffect, useRef, useState, type RefObject } from "react";
import { deveRolarPeloTeclado, rolarPeloTeclado } from "../dialogo";
import { t, type Chave } from "../i18n";
import {
  aplicarOpcoesDaImpressora,
  codigoDaFalhaDasOpcoes,
  coresOferecidas,
  destinoDaFalha,
  escolhasIniciais,
  ESCOLHAS_VAZIAS,
  lerOpcoesDaImpressora,
  rotuloDaResolucao,
  temOpcoes,
  type CodigoFalhaDasOpcoes,
  type CorDaImpressora,
  type EscolhasDaImpressora,
  type OpcoesDaImpressora,
} from "../opcoes-impressora";
import { voltarAoPadraoDaFila } from "../preferencias";
import botoes from "./Botao.module.css";
import styles from "./OpcoesDaImpressoraDialog.module.css";

interface Props {
  /** `false` mantem o painel fora da tela, para a animacao de saida terminar. */
  aberto: boolean;
  /** A fila aberta; `null` fecha o painel. */
  fila: string | null;
  /** Nome que a lista mostra para a fila. */
  nome: string;
  /** A fila ja tem ajuste salvo: so entao "Usar padrao" aparece. */
  temAjusteSalvo: boolean;
  aoFechar: () => void;
  /** Fecha as opcoes e abre a janela do driver. */
  aoAbrirEngrenagem: () => void;
  /** O ajuste da fila mudou: a lateral rele o selo. */
  aoAplicar: () => void;
  /** A fila sumiu do Windows: a lista precisa ser lida de novo. */
  aoFilaSumiu: () => void;
}

export interface EstadoDoPainel {
  carregando: boolean;
  erro: string | null;
  opcoes: OpcoesDaImpressora | null;
  escolhas: EscolhasDaImpressora;
  aplicando: boolean;
  temAjusteSalvo: boolean;
}

interface PropsDoPainel {
  estado: EstadoDoPainel;
  corpoRef?: RefObject<HTMLDivElement | null>;
  cancelarRef?: RefObject<HTMLButtonElement | null>;
  aoMudar: (mudanca: Partial<EscolhasDaImpressora>) => void;
  aoAplicar: () => void;
  aoCancelar: () => void;
  aoUsarPadrao: () => void;
  aoAbrirEngrenagem: () => void;
  /** Relê a fila depois de uma falha de leitura. */
  aoTentarDeNovo: () => void;
}

const TEXTO_DA_FALHA: Record<CodigoFalhaDasOpcoes, Chave> = {
  fila_sumiu: "opcoes.erro.filaSumiu",
  opcao_recusada: "opcoes.erro.opcaoRecusada",
  nao_abriu: "opcoes.erro.ler",
  nao_gravou: "opcoes.erro.naoGravou",
  grande: "opcoes.erro.grande",
};

function textoDaFalha(codigo: CodigoFalhaDasOpcoes, nome: string): string {
  return t(TEXTO_DA_FALHA[codigo], { nome });
}

function textoDaCor(cor: CorDaImpressora): Chave {
  return cor === "colorida" ? "opcoes.colorida" : "opcoes.monocromatica";
}

/** O corpo e o rodape do painel, sem o dialogo: os estados sao testados sem abrir janela. */
export function PainelDasOpcoes({
  estado,
  corpoRef,
  cancelarRef,
  aoMudar,
  aoAplicar,
  aoCancelar,
  aoUsarPadrao,
  aoAbrirEngrenagem,
  aoTentarDeNovo,
}: PropsDoPainel) {
  const { carregando, erro, opcoes, escolhas, aplicando, temAjusteSalvo } = estado;
  const cores = opcoes === null ? [] : coresOferecidas(opcoes.cor);

  let opcoesDoCorpo = null;
  if (opcoes !== null && temOpcoes(opcoes)) {
    opcoesDoCorpo = (
      <>
        {opcoes.bandejas.length > 0 && (
          <label className={styles.campo}>
            <span className={styles.rotulo}>{t("opcoes.bandeja")}</span>
            <select
              className={styles.seletor}
              value={escolhas.bandeja === null ? "" : String(escolhas.bandeja)}
              onChange={(evento) => {
                const valor = Number(evento.target.value);
                aoMudar({
                  bandeja:
                    evento.target.value === "" || !Number.isFinite(valor) ? null : valor,
                });
              }}
            >
              <option value="">{t("opcoes.padrao")}</option>
              {opcoes.bandejas.map((bandeja) => (
                <option key={bandeja.valor} value={bandeja.valor}>
                  {bandeja.nome}
                </option>
              ))}
            </select>
          </label>
        )}
        {opcoes.resolucoes.length > 0 && (
          <label className={styles.campo}>
            <span className={styles.rotulo}>{t("opcoes.resolucao")}</span>
            <select
              className={styles.seletor}
              value={
                escolhas.resolucao === null
                  ? ""
                  : `${escolhas.resolucao.x}x${escolhas.resolucao.y}`
              }
              onChange={(evento) => {
                const escolhida = opcoes.resolucoes.find(
                  (resolucao) => `${resolucao.x}x${resolucao.y}` === evento.target.value,
                );
                aoMudar({ resolucao: escolhida ?? null });
              }}
            >
              <option value="">{t("opcoes.padrao")}</option>
              {opcoes.resolucoes.map((resolucao) => (
                <option
                  key={`${resolucao.x}x${resolucao.y}`}
                  value={`${resolucao.x}x${resolucao.y}`}
                >
                  {rotuloDaResolucao(resolucao)}
                </option>
              ))}
            </select>
          </label>
        )}
        {cores.length > 0 && (
          <fieldset className={styles.grupo}>
            <legend className={styles.rotulo}>{t("opcoes.cor")}</legend>
            <label className={styles.opcao}>
              <input
                type="radio"
                name="opcoes-cor"
                checked={escolhas.cor === null}
                onChange={() => aoMudar({ cor: null })}
              />
              <span>{t("opcoes.padrao")}</span>
            </label>
            {cores.map((cor) => (
              <label key={cor} className={styles.opcao}>
                <input
                  type="radio"
                  name="opcoes-cor"
                  checked={escolhas.cor === cor}
                  onChange={() => aoMudar({ cor })}
                />
                <span>{t(textoDaCor(cor))}</span>
              </label>
            ))}
          </fieldset>
        )}
      </>
    );
  } else if (opcoes !== null) {
    opcoesDoCorpo = (
      <div className={styles.vazio}>
        <p className={styles.aviso}>{t("opcoes.vazio")}</p>
        <button
          type="button"
          className={`${botoes.botao} ${botoes.primario}`}
          onClick={aoAbrirEngrenagem}
        >
          <Settings size={14} aria-hidden="true" />
          {t("opcoes.abrirEngrenagem")}
        </button>
      </div>
    );
  }

  return (
    <>
      <div ref={corpoRef} className={styles.corpo}>
        {erro !== null && (
          <>
            <p className={styles.erro} role="alert">
              <CircleAlert size={14} aria-hidden="true" />
              <span>{erro}</span>
            </p>
            {opcoes === null && !carregando && (
              <button
                type="button"
                className={`${botoes.botao} ${botoes.secundario} ${styles.tentarDeNovo}`}
                onClick={aoTentarDeNovo}
              >
                <RefreshCw size={14} aria-hidden="true" />
                {t("opcoes.tentarDeNovo")}
              </button>
            )}
          </>
        )}
        {carregando ? <p className={styles.aviso}>{t("opcoes.carregando")}</p> : opcoesDoCorpo}
      </div>
      <div className={styles.rodape}>
        {temAjusteSalvo && opcoes !== null && (
          <button
            type="button"
            className={`${botoes.botao} ${botoes.secundario} ${styles.usarPadrao}`}
            aria-busy={aplicando}
            onClick={aoUsarPadrao}
          >
            {t("opcoes.usarPadrao")}
          </button>
        )}
        <button
          ref={cancelarRef}
          type="button"
          className={`${botoes.botao} ${botoes.secundario}`}
          onClick={aoCancelar}
        >
          {t("opcoes.cancelar")}
        </button>
        {opcoes !== null && temOpcoes(opcoes) && (
          <button
            type="button"
            className={`${botoes.botao} ${botoes.primario}`}
            aria-busy={aplicando}
            onClick={aoAplicar}
          >
            {aplicando && (
              <LoaderCircle size={14} className={styles.girando} aria-hidden="true" />
            )}
            {t(aplicando ? "opcoes.aplicando" : "opcoes.aplicar")}
          </button>
        )}
      </div>
    </>
  );
}

/**
 * Painel com o que o driver da fila declara. Só bandeja, resolução e cor entram aqui: papel,
 * face, cópias, escala, orientação e uma página por folha continuam sendo decisão do envio.
 */
export function OpcoesDaImpressoraDialog({
  aberto,
  fila,
  nome,
  temAjusteSalvo,
  aoFechar,
  aoAbrirEngrenagem,
  aoAplicar,
  aoFilaSumiu,
}: Props) {
  // O nome some durante a animacao de saida: o ultimo conteudo fica ate o painel fechar.
  const ultimo = useRef<{ fila: string; nome: string } | null>(null);
  if (fila !== null && nome !== "") ultimo.current = { fila, nome };
  const visivel = ultimo.current;

  const [opcoes, setOpcoes] = useState<OpcoesDaImpressora | null>(null);
  const [escolhas, setEscolhas] = useState<EscolhasDaImpressora>(ESCOLHAS_VAZIAS);
  const [carregando, setCarregando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  const [aplicando, setAplicando] = useState(false);
  const [tentativa, setTentativa] = useState(0);
  const filaSumiu = useRef(aoFilaSumiu);
  useEffect(() => {
    filaSumiu.current = aoFilaSumiu;
  });

  const origem = useRef<HTMLElement | null>(null);
  const corpo = useRef<HTMLDivElement>(null);
  const refCancelar = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!aberto || fila === null) return undefined;
    let vivo = true;
    const nomeDaFila = visivel?.nome ?? fila;
    setCarregando(true);
    setOpcoes(null);
    setErro(null);
    setAplicando(false);
    lerOpcoesDaImpressora(fila)
      .then((lidas) => {
        if (!vivo) return;
        setOpcoes(lidas);
        setEscolhas(escolhasIniciais(lidas));
        setCarregando(false);
      })
      .catch((falha: unknown) => {
        if (!vivo) return;
        const codigo = codigoDaFalhaDasOpcoes(falha);
        setErro(textoDaFalha(codigo, nomeDaFila));
        if (destinoDaFalha(codigo) === "fechar") {
          filaSumiu.current();
          aoFechar();
        }
        setCarregando(false);
      });
    return () => {
      vivo = false;
    };
  }, [aberto, fila, tentativa]);

  // O foco entra no primeiro controle que o painel mostrar; num grupo de radios, no marcado.
  useEffect(() => {
    if (!aberto || carregando) return;
    const controles = corpo.current?.querySelectorAll<HTMLElement>(
      'select, input[type="radio"], button',
    );
    const primeiro = controles?.[0];
    const alvo = primeiro?.matches('input[type="radio"]')
      ? (corpo.current?.querySelector<HTMLElement>('input[type="radio"]:checked') ?? primeiro)
      : (primeiro ?? refCancelar.current);
    alvo?.focus();
  }, [aberto, carregando, opcoes]);

  const mudar = (mudanca: Partial<EscolhasDaImpressora>) =>
    setEscolhas((atuais) => ({ ...atuais, ...mudanca }));

  const aplicar = () => {
    if (fila === null || aplicando) return;
    setAplicando(true);
    setErro(null);
    aplicarOpcoesDaImpressora(fila, escolhas)
      .then(() => {
        aoAplicar();
        aoFechar();
      })
      .catch((falha: unknown) => {
        const codigo = codigoDaFalhaDasOpcoes(falha);
        setErro(textoDaFalha(codigo, nome));
        if (destinoDaFalha(codigo) === "fechar") {
          filaSumiu.current();
          aoFechar();
        }
      })
      .finally(() => setAplicando(false));
  };

  const usarPadrao = () => {
    if (fila === null || aplicando) return;
    setAplicando(true);
    setErro(null);
    voltarAoPadraoDaFila(fila)
      .then(() => {
        aoAplicar();
        aoFechar();
      })
      .catch((falha: unknown) => {
        const codigo = codigoDaFalhaDasOpcoes(falha);
        setErro(textoDaFalha(codigo, nome));
        if (destinoDaFalha(codigo) === "fechar") {
          filaSumiu.current();
          aoFechar();
        }
      })
      .finally(() => setAplicando(false));
  };

  const tentarDeNovo = () => setTentativa((atual) => atual + 1);

  return (
    <Dialog.Root
      open={aberto}
      onOpenChange={(estaAberto) => {
        if (!estaAberto) aoFechar();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className={styles.veu} />
        <Dialog.Content
          className={styles.conteudo}
          onKeyDown={(evento) => {
            const alvo = evento.target instanceof Element ? evento.target : null;
            if (
              deveRolarPeloTeclado(alvo) &&
              rolarPeloTeclado(evento.key, corpo.current)
            ) {
              evento.preventDefault();
            }
          }}
          onOpenAutoFocus={(evento) => {
            origem.current =
              document.activeElement instanceof HTMLElement ? document.activeElement : null;
            evento.preventDefault();
          }}
          onCloseAutoFocus={(evento) => {
            // O gatilho e a chave de boca, que continua na lista depois do painel fechar.
            evento.preventDefault();
            if (origem.current?.isConnected) origem.current.focus();
          }}
        >
          <Dialog.Title className={styles.titulo}>
            {t("opcoes.tituloComNome", { nome: visivel?.nome ?? nome })}
          </Dialog.Title>
          <Dialog.Description className={styles.descricao}>
            {t("opcoes.descricao")}
          </Dialog.Description>
          <PainelDasOpcoes
            estado={{ carregando, erro, opcoes, escolhas, aplicando, temAjusteSalvo }}
            corpoRef={corpo}
            cancelarRef={refCancelar}
            aoMudar={mudar}
            aoAplicar={aplicar}
            aoCancelar={aoFechar}
            aoUsarPadrao={usarPadrao}
            aoAbrirEngrenagem={aoAbrirEngrenagem}
            aoTentarDeNovo={tentarDeNovo}
          />
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
