import { describe, expect, it } from "vitest";

// Os CSS de modulo nao voltam como texto sob o teste: a folha e lida do disco.
interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;

describe("rodape em altura curta", () => {
  it("recolhe o secundario sem depender de cartao na lista", () => {
    const css = fs.readFileSync(new URL("./components/Sidebar.module.css", import.meta.url), "utf8");
    const inicio = css.indexOf("@media (max-height: 560px)");
    expect(inicio).toBeGreaterThanOrEqual(0);
    const bloco = css.slice(inicio);
    // Papel de ajuste, folha de controle, grupos e resumo saem nesta altura, mas o botao Mais opcoes
    // os abre: nada fica inalcancavel.
    expect(bloco).toContain(".lateral:not([data-mais-opcoes]) .corpoMaisOpcoes");
    expect(bloco).toContain(".lateral .maisOpcoes");
    // O estado de cartao na lista sumiu com os cartoes: a regra nao pode depender dele.
    expect(bloco).not.toContain("data-com-cartao");
  });
});
