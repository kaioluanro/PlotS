import { Check, ChevronDown, CircleAlert, LoaderCircle, RefreshCw } from "lucide-react";
import { Fragment, useEffect, useId, useRef, useState } from "react";
import {
  cartaoEmAndamento,
  estadoDoEnvio,
  marcadasTodasOcupadas,
  type Cartao,
  type Comando,
} from "../cartoes";
import { t } from "../i18n";
import {
  envioComIntervalo,
  lerIntervaloDaAba,
  paginasDoResumo,
  type RascunhoIntervalo,
} from "../intervalo";
import { useEsperaDoErro } from "../intervalo-espera";
import {
  abrirListaPeloTeclado,
  ATRIBUTO_MENU_DE_CONTEXTO,
  avisosDoTopo,
  caminhoDoAviso,
  confirmarLote,
  conteudoAbaixoDoLote,
  eModelo,
  ESPERA_DO_INDICADOR_MS,
  estadoDoSeletor,
  indicaGravacao,
  interpretarEscolha,
  interpretarLote,
  limitarAoIntervaloDoLote,
  linhaDaFalha,
  LOTE_MAXIMO,
  LOTE_MINIMO,
  LOTE_PADRAO,
  nomeDeExibicao,
  nomeDoModelo,
  pedeConfirmacao,
  resumoDoLote,
  teclaAbreALista,
  textoDoAviso,
  textoDoEstado,
  type Aviso,
  type Impressora,
  type Modelo,
} from "../impressoras";
import type { Aba } from "../tabs";
import { BlocoIntervalo } from "./BlocoIntervalo";
import botoes from "./Botao.module.css";
import { CartaoImpressao } from "./CartaoImpressao";
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
  aoAlternar: (impressora: string) => void;
  aoEscolherModelo: (fila: string, modelo: Modelo) => void;
  aoPedirSubstituicao: (fila: string, modelo: Modelo) => void;
  aoDefinirLote: (tamanho: number) => void;
  aoDefinirIntervalo: (intervalo: RascunhoIntervalo | null) => void;
  aoCalibrar: (fila: string) => void;
  /** Impressoras com trabalho em andamento em qualquer aba. */
  ocupadas: ReadonlySet<string>;
  aoImprimir: () => void;
  aoComandar: (cartao: Cartao, comando: Comando) => void;
  aoPedirCancelar: (cartao: Cartao) => void;
  aoDispensar: (cartao: Cartao) => void;
}

export function Sidebar({
  aba,
  impressoras,
  avisos,
  falhaAoGravar,
  lendo,
  gravacoes,
  aoAtualizar,
  aoAlternar,
  aoEscolherModelo,
  aoPedirSubstituicao,
  aoDefinirLote,
  aoDefinirIntervalo,
  aoCalibrar,
  ocupadas,
  aoImprimir,
  aoComandar,
  aoPedirCancelar,
  aoDispensar,
}: Props) {
  const idExplicacao = useId();
  const marcadas = aba?.impressorasMarcadas ?? [];
  const espera = useEsperaDoErro(aba);
  const botao = envioComIntervalo(estadoDoEnvio(aba, impressoras, ocupadas), aba, espera.estado);
  const cartoes = aba?.cartoes ?? [];
  const idBotao = useId();
  const idNotaDoModelo = useId();
  const idDaLinhaComFalha = linhaDaFalha(falhaAoGravar, impressoras);
  const haSeletor = impressoras.some(
    (impressora) => estadoDoSeletor(impressora, undefined) !== null,
  );

  return (
    <aside
      id={ID_LATERAL}
      className={styles.lateral}
      aria-labelledby="titulo-lateral"
      // Com um cartao na tela, a altura curta esconde a linha de lote e o resumo (ver o CSS).
      data-com-cartao={cartoes.length > 0 ? "" : undefined}
      // Com um cartao na tela e todas as marcadas ocupadas, o rodape nao tem o que oferecer: a altura curta o esconde.
      data-sem-destino={
        cartoes.length > 0 && marcadasTodasOcupadas(impressoras, marcadas, ocupadas) ? "" : undefined
      }
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
        {!lendo && impressoras.length === 0 && avisos.length === 0 && (
          <p className={styles.vazia}>{t("lateral.vazia")}</p>
        )}
        {cartoes.length > 0 && (
          <section className={styles.emAndamento} aria-labelledby="titulo-em-andamento">
            <h3 id="titulo-em-andamento" className={styles.tituloSecao}>
              {cartoes.some(cartaoEmAndamento) ? t("cartao.secao") : t("cartao.secao.encerrados")}
            </h3>
            {cartoes.map((cartao) => (
              <CartaoImpressao
                key={cartao.impressora}
                cartao={cartao}
                aoComandar={aoComandar}
                aoPedirCancelar={aoPedirCancelar}
                aoDispensar={aoDispensar}
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
        <BlocoIntervalo
          key={`intervalo-${aba?.id ?? "sem-aba"}`}
          aba={aba}
          espera={espera}
          aoDefinir={aoDefinirIntervalo}
        />
        {/* A chave recria o campo: o rascunho digitado em uma aba nao passa para a outra. */}
        <CampoLote key={aba?.id ?? "sem-aba"} aba={aba} aoDefinir={aoDefinirLote} />
        <div className={styles.grupoEnvio}>
          <button
            type="button"
            className={`${botoes.botao} ${botoes.primario} ${styles.imprimir}`}
            disabled={botao.desabilitado}
            title={botao.motivo ?? undefined}
            aria-describedby={botao.motivo === null ? undefined : idBotao}
            onClick={aoImprimir}
          >
            {botao.rotulo}
          </button>
          {botao.motivo !== null && (
            <p id={idBotao} className={styles.dica}>
              {botao.motivo}
            </p>
          )}
        </div>
      </div>
    </aside>
  );
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
  aoCalibrar: (fila: string) => void;
}

const CLASSE_DO_NIVEL = {
  pronta: styles.pontoPronta,
  ativa: styles.pontoAtiva,
  atencao: styles.pontoAtencao,
  problema: styles.pontoProblema,
} as const;

function LinhaImpressora({
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
}: PropsLinha) {
  const linha = useRef<HTMLLIElement>(null);
  const [decorridoDaGravacao, setDecorridoDaGravacao] = useState(0);
  const idCaixa = useId();
  const idModo = useId();
  const idFace = useId();
  const nome = nomeDeExibicao(impressora);
  const estado = textoDoEstado(impressora);
  const seletor = estadoDoSeletor(impressora, gravando);
  const modelo = eModelo(impressora.modo) ? nomeDoModelo(impressora.modo) : null;
  const face = modelo === null ? null : t("modo.entrada");
  const classes = [styles.linha];
  if (marcada) classes.push(styles.marcada);
  if (desabilitada) classes.push(styles.inerte);
  if (falha !== null) classes.push(styles.comFalha);
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
        <div
          className={`${styles.fileira} ${styles.fileiraDoNome} ${seletor !== null ? styles.fileiraComSeletor : ""}`}
        >
          <label htmlFor={idCaixa} className={styles.nome} title={nome}>
            {nome}
          </label>
          {seletor !== null && (
            <span className={styles.seletorCaixa}>
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
                    aoCalibrar(impressora.nome);
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
        </div>
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
      </div>
      {falha !== null && (
        <p className={`${styles.aviso} ${styles.avisoNaLinha}`} role="alert">
          <CircleAlert size={12} className={styles.iconeAviso} aria-hidden="true" />
          <AvisoTexto aviso={falha} />
        </p>
      )}
    </li>
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

interface PropsCampo {
  aba: Aba | null;
  aoDefinir: (tamanho: number) => void;
}

function CampoLote({ aba, aoDefinir }: PropsCampo) {
  const valor = aba?.tamanhoLote ?? LOTE_PADRAO;
  const desabilitado = aba === null;
  const id = useId();
  const idDica = useId();
  const [texto, setTexto] = useState(String(valor));

  useEffect(() => setTexto(String(valor)), [valor]);

  const invalido = texto.trim() !== "" && interpretarLote(texto) === null;

  const confirmar = () => {
    const confirmado = confirmarLote(texto, valor);
    setTexto(String(confirmado));
    if (confirmado !== valor) aoDefinir(confirmado);
  };

  const passo = (delta: number) => {
    const limitado = limitarAoIntervaloDoLote((interpretarLote(texto) ?? valor) + delta);
    setTexto(String(limitado));
    if (limitado !== valor) aoDefinir(limitado);
  };

  return (
    <div className={styles.lote}>
      <div className={styles.loteLinha}>
        <label htmlFor={id} className={styles.rotuloLote}>
          {t("folhas.rotulo")}
        </label>
        <input
          id={id}
          className={invalido ? `${styles.campoLote} ${styles.campoInvalido}` : styles.campoLote}
          type="text"
          inputMode="numeric"
          role="spinbutton"
          autoComplete="off"
          spellCheck={false}
          maxLength={6}
          title={t("folhas.intervalo")}
          disabled={desabilitado}
          value={texto}
          aria-valuemin={LOTE_MINIMO}
          aria-valuemax={LOTE_MAXIMO}
          aria-valuenow={valor}
          aria-invalid={invalido}
          aria-describedby={invalido ? idDica : undefined}
          {...{ [ATRIBUTO_MENU_DE_CONTEXTO]: "" }}
          onChange={(evento) => {
            setTexto(evento.target.value);
            const lido = interpretarLote(evento.target.value);
            if (lido !== null && lido !== valor) aoDefinir(lido);
          }}
          onBlur={confirmar}
          onKeyDown={(evento) => {
            if (evento.key === "Enter") confirmar();
            if (evento.key === "Escape") setTexto(String(valor));
            if (evento.key === "ArrowUp" || evento.key === "ArrowDown") {
              evento.preventDefault();
              passo(evento.key === "ArrowUp" ? 1 : -1);
            }
          }}
        />
      </div>
      <AbaixoDoLote aba={aba} invalido={invalido} idDica={idDica} />
    </div>
  );
}

function AbaixoDoLote({
  aba,
  invalido,
  idDica,
}: {
  aba: Aba | null;
  invalido: boolean;
  idDica: string;
}) {
  const conteudo = conteudoAbaixoDoLote(invalido, aba?.paginas ?? null);
  if (conteudo === "erro") {
    return (
      <p id={idDica} className={styles.erroLote} role="alert">
        <CircleAlert size={12} aria-hidden="true" />
        <span>{t("folhas.invalido")}</span>
      </p>
    );
  }
  if (aba === null || conteudo === "nada") return null;
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
  return <p className={styles.resumo}>{resumoDoLote(contadas, aba.tamanhoLote)}</p>;
}
