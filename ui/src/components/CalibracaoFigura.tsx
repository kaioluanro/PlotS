import type { ReactNode } from "react";
import type { FiguraDaResposta } from "../calibracao";
import styles from "./CalibracaoFigura.module.css";

const LARGURA = 34;
const ALTURA = 46;
/** Duas folhas lado a lado, com o gesto no meio. */
const ESQUERDA = 10;
const DIREITA = 82;
const TOPO = 18;
/** Uma folha so, no centro do desenho. */
const CENTRO = 49;
/** Dois macos ocupam mais que duas folhas: ficam mais a esquerda, sem encostar no selo. */
const MACO_ESQUERDO = 2;
const MACO_DIREITO = 68;

interface Posicao {
  x: number;
  y?: number;
}

function Folha({ x, y = TOPO, vazia = false }: Posicao & { vazia?: boolean }) {
  return (
    <rect
      className={vazia ? styles.vazia : styles.folha}
      x={x}
      y={y}
      width={LARGURA}
      height={ALTURA}
      rx={1.5}
    />
  );
}

/** Mesmo numero grande do teste: o que se le aqui e o que se le no papel. */
function Numero({ x, y = TOPO, n, deslocado = 0 }: Posicao & { n: string; deslocado?: number }) {
  return (
    <text
      className={styles.numero}
      x={x + LARGURA / 2 + deslocado}
      y={y + ALTURA / 2 + 8 + deslocado}
      textAnchor="middle"
    >
      {n}
    </text>
  );
}

function FolhaComNumero({ x, n }: Posicao & { n: string }) {
  return (
    <>
      <Folha x={x} />
      <Numero x={x} n={n} />
    </>
  );
}

/** Um maco: folhas de baixo aparecendo atras da de cima. */
function Maco({ x, n }: Posicao & { n: string | null }) {
  return (
    <>
      <rect className={styles.folha} x={x + 5} y={TOPO - 5} width={LARGURA} height={ALTURA} rx={1.5} />
      <FolhaOuVerso x={x} n={n} />
    </>
  );
}

/** Sem numero, a folha esta com a face impressa para baixo: o canto levantado mostra a tinta. */
function FolhaOuVerso({ x, n }: Posicao & { n: string | null }) {
  if (n !== null) return <FolhaComNumero x={x} n={n} />;
  const direita = x + LARGURA;
  const baixo = TOPO + ALTURA;
  const dobra = 16;
  return (
    <g data-marca="canto">
      <path
        className={styles.folha}
        d={`M${x} ${TOPO}H${direita}V${baixo - dobra}L${direita - dobra} ${baixo}H${x}Z`}
      />
      <path
        className={styles.folha}
        d={`M${direita - dobra} ${baixo}L${direita} ${baixo - dobra}L${direita - dobra} ${baixo - dobra}Z`}
      />
      <path className={styles.tinta} d={`M${direita - dobra + 3} ${baixo - dobra + 4}H${direita - 6}`} />
      <path className={styles.tinta} d={`M${direita - dobra + 3} ${baixo - dobra + 8}H${direita - 10}`} />
    </g>
  );
}

/** A seta impressa no teste, grande, no meio da folha. */
function SetaNaFolha({ x, sentido }: Posicao & { sentido: "cima" | "baixo" }) {
  const meio = x + LARGURA / 2;
  const alto = TOPO + 8;
  const baixo = TOPO + ALTURA - 8;
  const [inicio, fim, volta] = sentido === "cima" ? [baixo, alto, alto + 9] : [alto, baixo, baixo - 9];
  return (
    <g className={styles.seta} data-seta={sentido}>
      <path d={`M${meio} ${inicio}V${fim}`} />
      <path d={`M${meio - 8} ${volta}L${meio} ${fim}L${meio + 8} ${volta}`} />
    </g>
  );
}

/** O arco de virar a folha, de uma posicao para a outra. */
function ArcoDeVirar({ de = ESQUERDA + LARGURA + 3, para = DIREITA - 3 }: { de?: number; para?: number }) {
  const inicio = de;
  const fim = para;
  return (
    <g className={styles.gesto} data-gesto="virar">
      <path d={`M${inicio} ${TOPO + 16}C${inicio + 8} ${TOPO - 12} ${fim - 8} ${TOPO - 12} ${fim} ${TOPO + 14}`} />
      <path d={`M${fim - 8} ${TOPO + 10}L${fim} ${TOPO + 14}L${fim + 2} ${TOPO + 5}`} />
    </g>
  );
}

/** A borda longa por onde a folha vira: linha cheia, grossa, encostada na folha da esquerda. */
function BordaLonga() {
  const x = ESQUERDA + LARGURA;
  return <path className={styles.dobradica} data-gesto="borda_longa" d={`M${x} ${TOPO - 3}V${TOPO + ALTURA + 3}`} />;
}

/** Marca de certo ou errado, sempre no mesmo canto: quem difere e a forma. */
function Selo({ certo }: { certo: boolean }) {
  const cx = 121;
  const cy = 13;
  return (
    <g data-selo={certo ? "certo" : "errado"}>
      <circle className={styles.selo} cx={cx} cy={cy} r={9} />
      {certo ? (
        <path className={styles.marca} d={`M${cx - 4.5} ${cy}L${cx - 1} ${cy + 3.5}L${cx + 5} ${cy - 3.5}`} />
      ) : (
        <path className={styles.marca} d={`M${cx - 4} ${cy - 4}L${cx + 4} ${cy + 4}M${cx + 4} ${cy - 4}L${cx - 4} ${cy + 4}`} />
      )}
    </g>
  );
}

/** Impressora sem marca nem modelo, soltando uma folha com o numero. */
function FolhaSaindo({ n }: { n: string }) {
  return (
    <>
      <g className={styles.impressora} data-marca="impressora">
        <path d="M18 18H52V30H18Z" />
        <rect x={8} y={30} width={54} height={24} rx={3} />
        <path d="M18 54H52V64H18Z" />
      </g>
      <g className={styles.gesto}>
        <path d="M66 42H76" />
        <path d="M71 37L77 42L71 47" />
      </g>
      <FolhaComNumero x={DIREITA} n={n} />
    </>
  );
}

const DESENHOS: Readonly<Record<FiguraDaResposta, () => ReactNode>> = {
  saida_a_vista: () => <Maco x={CENTRO} n="2" />,
  saida_escondida: () => <Maco x={CENTRO} n={null} />,
  lado_em_branco: () => (
    <>
      <FolhaComNumero x={ESQUERDA} n="2" />
      <ArcoDeVirar />
      <FolhaComNumero x={DIREITA} n="1" />
      <Selo certo />
    </>
  ),
  lado_por_cima: () => (
    <>
      <Folha x={ESQUERDA} />
      <Numero x={ESQUERDA} n="2" deslocado={-5} />
      <Numero x={ESQUERDA} n="1" deslocado={5} />
      <ArcoDeVirar />
      <Folha x={DIREITA} vazia />
      <Selo certo={false} />
    </>
  ),
  par_certo: () => (
    <>
      <FolhaComNumero x={ESQUERDA} n="1" />
      <ArcoDeVirar />
      <FolhaComNumero x={DIREITA} n="2" />
      <Selo certo />
    </>
  ),
  par_errado: () => (
    <>
      <FolhaComNumero x={ESQUERDA} n="1" />
      <ArcoDeVirar />
      <FolhaComNumero x={DIREITA} n="4" />
      <Selo certo={false} />
    </>
  ),
  setas_iguais: () => (
    <>
      <Folha x={ESQUERDA} />
      <SetaNaFolha x={ESQUERDA} sentido="cima" />
      <BordaLonga />
      <ArcoDeVirar />
      <Folha x={DIREITA} />
      <SetaNaFolha x={DIREITA} sentido="cima" />
      <Selo certo />
    </>
  ),
  setas_opostas: () => (
    <>
      <Folha x={ESQUERDA} />
      <SetaNaFolha x={ESQUERDA} sentido="cima" />
      <BordaLonga />
      <ArcoDeVirar />
      <Folha x={DIREITA} />
      <SetaNaFolha x={DIREITA} sentido="baixo" />
      <Selo certo={false} />
    </>
  ),
  leitura_sem_virar: () => (
    <>
      <Maco x={CENTRO} n="1" />
      <Selo certo />
    </>
  ),
  leitura_virando: () => (
    <>
      <Maco x={MACO_ESQUERDO} n="4" />
      <Maco x={MACO_DIREITO} n="1" />
      <ArcoDeVirar de={MACO_ESQUERDO + LARGURA + 7} para={MACO_DIREITO - 2} />
      <Selo certo />
    </>
  ),
  leitura_nao_fica: () => (
    <>
      <Maco x={MACO_ESQUERDO} n="3" />
      <Maco x={MACO_DIREITO} n="2" />
      <ArcoDeVirar de={MACO_ESQUERDO + LARGURA + 7} para={MACO_DIREITO - 2} />
      <Selo certo={false} />
    </>
  ),
  primeira_1: () => <FolhaSaindo n="1" />,
  primeira_3: () => <FolhaSaindo n="3" />,
  primeira_nao_reparei: () => <FolhaSaindo n="?" />,
};

/** A resposta desenhada. A frase esta ao lado: para leitor de tela, a figura nao repete o texto. */
export function CalibracaoFigura({ figura }: { figura: FiguraDaResposta }) {
  return (
    <svg
      className={styles.figura}
      viewBox="0 0 132 70"
      aria-hidden="true"
      focusable="false"
      data-figura={figura}
    >
      {DESENHOS[figura]()}
    </svg>
  );
}
