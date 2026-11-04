import { describe, expect, it } from "vitest";

// Os campos de Páginas usam classes da folha da lateral. Quando alguma some, o campo perde o
// tamanho, a borda de erro e o contorno de cores forçadas sem nenhum aviso de compilação.
async function lerArquivo(caminho: string): Promise<string> {
  const modulofs = "node:fs";
  const fs = (await import(/* @vite-ignore */ modulofs)) as {
    readFileSync(caminho: URL, codificacao: "utf8"): string;
  };
  return fs.readFileSync(new URL(caminho, import.meta.url), "utf8").replace(/\r\n/g, "\n");
}

const folhaDaLateral = () => lerArquivo("./components/Sidebar.module.css");
const fonteDoBloco = () => lerArquivo("./components/BlocoIntervalo.tsx");

const regra = (folha: string, seletor: string) =>
  new RegExp(`(?:^|\\n)${seletor.replace(/\./g, "\\.")}[^{]*\\{([^}]*)\\}`).exec(folha)?.[1] ?? "";

describe("estilos dos campos de Páginas", () => {
  it("cada classe da lateral usada pelo bloco de Páginas existe na folha da lateral", async () => {
    const folha = await folhaDaLateral();
    const fonte = await fonteDoBloco();
    const usadas = new Set(
      [...fonte.matchAll(/styles\.([A-Za-z]+)/g)].map((casamento) => casamento[1]),
    );
    expect(usadas.has("campoLote")).toBe(true);
    for (const nome of usadas) {
      expect(folha, `classe ${nome}`).toMatch(new RegExp(`\\.${nome}\\b`));
    }
  });

  it("o campo tem 56 por 32 px, números tabulares à direita e borda de erro", async () => {
    const folha = await folhaDaLateral();
    const base = regra(folha, ".campoLote");
    expect(base).toMatch(/width: 56px/);
    expect(base).toMatch(/height: 32px/);
    expect(base).toMatch(/text-align: right/);
    expect(base).toMatch(/font-variant-numeric: tabular-nums/);
    expect(regra(folha, ".campoInvalido")).toMatch(/--status-offline\)/);
    expect(regra(folha, ".erroLote")).toMatch(/--status-offline-text/);
  });

  it("cores forçadas mantêm o foco e a borda de erro visíveis", async () => {
    const folha = await folhaDaLateral();
    const inicio = folha.indexOf("@media (forced-colors: active)");
    const fim = folha.indexOf("\n}\n", inicio);
    const bloco = inicio < 0 || fim < 0 ? "" : folha.slice(inicio, fim);
    expect(bloco).toMatch(/\.campoLote:focus-visible:not\(:disabled\)/);
    expect(bloco).toMatch(/outline: 2px solid Highlight/);
    expect(bloco).toMatch(/\.campoInvalido,\n\s*\.campoInvalido:hover:not\(:disabled\) \{\n\s*border-color: Highlight;/);
  });

  it("o campo de quantidade usa a linha de Páginas e esconde a dica em altura curta", async () => {
    const componente = await lerArquivo("./components/CampoCopias.tsx");
    expect(componente).toContain("lote.loteLinha");
    expect(componente).toContain("lote.campoLote");
    const folha = await lerArquivo("./components/CampoCopias.module.css");
    expect(folha).toMatch(/@media \(max-height: 560px\) \{\s*\.dica \{\s*display: none;/);
  });
});
