import { describe, expect, it } from "vitest";
import {
  abasEmImpressao,
  decidirFechamentoJanela,
  deveOuvirFechamentoJanela,
  estadoInicial,
  indiceDeTabulacao,
  reduzirAbas,
  vizinhaPorSeta,
  type EstadoAbas,
} from "./tabs";

const doc = (nome: string) => ({ caminho: `C:\\pdfs\\${nome}`, nome });

function comAbas(...nomes: string[]): EstadoAbas {
  return nomes.reduce(
    (estado, nome) => reduzirAbas(estado, { tipo: "abrir", documento: doc(nome) }),
    estadoInicial,
  );
}

function idDa(estado: EstadoAbas, posicao: number): number {
  const aba = estado.abas[posicao];
  if (!aba) throw new Error(`sem aba na posicao ${posicao}`);
  return aba.id;
}

describe("abrir", () => {
  it("cria aba ativa, sem impressao em andamento", () => {
    const estado = comAbas("a.pdf");
    expect(estado.abas).toHaveLength(1);
    expect(estado.abas[0]?.documento.nome).toBe("a.pdf");
    expect(estado.abas[0]?.impressaoEmAndamento).toBe(false);
    expect(estado.ativa).toBe(idDa(estado, 0));
  });

  it("ativa sempre a aba recem-aberta e nao repete ids", () => {
    const estado = comAbas("a.pdf", "b.pdf");
    expect(estado.ativa).toBe(idDa(estado, 1));
    expect(idDa(estado, 0)).not.toBe(idDa(estado, 1));
  });

  it("nao reaproveita o id de uma aba fechada", () => {
    const uma = comAbas("a.pdf");
    const fechada = reduzirAbas(uma, { tipo: "pedir_fechamento", id: idDa(uma, 0) });
    const nova = reduzirAbas(fechada, { tipo: "abrir", documento: doc("b.pdf") });
    expect(idDa(nova, 0)).not.toBe(idDa(uma, 0));
  });
});

describe("fechar", () => {
  it("fechar a ativa do meio passa para a vizinha da direita", () => {
    let estado = comAbas("a.pdf", "b.pdf", "c.pdf");
    const [a, b, c] = [idDa(estado, 0), idDa(estado, 1), idDa(estado, 2)];
    estado = reduzirAbas(estado, { tipo: "ativar", id: b });
    estado = reduzirAbas(estado, { tipo: "pedir_fechamento", id: b });
    expect(estado.abas.map((aba) => aba.id)).toEqual([a, c]);
    expect(estado.ativa).toBe(c);
  });

  it("fechar a ativa da ponta direita passa para a da esquerda", () => {
    let estado = comAbas("a.pdf", "b.pdf");
    const [a, b] = [idDa(estado, 0), idDa(estado, 1)];
    estado = reduzirAbas(estado, { tipo: "pedir_fechamento", id: b });
    expect(estado.ativa).toBe(a);
  });

  it("fechar a unica aba deixa a janela sem aba ativa", () => {
    let estado = comAbas("a.pdf");
    estado = reduzirAbas(estado, { tipo: "pedir_fechamento", id: idDa(estado, 0) });
    expect(estado.abas).toEqual([]);
    expect(estado.ativa).toBeNull();
  });

  it("fechar uma aba inativa nao muda a ativa", () => {
    let estado = comAbas("a.pdf", "b.pdf");
    const b = idDa(estado, 1);
    estado = reduzirAbas(estado, { tipo: "pedir_fechamento", id: idDa(estado, 0) });
    expect(estado.ativa).toBe(b);
  });

  it("pedir para fechar um id desconhecido nao muda nada", () => {
    const estado = comAbas("a.pdf");
    expect(reduzirAbas(estado, { tipo: "pedir_fechamento", id: 999 })).toEqual(estado);
  });
});

describe("trocar", () => {
  it("avanca para a proxima aba e volta ao inicio depois da ultima", () => {
    let estado = comAbas("a.pdf", "b.pdf");
    const [a, b] = [idDa(estado, 0), idDa(estado, 1)];
    estado = reduzirAbas(estado, { tipo: "trocar" });
    expect(estado.ativa).toBe(a);
    estado = reduzirAbas(estado, { tipo: "trocar" });
    expect(estado.ativa).toBe(b);
  });

  it("sem abas nao faz nada", () => {
    expect(reduzirAbas(estadoInicial, { tipo: "trocar" })).toEqual(estadoInicial);
  });

  it("com uma aba so mantem a mesma ativa", () => {
    const estado = comAbas("a.pdf");
    expect(reduzirAbas(estado, { tipo: "trocar" }).ativa).toBe(estado.ativa);
  });

  it("ativar um id desconhecido nao muda nada", () => {
    const estado = comAbas("a.pdf");
    expect(reduzirAbas(estado, { tipo: "ativar", id: 999 })).toEqual(estado);
  });
});

describe("confirmacao com impressao em andamento", () => {
  function imprimindo(): { estado: EstadoAbas; id: number } {
    const inicial = comAbas("a.pdf", "b.pdf");
    const id = idDa(inicial, 1);
    return {
      id,
      estado: reduzirAbas(inicial, { tipo: "marcar_impressao", id, emAndamento: true }),
    };
  }

  it("o estado de impressao e de cada aba", () => {
    const { estado, id } = imprimindo();
    expect(estado.abas.find((aba) => aba.id === id)?.impressaoEmAndamento).toBe(true);
    expect(estado.abas.find((aba) => aba.id !== id)?.impressaoEmAndamento).toBe(false);
  });

  it("pede confirmacao em vez de fechar", () => {
    const { estado, id } = imprimindo();
    const pedido = reduzirAbas(estado, { tipo: "pedir_fechamento", id });
    expect(pedido.abas).toHaveLength(2);
    expect(pedido.aguardandoConfirmacao).toBe(id);
  });

  it("cancelar mantem a aba e encerra o pedido", () => {
    const { estado, id } = imprimindo();
    const pedido = reduzirAbas(estado, { tipo: "pedir_fechamento", id });
    const cancelado = reduzirAbas(pedido, { tipo: "cancelar_fechamento" });
    expect(cancelado.abas).toHaveLength(2);
    expect(cancelado.aguardandoConfirmacao).toBeNull();
  });

  it("confirmar fecha a aba e encerra o pedido", () => {
    const { estado, id } = imprimindo();
    const pedido = reduzirAbas(estado, { tipo: "pedir_fechamento", id });
    const confirmado = reduzirAbas(pedido, { tipo: "confirmar_fechamento" });
    expect(confirmado.abas.map((aba) => aba.id)).not.toContain(id);
    expect(confirmado.aguardandoConfirmacao).toBeNull();
  });

  it("confirmar sem pedido pendente nao muda nada", () => {
    const { estado } = imprimindo();
    expect(reduzirAbas(estado, { tipo: "confirmar_fechamento" })).toEqual(estado);
  });

  it("marcar impressao em id desconhecido nao muda nada", () => {
    const { estado } = imprimindo();
    expect(
      reduzirAbas(estado, { tipo: "marcar_impressao", id: 999, emAndamento: true }),
    ).toEqual(estado);
  });
});

describe("fechar a janela", () => {
  it("sem impressao em andamento fecha direto", () => {
    expect(decidirFechamentoJanela(estadoInicial)).toBe("fechar");
    expect(decidirFechamentoJanela(comAbas("a.pdf", "b.pdf"))).toBe("fechar");
  });

  it("com alguma aba em impressao pede confirmacao", () => {
    const estado = comAbas("a.pdf", "b.pdf");
    const marcado = reduzirAbas(estado, { tipo: "marcar_impressao", id: idDa(estado, 0), emAndamento: true });
    expect(decidirFechamentoJanela(marcado)).toBe("confirmar");
    expect(abasEmImpressao(marcado).map((aba) => aba.id)).toEqual([idDa(estado, 0)]);
  });

  it("o pedido sem impressao em andamento nao abre confirmacao", () => {
    const pedido = reduzirAbas(comAbas("a.pdf"), { tipo: "pedir_fechamento_janela" });
    expect(pedido.confirmandoJanela).toBe(false);
  });

  it("o pedido da janela substitui o de uma aba e cancelar encerra", () => {
    const estado = comAbas("a.pdf");
    const id = idDa(estado, 0);
    let pedido = reduzirAbas(estado, { tipo: "marcar_impressao", id, emAndamento: true });
    pedido = reduzirAbas(pedido, { tipo: "pedir_fechamento", id });
    pedido = reduzirAbas(pedido, { tipo: "pedir_fechamento_janela" });
    expect(pedido.confirmandoJanela).toBe(true);
    expect(pedido.aguardandoConfirmacao).toBeNull();
    const cancelado = reduzirAbas(pedido, { tipo: "cancelar_fechamento" });
    expect(cancelado.confirmandoJanela).toBe(false);
  });

  it("se a ultima impressao termina, o pedido da janela perde o motivo", () => {
    const estado = comAbas("a.pdf");
    const id = idDa(estado, 0);
    let pedido = reduzirAbas(estado, { tipo: "marcar_impressao", id, emAndamento: true });
    pedido = reduzirAbas(pedido, { tipo: "pedir_fechamento_janela" });
    pedido = reduzirAbas(pedido, { tipo: "marcar_impressao", id, emAndamento: false });
    expect(pedido.confirmandoJanela).toBe(false);
  });

  it("se a impressao da aba termina, o pedido da aba perde o motivo", () => {
    const estado = comAbas("a.pdf");
    const id = idDa(estado, 0);
    let pedido = reduzirAbas(estado, { tipo: "marcar_impressao", id, emAndamento: true });
    pedido = reduzirAbas(pedido, { tipo: "pedir_fechamento", id });
    pedido = reduzirAbas(pedido, { tipo: "marcar_impressao", id, emAndamento: false });
    expect(pedido.aguardandoConfirmacao).toBeNull();
    expect(pedido.abas).toHaveLength(1);
  });
});

describe("foco itinerante", () => {
  it("setas andam em volta da lista", () => {
    const estado = comAbas("a.pdf", "b.pdf", "c.pdf");
    expect(vizinhaPorSeta(estado.abas, 0, "ArrowRight")?.id).toBe(idDa(estado, 1));
    expect(vizinhaPorSeta(estado.abas, 2, "ArrowRight")?.id).toBe(idDa(estado, 0));
    expect(vizinhaPorSeta(estado.abas, 0, "ArrowLeft")?.id).toBe(idDa(estado, 2));
  });

  it("outra tecla ou lista vazia nao navega", () => {
    const estado = comAbas("a.pdf");
    expect(vizinhaPorSeta(estado.abas, 0, "Enter")).toBeNull();
    expect(vizinhaPorSeta([], 0, "ArrowRight")).toBeNull();
  });

  it("so a aba ativa entra na ordem de tabulacao", () => {
    expect(indiceDeTabulacao(3, 3)).toBe(0);
    expect(indiceDeTabulacao(2, 3)).toBe(-1);
  });
});

describe("ouvinte de fechamento da janela", () => {
  const abrirDuas = comAbas("a.pdf", "b.pdf");
  const [primeira, segunda] = abrirDuas.abas;
  const marcar = (estado: EstadoAbas, id: number, emAndamento: boolean) =>
    reduzirAbas(estado, { tipo: "marcar_impressao", id, emAndamento });

  it("nao existe sem abas nem com abas paradas", () => {
    expect(deveOuvirFechamentoJanela(estadoInicial)).toBe(false);
    expect(deveOuvirFechamentoJanela(abrirDuas)).toBe(false);
  });

  it("existe enquanto alguma aba imprime e some quando a ultima termina", () => {
    if (!primeira || !segunda) throw new Error("faltam abas");
    const uma = marcar(abrirDuas, primeira.id, true);
    expect(deveOuvirFechamentoJanela(uma)).toBe(true);
    const duas = marcar(uma, segunda.id, true);
    const restaUma = marcar(duas, primeira.id, false);
    expect(deveOuvirFechamentoJanela(restaUma)).toBe(true);
    expect(deveOuvirFechamentoJanela(marcar(restaUma, segunda.id, false))).toBe(false);
  });

  it("some quando a aba que imprimia e fechada", () => {
    if (!primeira) throw new Error("falta aba");
    const pedido = reduzirAbas(marcar(abrirDuas, primeira.id, true), {
      tipo: "pedir_fechamento",
      id: primeira.id,
    });
    expect(deveOuvirFechamentoJanela(pedido)).toBe(true);
    expect(deveOuvirFechamentoJanela(reduzirAbas(pedido, { tipo: "confirmar_fechamento" }))).toBe(false);
  });

  it("segue existindo enquanto o dialogo da janela esta aberto", () => {
    if (!primeira) throw new Error("falta aba");
    const aberto = reduzirAbas(marcar(abrirDuas, primeira.id, true), { tipo: "pedir_fechamento_janela" });
    expect(aberto.confirmandoJanela).toBe(true);
    expect(deveOuvirFechamentoJanela(aberto)).toBe(true);
  });
});
