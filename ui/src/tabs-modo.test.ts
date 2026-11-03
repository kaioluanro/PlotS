import { describe, expect, it } from "vitest";
import type { Cartao, EstadoCartao } from "./cartoes";
import {
  estadoInicial,
  modoDoEnvio,
  reduzirAbas,
  type Aba,
  type DivisaoDoEnvio,
  type EstadoAbas,
} from "./tabs";

function aberta(): EstadoAbas {
  return reduzirAbas(estadoInicial, {
    tipo: "abrir",
    documento: { caminho: "a.pdf", nome: "a.pdf" },
  });
}

function aba(estado: EstadoAbas): Aba {
  const primeira = estado.abas[0];
  if (!primeira) throw new Error("sem aba");
  return primeira;
}

function cartao(impressora: string, estado: EstadoCartao, lote = 1): Cartao {
  return {
    aba: 1,
    impressora,
    nome: impressora,
    simulada: true,
    lote,
    total: 3,
    papel: null,
    estado,
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
  };
}

const DIVISAO: DivisaoDoEnvio = {
  partes: [
    { impressora: "a", nome: "A", lotes: [1, 3, 5], viraParaLer: false },
    { impressora: "b", nome: "B", lotes: [2, 4, 6], viraParaLer: true },
  ],
};

describe("modo de distribuicao da aba", () => {
  it("guarda a escolha de papel somente na aba correspondente", () => {
    let estado = aberta();
    expect(aba(estado).papelDeAjuste).toBeNull();
    estado = reduzirAbas(estado, { tipo: "definir_papel_ajuste", id: 1, papel: "a4" });
    estado = reduzirAbas(estado, { tipo: "abrir", documento: { caminho: "b.pdf", nome: "b.pdf" } });
    expect(estado.abas.map((item) => item.papelDeAjuste)).toEqual(["a4", null]);
  });
  it("comeca em espelho e cada aba guarda o seu", () => {
    let estado = aberta();
    expect(aba(estado).modo).toBe("espelho");
    estado = reduzirAbas(estado, { tipo: "definir_modo", id: 1, modo: "divisao" });
    estado = reduzirAbas(estado, { tipo: "abrir", documento: { caminho: "b.pdf", nome: "b.pdf" } });
    expect(estado.abas.map((item) => item.modo)).toEqual(["divisao", "espelho"]);
  });

  it("com menos de duas que recebem, o modo nao altera o envio", () => {
    expect(modoDoEnvio("divisao", 0)).toBe("espelho");
    expect(modoDoEnvio("divisao", 1)).toBe("espelho");
    expect(modoDoEnvio("divisao", 2)).toBe("divisao");
    expect(modoDoEnvio("espelho", 3)).toBe("espelho");
  });

  it("aplicar um grupo troca a selecao e o modo so daquela aba", () => {
    let estado = reduzirAbas(aberta(), { tipo: "alternar_impressora", id: 1, impressora: "x" });
    estado = reduzirAbas(estado, {
      tipo: "aplicar_grupo",
      id: 1,
      impressoras: ["a", "b"],
      modo: "divisao",
    });
    expect(aba(estado).impressorasMarcadas).toEqual(["a", "b"]);
    expect(aba(estado).modo).toBe("divisao");
    const igual = reduzirAbas(estado, {
      tipo: "aplicar_grupo",
      id: 99,
      impressoras: [],
      modo: "espelho",
    });
    expect(igual).toEqual(estado);
  });
});

describe("numeros dos lotes nos cartoes", () => {
  it("cartao de impressora da divisao recebe os numeros do documento", () => {
    let estado = reduzirAbas(aberta(), { tipo: "divisao_enviada", id: 1, divisao: DIVISAO });
    estado = reduzirAbas(estado, {
      tipo: "cartao",
      cartao: cartao("b", { tipo: "imprimindo", passada: "frente" }),
    });
    expect(aba(estado).cartoes[0]?.lotesNoDocumento).toEqual([2, 4, 6]);
  });

  it("no espelho o cartao fica como veio", () => {
    let estado = reduzirAbas(aberta(), { tipo: "divisao_enviada", id: 1, divisao: null });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao("a", { tipo: "aguardando" }) });
    expect(aba(estado).cartoes[0]?.lotesNoDocumento).toBeUndefined();
  });

  it("um trabalho que ainda corre continua com os numeros depois de um envio novo a outra", () => {
    let estado = reduzirAbas(aberta(), { tipo: "divisao_enviada", id: 1, divisao: DIVISAO });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao("a", { tipo: "aguardando" }) });
    estado = reduzirAbas(estado, { tipo: "divisao_enviada", id: 1, divisao: null });
    estado = reduzirAbas(estado, {
      tipo: "cartao",
      cartao: cartao("a", { tipo: "proximo_lote" }, 2),
    });
    expect(aba(estado).cartoes[0]?.lotesNoDocumento).toEqual([1, 3, 5]);
  });

  it("nenhum envio apaga a divisao enquanto ela corre", () => {
    let estado = reduzirAbas(aberta(), { tipo: "divisao_enviada", id: 1, divisao: DIVISAO });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao("a", { tipo: "aguardando" }) });
    estado = reduzirAbas(estado, { tipo: "divisao_enviada", id: 1, divisao: null });
    expect(aba(estado).divisao).toEqual(DIVISAO);
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao("a", { tipo: "concluido" }, 3) });
    estado = reduzirAbas(estado, { tipo: "divisao_enviada", id: 1, divisao: null });
    expect(aba(estado).divisao).toBeNull();
  });

  it("o envio novo em espelho a mesma impressora comeca sem os numeros antigos", () => {
    let estado = reduzirAbas(aberta(), { tipo: "divisao_enviada", id: 1, divisao: DIVISAO });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao("a", { tipo: "concluido" }, 3) });
    estado = reduzirAbas(estado, { tipo: "divisao_enviada", id: 1, divisao: null });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao("a", { tipo: "preparando" }, 0) });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao("a", { tipo: "aguardando" }) });
    expect(aba(estado).cartoes[0]?.lotesNoDocumento).toBeUndefined();
  });

  it("dispensar guarda o que faltava naquela impressora", () => {
    let estado = reduzirAbas(aberta(), { tipo: "divisao_enviada", id: 1, divisao: DIVISAO });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao("a", { tipo: "cancelado" }, 2) });
    estado = reduzirAbas(estado, { tipo: "dispensar_cartao", id: 1, impressora: "a" });
    expect(aba(estado).cartoes).toEqual([]);
    expect(aba(estado).divisao?.partes[0]?.semImprimirAoDispensar).toEqual([3, 5]);
    expect(aba(estado).divisao?.partes[1]?.semImprimirAoDispensar).toBeUndefined();
  });
});
