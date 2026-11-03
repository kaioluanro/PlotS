import { Fragment, useEffect, useRef } from "react";
import {
  acoesDoCartao,
  alinhamentoDoCartao,
  cartaoPedeAcao,
  linhaDoLote,
  linhasDoLote,
  linhasDoCartao,
  nomeDoCartao,
  nomeDoPapel,
  resultadoDoCartao,
  rotuloDaFrente,
  rotuloDoVerso,
  sequenciaDoCartao,
  textoDoEstadoDoCartao,
  tomDoCartao,
  type Cartao,
  type Comando,
  type PapelDaLinha,
} from "../cartoes";
import { desenhoDoCartao, posicaoDoCartao, rolarAteOCartao } from "../cartoes-gesto";
import { t } from "../i18n";
import modo from "./BlocoModo.module.css";
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
  aoComandar: (cartao: Cartao, comando: Comando) => void;
  aoPedirCancelar: (cartao: Cartao) => void;
  aoDispensar: (cartao: Cartao) => void;
}

export function CartaoImpressao({ cartao, aoComandar, aoPedirCancelar, aoDispensar }: Props) {
  const nome = nomeDoCartao(cartao);
  const acoes = acoesDoCartao(cartao);
  const resultado = resultadoDoCartao(cartao);
  const sequencia = sequenciaDoCartao(cartao);
  const linhas = linhasDoCartao(cartao);
  const raiz = useRef<HTMLElement>(null);
  const topo = useRef<HTMLDivElement>(null);
  const pedeAcao = cartaoPedeAcao(cartao);
  const desenho = desenhoDoCartao(cartao);
  // Quando o cartao passa a pedir algo, a lista desce ate ele; o foco de quem usa o teclado fica onde esta.
  useEffect(() => {
    const elemento = raiz.current;
    if (!pedeAcao || !elemento) return;
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
  // A faixa do nome tem 1 ou 2 linhas: as acoes precisam da altura dela para parar logo abaixo.
  useEffect(() => {
    const faixa = topo.current;
    if (!faixa || typeof ResizeObserver === "undefined") return;
    const medir = () => {
      raiz.current?.style.setProperty("--altura-da-faixa", `${faixa.offsetHeight}px`);
    };
    medir();
    const observador = new ResizeObserver(medir);
    observador.observe(faixa);
    return () => observador.disconnect();
  }, []);
  const classeDoBotao = `${botoes.botao} ${botoes.secundario} ${styles.botaoDoCartao}`;
  const lote = linhasDoLote(cartao);
  const dados = [
    lote ? lote.detalhe : linhaDoLote(cartao),
    cartao.papel ? nomeDoPapel(cartao.papel) : null,
  ].filter((parte) => parte !== null);
  const problema = cartao.estado.tipo === "erro" || cartao.estado.tipo === "aviso";
  // A figura fica logo antes da frase que ela ilustra.
  const inicioDoGesto = desenho === null ? -1 : linhas.findIndex((linha) => linha.papel === "gesto");

  return (
    <article ref={raiz} className={styles.cartao} aria-label={nome} tabIndex={-1}>
      <div ref={topo} className={styles.topo}>
        <span className={styles.nome} title={nome}>
          {nome}
        </span>
        {cartao.simulada && <span className={styles.selo}>{t("cartao.selo")}</span>}
      </div>
      <p className={styles.estado} aria-live="polite">
        <span className={`${styles.ponto} ${CLASSE_DO_TOM[tomDoCartao(cartao)]}`} aria-hidden="true" />
        <span>{textoDoEstadoDoCartao(cartao)}</span>
      </p>
      {lote?.titulo != null && <p className={modo.loteTitulo}>{lote.titulo}</p>}
      {dados.length > 0 && (
        <p className={lote ? modo.loteDetalhe : styles.dados}>{dados.join(" · ")}</p>
      )}
      {lote?.semImprimir != null && <p className={modo.loteTitulo}>{lote.semImprimir}</p>}
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
        {linhas.map((linha, indice) => (
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
      <div className={styles.acoes}>
        {acoes.frente && (
          <button
            type="button"
            className={classeDoBotao}
            onClick={() => aoComandar(cartao, "enviar_frente")}
          >
            {rotuloDaFrente(cartao)}
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
          <button
            type="button"
            className={classeDoBotao}
            onClick={() => aoComandar(cartao, "continuar_esperando")}
          >
            {t("cartao.continuar")}
          </button>
        )}
        {acoes.reimprimir && (
          <button
            type="button"
            className={classeDoBotao}
            onClick={() => aoComandar(cartao, "reimprimir")}
          >
            {t("cartao.reimprimir")}
          </button>
        )}
        {acoes.cancelar && (
          <button type="button" className={classeDoBotao} onClick={() => aoPedirCancelar(cartao)}>
            {t("cartao.cancelar")}
          </button>
        )}
        {acoes.dispensar && (
          <button type="button" className={classeDoBotao} onClick={() => aoDispensar(cartao)}>
            {t("cartao.dispensar")}
          </button>
        )}
      </div>
    </article>
  );
}
