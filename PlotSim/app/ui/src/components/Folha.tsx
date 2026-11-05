import { CircleCheck, CircleHelp, CircleX, Clock, LoaderCircle } from "lucide-react";
import { memo, useCallback, useEffect, useRef, useState } from "react";
import * as backend from "../backend";
import { t } from "../i18n";
import {
  camadasVisiveis,
  chaveDaSituacao,
  chaveDoDesenhoFalho,
  estiloDaCamada,
  GuardaDeDesenhos,
  pendenteDoPapel,
  proporcaoDaFolha,
  textoDasPaginas,
  type CamadaTela,
  type FolhaTela,
  type SituacaoDoDesenho,
  type SituacaoFolha,
} from "../tela";
import styles from "./Folha.module.css";

/** Uns duzentos desenhos cobrem o que cabe na tela e o entorno, sem crescer sem limite. */
const guarda = new GuardaDeDesenhos(200);

/** O indicador de desenho só aparece depois disso, para um desenho rápido não fazer a folha piscar. */
const ATRASO_DO_INDICADOR = 150;

export function esquecerDesenhos(): void {
  guarda.esquecerTudo();
}

function useVisivel<T extends Element>() {
  const elemento = useRef<T>(null);
  const [visivel, setVisivel] = useState(false);
  useEffect(() => {
    const alvo = elemento.current;
    if (!alvo) return;
    const observador = new IntersectionObserver(
      ([entrada]) => {
        if (entrada?.isIntersecting) {
          setVisivel(true);
          observador.disconnect();
        }
      },
      { rootMargin: "200px" },
    );
    observador.observe(alvo);
    return () => observador.disconnect();
  }, []);
  return { elemento, visivel };
}

/** Só vira verdadeiro depois de `atraso` ms seguidos com `ativo`; sai na hora quando `ativo` cai. */
function useAtrasado(ativo: boolean, atraso: number): boolean {
  const [mostrar, setMostrar] = useState(false);
  useEffect(() => {
    if (!ativo) {
      setMostrar(false);
      return;
    }
    const relogio = window.setTimeout(() => setMostrar(true), atraso);
    return () => window.clearTimeout(relogio);
  }, [ativo, atraso]);
  return mostrar;
}

function Camada({
  fila,
  camada,
  virada,
  proporcao,
  ligada,
  aoSituar,
}: {
  fila: string;
  camada: CamadaTela;
  virada: boolean;
  proporcao: number;
  ligada: boolean;
  aoSituar: (pagina: number, situacao: SituacaoDoDesenho) => void;
}) {
  const chave = `${fila}/${camada.pagina}`;
  const [desenho, setDesenho] = useState(() => guarda.ler(chave) ?? null);
  const [falhou, setFalhou] = useState(false);

  useEffect(() => {
    if (!ligada) return;
    const pronto = guarda.ler(chave);
    if (pronto !== undefined) {
      setDesenho(pronto);
      aoSituar(camada.pagina, "pronta");
      return;
    }
    let ativo = true;
    backend
      .desenharPagina(fila, camada.pagina)
      .then((lido) => {
        guarda.guardar(chave, lido);
        if (ativo) setDesenho(lido);
        aoSituar(camada.pagina, "pronta");
      })
      .catch(() => {
        if (ativo) setFalhou(true);
        aoSituar(camada.pagina, "falhou");
      });
    return () => {
      ativo = false;
    };
  }, [chave, fila, camada.pagina, ligada, aoSituar]);

  const classes = `${styles.camada} ${camada.atras ? styles.atras : styles.perto}`;
  if (desenho === null) {
    if (!falhou) return null;
    const versoEmBaixo = chaveDoDesenhoFalho(camada, virada) === "folha.semDesenhoVerso";
    return (
      <span className={`${styles.semDesenho} ${versoEmBaixo ? styles.semDesenhoVerso : ""}`}>
        <span className={styles.textoSemDesenho}>{t(chaveDoDesenhoFalho(camada, virada))}</span>
      </span>
    );
  }
  return (
    <img
      className={classes}
      src={desenho}
      alt=""
      draggable={false}
      style={estiloDaCamada(camada, proporcao)}
    />
  );
}

function IconeDaSituacao({ situacao }: { situacao: SituacaoFolha }) {
  switch (situacao) {
    case "certa":
      return <CircleCheck size={14} aria-hidden="true" className={styles.iconeCerta} />;
    case "errada":
      return <CircleX size={14} aria-hidden="true" className={styles.iconeErrada} />;
    case "sem_marca":
      return <CircleHelp size={14} aria-hidden="true" className={styles.iconeSemMarca} />;
    case "em_andamento":
      return <Clock size={14} aria-hidden="true" className={styles.iconeAndamento} />;
  }
}

interface Props {
  fila: string;
  folha: FolhaTela;
  selecionada: boolean;
  virada: boolean;
  versoRecebido: boolean;
  aoEscolher: (numero: number) => void;
}

function FolhaSemMemo({ fila, folha, selecionada, virada, versoRecebido, aoEscolher }: Props) {
  const camadas = camadasVisiveis(folha, virada);
  const proporcao = proporcaoDaFolha(camadas);
  const { elemento, visivel } = useVisivel<HTMLButtonElement>();
  const [situacoes, setSituacoes] = useState<ReadonlyMap<number, SituacaoDoDesenho>>(() => new Map());
  const aoSituar = useCallback((pagina: number, situacao: SituacaoDoDesenho) => {
    setSituacoes((atuais) =>
      atuais.get(pagina) === situacao ? atuais : new Map(atuais).set(pagina, situacao),
    );
  }, []);
  // Só conta o que já está na tela: uma folha fora da vista não começou a desenhar.
  const pendente = visivel && pendenteDoPapel(camadas, situacoes);
  const desenhando = useAtrasado(pendente, ATRASO_DO_INDICADOR);
  const situacao = t(chaveDaSituacao(folha.situacao, versoRecebido));
  const paginas = textoDasPaginas(camadas, virada);
  const motivos = folha.situacao === "sem_marca" ? [t("folha.sem_marca.texto")] : folha.motivos;
  const rotulo = t("folha.rotulo", { n: String(folha.numero), situacao, paginas });
  const classes = [
    styles.folha,
    selecionada ? styles.selecionada : "",
    folha.situacao === "errada" ? styles.errada : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      ref={elemento}
      type="button"
      className={classes}
      aria-pressed={selecionada}
      aria-busy={pendente}
      aria-label={[rotulo, ...motivos].join(" ")}
      onClick={() => aoEscolher(folha.numero)}
    >
      <span className={styles.papel} style={{ aspectRatio: `1 / ${proporcao}` }}>
        {camadas.map((camada) => (
          <Camada
            key={camada.pagina}
            fila={fila}
            camada={camada}
            virada={virada}
            proporcao={proporcao}
            ligada={visivel}
            aoSituar={aoSituar}
          />
        ))}
        {desenhando && (
          <span className={styles.desenhando}>
            <LoaderCircle size={12} aria-hidden="true" className={styles.girando} />
            {t("folha.desenhando")}
          </span>
        )}
      </span>
      <span className={styles.cabeca}>
        <span className={styles.titulo}>{t("folha.titulo", { n: String(folha.numero) })}</span>
        {virada && <span className={styles.etiqueta}>{t("folha.virada")}</span>}
      </span>
      <span className={styles.situacao}>
        <IconeDaSituacao situacao={folha.situacao} />
        {situacao}
      </span>
      <span className={styles.paginas}>{paginas}</span>
      {motivos.map((motivo) => (
        <span key={motivo} className={styles.motivo}>
          {motivo}
        </span>
      ))}
    </button>
  );
}

export const Folha = memo(FolhaSemMemo);
