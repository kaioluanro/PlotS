import { describe, expect, it } from "vitest";
import {
  linhaDoLote,
  linhasDoCartao,
  linhasDoLote,
  listaDeLotes,
  loteNoDocumento,
  lotesSemImprimir,
  type Cartao,
  type EstadoCartao,
} from "./cartoes";
import { traduzir } from "./i18n";

function cartao(estado: EstadoCartao, lote: number, lotes?: readonly number[], total = 3): Cartao {
  return {
    aba: 1,
    impressora: "a",
    nome: "a",
    simulada: false,
    lote,
    total,
    papel: null,
    estado,
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    ...(lotes ? { lotesNoDocumento: lotes } : {}),
  };
}

const AGUARDANDO: EstadoCartao = { tipo: "aguardando" };
const ERRO: EstadoCartao = { tipo: "erro", motivo: { codigo: "fila", texto: "" } };
const LOTES = [2, 4, 6];

describe("linhas do lote no cartao", () => {
  it("fora da divisao continua igual", () => {
    expect(linhasDoLote(cartao(AGUARDANDO, 1), "pt-BR")).toBeNull();
    expect(linhaDoLote(cartao(AGUARDANDO, 1), "pt-BR")).toBe("Lote 1 de 3");
    expect(linhaDoLote(cartao({ tipo: "preparando" }, 0, undefined, 0), "pt-BR")).toBeNull();
  });

  it("o numero no documento vem em linha propria, e o que falta nesta impressora abaixo", () => {
    expect(linhasDoLote(cartao(AGUARDANDO, 2, LOTES), "pt-BR")).toEqual({
      titulo: "Lote 4 do documento",
      detalhe: "mais 1 nesta impressora",
      semImprimir: null,
    });
    expect(linhasDoLote(cartao(AGUARDANDO, 3, LOTES), "en")?.detalhe).toBe("last on this printer");
  });

  it("ao preparar e ao concluir, mostra todos os lotes desta impressora", () => {
    const todos = "Lotes 2, 4, 6 do documento";
    expect(linhasDoLote(cartao({ tipo: "preparando" }, 0, LOTES, 0), "pt-BR")?.titulo).toBe(todos);
    expect(linhasDoLote(cartao({ tipo: "concluido" }, 3, LOTES), "pt-BR")?.titulo).toBe(todos);
  });

  it("no erro e no cancelado, o que ficou sem imprimir vem em linha propria", () => {
    expect(linhasDoLote(cartao(ERRO, 2, LOTES), "pt-BR")).toEqual({
      titulo: "Lote 4 do documento",
      detalhe: null,
      semImprimir: "Sem imprimir: lotes 4, 6",
    });
    expect(linhasDoLote(cartao({ tipo: "cancelado" }, 3, LOTES), "en")?.semImprimir).toBe(
      "Not printed: batch 6",
    );
    expect(linhasDoLote(cartao(ERRO, 0, LOTES, 0), "pt-BR")).toEqual({
      titulo: null,
      detalhe: null,
      semImprimir: "Sem imprimir: lotes 2, 4, 6",
    });
  });

  it("contagem que nao bate com o envio nao afirma numero de documento", () => {
    expect(linhasDoLote(cartao(AGUARDANDO, 2, LOTES, 7), "pt-BR")).toBeNull();
    expect(linhaDoLote(cartao(AGUARDANDO, 2, LOTES, 7), "pt-BR")).toBe("Lote 2 de 7");
  });
});

describe("cartao de proximo lote na divisao", () => {
  const FAIXAS = [
    { lote: 2, inicio: 5, fim: 8 },
    { lote: 4, inicio: 13, fim: 16 },
    { lote: 6, inicio: 21, fim: 24 },
  ];
  const proximo = (simulada: boolean): Cartao => ({
    ...cartao({ tipo: "proximo_lote" }, 2, LOTES),
    simulada,
    faixasNoDocumento: FAIXAS,
  });

  it("manda retirar o maco do lote que acabou, com as paginas", () => {
    const textos = linhasDoCartao(proximo(false), "pt-BR").map((linha) => linha.texto);
    expect(textos).toContain("Retire o maço do lote 2 (págs. 5–8) e guarde-o separado.");
    expect(textos).toContain("Depois, coloque papel em branco.");
    expect(textos).not.toContain(traduzir("pt-BR", "cartao.proximo.semTroca"));
  });

  it("com troca de papel, a troca fica no lugar do papel em branco", () => {
    const papel = { formato: "a5", larguraMm: 148, alturaMm: 210 };
    const textos = linhasDoCartao({ ...proximo(false), trocaDePapel: papel }, "en").map(
      (linha) => linha.texto,
    );
    expect(textos).toContain("Take out the stack of batch 2 (pp. 5–8) and keep it apart.");
    expect(textos).not.toContain("Then load blank paper.");
  });

  it("fora da divisao continua a frase de antes", () => {
    const textos = linhasDoCartao(
      cartao({ tipo: "proximo_lote" }, 2),
      "pt-BR",
    ).map((linha) => linha.texto);
    expect(textos).toContain(traduzir("pt-BR", "cartao.proximo.semTroca"));
  });
});

describe("numeros de lote", () => {
  it("lote fora da lista nao vira numero", () => {
    expect(loteNoDocumento(cartao(AGUARDANDO, 2))).toBeNull();
    expect(loteNoDocumento(cartao(AGUARDANDO, 0, LOTES))).toBeNull();
    expect(loteNoDocumento(cartao(AGUARDANDO, 9, LOTES))).toBeNull();
    expect(loteNoDocumento(cartao(AGUARDANDO, 3, LOTES))).toBe(6);
  });

  it("concluido nao deixa nada para tras", () => {
    expect(lotesSemImprimir(cartao({ tipo: "concluido" }, 3, LOTES))).toEqual([]);
    expect(lotesSemImprimir(cartao(ERRO, 1, LOTES))).toEqual(LOTES);
    expect(lotesSemImprimir(cartao(ERRO, 1))).toEqual([]);
  });

  it("muitos lotes viram comeco, fim e contagem", () => {
    const muitos = Array.from({ length: 30 }, (_, i) => i * 2 + 1);
    expect(listaDeLotes(muitos, "pt-BR")).toBe("1, 3, 5, 7, 9, 11 … 59 (30 no total)");
    expect(listaDeLotes([1, 3], "pt-BR")).toBe("1, 3");
  });
});
