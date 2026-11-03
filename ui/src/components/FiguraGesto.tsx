import type { DesenhoDoGesto, GestoDaFigura, LadoDaFace } from "../cartoes-gesto";
import styles from "./FiguraGesto.module.css";

/** Folha vista de cima, com o tamanho de uma pagina em pe. */
const LARGURA = 28;
const ALTURA = 36;
/** Quanto cada folha de baixo aparece: o bastante para ler como um maco. */
const PASSO_DO_MACO = 3;
const DOBRA = 14;

interface Posicao {
  x: number;
  y: number;
}

/**
 * Face impressa para cima: as linhas estao a vista. A ultima e mais curta e o bloco fica no alto da
 * folha: girada 180 graus, a pagina aparece de cabeca para baixo.
 */
function LinhasDeTexto({ x, y, girada }: Posicao & { girada: boolean }) {
  const margem = 6;
  const fim = x + LARGURA - margem;
  return (
    <g
      className={styles.tinta}
      data-marca="linhas"
      transform={girada ? `rotate(180 ${x + LARGURA / 2} ${y + ALTURA / 2})` : undefined}
    >
      <path d={`M${x + margem} ${y + 8}H${fim}`} />
      <path d={`M${x + margem} ${y + 12}H${fim}`} />
      <path d={`M${x + margem} ${y + 16}H${fim}`} />
      <path d={`M${x + margem} ${y + 20}H${fim - 6}`} />
    </g>
  );
}

/** Face impressa para baixo: o verso em branco, com o canto levantado mostrando a tinta embaixo. */
function CantoLevantado({ x, y }: Posicao) {
  const direita = x + LARGURA;
  const baixo = y + ALTURA;
  return (
    <g data-marca="canto">
      <path
        className={styles.dobra}
        d={`M${direita - DOBRA} ${baixo}L${direita} ${baixo - DOBRA}L${direita - DOBRA} ${baixo - DOBRA}Z`}
      />
      <g className={styles.tinta}>
        <path d={`M${direita - DOBRA + 2} ${baixo - DOBRA + 3}H${direita - 6}`} />
        <path d={`M${direita - DOBRA + 2} ${baixo - DOBRA + 7}H${direita - 10}`} />
      </g>
    </g>
  );
}

/** O maco visto de cima: folhas de baixo aparecendo e a marca de qual face esta para cima. */
function MacoDeCima({
  x,
  y,
  face,
  linhasGiradas = false,
}: Posicao & { face: LadoDaFace; linhasGiradas?: boolean }) {
  const direita = x + LARGURA;
  const baixo = y + ALTURA;
  // Com o canto levantado, a folha de cima termina na diagonal da dobra.
  const contorno =
    face === "baixo"
      ? `M${x} ${y}H${direita}V${baixo - DOBRA}L${direita - DOBRA} ${baixo}H${x}Z`
      : `M${x} ${y}H${direita}V${baixo}H${x}Z`;
  return (
    <g>
      {[2, 1].map((n) => (
        <rect
          key={n}
          className={styles.folha}
          x={x + PASSO_DO_MACO * n}
          y={y + PASSO_DO_MACO * n}
          width={LARGURA}
          height={ALTURA}
          rx={1}
        />
      ))}
      <path className={styles.folha} d={contorno} />
      {face === "cima" ? (
        <LinhasDeTexto x={x} y={y} girada={linhasGiradas} />
      ) : (
        <CantoLevantado x={x} y={y} />
      )}
    </g>
  );
}

function Seta({ d, ponta }: { d: string; ponta: string }) {
  return (
    <g className={styles.gesto}>
      <path d={d} />
      <path d={ponta} />
    </g>
  );
}

/** Onde o maco fica em cada gesto: virar precisa de espaco do lado para onde ele cai. */
const POSICAO_DO_MACO: Readonly<Record<GestoDaFigura, Posicao>> = {
  borda_de_cima: { x: 26, y: 40 },
  lateral: { x: 44, y: 22 },
  nao_virar: { x: 26, y: 40 },
  girar: { x: 30, y: 22 },
};

/** O arco de virar pela borda de cima, de um lado ao outro do maco. */
function ArcoDeVirar({ x, y }: Posicao) {
  const direita = x + LARGURA;
  const meio = x + LARGURA / 2;
  return (
    <Seta
      d={`M${direita + 9} ${y + 26}C${direita + 26} ${y + 6} ${direita + 22} ${y - 28} ${meio + 3} ${y - 28}`}
      ponta={`M${meio + 9} ${y - 33}L${meio + 3} ${y - 28}L${meio + 9} ${y - 23}`}
    />
  );
}

/** Vira pela borda de cima: a borda fica marcada, e o maco cai para cima dela. */
function ViraPelaBordaDeCima({ x, y }: Posicao) {
  return (
    <g data-gesto="borda_de_cima">
      <rect className={styles.destino} x={x} y={y - ALTURA} width={LARGURA} height={ALTURA} rx={1} />
      <path className={styles.dobradica} d={`M${x - 3} ${y}H${x + LARGURA + 3}`} />
      <ArcoDeVirar x={x} y={y} />
    </g>
  );
}

/** Vira pela lateral: a borda esquerda fica marcada, e o maco cai para a esquerda dela. */
function ViraPelaLateral({ x, y }: Posicao) {
  const meio = y + ALTURA / 2;
  return (
    <g data-gesto="lateral">
      <rect className={styles.destino} x={x - LARGURA} y={y} width={LARGURA} height={ALTURA} rx={1} />
      <path className={styles.dobradica} d={`M${x} ${y - 3}V${y + ALTURA + 3}`} />
      <Seta
        d={`M${x + LARGURA - 4} ${y - 4}C${x + LARGURA - 8} ${y - 20} ${x - 18} ${y - 20} ${x - 20} ${meio - 16}`}
        ponta={`M${x - 26} ${meio - 22}L${x - 20} ${meio - 16}L${x - 14} ${meio - 22}`}
      />
    </g>
  );
}

/**
 * Nao vira: o mesmo arco de virar, na mesma escala, cortado por um X no meio dele. A folha pontilhada
 * mostra para onde o maco iria: sozinho, um arco com ponta lembraria o desenho de desfazer.
 */
function NaoVira({ x, y }: Posicao) {
  // O ponto medio da curva de `ArcoDeVirar`.
  const cx = x + LARGURA + 17.75;
  const cy = y - 8.5;
  const braco = 6;
  return (
    <g data-gesto="nao_virar">
      <rect className={styles.destino} x={x} y={y - ALTURA} width={LARGURA} height={ALTURA} rx={1} />
      <ArcoDeVirar x={x} y={y} />
      <path className={styles.corte} d={`M${cx - braco} ${cy - braco}L${cx + braco} ${cy + braco}`} />
      <path className={styles.corte} d={`M${cx + braco} ${cy - braco}L${cx - braco} ${cy + braco}`} />
    </g>
  );
}

/** Gira no plano: meia volta em torno do maco, sem virar. */
function Gira({ x, y }: Posicao) {
  const cx = x + LARGURA / 2 + PASSO_DO_MACO;
  const cy = y + ALTURA / 2 + PASSO_DO_MACO;
  const raio = 28;
  return (
    <g data-gesto="girar">
      <Seta
        d={`M${cx} ${cy - raio}A${raio} ${raio} 0 0 1 ${cx} ${cy + raio}`}
        ponta={`M${cx + 6} ${cy + raio - 5}L${cx} ${cy + raio}L${cx + 6} ${cy + raio + 5}`}
      />
    </g>
  );
}

const DESENHO_DO_GESTO = {
  borda_de_cima: ViraPelaBordaDeCima,
  lateral: ViraPelaLateral,
  nao_virar: NaoVira,
  girar: Gira,
} as const;

/** Gestos que deixam a pagina de cabeca para baixo em relacao a como saiu. */
const GIRA_A_FOLHA: ReadonlySet<GestoDaFigura> = new Set(["borda_de_cima", "girar"]);

/** Impressora sem marca nem modelo: corpo, folha em cima e bandeja. */
function Impressora({ x, y }: Posicao) {
  return (
    <g className={styles.impressora}>
      <path d={`M${x + 8} ${y}H${x + 28}V${y + 10}H${x + 8}Z`} />
      <rect x={x} y={y + 10} width={36} height={20} rx={3} />
      <path d={`M${x + 8} ${y + 22}H${x + 28}V${y + 34}H${x + 8}Z`} />
    </g>
  );
}

interface Props {
  desenho: DesenhoDoGesto;
}

/**
 * O gesto em duas cenas: o maco como saiu e o que fazer com ele; o maco como entra na impressora.
 * A frase ao lado ja diz tudo isso: para leitor de tela, a figura nao repete o texto.
 */
export function FiguraGesto({ desenho }: Props) {
  const { x, y } = POSICAO_DO_MACO[desenho.gesto];
  const Gesto = DESENHO_DO_GESTO[desenho.gesto];
  return (
    <svg
      className={styles.figura}
      viewBox="0 0 196 84"
      aria-hidden="true"
      focusable="false"
      data-gesto={desenho.gesto}
      data-saida={desenho.saida}
      data-entrada={desenho.entrada}
    >
      <g data-saida={desenho.saida}>
        <MacoDeCima x={x} y={y} face={desenho.saida} />
      </g>
      <Gesto x={x} y={y} />
      <path className={styles.passo} d="M84 42H94M90 38L94 42L90 46" />
      <g data-entrada={desenho.entrada}>
        <MacoDeCima
          x={104}
          y={30}
          face={desenho.entrada}
          linhasGiradas={GIRA_A_FOLHA.has(desenho.gesto)}
        />
      </g>
      {/* Sai da borda curta: o modelo nao distingue qual delas, entao nao marca cabeca nem pe. */}
      <Seta d="M118 27V18H149" ponta="M145 14L149 18L145 22" />
      <Impressora x={152} y={1} />
    </svg>
  );
}
