import { describe, expect, it } from "vitest";

// Os CSS de modulo nao voltam como texto sob o teste: a folha e lida do disco.
interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;

const folha = fs.readFileSync(new URL("./Visualizador.module.css", import.meta.url), "utf8");
const cores = folha.slice(folha.indexOf("@media (forced-colors: active)"));

/** Corpo da primeira regra com este seletor exato, ou vazio se ela nao existe no trecho. */
function regra(trecho: string, seletor: string): string {
  const inicio = trecho.indexOf(`${seletor} {`);
  if (inicio < 0) return "";
  return trecho.slice(inicio, trecho.indexOf("}", inicio));
}

describe("folha do visualizador em cores forcadas", () => {
  it("o fundo do estado nao cobre o icone do botao", () => {
    expect(regra(cores, ".botao::before")).toContain("background: transparent !important");
  });

  it("o foco de teclado em botao marcado continua visivel sobre o destaque", () => {
    const corpo = regra(cores, ".botao[aria-pressed='true']:focus-visible");
    expect(corpo).toContain("outline-color: HighlightText");
    expect(corpo).toContain("outline-offset: -5px");
  });
});

describe("texto de erro da pagina", () => {
  it("quebra as palavras da instrucao em vez de corta-las", () => {
    const corpo = regra(folha, ".textoErroDaPagina");
    expect(corpo).toContain("white-space: normal");
    expect(corpo).toContain("overflow-wrap: anywhere");
    expect(corpo).toContain("text-align: left");
    expect(corpo).toContain("min-width: 0");
    expect(corpo).not.toMatch(/overflow:|text-overflow|nowrap/);
  });
});
