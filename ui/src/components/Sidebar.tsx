import {
  Check,
  ChevronDown,
  CircleAlert,
  Info,
  LoaderCircle,
  RefreshCw,
  Settings,
  SlidersHorizontal,
  X,
} from "lucide-react";
import { Fragment, useEffect, useId, useRef, useState, type ReactNode, type RefObject } from "react";
import {
  estadoDoEnvio,
  rodapeCedeAoCartao,
  tituloDaSecao,
  type Cartao,
  type Comando,
} from "../cartoes";
import { envioComFace, faceDoEnvio } from "../face-do-envio";
import { idiomaAtual, t } from "../i18n";
import {
  envioComIntervalo,
  lerIntervaloDaAba,
  paginasDoResumo,
  type RascunhoIntervalo,
} from "../intervalo";
import { useEsperaDoErro } from "../intervalo-espera";
import {
  abrirListaPeloTeclado,
  avisosDoTopo,
  caminhoDoAviso,
  eModelo,
  ESPERA_DO_INDICADOR_MS,
  estadoDoSeletor,
  indicaGravacao,
  interpretarEscolha,
  linhaDaFalha,
  nomeDeExibicao,
  nomeDoModelo,
  pedeConfirmacao,
  resumoDoDocumento,
  teclaAbreALista,
  textoDoAviso,
  textoDoEstado,
  type Aviso,
  type Impressora,
  type Modelo,
} from "../impressoras";
import {
  ajustesSemLinha,
  aoVoltarOFoco,
  avisosSemLinha,
  conteudoDoApagarAjuste,
  conteudoDoVoltarAoPadrao,
  temPreferencias,
  textoDoAvisoDoAjuste,
  type CodigoAvisoDoAjuste,
} from "../preferencias";
import { usePreferencias } from "../preferencias-estado";
import { copiasDaFace, type Aba } from "../tabs";
import type { AnaliseDoDocumento } from "../analise-documento";
import { mostraOpcaoDoDriver } from "../opcoes-envio";
import { OpcaoInversao } from "./BlocoAjuste";
import { BlocoIntervalo } from "./BlocoIntervalo";
import { CartaoImpressao } from "./CartaoImpressao";
import { ConfirmDialog } from "./ConfirmDialog";
import { GrupoDeEnvio } from "./GrupoDeEnvio";
import { LinhaRolagemAdiada } from "./LinhaRolagemAdiada";
import styles from "./Sidebar.module.css";

export const ID_LATERAL = "barra-lateral";

interface Props {
  aba: Aba | null;
  impressoras: readonly Impressora[];
  avisos: readonly Aviso[];
  /** A gravacao do modelo que falhou: aparece junto da impressora afetada. */
  falhaAoGravar: Aviso | null;
  lendo: boolean;
  /** Escolhas ainda sendo gravadas, por nome de fila. */
  gravacoes: ReadonlyMap<string, Modelo>;
  aoAtualizar: () => void;
  /** O que se sabe do papel do envio que esta na tela. */
  documento?: AnaliseDoDocumento | null;
  aoAlternar: (impressora: string) => void;
  aoEscolherModelo: (fila: string, modelo: Modelo) => void;
  aoPedirSubstituicao: (fila: string, modelo: Modelo) => void;
  aoDefinirIntervalo: (intervalo: RascunhoIntervalo | null) => void;
  aoCalibrar: (impressora: Impressora) => void;
  /** Impressoras com trabalho em andamento em qualquer aba. */
  ocupadas: ReadonlySet<string>;
  aoImprimir: () => void;
  aoComandar: (cartao: Cartao, comando: Comando) => void;
  aoPedirVerso: (cartao: Cartao) => void;
  aoPedirCancelar: (cartao: Cartao) => void;
  aoDispensar: (cartao: Cartao) => void;
  blocoGrupos: ReactNode;
  blocoAjuste: ReactNode;
  blocoFace: ReactNode;
  campoCopias: ReactNode;
  /** Falso enquanto a quantidade digitada nao e um numero de 1 a 999: o envio fica parado. */
  copiasValidas?: boolean;
}

export function Sidebar({
  aba,
  impressoras,
  avisos,
  falhaAoGravar,
  lendo,
  gravacoes,
  aoAtualizar,
  documento = null,
  aoAlternar,
  aoEscolherModelo,
  aoPedirSubstituicao,
  aoDefinirIntervalo,
  aoCalibrar,
  ocupadas,
  aoImprimir,
  aoComandar,
  aoPedirVerso,
  aoPedirCancelar,
  aoDispensar,
  blocoGrupos,
  blocoAjuste,
  blocoFace,
  campoCopias,
  copiasValidas = true,
}: Props) {
  const idExplicacao = useId();
  const marcadas = aba?.impressorasMarcadas ?? [];
  // O resumo do rodape conta as copias da face que o envio vai usar.
  const faceDaAbaNaLista =
    aba === null ? "frente" : faceDoEnvio(aba, impressoras.filter((item) => marcadas.includes(item.id)));
  const copiasDoResumo = aba === null ? 1 : copiasDaFace(aba, faceDaAbaNaLista);
  // Com o campo de Verso invalido, o envio de versos de cada cartao espera a correcao, como o rodape.
  const motivoDoVersoNoCartao = !copiasValidas && faceDaAbaNaLista === "verso" ? t("copias.invalida") : null;
  const espera = useEsperaDoErro(aba);
  const botao = envioComFace(
    envioComIntervalo(estadoDoEnvio(aba, impressoras, ocupadas), aba, espera.estado),
    aba,
    impressoras,
    documento,
  );
  const lateralRef = useRef<HTMLElement>(null);
  const botaoEnvio = useRef<HTMLButtonElement>(null);
  const focoNoEnvio = useFocoQuandoOEnvioSai(lateralRef, botaoEnvio, botao.desabilitado);
  const cartoes = aba?.cartoes ?? [];
  const idNotaDoModelo = useId();
  const preferencias = usePreferencias(lendo, impressoras);
  const botaoAtualizar = useRef<HTMLButtonElement>(null);
  const [pedidoDeVolta, setPedidoDeVolta] = useState<PedidoDeVolta | null>(null);
  const [dispensados, setDispensados] = useState<ReadonlySet<string>>(new Set());
  const idDaLinhaComFalha = linhaDaFalha(falhaAoGravar, impressoras);
  const haSeletor = impressoras.some(
    (impressora) => estadoDoSeletor(impressora, undefined) !== null,
  );

  return (
    <aside
      ref={lateralRef}
      id={ID_LATERAL}
      className={styles.lateral}
      aria-labelledby="titulo-lateral"
      // Com um cartao na tela, a altura curta esconde o resumo (ver o CSS).
      data-com-cartao={cartoes.length > 0 ? "" : undefined}
      // Sem nada que o rodape possa resolver agora, a altura curta o esconde.
      data-sem-destino={rodapeCedeAoCartao(aba, impressoras, ocupadas) ? "" : undefined}
    >
      <div className={styles.cabecalho}>
        <h2 id="titulo-lateral" className={styles.titulo}>
          {t("lateral.titulo")}
        </h2>
        <span
          className={styles.contador}
          title={t("lateral.total", { n: String(impressoras.length) })}
        >
          {impressoras.length}
        </span>
        {/* Nao usa `disabled`: o foco de quem usa o teclado continuaria no mesmo lugar ao terminar. */}
        <button
          ref={botaoAtualizar}
          type="button"
          className={styles.atualizar}
          aria-label={t("lateral.atualizar")}
          aria-busy={lendo}
          title={t("lateral.atualizar")}
          onClick={aoAtualizar}
        >
          <RefreshCw size={14} className={lendo ? styles.girando : undefined} aria-hidden="true" />
        </button>
      </div>

      {aba === null && (
        <p id={idExplicacao} className={styles.explicacao}>
          {t("lateral.semDocumento")}
        </p>
      )}

      <div className={styles.rolagem}>
        {/* Com a lista na tela, o icone mostra a leitura e o texto fica so para leitor de tela. */}
        <div
          role="status"
          className={lendo && impressoras.length === 0 ? styles.lendo : styles.ocultoVisual}
        >
          {lendo ? t("lateral.atualizando") : ""}
        </div>
        {avisosDoTopo(avisos, falhaAoGravar, impressoras).map((aviso) => (
          <p key={`${aviso.codigo}-${aviso.detalhe ?? ""}`} className={styles.aviso} role="alert">
            <CircleAlert size={12} className={styles.iconeAviso} aria-hidden="true" />
            <AvisoTexto aviso={aviso} />
          </p>
        ))}
        {avisosSemLinha(preferencias.avisos, impressoras).map(([fila, codigo]) => (
          <AvisoDoAjuste
            key={`ajuste-${fila}`}
            fila={fila}
            codigo={codigo}
            aoDispensar={() => {
              botaoAtualizar.current?.focus();
              preferencias.dispensarAviso(fila);
            }}
          />
        ))}
        {!lendo &&
          ajustesSemLinha(preferencias.salvas, impressoras, dispensados).map((fila) => (
            <AjusteSemLinha key={`sem-linha-${fila}`} fila={fila}
              aoDispensar={() => { botaoAtualizar.current?.focus(); setDispensados((atuais) => new Set([...atuais, fila])); }}
              aoApagar={(origem) =>
                  setPedidoDeVolta({
                    fila,
                    nome: fila,
                    origem,
                    destino: () => botaoAtualizar.current,
                    semLinha: true,
                  })
                }
            />
          ))}
        {!lendo && impressoras.length === 0 && avisos.length === 0 && (
          <p className={styles.vazia}>{t("lateral.vazia")}</p>
        )}
        {cartoes.length > 0 && (
          <section className={styles.emAndamento} aria-labelledby="titulo-em-andamento">
            <h3 id="titulo-em-andamento" className={styles.tituloSecao}>
              {t(tituloDaSecao(cartoes))}
            </h3>
            {cartoes.map((cartao) => (
              <CartaoImpressao
                key={cartao.impressora}
                cartao={cartao}
                aoComandar={aoComandar}
                aoPedirVerso={aoPedirVerso}
                aoPedirCancelar={aoPedirCancelar}
                aoDispensar={aoDispensar}
                motivoDoVerso={motivoDoVersoNoCartao}
              />
            ))}
          </section>
        )}
        <ul className={styles.lista}>
          {impressoras.map((impressora) => (
            <LinhaImpressora
              key={impressora.id}
              impressora={impressora}
              marcada={marcadas.includes(impressora.id)}
              desabilitada={aba === null}
              idExplicacao={aba === null ? idExplicacao : undefined}
              idNotaDoModelo={idNotaDoModelo}
              gravando={impressora.tipo === "fila" ? gravacoes.get(impressora.nome) : undefined}
              falha={impressora.id === idDaLinhaComFalha ? falhaAoGravar : null}
              aoAlternar={aoAlternar}
              aoEscolherModelo={aoEscolherModelo}
              aoPedirSubstituicao={aoPedirSubstituicao}
              aoCalibrar={aoCalibrar}
              aoMudarInversao={aoAtualizar}
              temPapelManual={documento?.temPapelManual === true}
              comAjuste={temPreferencias(impressora) && preferencias.salvas.has(impressora.nome)}
              duasFaces={temPreferencias(impressora) && preferencias.duasFaces.has(impressora.nome)}
              janelaAberta={temPreferencias(impressora) && preferencias.abertas.has(impressora.nome)}
              avisoDoAjuste={
                temPreferencias(impressora) ? (preferencias.avisos.get(impressora.nome) ?? null) : null
              }
              comCartaoNaTela={cartoes.some((cartao) => cartao.impressora === impressora.id)}
              aoAbrirPreferencias={preferencias.abrir}
              aoPedirVolta={setPedidoDeVolta}
              aoDispensarAviso={preferencias.dispensarAviso}
            />
          ))}
        </ul>
        {haSeletor && (
          <p id={idNotaDoModelo} className={styles.notaModelo}>
            {t("modo.seletor.nota")}
          </p>
        )}
      </div>

      <LinhaRolagemAdiada />

      <div className={styles.rodape}>
        {/* Em janela curta, so os campos rolam: o botao e o motivo ficam sempre embaixo. */}
        <div className={styles.camposDoRodape}>
          <BlocoIntervalo
            key={`intervalo-${aba?.id ?? "sem-aba"}`}
            aba={aba}
            espera={espera}
            aoDefinir={aoDefinirIntervalo}
          />
          <ResumoDoDocumento aba={aba} copias={copiasDoResumo} />
          {blocoGrupos}
          {blocoAjuste}
        </div>
        <GrupoDeEnvio
          blocoFace={blocoFace}
          campoCopias={campoCopias}
          botao={botao}
          copiasValidas={copiasValidas}
          botaoRef={botaoEnvio}
          aoFocarEnvio={focoNoEnvio}
          aoImprimir={aoImprimir}
        />
      </div>
      <ConfirmDialog
        conteudo={pedidoDeVolta === null ? null : pedidoDeVolta.semLinha ? conteudoDoApagarAjuste(pedidoDeVolta.nome) : conteudoDoVoltarAoPadrao(pedidoDeVolta.nome)}
        aoCancelar={() => setPedidoDeVolta(null)}
        aoConfirmar={() => {
          if (pedidoDeVolta === null) return;
          const { fila, origem, destino } = pedidoDeVolta;
          setPedidoDeVolta(null);
          // So depois de o dialogo devolver o foco: o botao de origem some junto com o ajuste.
          aoVoltarOFoco(origem, () => {
            destino()?.focus();
            preferencias.voltarAoPadrao(fila);
          });
        }}
      />
    </aside>
  );
}

/**
 * O botao de envio pode ficar desabilitado ou sair da tela logo depois de usado. Quem estava nele,
 * ou voltaria a ele ao fechar a confirmacao, segue para a primeira acao do primeiro cartao.
 */
function useFocoQuandoOEnvioSai(
  lateral: RefObject<HTMLElement | null>,
  botao: RefObject<HTMLButtonElement | null>,
  desabilitado: boolean,
): () => void {
  const pendente = useRef(false);
  useEffect(() => {
    if (!pendente.current) return;
    const ativo = document.activeElement;
    // O dialogo ainda esta na frente: quem decide e o estado depois que ele sair.
    if (ativo?.closest('[role="dialog"], [role="alertdialog"]')) return;
    if (ativo !== null && ativo !== document.body && ativo !== botao.current) {
      pendente.current = false;
      return;
    }
    const fora = desabilitado || botao.current?.offsetParent === null;
    if (!fora) return;
    const acao = lateral.current?.querySelector<HTMLElement>("article button:not(:disabled)");
    if (!acao) return;
    pendente.current = false;
    acao.focus();
  });
  return () => {
    pendente.current = true;
  };
}

interface PropsLinha {
  impressora: Impressora;
  marcada: boolean;
  desabilitada: boolean;
  idExplicacao: string | undefined;
  idNotaDoModelo: string;
  gravando: Modelo | undefined;
  falha: Aviso | null;
  aoAlternar: (impressora: string) => void;
  aoEscolherModelo: (fila: string, modelo: Modelo) => void;
  aoPedirSubstituicao: (fila: string, modelo: Modelo) => void;
  aoCalibrar: (impressora: Impressora) => void;
  aoMudarInversao?: () => void;
  temPapelManual?: boolean;
  comAjuste: boolean;
  duasFaces: boolean;
  janelaAberta: boolean;
  avisoDoAjuste: CodigoAvisoDoAjuste | null;
  comCartaoNaTela: boolean;
  aoAbrirPreferencias: (fila: string) => void;
  aoPedirVolta: (pedido: PedidoDeVolta) => void;
  aoDispensarAviso: (fila: string) => void;
}

/** O pedido de apagar um ajuste: de onde veio e para onde o foco vai se a pessoa confirmar. */
interface PedidoDeVolta {
  fila: string;
  nome: string;
  origem: HTMLElement;
  destino: () => HTMLElement | null;
  semLinha?: boolean;
}

/** Como o seletor do modelo, nao usa `disabled` ao gravar: o foco de quem usa o teclado ficaria perdido. */
export function SeletorDoGesto({
  valor,
  gravando,
  demorando,
  aoEscolher,
}: {
  valor: "jato" | "laser";
  gravando: boolean;
  demorando: boolean;
  aoEscolher: (gesto: "jato" | "laser") => void;
}) {
  return (
    <span className={styles.seletorCaixa}>
      <select
        className={styles.seletor}
        value={valor}
        title={demorando ? t("modo.seletor.gravando") : t("modo.gestoManual")}
        aria-busy={gravando}
        onChange={(evento) => {
          if (!gravando) aoEscolher(evento.target.value === "laser" ? "laser" : "jato");
        }}
      >
        <option value="jato">{nomeDoModelo("jato")}</option>
        <option value="laser">{nomeDoModelo("laser")}</option>
      </select>
      {demorando ? (
        <LoaderCircle size={12} className={`${styles.indicadorGravacao} ${styles.girando}`} aria-hidden="true" />
      ) : (
        <ChevronDown size={10} className={styles.setaSeletor} aria-hidden="true" />
      )}
      <span role="status" className={styles.ocultoVisual}>
        {demorando ? t("modo.seletor.gravando") : ""}
      </span>
    </span>
  );
}

export function AjusteSemLinha({ fila, aoApagar, aoDispensar }: { fila: string; aoApagar: (origem: HTMLElement) => void; aoDispensar: () => void }) {
  return <div className={styles.ajusteSemLinha}>
    <SlidersHorizontal size={12} className={styles.iconeNota} aria-hidden="true" />
    <span className={styles.ajusteSemLinhaTexto}>{t("preferencias.salvo.semFila", { nome: fila })}</span>
    <button type="button" className={styles.dispensarAviso} aria-label={t("preferencias.aviso.dispensar", { nome: fila })} onClick={aoDispensar}><X size={14} aria-hidden="true" /></button>
    <button type="button" className={styles.voltarAoPadrao} onClick={(evento) => aoApagar(evento.currentTarget)}>{t("preferencias.apagar")}</button>
  </div>;
}

const CLASSE_DO_NIVEL = {
  pronta: styles.pontoPronta,
  ativa: styles.pontoAtiva,
  atencao: styles.pontoAtencao,
  problema: styles.pontoProblema,
} as const;

export function LinhaImpressora({
  impressora,
  marcada,
  desabilitada,
  idExplicacao,
  idNotaDoModelo,
  gravando,
  falha,
  aoAlternar,
  aoEscolherModelo,
  aoPedirSubstituicao,
  aoCalibrar,
  aoMudarInversao,
  temPapelManual = false,
  comAjuste,
  duasFaces,
  janelaAberta,
  avisoDoAjuste,
  comCartaoNaTela,
  aoAbrirPreferencias,
  aoPedirVolta,
  aoDispensarAviso,
}: PropsLinha) {
  const linha = useRef<HTMLLIElement>(null);
  const engrenagem = useRef<HTMLButtonElement>(null);
  const [decorridoDaGravacao, setDecorridoDaGravacao] = useState(0);
  const idCaixa = useId();
  const idModo = useId();
  const idFace = useId();
  const nome = nomeDeExibicao(impressora);
  const estado = textoDoEstado(impressora);
  const seletor = estadoDoSeletor(impressora, gravando);
  const modelo = eModelo(impressora.modo) ? nomeDoModelo(impressora.modo) : null;
  const face = modelo === null || impressora.modo === "automatico" ? null : t("modo.entrada");
  const classes = [styles.linha];
  if (marcada) classes.push(styles.marcada);
  if (desabilitada) classes.push(styles.inerte);
  if (falha !== null || avisoDoAjuste !== null || janelaAberta) classes.push(styles.comFalha);
  const classesDoEstado = [styles.estado];
  if (impressora.nivel === "atencao" || impressora.nivel === "problema") {
    classesDoEstado.push(styles.estadoDestaque);
  }
  const modoEmTexto = seletor === null && (modelo !== null || impressora.modo === "desconhecido");
  const descricao = [
    idExplicacao,
    modoEmTexto ? idModo : undefined,
    face !== null ? idFace : undefined,
  ];
  const tituloDoModelo =
    modelo !== null && face !== null
      ? t(impressora.tipo === "simulada" ? "modo.simulada.titulo" : "modo.perfil.titulo", {
          modo: modelo,
          face,
        })
      : undefined;

  useEffect(() => {
    if (gravando === undefined) return undefined;
    const prazo = window.setTimeout(
      () => setDecorridoDaGravacao(ESPERA_DO_INDICADOR_MS),
      ESPERA_DO_INDICADOR_MS,
    );
    return () => {
      window.clearTimeout(prazo);
      setDecorridoDaGravacao(0);
    };
  }, [gravando]);
  const gravacaoDemorada = indicaGravacao(gravando, decorridoDaGravacao);

  // A lista pode estar rolada para longe: quem escolheu precisa ver a falha sem perder o foco.
  useEffect(() => {
    if (falha !== null) linha.current?.scrollIntoView({ block: "nearest" });
  }, [falha]);
  // Com o cartao da mesma impressora na tela, ele ja conta o que houve: a lista fica onde esta.
  const avisoQueRola = avisoDoAjuste !== null && !comCartaoNaTela;
  useEffect(() => {
    if (avisoQueRola) linha.current?.scrollIntoView({ block: "nearest" });
  }, [avisoQueRola, avisoDoAjuste]);

  const real = temPreferencias(impressora);
  // Fila real tem uma anatomia so: o modo fica sempre na segunda fileira, na mesma largura.
  const modoDaLinha = (
    <>
    {seletor !== null && (
      <span className={`${styles.seletorCaixa} ${impressora.modo === "automatico" ? styles.seletorCaixaLargo : ""}`}>
        {/* Nao usa `disabled` ao gravar: o foco de quem usa o teclado ficaria perdido. */}
        <select
          className={styles.seletor}
          value={seletor.valor}
          title={gravacaoDemorada ? t("modo.seletor.gravando") : seletor.titulo}
          aria-label={t("modo.seletor.rotulo", { nome })}
          aria-describedby={idNotaDoModelo}
          aria-busy={gravando !== undefined}
          onKeyDown={(evento) => {
            if (teclaAbreALista(evento.key) && abrirListaPeloTeclado(evento.currentTarget)) {
              evento.preventDefault();
            }
          }}
          onChange={(evento) => {
            const escolha = interpretarEscolha(evento.target.value);
            // A opcao escolhida some com a lista: o foco volta daqui quando o dialogo fechar.
            if (escolha.acao === "calibrar") {
              evento.target.focus();
              aoCalibrar(impressora);
            }
            if (escolha.acao !== "gravar") return;
            if (pedeConfirmacao(impressora.modo, gravando)) {
              evento.target.focus();
              aoPedirSubstituicao(impressora.nome, escolha.modelo);
            } else {
              aoEscolherModelo(impressora.nome, escolha.modelo);
            }
          }}
        >
          {seletor.opcoes.map((opcao) => (
            <Fragment key={opcao.valor}>
              {opcao.divisorAntes === true && <hr className={styles.divisorSeletor} />}
              <option
                value={opcao.valor}
                disabled={opcao.soEstado}
                hidden={opcao.soEstado}
                // O ponteiro leva o foco junto: a lista nunca mostra dois itens ativos.
                onPointerMove={(evento) => evento.currentTarget.focus()}
              >
                {opcao.rotulo}
              </option>
            </Fragment>
          ))}
        </select>
        {gravacaoDemorada ? (
          <LoaderCircle
            size={12}
            className={`${styles.indicadorGravacao} ${styles.girando}`}
            aria-hidden="true"
          />
        ) : (
          <ChevronDown size={10} className={styles.setaSeletor} aria-hidden="true" />
        )}
        <span role="status" className={styles.ocultoVisual}>
          {gravacaoDemorada ? t("modo.seletor.gravando") : ""}
        </span>
      </span>
    )}
    {impressora.modo === "desconhecido" && (
      <span id={idModo} className={styles.modoDesconhecido} title={t("modo.desconhecido.titulo")}>
        <CircleAlert size={12} aria-hidden="true" />
        <span>{t("modo.desconhecido")}</span>
      </span>
    )}
    {seletor === null && modelo !== null && (
      <span id={idModo} className={styles.modoNome} title={tituloDoModelo}>
        {modelo}
      </span>
    )}
    </>
  );

  return (
    <li ref={linha} className={classes.join(" ")}>
      <label className={styles.alvo} title={desabilitada ? t("lateral.semDocumento") : undefined}>
        <input
          id={idCaixa}
          type="checkbox"
          className={styles.caixa}
          checked={marcada}
          disabled={desabilitada}
          aria-describedby={descricao.filter(Boolean).join(" ") || undefined}
          onChange={() => aoAlternar(impressora.id)}
        />
        {marcada && <Check size={12} strokeWidth={3} className={styles.glifo} aria-hidden="true" />}
      </label>
      {/* Duas linhas independentes: o texto do modo nao tira largura do nome por causa da face. */}
      <div className={styles.textos}>
        <div className={`${styles.fileira} ${styles.fileiraDoNome}`}>
          <label htmlFor={idCaixa} className={styles.nome} title={nome}>
            {nome}
          </label>
          {real ? (
            // Fora dos rotulos da linha: clicar aqui nunca marca nem desmarca a impressora.
            <button
              ref={engrenagem}
              type="button"
              className={`${styles.engrenagem} ${janelaAberta ? styles.engrenagemAberta : ""}`}
              aria-label={t("preferencias.rotulo", { nome })}
              aria-haspopup="dialog"
              aria-busy={janelaAberta}
              title={t("preferencias.titulo")}
              onClick={() => aoAbrirPreferencias(impressora.nome)}
            >
              <Settings size={14} aria-hidden="true" />
            </button>
          ) : (
            modoDaLinha
          )}
        </div>
        {real && <div className={`${styles.fileira} ${styles.fileiraDoModo}`}>{modoDaLinha}</div>}
        {real && impressora.modo === "automatico" && (
          <label className={`${styles.fileira} ${styles.fileiraDoGesto}`}>
            <span className={styles.gestoRotulo}>{t("modo.gestoManual")}</span>
            <SeletorDoGesto
              valor={impressora.gestoManual ?? "jato"}
              gravando={gravando !== undefined}
              demorando={gravacaoDemorada}
              aoEscolher={(gesto) =>
                aoEscolherModelo(impressora.nome, gesto === "laser" ? "automatico:laser" : "automatico:jato")
              }
            />
          </label>
        )}
        {impressora.tipo === "simulada" && (
          // A simulada nao tem seletor: e daqui que ela entra no mesmo fluxo das filas.
          <div className={styles.fileira}>
            <button
              type="button"
              className={styles.calibrarSimulada}
              aria-label={t("calibracao.titulo", { nome })}
              onClick={() => aoCalibrar(impressora)}
            >
              {t("modo.calibrar")}
            </button>
          </div>
        )}
        <div className={`${styles.fileira} ${styles.fileiraDoEstado}`}>
          <label htmlFor={idCaixa} className={classesDoEstado.join(" ")} title={estado}>
            <span
              className={`${styles.ponto} ${CLASSE_DO_NIVEL[impressora.nivel]}`}
              aria-hidden="true"
            />
            <span className={styles.textoEstado} aria-live="polite">
              {estado}
            </span>
          </label>
          {face !== null && (
            <span id={idFace} className={styles.modoFace} title={tituloDoModelo}>
              {face}
            </span>
          )}
        </div>
        {marcada && mostraOpcaoDoDriver(impressora, temPapelManual) && (
          <OpcaoInversao impressora={impressora} aoMudar={aoMudarInversao} />
        )}
        {comAjuste && (
          <AjusteDaLinha
            nome={nome}
            duasFaces={duasFaces && impressora.modo !== "automatico"}
            aoVoltar={(origem) =>
              aoPedirVolta({
                fila: impressora.nome,
                nome,
                origem,
                destino: () => engrenagem.current,
              })
            }
          />
        )}
      </div>
      {janelaAberta && (
        <p className={styles.notaNaLinha} role="status">
          <Info size={12} className={styles.iconeNota} aria-hidden="true" />
          <span>{t("preferencias.aberta.nota")}</span>
        </p>
      )}
      {falha !== null && (
        <p className={`${styles.aviso} ${styles.avisoNaLinha}`} role="alert">
          <CircleAlert size={12} className={styles.iconeAviso} aria-hidden="true" />
          <AvisoTexto aviso={falha} />
        </p>
      )}
      {avisoDoAjuste !== null && (
        <AvisoDoAjuste
          fila={impressora.nome}
          codigo={avisoDoAjuste}
          naLinha
          aoDispensar={() => {
            engrenagem.current?.focus();
            aoDispensarAviso(impressora.nome);
          }}
        />
      )}
    </li>
  );
}

/** Mostra em texto e icone que a impressora tem ajuste proprio, e o caminho de volta. */
export function AjusteDaLinha({
  nome,
  duasFaces,
  aoVoltar,
}: {
  nome: string;
  duasFaces: boolean;
  aoVoltar: (origem: HTMLElement) => void;
}) {
  const idTexto = useId();
  return (
    <>
      <div className={styles.ajusteSalvo}>
        <span id={idTexto} className={styles.ajusteSalvoTexto} title={t("preferencias.salvo.titulo")}>
          <SlidersHorizontal size={10} aria-hidden="true" />
          {t("preferencias.salvo")}
          <span className={styles.ocultoVisual}>{t("preferencias.salvo.titulo")}</span>
        </span>
        <button
          type="button"
          className={styles.voltarAoPadrao}
          aria-label={t("preferencias.voltar.rotulo", { nome })}
          aria-describedby={idTexto}
          title={t("preferencias.voltar.rotulo", { nome })}
          onClick={(evento) => aoVoltar(evento.currentTarget)}
        >
          {t("preferencias.voltar")}
        </button>
      </div>
      {duasFaces && <p className={styles.ajusteDuasFaces}>{t("preferencias.salvo.duasFaces")}</p>}
    </>
  );
}

/** Aviso sobre o ajuste de uma fila, na linha dela ou no topo da lista, que a pessoa pode dispensar. */
export function AvisoDoAjuste({
  fila,
  codigo,
  naLinha = false,
  aoDispensar,
}: {
  fila: string;
  codigo: CodigoAvisoDoAjuste;
  naLinha?: boolean;
  aoDispensar: () => void;
}) {
  return (
    <p className={`${styles.aviso} ${naLinha ? styles.avisoNaLinha : ""}`} role="alert">
      <CircleAlert size={12} className={styles.iconeAviso} aria-hidden="true" />
      <span className={styles.avisoDoAjusteTexto}>{textoDoAvisoDoAjuste(codigo, fila)}</span>
      <button
        type="button"
        className={styles.dispensarAviso}
        aria-label={t("preferencias.aviso.dispensar", { nome: fila })}
        title={t("preferencias.aviso.dispensar", { nome: fila })}
        onClick={aoDispensar}
      >
        <X size={12} aria-hidden="true" />
      </button>
    </p>
  );
}

function AvisoTexto({ aviso }: { aviso: Aviso }) {
  const caminho = caminhoDoAviso(aviso);
  return (
    <span>
      {textoDoAviso(aviso)}
      {caminho !== null && <code className={styles.caminhoAviso}>{caminho}</code>}
    </span>
  );
}

/** O tamanho do que vai sair, ou por que ainda nao se sabe. */
function ResumoDoDocumento({ aba, copias = 1 }: { aba: Aba | null; copias?: number }) {
  if (aba === null) return null;
  const { paginas } = aba;
  if (paginas.estado === "contando") {
    return <p className={styles.contando}>{t("folhas.contando")}</p>;
  }
  if (paginas.estado === "recusado") {
    return (
      <p className={styles.recusado} title={paginas.motivo}>
        <CircleAlert size={12} aria-hidden="true" />
        <span>{t("folhas.recusado")}</span>
      </p>
    );
  }
  const contadas = paginasDoResumo(lerIntervaloDaAba(aba));
  if (contadas === null) return null;
  return <p className={styles.resumo}>{resumoDoDocumento(contadas, idiomaAtual, copias)}</p>;
}
