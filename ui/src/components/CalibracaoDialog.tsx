import * as Dialog from "@radix-ui/react-dialog";
import { CircleAlert, LoaderCircle } from "lucide-react";
import { useEffect, useId, useMemo, useReducer, useRef, useState, type ReactNode, type Ref } from "react";
import { assinar } from "../backend";
import {
  avaliarRodada,
  conferirAutomatica,
  encerrarCalibracao,
  gravarCalibracao,
  ouvirCartoesDaCalibracao,
  perfilInicial,
  recolocarDoPerfil,
  saidaSimulada,
} from "../calibracao-backend";
import {
  chaveDaConclusao,
  eUltimaPergunta,
  estadoInicial,
  filaNaoServe,
  FOLHAS_POR_RODADA,
  INSTRUCAO_DA_PERGUNTA,
  anuncioDoPasso,
  chavesDoAnuncio,
  leituraFinal,
  numeroDoPasso,
  OPCOES,
  PASSOS_TOTAIS,
  pedeConfirmacaoAoDesistir,
  perguntasDaRodada,
  proximaRodada,
  reduzir,
  resumoDasRespostas,
  respostasParaAvaliar,
  RODADAS_MAXIMAS,
  temPassadaNaFila,
  TEXTO_DA_MUDANCA,
  TEXTO_DA_PERGUNTA,
  type Acao,
  type Estado,
  type ModeloInicial,
  type PapelDoTeste,
  type Pergunta,
  type Recusa,
  type SaidaSimulada,
} from "../calibracao";
import { criarTrava, enviarVerso, iniciarRodada, type Ocupacao, type Trava } from "../calibracao-envio";
import { desenhoDoGesto } from "../cartoes-gesto";
import { frasesDoGesto, textoDoMotivo, type Recolocar } from "../cartoes";
import { rolarPeloTeclado } from "../dialogo";
import { t, type Chave } from "../i18n";
import { nomeDeExibicao, type Impressora } from "../impressoras";
import botoes from "./Botao.module.css";
import { CalibracaoFigura } from "./CalibracaoFigura";
import styles from "./CalibracaoDialog.module.css";
import base from "./ConfirmDialog.module.css";
import { FiguraGesto } from "./FiguraGesto";

interface Props {
  /** A impressora em calibracao; `null` mantem o dialogo fechado. */
  impressora: Impressora | null;
  /** `gravou` diz que o perfil mudou, e a lista precisa ser lida de novo. */
  aoFechar: (gravou: boolean) => void;
}

const PRIMARIO = `${botoes.botao} ${botoes.primario}`;
const SECUNDARIO = `${botoes.botao} ${botoes.secundario}`;

/** Abre um estado novo a cada calibracao: nada da anterior sobra. */
export function CalibracaoDialog({ impressora, aoFechar }: Props) {
  const gatilho = useRef<HTMLElement | null>(null);
  return (
    <Dialog.Root
      open={impressora !== null}
      onOpenChange={(aberto) => {
        // O fechamento com Esc ou pelo veu e tratado dentro, onde se sabe o que esta na fila.
        if (!aberto) document.dispatchEvent(new CustomEvent(EVENTO_DE_DESISTIR));
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className={base.veu} />
        <Dialog.Content
          className={`${base.conteudo} ${styles.conteudo}`}
          aria-describedby={undefined}
          onOpenAutoFocus={() => {
            gatilho.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
          }}
          onCloseAutoFocus={(evento) => {
            evento.preventDefault();
            gatilho.current?.focus();
          }}
        >
          {impressora !== null && (
            <Conteudo key={impressora.id} impressora={impressora} aoFechar={aoFechar} />
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

const EVENTO_DE_DESISTIR = "plots-calibracao-desistir";

interface GanchoDaCalibracao {
  forcar(estado: string): void;
}

const TEXTO_DA_RECUSA = {
  ocupada: "calibracao.ocupada",
  falha_envio: "calibracao.falhaEnvio",
  falha_perfil: "calibracao.falhaPerfil",
  falha_ao_gravar: "calibracao.gravarFalhou",
  falha_julgamento: "calibracao.julgamentoFalhou",
} as const satisfies Record<Recusa, Chave>;

interface ContextoDeSaida {
  estado: () => Estado;
  id: string;
  ehSimulada: boolean;
  despachar: (acao: Acao) => void;
  aoFechar: (gravou: boolean) => void;
  confirmando: () => boolean;
  confirmar: (aberta: boolean) => void;
  trava: Trava;
}

/**
 * As duas saidas da calibracao: gravar, e desistir, que nao grava nada em nenhum passo. Enquanto uma
 * chamada corre, Desistir e Esc nao fazem nada: a chamada pode ainda gastar ou gravar algo.
 */
export function criarSaidas(contexto: ContextoDeSaida) {
  const desistirDeVez = () => {
    if (contexto.trava.atual() !== null) return;
    const estado = contexto.estado();
    void encerrarCalibracao(contexto.id, temPassadaNaFila(estado)).catch(() => {});
    contexto.aoFechar(estado.passo === "gravada");
  };
  return {
    desistirDeVez,
    /** O Desistir e o Esc passam por aqui. Na confirmacao aberta, o Esc so volta. */
    pedirDesistir: () => {
      if (contexto.trava.atual() !== null) return;
      if (contexto.confirmando()) contexto.confirmar(false);
      else if (pedeConfirmacaoAoDesistir(contexto.estado(), contexto.ehSimulada)) contexto.confirmar(true);
      else desistirDeVez();
    },
    gravar: () => {
      if (contexto.trava.atual() !== null) return;
      const estado = contexto.estado();
      const perfil = estado.perfil;
      const saiu = estado.saiuParaCima;
      if (contexto.ehSimulada || perfil === null || saiu === null) {
        contexto.aoFechar(false);
        return;
      }
      const leitura = leituraFinal(estado);
      contexto.trava.executar("gravando", () => {
        contexto.despachar({ tipo: "chamada_iniciada" });
        return gravarCalibracao(contexto.id, perfil, saiu, leitura)
          .then(() => contexto.despachar({ tipo: "gravada" }))
          .catch(() => contexto.despachar({ tipo: "falha_ao_gravar" }));
      });
    },
  };
}

export function Conteudo({ impressora, aoFechar }: { impressora: Impressora; aoFechar: (gravou: boolean) => void }) {
  const [estado, despachar] = useReducer(reduzir, impressora, estadoInicial);
  const [recolocar, setRecolocar] = useState<Recolocar | null>(null);
  const [simulada, setSimulada] = useState<SaidaSimulada | null>(null);
  const [confirmando, setConfirmando] = useState(false);
  const [ocupacao, setOcupacao] = useState<Ocupacao>(null);
  // O foco de quem disparou a acao volta a ele quando a trava solta, se nada mais o levou.
  const disparador = useRef<HTMLElement | null>(null);
  const trava = useMemo(
    () =>
      criarTrava((ocupacao) => {
        if (ocupacao !== null) disparador.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        setOcupacao(ocupacao);
      }),
    [],
  );
  const botaoVoltar = useRef<HTMLButtonElement>(null);
  // O andamento e a recusa ficam dentro do corpo rolavel: precisam estar a vista quando aparecem.
  const avisoRef = useRef<HTMLParagraphElement>(null);
  const corpo = useRef<HTMLDivElement>(null);
  const atual = useRef(estado);
  atual.current = estado;
  const confirmandoAgora = useRef(confirmando);
  confirmandoAgora.current = confirmando;
  const id = impressora.id;
  const ehSimulada = impressora.tipo === "simulada";
  const nome = nomeDeExibicao(impressora);

  useEffect(() => assinar(ouvirCartoesDaCalibracao((cartao) => {
    if (cartao.impressora === id) despachar({ tipo: "cartao", cartao });
  })), [id]);

  const saidas = useMemo(
    () =>
      criarSaidas({
        estado: () => atual.current,
        id,
        ehSimulada,
        despachar,
        aoFechar,
        confirmando: () => confirmandoAgora.current,
        confirmar: setConfirmando,
        trava,
      }),
    [aoFechar, ehSimulada, id, trava],
  );

  useEffect(() => {
    document.addEventListener(EVENTO_DE_DESISTIR, saidas.pedirDesistir);
    return () => document.removeEventListener(EVENTO_DE_DESISTIR, saidas.pedirDesistir);
  }, [saidas]);

  // O que muda de passo leva o foco ao corpo: quem usa o teclado continua de onde a tela mudou.
  // Na confirmacao de desistir, o foco vai para Voltar: Enter ali nao desiste por engano.
  useEffect(() => {
    if (confirmando) botaoVoltar.current?.focus({ preventScroll: true });
    else corpo.current?.focus({ preventScroll: true });
    corpo.current?.scrollTo?.({ top: 0 });
  }, [estado.passo, estado.pergunta, confirmando]);

  useEffect(() => {
    if (ocupacao !== null) return;
    const ativo = document.activeElement;
    // O foco cai no corpo ou na raiz do dialogo quando o botao desabilitado o perde: so ai ele volta.
    const perdido = !(ativo instanceof HTMLElement) || ativo === document.body || ativo.getAttribute("role") === "dialog";
    if (!perdido) return;
    const alvo = disparador.current;
    if (alvo?.isConnected && !(alvo as HTMLButtonElement).disabled) alvo.focus({ preventScroll: true });
  }, [ocupacao]);

  // Depois do efeito que leva o passo ao topo: a linha de andamento ou o alerta nao podem ficar abaixo da dobra.
  useEffect(() => {
    if (ocupacao === null && estado.recusa === null) return;
    avisoRef.current?.scrollIntoView?.({ block: "nearest" });
  }, [ocupacao, estado.recusa]);

  useEffect(() => {
    if (estado.perfil === null || estado.saiuParaCima === null) return;
    let valido = true;
    recolocarDoPerfil(estado.perfil, estado.saiuParaCima)
      .then((lido) => {
        if (valido) setRecolocar(lido);
      })
      .catch(() => {
        if (valido) setRecolocar(null);
      });
    return () => {
      valido = false;
    };
  }, [estado.perfil, estado.saiuParaCima]);

  // Sem papel na mao, a simulada conta o que saiu, para a pessoa poder responder.
  const mostraSimulada = ehSimulada && (estado.passo === "saida" || estado.passo === "perguntas");
  useEffect(() => {
    if (!mostraSimulada || estado.perfil === null) return;
    let valido = true;
    saidaSimulada(id, estado.perfil)
      .then((lida) => {
        if (valido) setSimulada(lida);
      })
      .catch(() => {
        if (valido) setSimulada(null);
      });
    return () => {
      valido = false;
    };
  }, [id, mostraSimulada, estado.perfil]);

  // Um clique manda o teste uma vez so: a trava vale ainda antes de o perfil ficar pronto.
  const imprimirFrente = () => {
    if (trava.atual() !== null) return;
    const perfil = () =>
      estado.perfil !== null && estado.rodada > 1
        ? Promise.resolve(estado.perfil)
        : perfilInicial(estado.modelo, impressora.driverInverte === true);
    iniciarRodada({ trava, id, papel: estado.papel, rodada: estado.rodada, perfil, despachar });
  };

  // Um clique manda o verso uma vez so: enquanto o comando corre, Desistir e Esc tambem esperam.
  const imprimirVerso = () => {
    if (trava.atual() !== null) return;
    despachar({ tipo: "verso_pedido" });
    enviarVerso({ trava, id, despachar });
  };

  // So no modo e2e: forca os estados de chamada e de recusa para a captura de verificacao.
  useEffect(() => {
    if (import.meta.env.MODE !== "e2e") return;
    const alvo = window as unknown as { __plotsE2eCalibracao?: GanchoDaCalibracao };
    let soltar: (() => void) | null = null;
    alvo.__plotsE2eCalibracao = {
      forcar(estado) {
        if (estado === "liberar") {
          soltar?.();
          soltar = null;
        } else if (estado === "enviando" || estado === "gravando") {
          despachar({ tipo: "chamada_iniciada" });
          trava.executar(estado, () => new Promise<void>((ok) => {
            soltar = ok;
          }));
        } else if (estado === "falha_julgamento") {
          despachar({ tipo: "falha_julgamento" });
        } else if (estado === "verso_falhou") {
          despachar({ tipo: "verso_falhou" });
        } else {
          despachar({ tipo: "recusada", recusa: estado as Recusa });
        }
      },
    };
    return () => {
      soltar?.();
      delete alvo.__plotsE2eCalibracao;
    };
  }, [trava]);

  // Um clique manda a avaliacao uma vez so: a trava vale ate o resultado chegar.
  const julgar = () => {
    const respostas = respostasParaAvaliar(estado);
    const perfil = estado.perfil;
    if (respostas === null || perfil === null) return;
    trava.executar("conferindo", () => {
      despachar({ tipo: "chamada_iniciada" });
      const julgamento = estado.automatica
        ? conferirAutomatica(respostas.parCerto, respostas.setasIguais, respostas.leitura).then((conclusao) =>
            despachar({ tipo: "conferida", conclusao }),
          )
        : avaliarRodada(perfil, respostas).then((avaliacao) => despachar({ tipo: "avaliada", avaliacao }));
      return julgamento.catch(() => despachar({ tipo: "falha_julgamento" }));
    });
  };

  const continuar = () => {
    if (eUltimaPergunta(estado)) julgar();
    else despachar({ tipo: "continuar" });
  };

  const imprimirDeNovo = () => {
    const proxima = proximaRodada(estado);
    if (proxima === null) return;
    iniciarRodada({
      trava,
      id,
      papel: estado.papel,
      rodada: proxima.rodada,
      perfil: () => Promise.resolve(proxima.perfil),
      despachar,
    });
  };

  const refazer = () => {
    void encerrarCalibracao(id, true).catch(() => {});
    despachar({ tipo: "refazer" });
  };

  const corrigir = () => despachar({ tipo: "corrigir" });
  const fechar = () => aoFechar(estado.passo === "gravada");
  const pergunta = estado.pergunta;
  const respondida = pergunta !== null && estado.respostas[pergunta] !== undefined;
  const naoServe = filaNaoServe(estado);
  const motivo = naoServe
    ? t("calibracao.pedeArquivo")
    : estado.cartao?.estado.tipo === "erro" || estado.cartao?.estado.tipo === "aviso"
      ? textoDoMotivo(estado.cartao.estado.motivo)
      : null;
  const comErro = estado.cartao?.estado.tipo === "erro";
  const rodadasDoProgresso = estado.automatica ? 1 : RODADAS_MAXIMAS;
  const progresso = t("calibracao.progresso", {
    passo: String(numeroDoPasso(estado.passo)),
    passos: String(PASSOS_TOTAIS),
    rodada: String(estado.rodada),
    rodadas: String(rodadasDoProgresso),
  });
  const textos = chavesDoAnuncio(estado, confirmando).map((chave) => t(chave, { rodadas: String(RODADAS_MAXIMAS) }));
  const anuncio = anuncioDoPasso(progresso, textos, naoServe);

  return (
    <>
      <Dialog.Title className={base.titulo}>{t("calibracao.titulo", { nome })}</Dialog.Title>
      {!naoServe && <p className={styles.progresso}>{progresso}</p>}
      <p className={styles.anuncio} aria-live="polite" aria-atomic="true">
        {anuncio}
      </p>
      <div
        ref={corpo}
        tabIndex={-1}
        role="group"
        aria-label={t("calibracao.corpo")}
        className={`${base.corpo} ${styles.corpo}`}
        onKeyDown={(evento) => {
          // Nas respostas, as setas trocam a escolha; nos outros lugares, rolam o corpo.
          if (evento.target instanceof HTMLInputElement || evento.target instanceof HTMLSelectElement) return;
          if (rolarPeloTeclado(evento.key, corpo.current)) evento.preventDefault();
        }}
      >
        {confirmando ? (
          <>
            <p className={styles.pergunta}>{t("calibracao.desistir.pergunta")}</p>
            <Paragrafo>{t("calibracao.desistir.explicacao")}</Paragrafo>
          </>
        ) : (
          <CorpoDoPasso
            estado={estado}
            nome={nome}
            ehSimulada={ehSimulada}
            recolocar={recolocar}
            aoEscolherModelo={(modelo) => despachar({ tipo: "modelo", modelo })}
            aoEscolherPapel={(papel) => despachar({ tipo: "papel", papel })}
            aoResponder={(p, valor) => despachar({ tipo: "responder", pergunta: p, valor })}
            aoConfirmarResposta={() => {
              if (respondida) continuar();
            }}
          />
        )}
        {motivo !== null && (
          <p className={styles.alerta} role="alert">
            <CircleAlert size={14} aria-hidden="true" />
            <span>{motivo}</span>
          </p>
        )}
        <LinhaDeEnvio ocupacao={ocupacao} referencia={avisoRef} />
        {ocupacao === null && estado.recusa !== null && !confirmando && (
          <p ref={avisoRef} className={styles.alerta} role="alert">
            <CircleAlert size={14} aria-hidden="true" />
            <span>{t(TEXTO_DA_RECUSA[estado.recusa])}</span>
          </p>
        )}
        {mostraSimulada && simulada !== null && <SaidaDaSimulada saida={simulada} passo={estado.passo} />}
      </div>
      <div className={base.rodape}>
        <Rodape
          estado={estado}
          ehSimulada={ehSimulada}
          comErro={comErro}
          naoServe={naoServe}
          confirmando={confirmando}
          respondida={respondida}
          aoDesistir={saidas.pedirDesistir}
          aoDesistirDeVez={saidas.desistirDeVez}
          aoFecharConfirmacao={() => setConfirmando(false)}
          aoFechar={fechar}
          aoVoltar={() => despachar({ tipo: "voltar" })}
          aoContinuar={continuar}
          aoSeguir={() => despachar({ tipo: "seguir_para_confirmar" })}
          aoImprimirFrente={imprimirFrente}
          aoImprimirVerso={imprimirVerso}
          aoImprimirDeNovo={imprimirDeNovo}
          aoRefazer={refazer}
          aoGravar={saidas.gravar}
          ocupado={ocupacao !== null}
          aoCorrigir={corrigir}
          refVoltar={botaoVoltar}
        />
      </div>
    </>
  );
}

interface PropsDoCorpo {
  estado: Estado;
  nome: string;
  ehSimulada: boolean;
  recolocar: Recolocar | null;
  aoEscolherModelo: (modelo: ModeloInicial) => void;
  aoEscolherPapel: (papel: PapelDoTeste) => void;
  aoResponder: (pergunta: Pergunta, valor: string) => void;
  aoConfirmarResposta: () => void;
}

function Paragrafo({ children }: { children: ReactNode }) {
  return <p className={base.texto}>{children}</p>;
}

function PapelGasto() {
  return (
    <Paragrafo>
      {t("calibracao.papelGasto", {
        folhasPorRodada: String(FOLHAS_POR_RODADA),
        rodadas: String(RODADAS_MAXIMAS),
        folhas: String(FOLHAS_POR_RODADA * RODADAS_MAXIMAS),
      })}
    </Paragrafo>
  );
}

export function CorpoDoPasso({
  estado,
  nome,
  ehSimulada,
  recolocar,
  aoEscolherModelo,
  aoEscolherPapel,
  aoResponder,
  aoConfirmarResposta,
}: PropsDoCorpo) {
  const pagina = t(estado.papel === "carta" ? "calibracao.papel.carta" : "calibracao.papel.a4");
  switch (estado.passo) {
    case "preparar":
      return (
        <>
          <Paragrafo>
            {t(
              estado.automatica
                ? "calibracao.preparar.automatica"
                : ehSimulada
                  ? "calibracao.preparar.simulada"
                  : "calibracao.preparar.texto",
            )}
          </Paragrafo>
          {!estado.automatica && !ehSimulada && <PapelGasto />}
          <div className={styles.campos}>
            {!estado.automatica && (
              <label className={styles.campo}>
                {t("calibracao.preparar.modelo")}
                <select
                  value={estado.modelo}
                  onChange={(evento) => aoEscolherModelo(evento.target.value === "laser" ? "laser" : "jato")}
                >
                  <option value="jato">{t("calibracao.modelo.jato")}</option>
                  <option value="laser">{t("calibracao.modelo.laser")}</option>
                </select>
              </label>
            )}
            <label className={styles.campo}>
              {t("calibracao.preparar.papel")}
              <select
                value={estado.papel}
                onChange={(evento) => aoEscolherPapel(evento.target.value === "carta" ? "carta" : "a4")}
              >
                <option value="a4">{t("calibracao.papel.a4")}</option>
                <option value="carta">{t("calibracao.papel.carta")}</option>
              </select>
            </label>
          </div>
        </>
      );
    case "confirmar":
      return (
        <Paragrafo>
          {t(ehSimulada ? "calibracao.confirmar.simulada" : "calibracao.confirmar.texto", { nome, papel: pagina })}
        </Paragrafo>
      );
    case "frente":
    case "verso":
      // Com erro, nada esta andando: quem fala e o alerta.
      if (estado.cartao?.estado.tipo === "erro") return null;
      return (
        <Andamento
          texto={t(
            estado.passo === "verso"
              ? "calibracao.imprimindo.verso"
              : estado.automatica
                ? "calibracao.imprimindo.duasFaces"
                : "calibracao.imprimindo.frente",
          )}
        />
      );
    case "saida":
    case "perguntas":
      return estado.pergunta === null ? null : (
        <Respostas
          pergunta={estado.pergunta}
          valor={estado.respostas[estado.pergunta]}
          aoResponder={aoResponder}
          aoConfirmar={aoConfirmarResposta}
        />
      );
    case "recolocar":
      return <Recolocacao recolocar={recolocar} />;
    case "ajustar":
      return (
        <>
          <p className={styles.pergunta}>{t("calibracao.ajustar.titulo")}</p>
          <ul className={styles.lista}>
            {estado.mudancas.map((mudanca) => {
              const chave = TEXTO_DA_MUDANCA.get(mudanca);
              return chave === undefined ? null : <li key={mudanca}>{t(chave)}</li>;
            })}
          </ul>
          <Paragrafo>{t(ehSimulada ? "calibracao.ajustar.simulada" : "calibracao.ajustar.nota")}</Paragrafo>
        </>
      );
    case "certo": {
      const resumo = resumoDasRespostas(estado);
      return (
        <>
          <p className={styles.pergunta}>{t("calibracao.certo.titulo")}</p>
          <Paragrafo>{t(ehSimulada ? "calibracao.certo.simulada" : "calibracao.certo.gravar")}</Paragrafo>
          <div className={styles.grupo}>
            <p className={styles.grupoTitulo}>
              {t(ehSimulada ? "calibracao.certo.comEstePerfil" : "calibracao.certo.depoisDeGravar")}
            </p>
            <ul className={styles.lista}>
              {recolocar !== null &&
                frasesDoGesto(recolocar).map((frase) => (
                  <li key={frase}>{frase}</li>
                ))}
              <li>
                {t(leituraFinal(estado) === "virando" ? "calibracao.certo.lerVirando" : "calibracao.certo.lerSemVirar")}
              </li>
            </ul>
          </div>
          {resumo.length > 0 && (
            <div className={styles.grupo}>
              <p className={styles.grupoTitulo}>{t("calibracao.certo.respostas")}</p>
              <ul className={styles.lista}>
                {resumo.map((item) => (
                  <li key={item.rotulo}>
                    <span className={styles.resumoRotulo}>{t(item.rotulo)} </span>
                    <span className={styles.resumoValor}>{t(item.valor)}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </>
      );
    }
    case "gravada":
      return <p className={styles.pergunta}>{t("calibracao.gravada")}</p>;
    case "nao_encaixou":
      return (
        <>
          <p className={styles.pergunta}>{t("calibracao.naoEncaixou.titulo", { rodadas: String(RODADAS_MAXIMAS) })}</p>
          <Paragrafo>{t("calibracao.naoEncaixou.texto")}</Paragrafo>
        </>
      );
    case "conferida":
      return (
        <>
          <p className={styles.pergunta}>{t(chaveDaConclusao(estado.conclusao))}</p>
          <Paragrafo>{t("calibracao.conferida.nada")}</Paragrafo>
        </>
      );
  }
}

/** Enquanto a chamada corre, no lugar do alerta: diz o que a tela faz, e nao so que esta parada. */
export function LinhaDeEnvio({ ocupacao, referencia }: { ocupacao: Ocupacao; referencia?: Ref<HTMLParagraphElement> }) {
  if (ocupacao !== "enviando" && ocupacao !== "gravando") return null;
  return (
    <p ref={referencia} className={styles.andamentoLinha} role="status">
      <LoaderCircle size={14} className={styles.girando} aria-hidden="true" />
      <span>{t(ocupacao === "enviando" ? "calibracao.ocupacao.enviando" : "calibracao.ocupacao.gravando")}</span>
    </p>
  );
}

function Andamento({ texto }: { texto: string }) {
  return (
    <p className={styles.andamento} aria-live="polite">
      <LoaderCircle size={14} className={styles.girando} aria-hidden="true" />
      <span>{texto}</span>
    </p>
  );
}

function Recolocacao({ recolocar }: { recolocar: Recolocar | null }) {
  if (recolocar === null) return null;
  const desenho = desenhoDoGesto(recolocar);
  return (
    <>
      {frasesDoGesto(recolocar).map((frase) => (
        <p key={frase} className={base.texto}>
          {frase}
        </p>
      ))}
      {desenho !== null && (
        <div className={styles.gesto}>
          <FiguraGesto desenho={desenho} />
        </div>
      )}
      <Paragrafo>{t("calibracao.recolocar.repare")}</Paragrafo>
    </>
  );
}

export function Respostas({
  pergunta,
  valor,
  aoResponder,
  aoConfirmar,
}: {
  pergunta: Pergunta;
  valor: string | undefined;
  aoResponder: (pergunta: Pergunta, valor: string) => void;
  aoConfirmar: () => void;
}) {
  const idDaPergunta = useId();
  const instrucao = INSTRUCAO_DA_PERGUNTA[pergunta];
  return (
    <>
      <div id={idDaPergunta} className={styles.enunciado}>
        {instrucao !== undefined && <p className={base.texto}>{t(instrucao)}</p>}
        <p className={styles.pergunta}>{t(TEXTO_DA_PERGUNTA[pergunta])}</p>
      </div>
      <div
        className={styles.opcoes}
        role="radiogroup"
        aria-labelledby={idDaPergunta}
        onKeyDown={(evento) => {
          if (evento.key === "Enter") {
            evento.preventDefault();
            aoConfirmar();
          }
        }}
      >
        {OPCOES[pergunta].map((opcao) => (
          // O rotulo envolve a figura e o texto: o clique em qualquer um dos dois responde.
          <label key={opcao.valor} className={styles.opcao}>
            <CalibracaoFigura figura={opcao.figura} />
            <span className={styles.rotulo}>
              <input
                type="radio"
                name={`calibracao-${pergunta}`}
                value={opcao.valor}
                checked={valor === opcao.valor}
                onChange={() => aoResponder(pergunta, opcao.valor)}
              />
              <span>{t(opcao.rotulo)}</span>
            </span>
          </label>
        ))}
      </div>
    </>
  );
}

function numerosEmTexto(lista: readonly number[]): string {
  return lista.length === 0 ? t("calibracao.simulada.nada") : lista.join(" + ");
}

function SaidaDaSimulada({ saida, passo }: { saida: SaidaSimulada; passo: Estado["passo"] }) {
  return (
    <div className={styles.simulada}>
      {passo === "saida" ? (
        <p>{t(saida.saiuParaCima ? "calibracao.simulada.saiuParaCima" : "calibracao.simulada.saiuParaBaixo")}</p>
      ) : (
        <>
          <p>{t("calibracao.simulada.titulo")}</p>
          {saida.folhas.map((folha, indice) => (
            <p key={indice}>
              {t(folha.setasIguais ? "calibracao.simulada.folhaIguais" : "calibracao.simulada.folhaOpostas", {
                n: String(indice + 1),
                cima: numerosEmTexto(folha.paraCima),
                baixo: numerosEmTexto(folha.paraBaixo),
              })}
            </p>
          ))}
          {saida.primeiraASair !== null && (
            <p>{t("calibracao.simulada.primeira", { n: String(saida.primeiraASair) })}</p>
          )}
        </>
      )}
    </div>
  );
}

interface PropsDoRodape {
  estado: Estado;
  ehSimulada: boolean;
  comErro: boolean;
  naoServe: boolean;
  confirmando: boolean;
  respondida: boolean;
  /** Enquanto uma chamada corre, nada do rodape responde. */
  ocupado?: boolean;
  aoDesistir: () => void;
  aoDesistirDeVez: () => void;
  aoFecharConfirmacao: () => void;
  aoFechar: () => void;
  aoVoltar: () => void;
  aoContinuar: () => void;
  aoSeguir: () => void;
  aoImprimirFrente: () => void;
  aoImprimirVerso: () => void;
  aoImprimirDeNovo: () => void;
  aoRefazer: () => void;
  aoGravar: () => void;
  aoCorrigir?: () => void;
  refVoltar?: Ref<HTMLButtonElement>;
}

/** O botao principal fica sempre no mesmo lugar; desistir nao grava nada, em nenhum passo. */
export function Rodape(props: PropsDoRodape) {
  const { estado, ehSimulada, comErro, respondida } = props;
  const travado = props.ocupado === true;
  const desistir = (
    <button type="button" className={`${SECUNDARIO} ${styles.esquerda}`} disabled={travado} onClick={props.aoDesistir}>
      {t("calibracao.desistir")}
    </button>
  );
  const secundario = (rotulo: string, aoClicar: (() => void) | undefined) => (
    <button type="button" className={SECUNDARIO} disabled={travado} onClick={aoClicar}>
      {rotulo}
    </button>
  );
  const principal = (rotulo: string, aoClicar: () => void, desabilitado = false, referencia?: Ref<HTMLButtonElement>) => (
    <button type="button" ref={referencia} className={PRIMARIO} disabled={travado || desabilitado} onClick={aoClicar}>
      {rotulo}
    </button>
  );
  if (props.confirmando) {
    return (
      <>
        <button
          type="button"
          className={`${SECUNDARIO} ${styles.esquerda}`}
          disabled={travado}
          onClick={props.aoDesistirDeVez}
        >
          {t("calibracao.desistir.confirmar")}
        </button>
        {principal(t("calibracao.voltar"), props.aoFecharConfirmacao, false, props.refVoltar)}
      </>
    );
  }
  if (props.naoServe) return principal(t("calibracao.fechar"), props.aoDesistirDeVez);
  if (comErro) {
    return (
      <>
        {desistir}
        {principal(t("calibracao.tentarDeNovo"), props.aoRefazer)}
      </>
    );
  }
  switch (estado.passo) {
    case "preparar":
      return (
        <>
          {desistir}
          {principal(t("calibracao.continuar"), props.aoSeguir)}
        </>
      );
    case "confirmar":
      return (
        <>
          {desistir}
          {secundario(t("calibracao.voltar"), props.aoVoltar)}
          {principal(
            t(estado.automatica ? "calibracao.imprimirDuasFaces" : "calibracao.imprimirFrente"),
            props.aoImprimirFrente,
          )}
        </>
      );
    case "frente":
    case "verso":
      return desistir;
    case "saida":
      return (
        <>
          {desistir}
          {principal(t("calibracao.continuar"), props.aoContinuar, !respondida)}
        </>
      );
    case "recolocar":
      return (
        <>
          {desistir}
          {principal(t("calibracao.imprimirVerso"), props.aoImprimirVerso)}
        </>
      );
    case "perguntas": {
      const primeira = estado.pergunta === perguntasDaRodada(estado.automatica, {})[0];
      return (
        <>
          {desistir}
          {!primeira && secundario(t("calibracao.voltar"), props.aoVoltar)}
          {principal(t("calibracao.continuar"), props.aoContinuar, !respondida)}
        </>
      );
    }
    case "ajustar":
      return (
        <>
          {desistir}
          {principal(t("calibracao.imprimirDeNovo"), props.aoImprimirDeNovo)}
        </>
      );
    case "certo":
      return (
        <>
          {desistir}
          {secundario(t("calibracao.certo.corrigir"), props.aoCorrigir)}
          {principal(t(ehSimulada ? "calibracao.concluir" : "calibracao.gravar"), props.aoGravar)}
        </>
      );
    case "gravada":
    case "nao_encaixou":
    case "conferida":
      return principal(t("calibracao.fechar"), props.aoFechar);
  }
}
