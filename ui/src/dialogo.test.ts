import { describe, expect, it } from "vitest";
import { rolarPeloTeclado } from "./dialogo";

function corpo(scrollTop: number) {
  return { scrollTop, clientHeight: 100, scrollHeight: 400 };
}

describe("rolagem do corpo do dialogo pelo teclado", () => {
  it.each([
    ["ArrowDown", 50, 90],
    ["ArrowUp", 50, 10],
    ["PageDown", 50, 140],
    ["PageUp", 100, 10],
    ["Home", 50, 0],
    ["End", 50, 300],
  ])("%s a partir de %i vai a %i", (tecla, inicio, fim) => {
    const alvo = corpo(inicio);
    expect(rolarPeloTeclado(tecla, alvo)).toBe(true);
    expect(alvo.scrollTop).toBe(fim);
  });

  it("nao passa dos limites", () => {
    const alvo = corpo(290);
    rolarPeloTeclado("PageDown", alvo);
    expect(alvo.scrollTop).toBe(300);
    const topo = corpo(5);
    rolarPeloTeclado("ArrowUp", topo);
    expect(topo.scrollTop).toBe(0);
  });

  it("outras teclas seguem seu caminho", () => {
    const alvo = corpo(50);
    expect(rolarPeloTeclado("Tab", alvo)).toBe(false);
    expect(rolarPeloTeclado("Enter", alvo)).toBe(false);
    expect(alvo.scrollTop).toBe(50);
  });

  it("sem nada para rolar a tecla nao e consumida", () => {
    const alvo = { scrollTop: 0, clientHeight: 100, scrollHeight: 100 };
    expect(rolarPeloTeclado("ArrowDown", alvo)).toBe(false);
    expect(rolarPeloTeclado("ArrowDown", null)).toBe(false);
  });
});
