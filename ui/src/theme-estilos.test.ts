import { describe, expect, it } from "vitest";

// A folha de temas e lida como texto: um seletor de tema que sobrevive nao quebra a compilacao,
// e o app so mostraria as cores erradas com a preferencia do sistema.
async function lerArquivo(caminho: string | URL): Promise<string> {
  const modulofs = "node:fs";
  const fs = (await import(/* @vite-ignore */ modulofs)) as {
    readFileSync(caminho: string | URL, codificacao: "utf8"): string;
  };
  return fs.readFileSync(caminho, "utf8").replace(/\r\n/g, "\n");
}

const folhaDeTemas = () => lerArquivo(new URL("./theme.css", import.meta.url));

describe("folha de temas", () => {
  it("tem o tema cinza e nenhum seletor do tema claro", async () => {
    const folha = await folhaDeTemas();
    expect(folha).toContain("[data-theme='gray']");
    expect(folha).not.toContain("[data-theme='light']");
  });

  it("o aviso de atencao existe nos dois temas", async () => {
    const folha = await folhaDeTemas();
    const ocorrencias = folha.match(/--icon-warning:/g) ?? [];
    expect(ocorrencias).toHaveLength(2);
  });

  it("nenhuma folha de componente guarda regra do tema claro", async () => {
    const modulofs = "node:fs";
    const fs = (await import(/* @vite-ignore */ modulofs)) as {
      readdirSync(caminho: URL, opcoes: { recursive: true }): string[];
    };
    const arquivos = fs
      .readdirSync(new URL("./", import.meta.url), { recursive: true })
      .filter((nome) => nome.endsWith(".css"));
    expect(arquivos.length).toBeGreaterThan(0);
    for (const nome of arquivos) {
      const folha = await lerArquivo(new URL(nome, import.meta.url));
      expect(folha, nome).not.toContain("[data-theme='light']");
    }
  });
});
