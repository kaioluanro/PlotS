import { describe, expect, it } from "vitest";
import { rotuloDoRegistro, type RegistroDoHistorico } from "./historico";

function registro(mudancas: Partial<RegistroDoHistorico> = {}): RegistroDoHistorico {
  return {
    arquivo: "a.pdf",
    data: 1_700_000_000,
    impressoras: ["A"],
    paginas: 4,
    lotes: 1,
    resultado: "concluido",
    lotes_faltando: 0,
    ...mudancas,
  };
}

describe("rotulo de Frente, Verso ou duas faces no historico", () => {
  it("frente vira Frentes, e em ingles Fronts", () => {
    expect(rotuloDoRegistro(registro({ face: "frente" }), "pt-BR")).toBe("Frentes");
    expect(rotuloDoRegistro(registro({ face: "frente" }), "en")).toBe("Fronts");
  });

  it("verso vira Versos, e em ingles Backs", () => {
    expect(rotuloDoRegistro(registro({ face: "verso" }), "pt-BR")).toBe("Versos");
    expect(rotuloDoRegistro(registro({ face: "verso" }), "en")).toBe("Backs");
  });

  it("frente que saiu de uma impressora automatica, de uma vez nas duas faces, vira Duas faces", () => {
    const automatico = [{ impressora: "A", milissegundos: 1000, folhas: 2, resultado: "concluido" as const, automatico: true }];
    expect(rotuloDoRegistro(registro({ face: "frente", tempos: automatico }), "pt-BR")).toBe("Duas faces");
    expect(rotuloDoRegistro(registro({ face: "frente", tempos: automatico }), "en")).toBe("Both sides");
  });

  it("frente mista, com manual e automatica, continua Frentes", () => {
    const mistos = [
      { impressora: "A", milissegundos: 1000, folhas: 2, resultado: "concluido" as const, automatico: true },
      { impressora: "B", milissegundos: 1000, folhas: 2, resultado: "concluido" as const },
    ];
    expect(rotuloDoRegistro(registro({ face: "frente", tempos: mistos }), "pt-BR")).toBe("Frentes");
  });

  it("registro antigo, sem face, nao ganha rotulo", () => {
    expect(rotuloDoRegistro(registro(), "pt-BR")).toBeNull();
    expect(rotuloDoRegistro(registro(), "en")).toBeNull();
  });
});
