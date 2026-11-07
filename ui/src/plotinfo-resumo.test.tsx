import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { formatarMl, formatarMoeda, type ResumoDaBarra } from "./analise-tinta";
import { BotaoDoResumo, Visualizador, textoDoResumo } from "./visualizador/Visualizador";
import { traduzir } from "./i18n";

const RESUMO: ResumoDaBarra = {
  totalPaginas: 4,
  ml: 0.34,
  valor: 1.12,
  moeda: "BRL",
};

function rotulo(): string {
  return traduzir("pt-BR", "plotinfo.resumo.rotulo", {
    paginas: "4",
    ml: formatarMl(0.34, "pt-BR"),
    custo: formatarMoeda(1.12, "BRL", "pt-BR"),
  });
}

describe("resumo na barra do visualizador", () => {
  it("escreve paginas, mL e custo no formato curto", () => {
    expect(textoDoResumo(RESUMO)).toBe(
      traduzir("pt-BR", "plotinfo.resumo.curto", {
        paginas: "4",
        ml: formatarMl(0.34, "pt-BR"),
        custo: formatarMoeda(1.12, "BRL", "pt-BR"),
      }),
    );
    expect(textoDoResumo(RESUMO)).toContain("4 pág");
    expect(textoDoResumo(RESUMO)).toContain("0,34 mL");
    expect(textoDoResumo(RESUMO)).toContain("R$");
  });

  it("e um botao que abre a gaveta, com nome acessivel e o icone da calculadora", () => {
    const html = renderToStaticMarkup(<BotaoDoResumo resumo={RESUMO} aoAbrir={() => undefined} />);
    expect(html).toContain(`aria-label="${rotulo()}"`);
    expect(html).toContain(`title="${rotulo()}"`);
    expect(html).toContain('aria-controls="gaveta-plotinfo"');
    expect(html).toContain("lucide-calculator");
  });

  it("aparece na barra do visualizador so quando ha resumo", () => {
    const documento = { caminho: "x.pdf", nome: "x.pdf", tipo: "pdf" as const };
    const comResumo = renderToStaticMarkup(
      <Visualizador documento={documento} resumoTinta={RESUMO} aoAbrirTinta={() => undefined} />,
    );
    expect(comResumo).toContain("4 pág");
    expect(comResumo).toContain('aria-controls="gaveta-plotinfo"');
    const semResumo = renderToStaticMarkup(<Visualizador documento={documento} />);
    expect(semResumo).not.toContain('aria-controls="gaveta-plotinfo"');
  });
});