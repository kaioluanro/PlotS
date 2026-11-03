import { ChevronRight, CircleAlert } from "lucide-react";
import type { PDFDocumentProxy } from "pdfjs-dist";
import {
  memo,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type FocusEvent,
  type KeyboardEvent,
} from "react";
import { t } from "../i18n";
import {
  chaveDoRotuloDaMiniatura,
  faixaVisivel,
  miniaturaPorTecla,
  miniaturasAMontar,
  pilhaDeMiniaturas,
  rolagemParaMostrar,
  tamanhoDaMiniatura,
  type TamanhoPagina,
} from "./contas";
import { PRIORIDADE_MINIATURA, type FilaDeDesenho } from "./fila";
import { useDesenho } from "./useDesenho";
import styles from "./Visualizador.module.css";

const ATRASO_DA_MINIATURA_MS = 160;
const MARGEM_DE_MINIATURAS = 2;

interface PropsItem {
  documento: PDFDocumentProxy;
  fila: FilaDeDesenho;
  numero: number;
  tamanho: TamanhoPagina;
  topo: number;
  atual: boolean;
  focavel: boolean;
  aoIr: (numero: number) => void;
}

const Item = memo(function Item({
  documento,
  fila,
  numero,
  tamanho,
  topo,
  atual,
  focavel,
  aoIr,
}: PropsItem) {
  const imagem = tamanhoDaMiniatura(tamanho);
  const pixelsPorPonto = (imagem.largura / tamanho.largura) * (window.devicePixelRatio || 1);
  const { caixa, estado } = useDesenho({
    documento,
    fila,
    numero,
    pixelsPorPonto,
    prioridade: PRIORIDADE_MINIATURA,
    atrasoMs: ATRASO_DA_MINIATURA_MS,
  });
  const n = String(numero);

  return (
    <li className={styles.itemMiniatura} style={{ top: topo }}>
      <button
        type="button"
        className={`${styles.miniatura} ${atual ? styles.miniaturaAtual : ""}`}
        data-pagina={numero}
        tabIndex={focavel ? 0 : -1}
        aria-label={t(chaveDoRotuloDaMiniatura(estado), { n })}
        aria-current={atual ? "page" : undefined}
        onClick={() => aoIr(numero)}
      >
        <span className={styles.imagemMiniatura} style={{ width: imagem.largura, height: imagem.altura }}>
          <span ref={caixa} className={styles.tela} />
          {estado === "erro" && (
            <CircleAlert
              size={14}
              className={`${styles.iconeMiniatura} ${styles.iconeErro}`}
              aria-hidden="true"
            />
          )}
        </span>
        <span className={styles.rotuloMiniatura}>
          {atual && <ChevronRight size={12} className={styles.marcaAtual} aria-hidden="true" />}
          {n}
        </span>
      </button>
    </li>
  );
});

interface Props {
  documento: PDFDocumentProxy;
  fila: FilaDeDesenho;
  tamanhos: readonly TamanhoPagina[];
  atual: number;
  aoIr: (numero: number) => void;
}

export function Miniaturas({ documento, fila, tamanhos, atual, aoIr }: Props) {
  const lista = useRef<HTMLDivElement>(null);
  const [rolagem, setRolagem] = useState(0);
  const [alturaDaVisao, setAlturaDaVisao] = useState(0);
  // Pagina com o foco do teclado; nula quando o foco esta fora da lista.
  const [foco, setFoco] = useState<number | null>(null);
  const total = tamanhos.length;
  const pilha = useMemo(() => pilhaDeMiniaturas(tamanhos), [tamanhos]);

  useLayoutEffect(() => {
    const alvo = lista.current;
    if (!alvo) return undefined;
    setAlturaDaVisao(alvo.clientHeight);
    const observador = new ResizeObserver(() => setAlturaDaVisao(alvo.clientHeight));
    observador.observe(alvo);
    return () => observador.disconnect();
  }, []);

  const mostrar = (pagina: number) => {
    const alvo = lista.current;
    if (!alvo) return;
    const nova = rolagemParaMostrar(pilha, pagina, alvo.scrollTop, alvo.clientHeight);
    if (nova !== alvo.scrollTop) {
      alvo.scrollTop = nova;
      setRolagem(nova);
    }
  };

  // A lista acompanha a pagina atual sempre que ela muda. As setas da lista nao mudam a atual.
  useEffect(() => {
    mostrar(atual);
  }, [atual, alturaDaVisao]);

  useEffect(() => {
    if (foco === null) return;
    lista.current?.querySelector<HTMLButtonElement>(`[data-pagina="${foco}"]`)?.focus();
  }, [foco]);

  const aoPressionar = (evento: KeyboardEvent<HTMLDivElement>) => {
    const proxima = miniaturaPorTecla(foco ?? atual, evento.key, total);
    if (proxima === null) return;
    evento.preventDefault();
    mostrar(proxima);
    setFoco(proxima);
  };

  const aoFocar = (evento: FocusEvent<HTMLDivElement>) => {
    const pagina = Number(evento.target.getAttribute("data-pagina"));
    if (Number.isInteger(pagina) && pagina >= 1) setFoco(pagina);
  };

  const aoSair = (evento: FocusEvent<HTMLDivElement>) => {
    if (!evento.currentTarget.contains(evento.relatedTarget)) setFoco(null);
  };

  const faixa = faixaVisivel(pilha, rolagem, alturaDaVisao);
  const paginas = miniaturasAMontar(faixa, total, MARGEM_DE_MINIATURAS, foco);
  // Com a atual fora da janela, o Tab entra pela primeira miniatura montada.
  const focavel = foco ?? (paginas.includes(atual) ? atual : (paginas[0] ?? atual));

  return (
    <aside className={styles.miniaturas} aria-label={t("visualizador.miniaturas.titulo")}>
      <div
        ref={lista}
        className={styles.listaMiniaturas}
        onScroll={(evento) => setRolagem(evento.currentTarget.scrollTop)}
        onKeyDown={aoPressionar}
        onFocus={aoFocar}
        onBlur={aoSair}
      >
        <ul className={styles.pilhaMiniaturas} style={{ height: pilha.alturaTotal }}>
          {paginas.map((numero) => {
            const tamanho = tamanhos[numero - 1];
            if (!tamanho) return null;
            return (
              <Item
                key={numero}
                documento={documento}
                fila={fila}
                numero={numero}
                tamanho={tamanho}
                topo={pilha.topos[numero - 1] ?? 0}
                atual={numero === atual}
                focavel={numero === focavel}
                aoIr={aoIr}
              />
            );
          })}
        </ul>
      </div>
    </aside>
  );
}
