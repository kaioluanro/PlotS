import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  PainelDasOpcoes,
  type EstadoDoPainel,
} from "./components/OpcoesDaImpressoraDialog";
import { traduzir } from "./i18n";
import { ESCOLHAS_VAZIAS, type OpcoesDaImpressora } from "./opcoes-impressora";

const CHEIAS: OpcoesDaImpressora = {
  bandejas: [
    { valor: 1, nome: "Bandeja 1" },
    { valor: 2, nome: "Bandeja 2" },
  ],
  resolucoes: [{ x: 600, y: 600 }],
  cor: "colorida",
  atual: { bandeja: 1, resolucao: { x: 600, y: 600 }, cor: "colorida" },
};

const SEM_OPCOES: OpcoesDaImpressora = {
  bandejas: [],
  resolucoes: [],
  cor: null,
  atual: ESCOLHAS_VAZIAS,
};

function estado(mudanca: Partial<EstadoDoPainel> = {}): EstadoDoPainel {
  return {
    carregando: false,
    erro: null,
    opcoes: CHEIAS,
    escolhas: CHEIAS.atual,
    aplicando: false,
    temAjusteSalvo: false,
    ...mudanca,
  };
}

function painel(mudanca: Partial<EstadoDoPainel> = {}): string {
  return renderToStaticMarkup(
    <PainelDasOpcoes
      estado={estado(mudanca)}
      aoMudar={() => undefined}
      aoAplicar={() => undefined}
      aoCancelar={() => undefined}
      aoUsarPadrao={() => undefined}
      aoAbrirEngrenagem={() => undefined}
      aoTentarDeNovo={() => undefined}
    />,
  );
}

interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;

function regra(css: string, seletor: string): string {
  const inicio = css.indexOf(`${seletor} {`);
  expect(inicio, seletor).toBeGreaterThanOrEqual(0);
  return css.slice(inicio, css.indexOf("}", inicio));
}

describe("painel com o que o driver declarou", () => {
  const html = painel();

  it("mostra bandeja, resolucao e cor, cada uma com o padrao da impressora", () => {
    expect(html).toContain(traduzir("pt-BR", "opcoes.bandeja"));
    expect(html).toContain("Bandeja 1");
    expect(html).toContain(traduzir("pt-BR", "opcoes.resolucao"));
    expect(html).toContain("600 × 600 dpi");
    expect(html).toContain(traduzir("pt-BR", "opcoes.cor"));
    expect(html).toContain(traduzir("pt-BR", "opcoes.colorida"));
    expect(html).toContain(traduzir("pt-BR", "opcoes.monocromatica"));
    expect(html.split(traduzir("pt-BR", "opcoes.padrao")).length - 1).toBe(3);
    expect(html).toContain(traduzir("pt-BR", "opcoes.aplicar"));
    expect(html).toContain(traduzir("pt-BR", "opcoes.cancelar"));
  });

  it("nao mostra papel, face, copias, escala, orientacao nem pagina por folha", () => {
    for (const proibido of [
      "Papel",
      "Face",
      "Cópias",
      "Escala",
      "Orientação",
      "página por folha",
    ]) {
      expect(html).not.toContain(proibido);
    }
  });

  it("so mostra as secoes que vieram: uma fila sem bandeja nao ganha bandeja", () => {
    const semBandeja = painel({ opcoes: { ...CHEIAS, bandejas: [] } });
    expect(semBandeja).not.toContain(traduzir("pt-BR", "opcoes.bandeja"));
    expect(semBandeja).toContain(traduzir("pt-BR", "opcoes.resolucao"));
  });

  it("uma impressora so de preto nao oferece a cor", () => {
    const soPreto = painel({
      opcoes: { ...CHEIAS, cor: "monocromatica" },
      escolhas: { ...CHEIAS.atual, cor: "monocromatica" },
    });
    expect(soPreto).toContain(traduzir("pt-BR", "opcoes.monocromatica"));
    expect(soPreto).not.toContain(traduzir("pt-BR", "opcoes.colorida"));
  });

  it("'Usar padrao' so aparece quando a fila ja tem ajuste salvo", () => {
    expect(html).not.toContain(traduzir("pt-BR", "opcoes.usarPadrao"));
    expect(painel({ temAjusteSalvo: true })).toContain(traduzir("pt-BR", "opcoes.usarPadrao"));
  });

  it("durante a gravacao o Aplicar mostra que esta ocupado", () => {
    const html = painel({ aplicando: true });
    expect(html).toMatch(/aria-busy="true"/);
    expect(html).toContain(traduzir("pt-BR", "opcoes.aplicando"));
    expect(html).toContain("lucide-loader-circle");
  });
});

describe("estado vazio e falha", () => {
  it("sem opcoes diz isso e aponta a engrenagem certa, sem Aplicar", () => {
    const html = painel({ opcoes: SEM_OPCOES, escolhas: ESCOLHAS_VAZIAS });
    expect(html).toContain(traduzir("pt-BR", "opcoes.vazio"));
    expect(html).toContain(traduzir("pt-BR", "opcoes.abrirEngrenagem"));
    expect(html).toContain("lucide-settings");
    expect(html).not.toContain("lucide-wrench");
    expect(html).not.toContain(traduzir("pt-BR", "opcoes.aplicar"));
  });

  it("a falha fica no corpo, anunciada, sem esconder as opcoes para corrigir", () => {
    const html = painel({ erro: "O driver de Sala não aceitou as opções. Nada mudou." });
    expect(html).toContain('role="alert"');
    expect(html).toContain("não aceitou as opções");
    expect(html).toContain("600 × 600 dpi");
    expect(html).toContain(traduzir("pt-BR", "opcoes.aplicar"));
    expect(html).not.toContain(traduzir("pt-BR", "opcoes.tentarDeNovo"));
  });

  it("falha ao ler oferece Tentar de novo e nao oferece Aplicar", () => {
    const html = painel({
      erro: "Não foi possível ler as opções que o driver de Sala declara.",
      opcoes: null,
    });
    expect(html).toContain('role="alert"');
    expect(html).toContain(traduzir("pt-BR", "opcoes.tentarDeNovo"));
    expect(html).not.toContain(traduzir("pt-BR", "opcoes.aplicar"));
    expect(html).not.toContain(traduzir("pt-BR", "opcoes.vazio"));
  });
});

describe("painel em 640x420", () => {
  const css = fs.readFileSync(
    new URL("./components/OpcoesDaImpressoraDialog.module.css", import.meta.url),
    "utf8",
  );

  it("a largura para antes da borda da janela e a altura cabe na tela curta", () => {
    const conteudo = regra(css, ".conteudo");
    expect(conteudo).toContain("width: min(460px, calc(100vw - 48px))");
    expect(conteudo).toContain("max-height: calc(100vh - 48px)");
    expect(conteudo).toContain("overflow: hidden");
  });

  it("so o corpo rola: os botoes ficam sempre visiveis", () => {
    expect(regra(css, ".corpo")).toMatch(/overflow-y:\s*auto/);
    expect(regra(css, ".corpo")).toContain("min-height: 0");
    expect(regra(css, ".rodape")).toContain("flex-shrink: 0");
  });

  it("nenhuma cor literal: o painel usa os tokens do tema", () => {
    expect(css).not.toMatch(/#[0-9a-fA-F]{3,8}\b/);
    expect(css).not.toMatch(/rgba?\(/);
  });
});

describe("foco e teclado", () => {
  it("o painel lembra de onde veio e devolve o foco ao fechar", async () => {
    const fonte = fs.readFileSync(
      new URL("./components/OpcoesDaImpressoraDialog.tsx", import.meta.url),
      "utf8",
    );
    expect(fonte).toContain("onOpenAutoFocus");
    expect(fonte).toContain("onCloseAutoFocus");
    expect(fonte).toContain("origem.current.focus()");
    // O primeiro controle recebe o foco quando as opcoes chegam; num grupo de radios, o marcado.
    expect(fonte).toContain("input[type=\"radio\"]:checked");
    expect(fonte).toContain("deveRolarPeloTeclado");
  });

  it("o titulo separa o nome da fila com dois-pontos", () => {
    expect(
      traduzir("pt-BR", "opcoes.tituloComNome", { nome: "PlotSim — U" }),
    ).toBe("Opções da impressora: PlotSim — U");
    expect(
      traduzir("en", "opcoes.tituloComNome", { nome: "PlotSim — U" }),
    ).toBe("Printer options: PlotSim — U");
  });
});
