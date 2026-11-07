import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { TitleBar } from "./components/TitleBar";
import { traduzir } from "./i18n";

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    isMaximized: () => Promise.resolve(false),
    onResized: () => Promise.resolve(() => undefined),
    minimize: () => Promise.resolve(),
    toggleMaximize: () => Promise.resolve(),
    close: () => Promise.resolve(),
  }),
}));

function barra(plotInfoAberto: boolean): string {
  return renderToStaticMarkup(
    <TitleBar
      abas={[]}
      ativa={null}
      lateralAberta
      aoAlternarLateral={() => undefined}
      aoAtivar={() => undefined}
      aoFechar={() => undefined}
      aoAbrir={() => undefined}
      aoAbrirHistorico={() => undefined}
      naoLidos={0}
      aoAbrirAvisos={() => undefined}
      plotInfoAberto={plotInfoAberto}
      aoAlternarPlotInfo={() => undefined}
      idLateral="lateral"
      idPainel="painel"
    />,
  );
}

describe("botao de tinta na barra de titulo", () => {
  it("fica ao lado do sino, com id, rotulo e o icone da calculadora", () => {
    const html = barra(false);
    expect(html).toContain('id="botao-plotinfo"');
    expect(html).toContain(`aria-label="${traduzir("pt-BR", "plotinfo.abrir")}"`);
    expect(html).toContain(`title="${traduzir("pt-BR", "plotinfo.abrir")}"`);
    expect(html).toContain("lucide-calculator");
    expect(html.indexOf("botao-plotinfo")).toBeLessThan(
      html.indexOf(`aria-label="${traduzir("pt-BR", "avisos.abrir")}"`),
    );
  });

  it("diz se a gaveta esta aberta e qual area ela controla", () => {
    const fechado = barra(false);
    expect(fechado).toContain('aria-expanded="false"');
    expect(fechado).toContain('aria-controls="gaveta-plotinfo"');
    const aberto = barra(true);
    expect(aberto).toContain('aria-expanded="true"');
  });
});