import { afterEach, describe, expect, it, vi } from "vitest";
import { acompanharTemaDoSistema } from "./theme";

/** O sistema emulado: o `matchMedia` que a tela do app consulta e o atributo que ela escreve. */
function sistema(prefereClaro: boolean) {
  const atributos = new Map<string, string>();
  const ouvintes: Array<() => void> = [];
  let clara = prefereClaro;
  vi.stubGlobal("document", {
    documentElement: {
      setAttribute: (nome: string, valor: string) => atributos.set(nome, valor),
    },
  });
  vi.stubGlobal("window", {
    matchMedia: () => ({
      get matches() {
        return clara;
      },
      addEventListener: (_evento: string, ouvinte: () => void) => ouvintes.push(ouvinte),
    }),
  });
  return {
    tema: () => atributos.get("data-theme"),
    trocarPreferencia: (valor: boolean) => {
      clara = valor;
      for (const ouvinte of ouvintes) ouvinte();
    },
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("tema do sistema", () => {
  it("a preferencia clara do sistema usa o cinza", () => {
    const emulado = sistema(true);
    acompanharTemaDoSistema();
    expect(emulado.tema()).toBe("gray");
  });

  it("a preferencia escura continua no escuro e a troca do sistema acompanha", () => {
    const emulado = sistema(false);
    acompanharTemaDoSistema();
    expect(emulado.tema()).toBe("dark");
    emulado.trocarPreferencia(true);
    expect(emulado.tema()).toBe("gray");
  });
});
