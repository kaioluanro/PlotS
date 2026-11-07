import { Calculator, CircleAlert, LoaderCircle, Minus, MoveHorizontal, PanelRight, Plus } from "lucide-react";
import type { PDFDocumentProxy } from "pdfjs-dist";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import botoes from "../components/Botao.module.css";
import { EmptyState } from "../components/EmptyState";
import { ID_GAVETA_PLOTINFO } from "../components/PlotInfoGaveta";
import {
  formatarMl,
  formatarMoeda,
  type ResumoDaBarra,
} from "../analise-tinta";
import { mensagemDeErro } from "../erros";
import { t } from "../i18n";
import type { Documento } from "../tabs";
import {
  acaoDeZoom,
  alturasNoZoom,
  ancoraDaRolagem,
  aguardarTextoDeCarga,
  empilhar,
  ESPACO_ENTRE_PAGINAS,
  esquerdaDaPagina,
  faixaVisivel,
  janelaDePaginas,
  limitarZoom,
  MARGEM_DA_ROLAGEM,
  maiorLargura,
  miniaturasAbertas,
  paginaAtual,
  pdfPediuSenha,
  percentualDoZoom,
  PIXELS_POR_PONTO,
  podeAumentar,
  podeDiminuir,
  proximoBotaoDaBarra,
  proximoZoom,
  rolagemDaAncora,
  rolagemDaPagina,
  zoomParaLargura,
  type Ancora,
  type AcaoDeZoom,
  type TamanhoPagina,
} from "./contas";
import { guardarEstadoDaAba, lerEstadoDaAba, tamanhosDaLeitura } from "./estadoDaAba";
import { FilaDeDesenho } from "./fila";
import { Miniaturas } from "./Miniaturas";
import { Pagina } from "./Pagina";
import { abrirPdf, lerDocumento, medirPagina, type PdfAberto } from "./pdf";
import styles from "./Visualizador.module.css";

/** Paginas fora da tela mantidas desenhadas, de cada lado. */
const MARGEM_DE_PAGINAS = 1;
/** Maximo de canvases de pagina ao mesmo tempo, mesmo com paginas minusculas. */
const TETO_DE_PAGINAS = 40;
/** A cada tantas paginas descartadas, o documento devolve a memoria que o pdf.js guardou. */
const DESCARTES_PARA_LIMPAR = 16;
/** Medidas novas sao aplicadas em lotes, para nao refazer a pilha a cada pagina. */
const MEDIDAS_POR_LOTE = 128;

// Vale para as outras abas: quem recolheu as miniaturas nao precisa recolher de novo.
let preferenciaDeMiniaturas: boolean | null = null;

type Carga =
  | { fase: "lendo" }
  | { fase: "pronto"; pdf: PdfAberto; fila: FilaDeDesenho }
  | { fase: "falha"; motivo: string };

/** Cada tentativa relê o arquivo da mesma aba; a tentativa muda quando a pessoa pede de novo. */
function useCarga(documento: Documento, tentativa: number): Carga {
  const [carga, setCarga] = useState<Carga>({ fase: "lendo" });
  const { caminho, nome } = documento;

  useEffect(() => {
    let ativo = true;
    let aberto: PdfAberto | null = null;
    let fila: FilaDeDesenho | null = null;
    setCarga({ fase: "lendo" });

    lerDocumento(caminho)
      .then(
        (bytes) => abrirPdf(bytes, () => ativo),
        (recusa: unknown) => {
          if (ativo) setCarga({ fase: "falha", motivo: mensagemDeErro(recusa) });
          return null;
        },
      )
      .then((pdf) => {
        if (pdf === null) return;
        if (!ativo) {
          pdf.fechar();
          return;
        }
        aberto = pdf;
        fila = new FilaDeDesenho();
        setCarga({ fase: "pronto", pdf, fila });
      })
      .catch((falha: unknown) => {
        if (!ativo) return;
        setCarga({
          fase: "falha",
          motivo: pdfPediuSenha(falha)
            ? t("erro.criptografado", { nome })
            : t("visualizador.erro.texto"),
        });
      });

    return () => {
      ativo = false;
      fila?.encerrar();
      aberto?.fechar();
    };
  }, [caminho, nome, tentativa]);

  return carga;
}

interface ResumoDaBarraProps {
  resumo: ResumoDaBarra;
  aoAbrir: () => void;
  aberto: boolean;
}

export const ID_RESUMO_PLOTINFO = "resumo-plotinfo";

export function textoDoResumo(resumo: ResumoDaBarra): string {
  return t("plotinfo.resumo.curto", {
    paginas: String(resumo.totalPaginas),
    ml: formatarMl(resumo.ml),
    custo: formatarMoeda(resumo.valor, resumo.moeda),
  });
}

export function BotaoDoResumo({ resumo, aoAbrir, aberto }: ResumoDaBarraProps) {
  const texto = textoDoResumo(resumo);
  const rotulo = t("plotinfo.resumo.rotulo", {
    paginas: String(resumo.totalPaginas),
    ml: formatarMl(resumo.ml),
    custo: formatarMoeda(resumo.valor, resumo.moeda),
  });
  return (
    <button
      type="button"
      id={ID_RESUMO_PLOTINFO}
      className={styles.resumo}
      aria-label={rotulo}
      title={rotulo}
      aria-expanded={aberto}
      aria-controls={ID_GAVETA_PLOTINFO}
      onClick={aoAbrir}
    >
      <Calculator size={12} aria-hidden="true" />
      <span>{texto}</span>
    </button>
  );
}

interface PropsBotao {
  rotulo: string;
  /** Texto da dica ao passar o mouse; quando falta, a dica repete o rotulo. */
  dica?: string;
  indice: number;
  ativo?: boolean;
  pressionado?: boolean;
  foco: number;
  aoFocar: (indice: number) => void;
  aoClicar: () => void;
  children: ReactNode;
}

/**
 * Fica focavel mesmo sem efeito: o foco nao pode cair fora da area ao chegar ao limite do zoom. Na
 * barra so um botao entra na sequencia do Tab; as setas levam o foco aos outros.
 */
function BotaoDaBarra({
  rotulo,
  dica,
  indice,
  ativo = true,
  pressionado,
  foco,
  aoFocar,
  aoClicar,
  children,
}: PropsBotao) {
  return (
    <button
      type="button"
      data-botao-da-barra="true"
      className={styles.botao}
      aria-label={rotulo}
      title={dica ?? rotulo}
      aria-disabled={!ativo}
      aria-pressed={pressionado}
      tabIndex={indice === foco ? 0 : -1}
      onFocus={() => aoFocar(indice)}
      onClick={() => {
        if (ativo) aoClicar();
      }}
    >
      {children}
    </button>
  );
}

interface PropsBarra {
  pagina: number | null;
  total: number | null;
  zoom: number | null;
  podeMais: boolean;
  podeMenos: boolean;
  ajustando: boolean;
  miniaturas: boolean;
  resumoTinta: ResumoDaBarra | null;
  aoAbrirTinta?: () => void;
  plotInfoAberto?: boolean;
  aoDiminuir: () => void;
  aoAumentar: () => void;
  aoAjustar: () => void;
  aoAlternarMiniaturas: () => void;
}

const BOTOES_DA_BARRA = 4;

function Barra({
  pagina,
  total,
  zoom,
  podeMais,
  podeMenos,
  ajustando,
  miniaturas,
  resumoTinta,
  aoAbrirTinta,
  plotInfoAberto = false,
  aoDiminuir,
  aoAumentar,
  aoAjustar,
  aoAlternarMiniaturas,
}: PropsBarra) {
  const barra = useRef<HTMLDivElement>(null);
  const [foco, setFoco] = useState(0);
  const pronta = pagina !== null && total !== null && zoom !== null;
  const rotuloPagina = pronta
    ? t("visualizador.paginaDeTotal", { atual: String(pagina), total: String(total) })
    : t("visualizador.semPagina");
  const rotuloMiniaturas = miniaturas
    ? t("visualizador.miniaturas.ocultar")
    : t("visualizador.miniaturas.mostrar");
  const percentual = pronta ? String(percentualDoZoom(zoom)) : null;

  const aoTeclar = (evento: KeyboardEvent<HTMLDivElement>) => {
    const proximo = proximoBotaoDaBarra(foco, evento.key, BOTOES_DA_BARRA);
    if (proximo === null) return;
    evento.preventDefault();
    setFoco(proximo);
    barra.current
      ?.querySelectorAll<HTMLButtonElement>("[data-botao-da-barra]")
      [proximo]?.focus();
  };

  return (
    <div
      ref={barra}
      className={styles.barra}
      role="toolbar"
      aria-label={t("visualizador.barra")}
      onKeyDown={aoTeclar}
    >
      <span className={styles.dado} title={rotuloPagina}>
        <span className={styles.oculto}>{rotuloPagina}</span>
        <span aria-hidden="true">{pronta ? `${pagina} / ${total}` : "– / –"}</span>
      </span>
      {resumoTinta !== null && aoAbrirTinta !== undefined && (
        <BotaoDoResumo resumo={resumoTinta} aoAbrir={aoAbrirTinta} aberto={plotInfoAberto} />
      )}
      <span className={styles.vao} />
      <BotaoDaBarra
        indice={0}
        foco={foco}
        aoFocar={setFoco}
        rotulo={t("visualizador.diminuir")}
        ativo={pronta && podeMenos}
        aoClicar={aoDiminuir}
      >
        <Minus size={14} aria-hidden="true" />
      </BotaoDaBarra>
      <span
        className={`${styles.dado} ${styles.zoom}`}
        role="status"
        title={percentual ? t("visualizador.zoomAtual", { percentual }) : undefined}
      >
        {percentual ? `${percentual}%` : "–"}
      </span>
      <BotaoDaBarra
        indice={1}
        foco={foco}
        aoFocar={setFoco}
        rotulo={t("visualizador.aumentar")}
        ativo={pronta && podeMais}
        aoClicar={aoAumentar}
      >
        <Plus size={14} aria-hidden="true" />
      </BotaoDaBarra>
      <BotaoDaBarra
        indice={2}
        foco={foco}
        aoFocar={setFoco}
        rotulo={t("visualizador.ajustar")}
        ativo={pronta}
        pressionado={ajustando}
        aoClicar={aoAjustar}
      >
        <MoveHorizontal size={14} aria-hidden="true" />
      </BotaoDaBarra>
      <span className={styles.divisor} />
      <BotaoDaBarra
        indice={3}
        foco={foco}
        aoFocar={setFoco}
        rotulo={t("visualizador.miniaturas.titulo")}
        dica={rotuloMiniaturas}
        ativo={pronta}
        pressionado={miniaturas}
        aoClicar={aoAlternarMiniaturas}
      >
        <PanelRight size={14} aria-hidden="true" />
      </BotaoDaBarra>
    </div>
  );
}

interface PropsLeitor {
  documento: Documento;
  pdf: PDFDocumentProxy;
  total: number;
  primeiraPagina: TamanhoPagina;
  fila: FilaDeDesenho;
  resumoTinta: ResumoDaBarra | null;
  aoAbrirTinta?: () => void;
  plotInfoAberto?: boolean;
  aoMudarPagina?: (pagina: number) => void;
}

function Leitor({
  documento,
  pdf,
  total,
  primeiraPagina,
  fila,
  resumoTinta,
  aoAbrirTinta,
  plotInfoAberto = false,
  aoMudarPagina,
}: PropsLeitor) {
  const raiz = useRef<HTMLDivElement>(null);
  const rolavel = useRef<HTMLDivElement>(null);
  // Volta para onde a aba estava, se ela ja foi lida antes nesta janela.
  const lembrado = lerEstadoDaAba(documento);
  const ancora = useRef<Ancora>(
    lembrado?.ancora ?? { pagina: 1, fracao: 0, folga: MARGEM_DA_ROLAGEM },
  );
  const quadroDaRolagem = useRef<number | null>(null);
  const descartes = useRef(0);

  // Todas comecam com o tamanho da primeira; as medidas reais chegam aos poucos, por tras.
  const [tamanhos, setTamanhos] = useState<readonly TamanhoPagina[]>(() =>
    tamanhosDaLeitura(lembrado, total, primeiraPagina),
  );
  const [zoomPedido, setZoomPedido] = useState(lembrado?.zoomPedido ?? 1);
  const [ajustando, setAjustando] = useState(lembrado?.ajustando ?? true);
  const [rolagem, setRolagem] = useState(0);
  const [visao, setVisao] = useState({ largura: 0, altura: 0 });
  const [larguraDaArea, setLarguraDaArea] = useState(0);
  const [preferencia, setPreferencia] = useState(preferenciaDeMiniaturas);

  useEffect(() => {
    let ativo = true;
    void (async () => {
      let lote = new Map<number, TamanhoPagina>();
      const aplicar = () => {
        const prontas = lote;
        lote = new Map();
        setTamanhos((atuais) => {
          const novos = [...atuais];
          prontas.forEach((tamanho, indice) => {
            novos[indice] = tamanho;
          });
          return novos;
        });
      };
      try {
        for (let numero = 2; numero <= total && ativo; numero += 1) {
          lote.set(numero - 1, await medirPagina(pdf, numero));
          if (lote.size >= MEDIDAS_POR_LOTE) aplicar();
        }
      } catch {
        // Pagina que nao abre fica com a medida estimada e mostra o proprio erro quando aparecer.
      }
      if (ativo && lote.size > 0) aplicar();
      // Medir carrega cada pagina na memoria do worker.
      if (ativo && total > MEDIDAS_POR_LOTE) fila.quandoOciosa(() => pdf.cleanup());
    })();
    return () => {
      ativo = false;
    };
  }, [fila, pdf, total]);

  const alturaBase = useMemo(
    () => empilhar(alturasNoZoom(tamanhos, 1), ESPACO_ENTRE_PAGINAS, MARGEM_DA_ROLAGEM).alturaTotal,
    [tamanhos],
  );
  // O ajuste usa a pagina mais larga medida: nenhuma pagina passa da largura e nao surge barra.
  const larguraDoDocumento = useMemo(() => maiorLargura(tamanhos), [tamanhos]);
  const zoom = ajustando
    ? zoomParaLargura(larguraDoDocumento, visao.largura, alturaBase)
    : limitarZoom(zoomPedido, alturaBase);
  const pilha = useMemo(
    () => empilhar(alturasNoZoom(tamanhos, zoom), ESPACO_ENTRE_PAGINAS, MARGEM_DA_ROLAGEM),
    [tamanhos, zoom],
  );
  const larguraMaxima = larguraDoDocumento * zoom * PIXELS_POR_PONTO;
  const larguraDoConteudo = Math.max(larguraMaxima + MARGEM_DA_ROLAGEM * 2, visao.largura);

  useLayoutEffect(() => {
    const area = raiz.current;
    const alvo = rolavel.current;
    if (!area || !alvo) return undefined;
    const medir = () => {
      setLarguraDaArea(area.clientWidth);
      setVisao({ largura: alvo.clientWidth, altura: alvo.clientHeight });
    };
    medir();
    const observador = new ResizeObserver(medir);
    observador.observe(area);
    observador.observe(alvo);
    return () => observador.disconnect();
  }, []);

  // Mudou a altura das paginas: a leitura continua no mesmo lugar.
  useLayoutEffect(() => {
    const alvo = rolavel.current;
    if (!alvo) return;
    const nova = rolagemDaAncora(pilha, ancora.current);
    if (Math.abs(alvo.scrollTop - nova) > 0.5) alvo.scrollTop = nova;
    setRolagem(alvo.scrollTop);
  }, [pilha]);

  useEffect(
    () => () => {
      if (quadroDaRolagem.current !== null) cancelAnimationFrame(quadroDaRolagem.current);
    },
    [],
  );

  const aoRolar = () => {
    const alvo = rolavel.current;
    if (!alvo) return;
    ancora.current = ancoraDaRolagem(pilha, alvo.scrollTop);
    if (quadroDaRolagem.current !== null) return;
    quadroDaRolagem.current = requestAnimationFrame(() => {
      quadroDaRolagem.current = null;
      if (rolavel.current) setRolagem(rolavel.current.scrollTop);
    });
  };

  const irPara = useCallback(
    (numero: number) => {
      const alvo = rolavel.current;
      if (!alvo) return;
      alvo.scrollTop = rolagemDaPagina(pilha, numero);
      ancora.current = ancoraDaRolagem(pilha, alvo.scrollTop);
      setRolagem(alvo.scrollTop);
    },
    [pilha],
  );

  const aoDescartar = useCallback(() => {
    descartes.current += 1;
    if (descartes.current < DESCARTES_PARA_LIMPAR) return;
    descartes.current = 0;
    fila.quandoOciosa(() => pdf.cleanup());
  }, [fila, pdf]);

  const atual = paginaAtual(pilha, rolagem, visao.altura);
  // A gaveta de tinta acompanha a pagina que a pessoa esta vendo, sem refazer a analise.
  const avisarPagina = useRef(aoMudarPagina);
  avisarPagina.current = aoMudarPagina;
  useEffect(() => {
    if (atual !== null) avisarPagina.current?.(atual);
  }, [atual]);
  // Um comando que nao muda o zoom (por exemplo, diminuir no limite) nao sai do ajuste a largura.
  const aplicarZoom = (acao: AcaoDeZoom) => {
    if (acao === "ajustar") {
      setAjustando(true);
      return;
    }
    const novo = proximoZoom(acao, zoom, alturaBase);
    if (novo === null) return;
    setAjustando(false);
    setZoomPedido(novo);
  };
  const diminuir = () => aplicarZoom("diminuir");
  const aumentar = () => aplicarZoom("aumentar");
  const ajustar = () => aplicarZoom("ajustar");

  // Quando a aba sai, o estado vai junto: a chave e o objeto da aba.
  const vivo = useRef({ ajustando, zoomPedido, tamanhos });
  vivo.current = { ajustando, zoomPedido, tamanhos };
  useEffect(
    () => () => {
      guardarEstadoDaAba(documento, {
        ancora: ancora.current,
        ajustando: vivo.current.ajustando,
        zoomPedido: vivo.current.zoomPedido,
        tamanhos: vivo.current.tamanhos,
      });
    },
    [documento],
  );

  const aoPressionar = (evento: KeyboardEvent<HTMLDivElement>) => {
    const acao = acaoDeZoom(evento);
    if (!acao) return;
    evento.preventDefault();
    evento.stopPropagation();
    aplicarZoom(acao);
  };

  const abertas = miniaturasAbertas(preferencia, larguraDaArea);
  const janela = janelaDePaginas(
    faixaVisivel(pilha, rolagem, visao.altura),
    total,
    MARGEM_DE_PAGINAS,
    TETO_DE_PAGINAS,
  );
  const pixelsPorPonto = zoom * PIXELS_POR_PONTO * (window.devicePixelRatio || 1);
  const paginas: ReactNode[] = [];
  for (let numero = janela.inicio; numero <= janela.fim; numero += 1) {
    const tamanho = tamanhos[numero - 1];
    if (!tamanho) continue;
    const largura = tamanho.largura * zoom * PIXELS_POR_PONTO;
    paginas.push(
      <Pagina
        key={numero}
        documento={pdf}
        fila={fila}
        numero={numero}
        topo={pilha.topos[numero - 1] ?? 0}
        esquerda={esquerdaDaPagina(largura, visao.largura)}
        largura={largura}
        altura={pilha.alturas[numero - 1] ?? 0}
        pixelsPorPonto={pixelsPorPonto}
        aoDescartar={aoDescartar}
      />,
    );
  }

  return (
    <div ref={raiz} className={styles.visualizador} onKeyDown={aoPressionar}>
      <Barra
        pagina={atual}
        total={total}
        zoom={zoom}
        podeMais={podeAumentar(zoom, alturaBase)}
        podeMenos={podeDiminuir(zoom, alturaBase)}
        ajustando={ajustando}
        miniaturas={abertas}
        resumoTinta={resumoTinta}
        plotInfoAberto={plotInfoAberto}
        aoAbrirTinta={aoAbrirTinta}
        aoDiminuir={diminuir}
        aoAumentar={aumentar}
        aoAjustar={ajustar}
        aoAlternarMiniaturas={() => {
          preferenciaDeMiniaturas = !abertas;
          setPreferencia(!abertas);
        }}
      />
      <div className={styles.corpo}>
        <div
          ref={rolavel}
          className={styles.rolavel}
          role="region"
          tabIndex={0}
          aria-label={t("visualizador.documento", { nome: documento.nome })}
          onScroll={aoRolar}
        >
          <div className={styles.conteudo} style={{ width: larguraDoConteudo, height: pilha.alturaTotal }}>
            {paginas}
          </div>
        </div>
        {abertas && (
          <Miniaturas documento={pdf} fila={fila} tamanhos={tamanhos} atual={atual} aoIr={irPara} />
        )}
      </div>
    </div>
  );
}

interface Props {
  documento: Documento;
  resumoTinta?: ResumoDaBarra | null;
  aoAbrirTinta?: () => void;
  plotInfoAberto?: boolean;
  aoMudarPagina?: (pagina: number) => void;
}

/**
 * Giratorio e texto "Abrindo o PDF" surgem juntos, so se a abertura demorar. Ficam sem nada antes
 * disso, para uma abertura rapida nao piscar.
 */
function TextoDeCarga() {
  const [visivel, setVisivel] = useState(false);
  useEffect(() => aguardarTextoDeCarga(() => setVisivel(true)), []);
  if (!visivel) return null;
  return (
    <>
      <LoaderCircle size={16} className={styles.girando} aria-hidden="true" />
      <span>{t("visualizador.carregando")}</span>
    </>
  );
}

/** Mostra o PDF da aba. Cada aba monta o seu: ao trocar, a memoria e os workers da anterior saem. */
export function Visualizador({
  documento,
  resumoTinta = null,
  aoAbrirTinta,
  plotInfoAberto = false,
  aoMudarPagina,
}: Props) {
  const [tentativa, setTentativa] = useState(0);
  const carga = useCarga(documento, tentativa);

  if (carga.fase === "falha") {
    return (
      <div className={styles.centro} role="alert">
        <EmptyState icone={<CircleAlert size={18} />} titulo={documento.nome} texto={carga.motivo}>
          <button
            type="button"
            className={`${botoes.botao} ${botoes.secundario}`}
            onClick={() => setTentativa((atual) => atual + 1)}
          >
            {t("visualizador.tentarDeNovo")}
          </button>
        </EmptyState>
      </div>
    );
  }

  if (carga.fase === "lendo") {
    return (
      <div className={styles.visualizador}>
        <Barra
          pagina={null}
          total={null}
          zoom={null}
          podeMais={false}
          podeMenos={false}
          ajustando={false}
          miniaturas={false}
          resumoTinta={resumoTinta}
          plotInfoAberto={plotInfoAberto}
          aoAbrirTinta={aoAbrirTinta}
          aoDiminuir={() => undefined}
          aoAumentar={() => undefined}
          aoAjustar={() => undefined}
          aoAlternarMiniaturas={() => undefined}
        />
        <div className={`${styles.corpo} ${styles.centro}`} role="status">
          <TextoDeCarga />
        </div>
      </div>
    );
  }

  return (
    <Leitor
      documento={documento}
      pdf={carga.pdf.documento}
      total={carga.pdf.total}
      primeiraPagina={carga.pdf.primeiraPagina}
      fila={carga.fila}
      resumoTinta={resumoTinta}
      plotInfoAberto={plotInfoAberto}
      aoAbrirTinta={aoAbrirTinta}
      aoMudarPagina={aoMudarPagina}
    />
  );
}
