import { describe, expect, it } from "vitest";

// Os CSS de modulo nao voltam como texto sob o teste: a folha e lida do disco.
interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;

describe("grade da janela principal", () => {
  it("tem uma trilha para cada filho direto: barra e corpo", () => {
    const css = fs.readFileSync(new URL("./App.module.css", import.meta.url), "utf8");
    const bloco = /\.app\s*\{([^}]*)\}/.exec(css);
    expect(bloco).not.toBeNull();
    const trilhas = /grid-template-rows:\s*([^;]+);/.exec(bloco?.[1] ?? "")?.[1]?.trim();
    // A faixa de avisos tem altura zero e vive dentro da area do documento: so barra e corpo.
    expect(trilhas).toBe("auto minmax(0, 1fr)");
  });

  it("a area do documento e a caixa da faixa, para o aviso nao cobrir a lista", () => {
    const css = fs.readFileSync(new URL("./App.module.css", import.meta.url), "utf8");
    const bloco = /\.areaDocumento\s*\{([^}]*)\}/.exec(css);
    expect(bloco).not.toBeNull();
    expect(bloco?.[1]).toMatch(/position:\s*relative/);
    expect(bloco?.[1]).toMatch(/flex:\s*1 1 0/);
  });
});

describe("largura do nome da fila na faixa", () => {
  it("a fila mede pelo conteudo e cede ate 60%: o nome longo nao passa da faixa", () => {
    const css = fs.readFileSync(
      new URL("./components/AvisosDoTopo.module.css", import.meta.url),
      "utf8",
    );
    const fila = /\.fila\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(fila).toMatch(/flex:\s*0 0 auto/);
    // `min-width: max-content` anulava o teto: a fila ficava maior que os 60% da faixa.
    expect(fila).toMatch(/width:\s*max-content/);
    expect(fila).toMatch(/min-width:\s*0/);
    expect(fila).toMatch(/max-width:\s*60%/);
    const documento = /\.documento\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    // O documento cede primeiro, mais depressa que o estado; nunca abaixo de 48px.
    expect(documento).toMatch(/flex:\s*0\s+20\s+auto/);
    expect(documento).toMatch(/min-width:\s*48px/);
  });

  it("o estado cede com reticencias, para o titulo nao passar por baixo do x", () => {
    const css = fs.readFileSync(
      new URL("./components/AvisosDoTopo.module.css", import.meta.url),
      "utf8",
    );
    const estado = /\.estado\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(estado).toMatch(/flex:\s*0 1 auto/);
    expect(estado).toMatch(/min-width:\s*0/);
    expect(estado).toMatch(/overflow:\s*hidden/);
    expect(estado).toMatch(/text-overflow:\s*ellipsis/);
    expect(estado).toMatch(/white-space:\s*nowrap/);
    const fechar = /\.fechar\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(fechar).toMatch(/flex-shrink:\s*0/);
    expect(fechar).toMatch(/width:\s*28px/);
    expect(fechar).toMatch(/height:\s*28px/);
  });
});
