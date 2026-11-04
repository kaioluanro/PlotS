import { describe, expect, it } from "vitest";
import { escolherIdioma, traduzir, traducoes, type Chave } from "./i18n";

function parametros(texto: string): string[] {
  return [...texto.matchAll(/\{(\w+)\}/g)].map((achado) => achado[1] ?? "").sort();
}

describe("traducoes", () => {
  const chavesPt = Object.keys(traducoes["pt-BR"]).sort();
  const chavesEn = Object.keys(traducoes.en).sort();

  it("toda chave existe nos dois idiomas", () => {
    expect(chavesEn).toEqual(chavesPt);
    expect(chavesPt.length).toBeGreaterThan(0);
  });

  it("nenhum texto esta vazio e os parametros coincidem", () => {
    for (const chave of chavesPt as Chave[]) {
      expect(traducoes["pt-BR"][chave].trim(), chave).not.toBe("");
      expect(traducoes.en[chave].trim(), chave).not.toBe("");
      expect(parametros(traducoes.en[chave]), chave).toEqual(parametros(traducoes["pt-BR"][chave]));
    }
  });
});

describe("escolherIdioma", () => {
  it("usa ingles para qualquer variante de ingles", () => {
    expect(escolherIdioma("en")).toBe("en");
    expect(escolherIdioma("en-US")).toBe("en");
    expect(escolherIdioma("EN-gb")).toBe("en");
  });

  it("usa portugues nas variantes de portugues", () => {
    expect(escolherIdioma("pt-BR")).toBe("pt-BR");
    expect(escolherIdioma("pt-PT")).toBe("pt-BR");
  });

  it("cai em portugues para idioma sem traducao ou ausente", () => {
    expect(escolherIdioma("fr-FR")).toBe("pt-BR");
    expect(escolherIdioma("")).toBe("pt-BR");
    expect(escolherIdioma(undefined)).toBe("pt-BR");
    expect(escolherIdioma("english")).toBe("pt-BR");
  });
});

describe("traduzir", () => {
  it("devolve o texto do idioma pedido", () => {
    expect(traduzir("en", "barra.fechar")).toBe(traducoes.en["barra.fechar"]);
    expect(traduzir("pt-BR", "barra.fechar")).toBe(traducoes["pt-BR"]["barra.fechar"]);
  });

  it("substitui parametros, inclusive com caracteres especiais no valor", () => {
    expect(traduzir("pt-BR", "aba.fechar", { nome: "Relatório $& final.pdf" })).toContain(
      "Relatório $& final.pdf",
    );
  });

  it("mantem o marcador quando o parametro nao vem", () => {
    expect(traduzir("en", "aba.fechar")).toContain("{nome}");
  });
});

describe("chaves retiradas", () => {
  it("a chave sem uso do documento nao existe mais em nenhum idioma", () => {
    expect(Object.keys(traducoes["pt-BR"])).not.toContain("documento.semVisualizacao");
    expect(Object.keys(traducoes.en)).not.toContain("documento.semVisualizacao");
  });
});

describe("texto sem a palavra cartao", () => {
  it("nenhum texto da interface cita cartao, nos dois idiomas", () => {
    for (const idioma of ["pt-BR", "en"] as const) {
      for (const [chave, texto] of Object.entries(traducoes[idioma])) {
        expect(texto, `${idioma} ${chave}`).not.toMatch(/cart(ã|a)o|cart(õ|o)es|cards?/i);
      }
    }
  });
});
