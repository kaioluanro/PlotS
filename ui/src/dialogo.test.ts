import { describe, expect, it } from "vitest";
import { deveRolarPeloTeclado, origemDoDialogo, rolarPeloTeclado } from "./dialogo";

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

  it("controle interativo usa a tecla para o proprio valor, nao para rolar", () => {
    const dentro = { closest: () => ({}) };
    for (const alvo of [dentro, { closest: () => ({}) }, { closest: () => ({}) }]) {
      expect(deveRolarPeloTeclado(alvo)).toBe(false);
    }
    expect(deveRolarPeloTeclado({ closest: () => null })).toBe(true);
    expect(deveRolarPeloTeclado(null)).toBe(true);
  });

  it("o seletor cobre select, input, button, link, textarea e papeis de controle", () => {
    let visto = "";
    deveRolarPeloTeclado({
      closest: (seletor: string) => {
        visto = seletor;
        return null;
      },
    });
    for (const trecho of [
      "select",
      "input",
      "button",
      "a",
      "textarea",
      '[role="slider"]',
      '[role="radio"]',
      '[role="listbox"]',
    ]) {
      expect(visto).toContain(trecho);
    }
  });
});

describe("origem do foco de um dialogo que reabre", () => {
  const sino = { isConnected: true, nome: "sino" };
  const corpoDaPagina = { isConnected: true, nome: "corpo" };

  it("mantem a origem da primeira abertura quando o foco ja caiu no corpo", () => {
    expect(origemDoDialogo(sino, corpoDaPagina)).toBe(sino);
  });

  it("na primeira abertura, vale quem tem o foco", () => {
    expect(origemDoDialogo(null, sino)).toBe(sino);
    expect(origemDoDialogo(null, null)).toBeNull();
  });

  it("origem que saiu da pagina nao serve: vale quem tem o foco agora", () => {
    expect(origemDoDialogo({ isConnected: false, nome: "velho" }, sino)).toBe(sino);
  });
});
