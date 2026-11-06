import { CircleAlert } from "lucide-react";
import { Fragment, useEffect, useRef } from "react";
import {
  acoesDoCartao,
  alinhamentoDoCartao,
  cartaoPedeAcao,
  linhaDaCopia,
  linhaDoLote,
  linhasDoCartao,
  nomeDoCartao,
  nomeDoPapel,
  resultadoDoCartao,
  rotuloDaFrente,
  rotuloDoVerso,
  sequenciaDoCartao,
  textoDasFolhas,
  textoDoEstadoDoCartao,
  tomDoCartao,
  trechosPorCopia,
  type Cartao,
  type Comando,
  type PapelDaLinha,
} from "../cartoes";
import { desenhoDoCartao, posicaoDoCartao, rolarAteOCartao } from "../cartoes-gesto";
import { t } from "../i18n";
import botoes from "./Botao.module.css";
import styles from "./CartaoImpressao.module.css";
import { FiguraGesto } from "./FiguraGesto";

const CLASSE_DO_TOM = {
  ativo: styles.pontoAtivo,
  espera: styles.pontoEspera,
  problema: styles.pontoProblema,
  parado: styles.pontoParado,
} as const;

/** A rolagem que contem o cartao: a lista da lateral, que e a unica que rola. */
function rolagemDoCartao(elemento: HTMLElement): HTMLElement | null {
  for (let atual = elemento.parentElement; atual; atual = atual.parentElement) {
    const { overflowY } = getComputedStyle(atual);
    if (overflowY === "auto" || overflowY === "scroll") return atual;
  }
  return null;
}

/** O cartao ja saiu da pagina: o foco fica na lista, sem pular para o comeco da janela. */
function focarALista(lista: HTMLElement | null): void {
  if (lista === null) return;
  if (!lista.hasAttribute("tabindex")) lista.tabIndex = -1;
  // A lista encosta nas bordas da lateral: o anel fica para dentro, para aparecer inteiro.
  lista.style.outlineOffset = "-2px";
  lista.focus({ preventScroll: true });
}

const CLASSE_DO_PAPEL = {
  acao: styles.fraseAcao,
  ressalva: styles.fraseRessalva,
  gesto: styles.fraseGesto,
  contexto: styles.frase,
} as const satisfies Record<PapelDaLinha, string | undefined>;

interface Props {
  cartao: Cartao;
  aoComandar?: (cartao: Cartao, comando: Comando) => void;
  aoPedirVerso?: (cartao: Cartao) => void;
  aoPedirCancelar?: (cartao: Cartao) => void;
  aoDispensar?: (cartao: Cartao) => void;
  /** Por que o botao de versos deste cartao nao envia agora: ele fica desabilitado, com o motivo no titulo. */
  motivoDoVerso?: string | null;
  /**
   * Registro de um momento do trabalho, no painel de avisos: mostra o mesmo texto e o mesmo gesto, sem
   * botoes e sem mexer na rolagem de quem le. As acoes ficam com o painel.
   */
  registro?: boolean;
}

const semAcao = () => undefined;

export function CartaoImpressao({
  cartao,
  aoComandar = semAcao,
  aoPedirVerso = semAcao,
  aoPedirCancelar = semAcao,
  aoDispensar = semAcao,
  motivoDoVerso = null,
  registro = false,
}: Props) {
  const nome = nomeDoCartao(cartao);
  const acoes = acoesDoCartao(cartao);
  const resultado = resultadoDoCartao(cartao);
  const sequencia = sequenciaDoCartao(cartao);
  const linhas = linhasDoCartao(cartao);
  const raiz = useRef<HTMLElement>(null);
  const topo = useRef<HTMLDivElement>(null);
  const linhaDoEstado = useRef<HTMLParagraphElement>(null);
  const botaoVerso = useRef<HTMLButtonElement>(null);
  const pedeAcao = cartaoPedeAcao(cartao);
  const desenho = desenhoDoCartao(cartao);
  // Com as frentes prontas, o botao de versos do primeiro cartao fica a vista na lista. Os outros cartoes
  // nao puxam a lista: senao o ultimo a mudar de estado levaria a rolagem para longe do primeiro.
  useEffect(() => {
    const elemento = raiz.current;
    if (registro || cartao.estado.tipo !== "frentes_prontas" || !elemento) return;
    if (elemento.previousElementSibling?.tagName !== "H3") return;
    botaoVerso.current?.scrollIntoView({ block: "nearest" });
  }, [cartao.estado.tipo]);
  // Quando o cartao passa a pedir algo, a lista desce ate ele; o foco de quem usa o teclado fica onde esta.
  useEffect(() => {
    const elemento = raiz.current;
    if (registro || !pedeAcao || !elemento) return;
    const lista = rolagemDoCartao(elemento);
    return rolarAteOCartao(
      lista,
      () => {
        // Cartao mais alto que a lista: o topo dele e o que quem le ve primeiro. Se cabe, so desce o necessario.
        elemento.scrollIntoView({
          block: alinhamentoDoCartao(elemento.offsetHeight, lista?.clientHeight ?? window.innerHeight),
        });
      },
      {
        posicao: () =>
          lista === null
            ? "em_vista"
            : posicaoDoCartao(elemento.getBoundingClientRect(), lista.getBoundingClientRect()),
        // Quem pediu pela linha fixa pode estar no teclado: o foco vai ao cartao, nunca a um botao
        // dele, para um Enter repetido nao mandar o verso nem refazer o lote.
        focar: () => {
          if (!elemento.isConnected) {
            focarALista(lista);
            return;
          }
          elemento.focus({ preventScroll: true });
          // Foco fora de vista nao serve: a lista anda so o necessario para mostrar o cartao.
          elemento.scrollIntoView({ block: "nearest" });
        },
      },
    );
  }, [pedeAcao, cartao.estado.tipo]);
  // Nome e estado podem ter 2 linhas: o estado e as acoes precisam dessas alturas para parar logo abaixo.
  useEffect(() => {
    const faixa = topo.current;
    const estado = linhaDoEstado.current;
    if (!faixa || !estado || typeof ResizeObserver === "undefined") return;
    const medir = () => {
      raiz.current?.style.setProperty("--altura-da-faixa", `${faixa.offsetHeight}px`);
      raiz.current?.style.setProperty("--altura-do-estado", `${estado.offsetHeight}px`);
    };
    medir();
    const observador = new ResizeObserver(medir);
    observador.observe(faixa);
    observador.observe(estado);
    return () => observador.disconnect();
  }, []);
  const classeDoBotao = `${botoes.botao} ${botoes.secundario} ${styles.botaoDoCartao}`;
  // Lote automatico de uma copia, em trabalho com varias: a linha diz a copia, o trecho e as folhas.
  const linhaCopia = linhaDaCopia(cartao);
  // Concluido, com varias copias e mais de um trecho por copia: as folhas do ultimo trecho nao sao as do trabalho, e nao aparecem.
  const concluidoEmTrechos =
    cartao.estado.tipo === "concluido" &&
    cartao.temAutomatico === true &&
    (cartao.copias ?? 1) > 1 &&
    trechosPorCopia(cartao) > 1;
  const dados =
    linhaCopia !== null
      ? [linhaCopia]
      : concluidoEmTrechos
        ? []
        : [
          linhaDoLote(cartao),
          cartao.folhasDoLote ? textoDasFolhas(cartao.folhasDoLote) : null,
          cartao.papel ? nomeDoPapel(cartao.papel) : null,
        ].filter((parte) => parte !== null);
  const problema = cartao.estado.tipo === "erro" || cartao.estado.tipo === "aviso";
  // Frases que saem depois do botao da acao, como a de espera das frentes prontas.
  const linhasAntes = linhas.filter((linha) => linha.depoisDaAcao !== true);
  const linhasDepois = linhas.filter((linha) => linha.depoisDaAcao === true);
  // A figura fica logo antes da frase que ela ilustra.
  const inicioDoGesto = desenho === null ? -1 : linhasAntes.findIndex((linha) => linha.papel === "gesto");
  // Nas frentes prontas, a fileira de acoes vem logo abaixo do estado, antes do gesto: a confirmacao do verso repete o gesto. Sem verso, não há ação nenhuma para pôr no topo.
  const acoesNoTopo =
    cartao.estado.tipo === "frentes_prontas" && (acoes.pedirVerso || acoes.verso.visivel);
  const blocoDeAcoes = (
    <div className={styles.acoes}>
      {acoes.frente && (
        <button type="button" className={classeDoBotao} onClick={() => aoComandar(cartao, "enviar_frente")}>
          {rotuloDaFrente(cartao)}
        </button>
      )}
      {acoes.pedirVerso && (
        <button
          ref={botaoVerso}
          type="button"
          className={classeDoBotao}
          disabled={motivoDoVerso !== null}
          title={motivoDoVerso ?? undefined}
          onClick={() => aoPedirVerso(cartao)}
        >
          {rotuloDoVerso(cartao)}
        </button>
      )}
      {acoes.verso.visivel && (
        // Fica visivel e desabilitado durante a passada de verso: o clique duplo nao cai em outra acao.
        <button
          type="button"
          className={classeDoBotao}
          disabled={!acoes.verso.liberado}
          onClick={() => aoComandar(cartao, "enviar_verso")}
        >
          {rotuloDoVerso(cartao)}
        </button>
      )}
      {acoes.continuar && (
        <button type="button" className={classeDoBotao} onClick={() => aoComandar(cartao, "continuar_esperando")}>
          {t("cartao.continuar")}
        </button>
      )}
      {acoes.reimprimir && (
        <button type="button" className={classeDoBotao} onClick={() => aoComandar(cartao, "reimprimir")}>
          {t("cartao.reimprimir")}
        </button>
      )}
      {acoes.cancelar && (
        <button type="button" className={classeDoBotao} disabled={acoes.cancelando} onClick={() => aoPedirCancelar(cartao)}>
          {t("cartao.cancelar")}
        </button>
      )}
      {acoes.dispensar && (
        <button type="button" className={classeDoBotao} onClick={() => aoDispensar(cartao)}>
          {t("cartao.dispensar")}
        </button>
      )}
    </div>
  );

  return (
    <article ref={raiz} className={styles.cartao} aria-label={nome} tabIndex={-1}>
      <div ref={topo} className={styles.topo}>
        <span className={styles.nome} title={nome}>
          {nome}
        </span>
        {cartao.simulada && <span className={styles.selo}>{t("cartao.selo")}</span>}
      </div>
      <p ref={linhaDoEstado} className={styles.estado} aria-live="polite">
        <span className={`${styles.ponto} ${CLASSE_DO_TOM[tomDoCartao(cartao)]}`} aria-hidden="true" />
        <span>{textoDoEstadoDoCartao(cartao)}</span>
      </p>
      {acoesNoTopo && !registro && blocoDeAcoes}
      {dados.length > 0 && <p className={styles.dados}>{dados.join(" · ")}</p>}
      {cartao.ajusteRecusado === true && (
        <p className={styles.ajusteRecusado}>
          <CircleAlert size={12} className={styles.iconeAjusteRecusado} aria-hidden="true" />
          <span>{t("preferencias.cartao.recusado")}</span>
        </p>
      )}
      {sequencia.length > 0 && (
        <div className={styles.sequencia}>
          {sequencia.map((linha, indice) => (
            <p key={`${indice}-${linha}`} className={styles.dados} title={linha}>
              {linha}
            </p>
          ))}
        </div>
      )}
      <div className={styles.frases} aria-live="polite">
        {linhasAntes.map((linha, indice) => (
          <Fragment key={`${indice}-${linha.texto}`}>
            {desenho !== null && indice === inicioDoGesto && <FiguraGesto desenho={desenho} />}
            <p
              className={[
                problema && linha.papel === "contexto" ? styles.fraseForte : CLASSE_DO_PAPEL[linha.papel],
                linha.limitada ? styles.fraseLimitada : undefined,
              ]
                .filter(Boolean)
                .join(" ")}
              title={linha.limitada ? linha.texto : undefined}
            >
              {linha.texto}
            </p>
          </Fragment>
        ))}
      </div>
      {resultado !== null && <p className={styles.resultado}>{resultado}</p>}
      {!acoesNoTopo && !registro && blocoDeAcoes}
      {linhasDepois.length > 0 && (
        <div className={styles.frases}>
          {linhasDepois.map((linha, indice) => (
            <p key={`${indice}-${linha.texto}`} className={CLASSE_DO_PAPEL[linha.papel]}>
              {linha.texto}
            </p>
          ))}
        </div>
      )}
    </article>
  );
}
