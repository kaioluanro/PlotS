import { describe, expect, it } from "vitest";

describe("botao desabilitado sob cores forcadas", () => {
  it("mostra o desabilitado por estilo de borda, e nao so pela cor, que pode ser igual a do habilitado", async () => {
    const modulofs = "node:fs";
    const fs = (await import(/* @vite-ignore */ modulofs)) as {
      readFileSync(caminho: URL, codificacao: "utf8"): string;
    };
    const css = fs.readFileSync(new URL("./components/Botao.module.css", import.meta.url), "utf8");
    const bloco = css.slice(css.indexOf("@media (forced-colors: active)"));
    const regra = bloco.slice(bloco.indexOf(".botao:disabled"), bloco.indexOf("}", bloco.indexOf(".botao:disabled")));
    expect(regra).toMatch(/border-style:\s*dashed/);
    expect(bloco).toContain(".botao:hover:not(:disabled)");
    expect(bloco).not.toMatch(/\.botao:hover \{/);
  });
});
