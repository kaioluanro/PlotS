import { describe, expect, it } from "vitest";
import {
  codigoDaFalhaDasOpcoes,
  coresOferecidas,
  destinoDaFalha,
  escolhasIniciais,
  mesmaResolucao,
  rotuloDaResolucao,
  temOpcoes,
  type OpcoesDaImpressora,
} from "./opcoes-impressora";

const COMPLETAS: OpcoesDaImpressora = {
  bandejas: [
    { valor: 1, nome: "Bandeja 1" },
    { valor: 2, nome: "Bandeja 2" },
  ],
  resolucoes: [
    { x: 600, y: 600 },
    { x: 300, y: 300 },
  ],
  cor: "colorida",
  atual: {
    bandeja: 2,
    resolucao: { x: 300, y: 300 },
    cor: "monocromatica",
  },
};

describe("opcoes que o driver declara", () => {
  it("uma impressora de cor oferece as duas cores; uma so de preto nao ganha a cor", () => {
    expect(coresOferecidas("colorida")).toEqual(["colorida", "monocromatica"]);
    expect(coresOferecidas("monocromatica")).toEqual(["monocromatica"]);
    expect(coresOferecidas(null)).toEqual([]);
  });

  it("a resolucao aparece com os dois lados e a unidade", () => {
    expect(rotuloDaResolucao({ x: 600, y: 300 })).toBe("600 × 300 dpi");
  });

  it("so ha painel quando o driver declarou alguma das tres opcoes", () => {
    expect(temOpcoes(COMPLETAS)).toBe(true);
    expect(
      temOpcoes({ bandejas: [], resolucoes: [], cor: null, atual: COMPLETAS.atual }),
    ).toBe(false);
    expect(
      temOpcoes({ bandejas: [], resolucoes: [], cor: "monocromatica", atual: COMPLETAS.atual }),
    ).toBe(true);
  });

  it("os valores atuais viram escolhas quando o driver ainda os declara", () => {
    expect(escolhasIniciais(COMPLETAS)).toEqual({
      bandeja: 2,
      resolucao: { x: 300, y: 300 },
      cor: "monocromatica",
    });
  });

  it("um ajuste antigo que aponta para uma opcao que sumiu comeca no padrao", () => {
    const antigas: OpcoesDaImpressora = {
      ...COMPLETAS,
      atual: {
        bandeja: 9,
        resolucao: { x: 1200, y: 1200 },
        cor: "monocromatica",
      },
      cor: "monocromatica",
    };
    expect(escolhasIniciais(antigas)).toEqual({
      bandeja: null,
      resolucao: null,
      cor: "monocromatica",
    });
  });

  it("cor que o driver nao aceita mais nao fica marcada", () => {
    const soPreto: OpcoesDaImpressora = {
      ...COMPLETAS,
      cor: "monocromatica",
      atual: { ...COMPLETAS.atual, cor: "colorida" },
    };
    expect(escolhasIniciais(soPreto).cor).toBeNull();
  });

  it("comparar resolucoes nao confunde 600x300 com 300x600", () => {
    expect(mesmaResolucao({ x: 600, y: 300 }, { x: 600, y: 300 })).toBe(true);
    expect(mesmaResolucao({ x: 600, y: 300 }, { x: 300, y: 600 })).toBe(false);
    expect(mesmaResolucao(null, null)).toBe(true);
    expect(mesmaResolucao({ x: 600, y: 600 }, null)).toBe(false);
  });
});

describe("codigos de falha do painel", () => {
  it("mantem os codigos que o backend manda", () => {
    for (const codigo of [
      "fila_sumiu",
      "opcao_recusada",
      "nao_abriu",
      "nao_gravou",
      "grande",
    ] as const) {
      expect(codigoDaFalhaDasOpcoes({ codigo, fila: "Sala" })).toBe(codigo);
    }
  });

  it("o que nao tem a forma esperada vira recusa do driver", () => {
    expect(codigoDaFalhaDasOpcoes(null)).toBe("opcao_recusada");
    expect(codigoDaFalhaDasOpcoes("falhou")).toBe("opcao_recusada");
    expect(codigoDaFalhaDasOpcoes({ codigo: "outro", fila: "Sala" })).toBe("opcao_recusada");
  });

  it("fila que sumiu fecha o painel; as outras falhas ficam no corpo", () => {
    expect(destinoDaFalha("fila_sumiu")).toBe("fechar");
    for (const codigo of ["opcao_recusada", "nao_abriu", "nao_gravou", "grande"] as const) {
      expect(destinoDaFalha(codigo)).toBe("mostrar");
    }
  });
});
