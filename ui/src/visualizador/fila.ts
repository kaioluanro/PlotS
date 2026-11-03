/** Desenho em andamento: a promessa termina com ele, e `cancelar` pede que ele pare. */
export interface Desenho {
  concluido: Promise<void>;
  cancelar(): void;
}

/** Quanto menor, mais cedo roda. */
export type Prioridade = 0 | 1;
export const PRIORIDADE_PAGINA: Prioridade = 0;
export const PRIORIDADE_MINIATURA: Prioridade = 1;

interface Pedido {
  prioridade: Prioridade;
  iniciar: () => Desenho;
}

/**
 * Um desenho por vez, com as paginas antes das miniaturas. Cancelar um desenho libera a vez na hora,
 * sem esperar a promessa dele: uma pagina hostil que nao termina nao pode prender as outras.
 */
export class FilaDeDesenho {
  private pendentes: Pedido[] = [];
  private emAndamento: { pedido: Pedido; desenho: Desenho } | null = null;
  private exclusividades: Array<() => Promise<void>> = [];
  private emExclusividade = false;
  private encerrada = false;

  /** Pede um desenho e devolve como desistir dele. */
  pedir(prioridade: Prioridade, iniciar: () => Desenho): () => void {
    if (this.encerrada) return () => undefined;
    const pedido: Pedido = { prioridade, iniciar };
    this.pendentes.push(pedido);
    this.avancar();
    return () => this.desistir(pedido);
  }

  /**
   * Roda `acao` quando nenhum desenho estiver em andamento, segurando os pedidos ate ela terminar.
   * O pdf.js exige isso para liberar a memoria do documento.
   */
  quandoOciosa(acao: () => Promise<void>): void {
    if (this.encerrada) return;
    this.exclusividades.push(acao);
    this.avancar();
  }

  /** Cancela tudo; a fila nao aceita mais nada. */
  encerrar(): void {
    this.encerrada = true;
    this.pendentes = [];
    this.exclusividades = [];
    const atual = this.emAndamento;
    this.emAndamento = null;
    atual?.desenho.cancelar();
  }

  private desistir(pedido: Pedido): void {
    if (this.emAndamento?.pedido === pedido) {
      const { desenho } = this.emAndamento;
      this.emAndamento = null;
      desenho.cancelar();
      this.avancar();
      return;
    }
    this.pendentes = this.pendentes.filter((pendente) => pendente !== pedido);
  }

  private retirarProximo(): Pedido | null {
    let escolhido: Pedido | null = null;
    for (const pedido of this.pendentes) {
      if (escolhido === null || pedido.prioridade < escolhido.prioridade) escolhido = pedido;
    }
    if (escolhido !== null) {
      const retirado = escolhido;
      this.pendentes = this.pendentes.filter((pendente) => pendente !== retirado);
    }
    return escolhido;
  }

  private avancar(): void {
    if (this.encerrada || this.emAndamento !== null || this.emExclusividade) return;

    const acao = this.exclusividades.shift();
    if (acao) {
      this.emExclusividade = true;
      const soltar = () => {
        this.emExclusividade = false;
        this.avancar();
      };
      try {
        acao().then(soltar, soltar);
      } catch {
        soltar();
      }
      return;
    }

    const pedido = this.retirarProximo();
    if (pedido === null) return;
    let desenho: Desenho;
    try {
      desenho = pedido.iniciar();
    } catch {
      this.avancar();
      return;
    }
    this.emAndamento = { pedido, desenho };
    const terminar = () => {
      if (this.emAndamento?.desenho !== desenho) return;
      this.emAndamento = null;
      this.avancar();
    };
    desenho.concluido.then(terminar, terminar);
  }
}
