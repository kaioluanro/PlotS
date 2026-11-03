import { describe, expect, it } from "vitest";

// Os CSS de modulo nao voltam como texto sob o teste: a folha e lida do disco.
interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;

function folha(caminho: string): string {
  return fs.readFileSync(new URL(caminho, import.meta.url), "utf8");
}

describe("indicador de carregamento em movimento reduzido", () => {
  // A regra geral do tema zera toda animacao: sem esta excecao, o giro para e parece travado.
  it.each([
    ["lateral", "./components/Sidebar.module.css"],
    ["visualizador", "./visualizador/Visualizador.module.css"],
  ])("na %s, o giro continua", (_, caminho) => {
    expect(folha(caminho)).toMatch(
      /@media \(prefers-reduced-motion: reduce\)\s*\{\s*\.girando\s*\{\s*animation-duration:\s*900ms\s*!important/,
    );
  });
});
