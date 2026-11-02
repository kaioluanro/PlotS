import { Check, CircleAlert, RefreshCw } from "lucide-react";
import { useEffect, useId, useState } from "react";
import { t, type Chave } from "../i18n";
import {
  ATRIBUTO_MENU_DE_CONTEXTO,
  caminhoDoAviso,
  confirmarLote,
  conteudoAbaixoDoLote,
  estadoDoBotao,
  impressorasValidas,
  interpretarLote,
  limitarAoIntervaloDoLote,
  LOTE_MAXIMO,
  LOTE_MINIMO,
  LOTE_PADRAO,
  nomeDeExibicao,
  resumoDoLote,
  textoDoAviso,
  textoDoEstado,
  type Aviso,
  type Impressora,
  type Modo,
} from "../impressoras";
import type { Aba } from "../tabs";
import botoes from "./Botao.module.css";
import styles from "./Sidebar.module.css";

export const ID_LATERAL = "barra-lateral";

interface Props {
  aba: Aba | null;
  impressoras: readonly Impressora[];
  avisos: readonly Aviso[];
  lendo: boolean;
  aoAtualizar: () => void;
  aoAlternar: (impressora: string) => void;
  aoDefinirLote: (tamanho: number) => void;
  aoCalibrar: (fila: string) => void;
}

export function Sidebar({
  aba,
  impressoras,
  avisos,
  lendo,
  aoAtualizar,
  aoAlternar,
  aoDefinirLote,
  aoCalibrar,
}: Props) {
  const idExplicacao = useId();
  const marcadas = aba?.impressorasMarcadas ?? [];
  const botao = estadoDoBotao(impressorasValidas(marcadas, impressoras).length);
  const idBotao = useId();

  return (
    <aside id={ID_LATERAL} className={styles.lateral} aria-labelledby="titulo-lateral">
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
        {avisos.map((aviso) => (
          <p key={`${aviso.codigo}-${aviso.detalhe ?? ""}`} className={styles.aviso} role="alert">
            <CircleAlert size={12} className={styles.iconeAviso} aria-hidden="true" />
            <AvisoTexto aviso={aviso} />
          </p>
        ))}
        {!lendo && impressoras.length === 0 && avisos.length === 0 && (
          <p className={styles.vazia}>{t("lateral.vazia")}</p>
        )}
        <ul className={styles.lista}>
          {impressoras.map((impressora) => (
            <LinhaImpressora
              key={impressora.id}
              impressora={impressora}
              marcada={marcadas.includes(impressora.id)}
              desabilitada={aba === null}
              idExplicacao={aba === null ? idExplicacao : undefined}
              aoAlternar={aoAlternar}
              aoCalibrar={aoCalibrar}
            />
          ))}
        </ul>
      </div>

      <div className={styles.rodape}>
        {/* A chave recria o campo: o rascunho digitado em uma aba nao passa para a outra. */}
        <CampoLote key={aba?.id ?? "sem-aba"} aba={aba} aoDefinir={aoDefinirLote} />
        <button
          type="button"
          className={`${botoes.botao} ${botoes.primario} ${styles.imprimir}`}
          disabled={botao.desabilitado}
          title={botao.titulo}
          aria-describedby={idBotao}
        >
          {botao.rotulo}
        </button>
        <p id={idBotao} className={styles.dica}>
          {botao.titulo}
        </p>
      </div>
    </aside>
  );
}

interface PropsLinha {
  impressora: Impressora;
  marcada: boolean;
  desabilitada: boolean;
  idExplicacao: string | undefined;
  aoAlternar: (impressora: string) => void;
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
  aoAlternar,
  aoCalibrar,
}: PropsLinha) {
  const idCaixa = useId();
  const idModo = useId();
  const idFace = useId();
  const nome = nomeDeExibicao(impressora);
  const estado = textoDoEstado(impressora);
  const modo = textosDoModo(impressora);
  const classes = [styles.linha];
  if (marcada) classes.push(styles.marcada);
  if (desabilitada) classes.push(styles.inerte);
  const classesDoEstado = [styles.estado];
  if (impressora.nivel === "atencao" || impressora.nivel === "problema") {
    classesDoEstado.push(styles.estadoDestaque);
  }
  const descricao = [
    idExplicacao,
    modo !== null || impressora.modo === "desconhecido" ? idModo : undefined,
    modo !== null ? idFace : undefined,
  ];

  return (
    <li className={classes.join(" ")}>
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
        <div className={styles.fileira}>
          <label htmlFor={idCaixa} className={styles.nome} title={nome}>
            {nome}
          </label>
          {impressora.modo === "sem_perfil" && (
            <button
              type="button"
              className={`${botoes.botao} ${botoes.secundario} ${botoes.pequeno} ${styles.calibrar}`}
              title={t("modo.calibrar.titulo")}
              onClick={() => aoCalibrar(impressora.nome)}
            >
              {t("modo.calibrar")}
            </button>
          )}
          {impressora.modo === "desconhecido" && (
            <span id={idModo} className={styles.modoDesconhecido} title={t("modo.desconhecido.titulo")}>
              <CircleAlert size={12} aria-hidden="true" />
              <span>{t("modo.desconhecido")}</span>
            </span>
          )}
          {modo !== null && (
            <span id={idModo} className={styles.modoNome} title={modo.titulo}>
              {modo.nome}
            </span>
          )}
        </div>
        <div className={styles.fileira}>
          <label htmlFor={idCaixa} className={classesDoEstado.join(" ")} title={estado}>
            <span
              className={`${styles.ponto} ${CLASSE_DO_NIVEL[impressora.nivel]}`}
              aria-hidden="true"
            />
            <span className={styles.textoEstado} aria-live="polite">
              {estado}
            </span>
          </label>
          {modo !== null && (
            <span id={idFace} className={styles.modoFace} title={modo.titulo}>
              {modo.face}
            </span>
          )}
        </div>
      </div>
    </li>
  );
}

const TEXTOS_DO_MODO: Partial<Record<Modo, { nome: Chave; face: Chave }>> = {
  traseira: { nome: "modo.traseira", face: "modo.traseira.face" },
  gaveta: { nome: "modo.gaveta", face: "modo.gaveta.face" },
};

function textosDoModo(impressora: Impressora) {
  const chaves = TEXTOS_DO_MODO[impressora.modo];
  if (!chaves) return null;
  const partes = { modo: t(chaves.nome), face: t(chaves.face) };
  return {
    nome: partes.modo,
    face: partes.face,
    titulo:
      impressora.tipo === "simulada"
        ? t("modo.simulada.titulo", partes)
        : t("modo.perfil.titulo", partes),
  };
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
  return <p className={styles.resumo}>{resumoDoLote(paginas.total, aba.tamanhoLote)}</p>;
}
