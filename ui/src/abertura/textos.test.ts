import { describe, expect, it } from "vitest";
import { traducoes } from "../i18n";
import {
  avisosDoToast,
  fracaoDoProgresso,
  linhaDeEstado,
  mensagemDoAviso,
  textoDoTempo,
  type AvisoAbertura,
} from "./textos";

describe("fracaoDoProgresso", () => {
  it("vai de zero a um", () => {
    expect(fracaoDoProgresso({ concluidas: 0, total: 3 })).toBe(0);
    expect(fracaoDoProgresso({ concluidas: 3, total: 3 })).toBe(1);
  });

  it("nao quebra com valores fora da faixa ou invalidos", () => {
    expect(fracaoDoProgresso({ concluidas: 9, total: 3 })).toBe(1);
    expect(fracaoDoProgresso({ concluidas: -2, total: 3 })).toBe(0);
    expect(fracaoDoProgresso({ concluidas: 1, total: 0 })).toBe(0);
    expect(fracaoDoProgresso({ concluidas: Number.NaN, total: 3 })).toBe(0);
    expect(fracaoDoProgresso({ concluidas: 1, total: Number.POSITIVE_INFINITY })).toBe(0);
  });
});

describe("textoDoTempo", () => {
  it("so aparece a partir de 2 segundos na mesma etapa", () => {
    expect(textoDoTempo(0)).toBeNull();
    expect(textoDoTempo(1999)).toBeNull();
    expect(textoDoTempo(2000)).toBe("2 s");
    expect(textoDoTempo(4700)).toBe("4 s");
  });

  it("valor invalido nao mostra nada", () => {
    expect(textoDoTempo(Number.NaN)).toBeNull();
    expect(textoDoTempo(-5000)).toBeNull();
    expect(textoDoTempo(Number.POSITIVE_INFINITY)).toBeNull();
  });
});

describe("textos da abertura", () => {
  it("a linha de estado traduz cada etapa e ignora etapa desconhecida", () => {
    expect(linhaDeEstado("impressoras")).toBe(traducoes["pt-BR"]["abertura.etapa.impressoras"]);
    expect(linhaDeEstado("componente_impressao")).toBe(
      traducoes["pt-BR"]["abertura.etapa.componente"],
    );
    expect(linhaDeEstado("pronto")).toBe(traducoes["pt-BR"]["abertura.etapa.pronto"]);
    expect(linhaDeEstado("outra")).toBe(traducoes["pt-BR"]["abertura.etapa.impressoras"]);
    expect(linhaDeEstado(undefined)).toBe(traducoes["pt-BR"]["abertura.etapa.impressoras"]);
  });

  it("o aviso de limite tem uma mensagem propria por etapa", () => {
    const porEtapa = ["impressoras", "documento", "componente_impressao"].map((detalhe) =>
      mensagemDoAviso({ codigo: "limite", detalhe }),
    );
    expect(new Set(porEtapa).size).toBe(3);
    expect(porEtapa[0]).toContain("Atualizar lista");
    expect(porEtapa[1]).toContain("Abrir arquivo");
    for (const texto of porEtapa) expect(texto).toContain("10 segundos");
  });

  it("o aviso de limite das impressoras aponta o botao que existe na tela", () => {
    const texto = mensagemDoAviso({ codigo: "limite", detalhe: "impressoras" });
    expect(texto).toContain(`botão ${traducoes["pt-BR"]["lateral.atualizar"]}`);
    expect(texto).toContain(traducoes["pt-BR"]["lateral.titulo"]);
    const ingles = mensagemDoAviso({ codigo: "limite", detalhe: "impressoras" }, "en");
    expect(ingles).toContain(`${traducoes.en["lateral.atualizar"]} button`);
    expect(ingles).toContain(traducoes.en["lateral.titulo"]);
  });

  it("o aviso do componente diz que a previa continua funcionando", () => {
    expect(mensagemDoAviso({ codigo: "componente_impressao" })).toContain("A prévia continua");
    expect(mensagemDoAviso({ codigo: "componente_impressao" }, "en")).toContain(
      "The preview still works",
    );
  });

  it("o componente de impressao nao e chamado de visualizador", () => {
    const texto = mensagemDoAviso({ codigo: "componente_impressao" });
    expect(texto).toContain("pdfium.dll");
    expect(texto.toLowerCase()).not.toContain("visualizador");
  });

  it("todo aviso do toast tem texto sem marcador solto", () => {
    for (const codigo of ["componente_impressao", "limite"] as const) {
      expect(mensagemDoAviso({ codigo, detalhe: "x" })).not.toMatch(/\{\w+\}/);
    }
  });
});

describe("avisosDoToast", () => {
  const todos: AvisoAbertura[] = [
    { codigo: "filas" },
    { codigo: "perfis" },
    { codigo: "documento", detalhe: "a.pdf" },
    { codigo: "componente_impressao" },
    { codigo: "limite", detalhe: "documento" },
  ];

  it("so fica o que nenhuma outra tela mostra", () => {
    expect(avisosDoToast(todos).map((aviso) => aviso.codigo)).toEqual([
      "componente_impressao",
      "limite",
    ]);
  });

  it("falhas que a barra lateral e o banner ja mostram nao viram toast", () => {
    expect(avisosDoToast([{ codigo: "filas" }, { codigo: "perfis" }])).toEqual([]);
    expect(avisosDoToast([{ codigo: "documento", detalhe: "a.pdf" }])).toEqual([]);
    expect(avisosDoToast([])).toEqual([]);
  });

  it("codigo desconhecido vindo do programa nao aparece", () => {
    const desconhecido = [{ codigo: "outro" }] as unknown as AvisoAbertura[];
    expect(avisosDoToast(desconhecido)).toEqual([]);
  });
});
