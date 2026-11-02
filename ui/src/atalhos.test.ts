import { describe, expect, it } from "vitest";
import { deveBloquearTecla, interpretarAtalho } from "./atalhos";

const tecla = (key: string, extra: Partial<{ ctrlKey: boolean; shiftKey: boolean; altKey: boolean }> = {}) => ({
  key,
  ctrlKey: true,
  shiftKey: false,
  altKey: false,
  ...extra,
});

describe("interpretarAtalho", () => {
  it("reconhece cada atalho, com ou sem caps lock", () => {
    expect(interpretarAtalho(tecla("o"))).toBe("abrir");
    expect(interpretarAtalho(tecla("O"))).toBe("abrir");
    expect(interpretarAtalho(tecla("t"))).toBe("abrir");
    expect(interpretarAtalho(tecla("w"))).toBe("fechar");
    expect(interpretarAtalho(tecla("Tab"))).toBe("trocar");
    expect(interpretarAtalho(tecla("b"))).toBe("lateral");
  });

  it("ignora tecla sem Ctrl", () => {
    expect(interpretarAtalho(tecla("w", { ctrlKey: false }))).toBeNull();
    expect(interpretarAtalho(tecla("Tab", { ctrlKey: false }))).toBeNull();
  });

  it("ignora combinacoes com Shift ou Alt", () => {
    expect(interpretarAtalho(tecla("w", { shiftKey: true }))).toBeNull();
    expect(interpretarAtalho(tecla("Tab", { shiftKey: true }))).toBeNull();
    expect(interpretarAtalho(tecla("b", { altKey: true }))).toBeNull();
  });

  it("ignora outras teclas com Ctrl", () => {
    expect(interpretarAtalho(tecla("p"))).toBeNull();
  });
});

describe("deveBloquearTecla", () => {
  it("bloqueia o que recarrega ou imprime a pagina da janela", () => {
    expect(deveBloquearTecla(tecla("r"))).toBe(true);
    expect(deveBloquearTecla(tecla("R", { shiftKey: true }))).toBe(true);
    expect(deveBloquearTecla(tecla("p"))).toBe(true);
    expect(deveBloquearTecla(tecla("F5", { ctrlKey: false }))).toBe(true);
    expect(deveBloquearTecla(tecla("F5"))).toBe(true);
  });

  it("deixa passar o resto", () => {
    expect(deveBloquearTecla(tecla("r", { ctrlKey: false }))).toBe(false);
    expect(deveBloquearTecla(tecla("p", { ctrlKey: false }))).toBe(false);
    expect(deveBloquearTecla(tecla("w"))).toBe(false);
  });
});
