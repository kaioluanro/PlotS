import { describe, expect, it } from "vitest";
import {
  copiaDoLote,
  linhaDaCopia,
  linhaDoLote,
  linhasDoCartao,
  sequenciaDoCartao,
  trechosPorCopia,
  type Cartao,
} from "./cartoes";
import { conteudoDaReimpressao } from "./confirmacao";

const A4 = { formato: "a4", larguraMm: 210, alturaMm: 297 };

// Forma real do plano: cada copia sai de um lote, e o total de lotes e o de copias.
function cartao(mudancas: Partial<Cartao>): Cartao {
  return {
    aba: 1,
    impressora: "fila:Sala",
    nome: "Sala",
    simulada: false,
    lote: 2,
    total: 2,
    papel: A4,
    estado: { tipo: "imprimindo", passada: "frente" },
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    loteAutomatico: true,
    temAutomatico: true,
    alcance: "frente_e_verso",
    folhasDoLote: 12,
    copia: 2,
    copias: 2,
    ...mudancas,
  } as Cartao;
}

describe("copias no cartao: linha do lote automatico", () => {
  it("diz a copia, o total de copias, as folhas e o papel", () => {
    expect(linhaDaCopia(cartao({}), "pt-BR")).toBe("Cópia 2 de 2 · 12 folhas · A4");
    expect(linhaDaCopia(cartao({}), "en")).toBe("Copy 2 of 2 · 12 sheets · A4");
  });

  it("com papeis mistos, diz tambem o trecho, e nao repete o papel", () => {
    const misto = cartao({ lote: 4, total: 4, copia: 2, copias: 2 });
    expect(linhaDaCopia(misto, "pt-BR")).toBe("Cópia 2 de 2 · trecho 2 de 2 · 12 folhas");
    expect(linhaDaCopia(misto, "en")).toBe("Copy 2 of 2 · section 2 of 2 · 12 sheets");
  });

  it("sem uma so copia no lote, ou sem varias copias, nao ha numero de copia", () => {
    expect(copiaDoLote(cartao({ copia: null }))).toBeNull();
    expect(copiaDoLote(cartao({ copias: 1 }))).toBeNull();
    expect(linhaDaCopia(cartao({ copia: null }), "pt-BR")).toBeNull();
  });

  it("lote manual nao leva numero de copia: o trecho segue como esta", () => {
    expect(copiaDoLote(cartao({ loteAutomatico: false }))).toBeNull();
  });

  it("os trechos por copia nunca caem abaixo de um, nem com zero copias ou zero lotes", () => {
    expect(trechosPorCopia(cartao({ copias: 2, total: 2 }))).toBe(1);
    expect(trechosPorCopia(cartao({ copias: 2, total: 4 }))).toBe(2);
    expect(trechosPorCopia(cartao({ copias: 0, total: 0 }))).toBe(1);
    expect(trechosPorCopia(cartao({ copias: 3, total: 0 }))).toBe(1);
  });
});

describe("copias no cartao: lista, virada, juntar e reimprimir", () => {
  it("a lista de folhas vira 'Cada copia' quando o lote e de uma copia", () => {
    const sequencia = sequenciaDoCartao(
      cartao({
        sequencia: { frente: [{ numero: 1, girada: false }, { numero: 2, girada: false }], verso: [] },
      }),
      "pt-BR",
    );
    expect(sequencia[0]).toMatch(/^Cada cópia: /);
  });

  it("com varias copias, a virada fala de cada copia", () => {
    const concluido = cartao({ estado: { tipo: "concluido" }, copia: null });
    expect(linhasDoCartao(concluido, "pt-BR").map((linha) => linha.texto)).toContain(
      "Em cada cópia, se a página de número menor não estiver em cima, vire a cópia.",
    );
  });

  it("juntar so aparece quando ha mais de um papel por copia", () => {
    const juntar = "Empilhe os maços na ordem em que saíram, com o primeiro por cima.";
    const uma = cartao({ estado: { tipo: "concluido" }, lote: 3, total: 3, copias: 3, copia: null, temAutomatico: false, loteAutomatico: false });
    expect(linhasDoCartao(uma, "pt-BR").map((linha) => linha.texto)).not.toContain(juntar);
    const dois = cartao({ estado: { tipo: "concluido" }, lote: 6, total: 6, copias: 3, copia: null, temAutomatico: false, loteAutomatico: false });
    expect(linhasDoCartao(dois, "pt-BR").map((linha) => linha.texto)).toContain(juntar);
  });

  it("reimprimir a copia diz o numero dela e o total", () => {
    const conteudo = conteudoDaReimpressao("Sala", cartao({}), "pt-BR");
    expect(conteudo.titulo).toBe("cartao.copia.reimprimir");
    expect(conteudo.parametros).toMatchObject({ n: "2", total: "2", nome: "Sala" });
  });

  it("reimprimir sem numero de copia segue o titulo de antes", () => {
    const conteudo = conteudoDaReimpressao("Sala", cartao({ copia: null }), "pt-BR");
    expect(conteudo.titulo).toBe("cartao.reimprimirDialogo.titulo");
  });
});

describe("copias no cartao: concluido", () => {
  const concluido = (mudancas: Partial<Cartao>) =>
    cartao({ estado: { tipo: "concluido" }, temAutomatico: true, loteAutomatico: false, ...mudancas });

  it("com um lote por copia, diz as copias, o total de folhas e o papel", () => {
    expect(linhaDaCopia(concluido({}), "pt-BR")).toBe("2 cópias · 24 folhas no total · A4");
    expect(linhaDaCopia(concluido({}), "en")).toBe("2 copies · 24 sheets in total · A4");
  });

  it("com mais de um trecho por copia, nunca diz trecho nem traz linha de dados", () => {
    const trechos = concluido({ lote: 4, total: 4, copia: 2, copias: 2 });
    expect(linhaDaCopia(trechos, "pt-BR")).toBeNull();
    expect(linhaDoLote(trechos, "pt-BR")).toBeNull();
  });

  it("sem varias copias automaticas, o texto de trecho segue como estava", () => {
    expect(linhaDoLote(concluido({ copias: 1, lote: 2, total: 4 }), "pt-BR")).toBe("Trecho 2 de 4");
  });
});
