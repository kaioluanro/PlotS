import { useEffect, useRef } from "react";
import {
  acoesDoCartao,
  alinhamentoDoCartao,
  cartaoPedeAcao,
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
import { t } from "../i18n";
import botoes from "./Botao.module.css";
import styles from "./CartaoImpressao.module.css";

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

const CLASSE_DO_PAPEL = {
  acao: styles.fraseAcao,
  ressalva: styles.fraseRessalva,
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
  const pedeAcao = cartaoPedeAcao(cartao);
  // Quando o cartao passa a pedir algo, a lista desce ate ele; o foco de quem usa o teclado fica onde esta.
  useEffect(() => {
    const elemento = raiz.current;
    if (!pedeAcao || !elemento) return;
    // Cartao mais alto que a lista: o topo dele e o que quem le ve primeiro. Se cabe, so desce o necessario.
    const alturaDaLista = rolagemDoCartao(elemento)?.clientHeight ?? window.innerHeight;
    elemento.scrollIntoView({
      block: alinhamentoDoCartao(elemento.offsetHeight, alturaDaLista),
    });
  }, [pedeAcao, cartao.estado.tipo]);
  const classeDoBotao = `${botoes.botao} ${botoes.secundario} ${styles.botaoDoCartao}`;
  const dados = [
    cartao.total > 0 ? t("cartao.lote", { n: String(cartao.lote), total: String(cartao.total) }) : null,
    cartao.papel ? nomeDoPapel(cartao.papel) : null,
  ].filter((parte) => parte !== null);
  const problema = cartao.estado.tipo === "erro" || cartao.estado.tipo === "aviso";

  return (
    <article ref={raiz} className={styles.cartao} aria-label={nome}>
      <div className={styles.topo}>
        <span className={styles.nome} title={nome}>
          {nome}
        </span>
        {cartao.simulada && <span className={styles.selo}>{t("cartao.selo")}</span>}
      </div>
      <p className={styles.estado} aria-live="polite">
        <span className={`${styles.ponto} ${CLASSE_DO_TOM[tomDoCartao(cartao)]}`} aria-hidden="true" />
        <span>{textoDoEstadoDoCartao(cartao)}</span>
      </p>
      {dados.length > 0 && <p className={styles.dados}>{dados.join(" · ")}</p>}
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
          <p
            key={`${indice}-${linha.texto}`}
            className={
              problema && linha.papel === "contexto" ? styles.fraseForte : CLASSE_DO_PAPEL[linha.papel]
            }
          >
            {linha.texto}
          </p>
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
        {acoes.reimprimir && (
          <button
            type="button"
            className={classeDoBotao}
            onClick={() => aoComandar(cartao, "reimprimir")}
          >
            {t("cartao.reimprimir")}
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
