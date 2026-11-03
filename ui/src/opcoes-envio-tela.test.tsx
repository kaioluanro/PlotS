import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { BlocoAjuste, OpcaoInversao } from "./components/BlocoAjuste";
import { VistaGuardarHistorico } from "./components/HistoricoDialog";
import { CampoLote, SeletorDoGesto } from "./components/Sidebar";
import { traduzir } from "./i18n";
import type { Impressora } from "./impressoras";
import { estadoInicial, reduzirAbas, type Aba } from "./tabs";

// Os CSS de modulo nao voltam como texto sob o teste: as folhas sao lidas do disco.
interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;
const folha = (nome: string) =>
  fs.readFileSync(new URL(`./components/${nome}.module.css`, import.meta.url), "utf8");

/** O corpo da primeira regra com este seletor exato. */
function regra(css: string, seletor: string): string {
  const inicio = css.indexOf(`${seletor} {`);
  expect(inicio, seletor).toBeGreaterThanOrEqual(0);
  return css.slice(inicio, css.indexOf("}", inicio));
}

const FILA: Impressora = {
  id: "fila:Sala",
  nome: "Sala",
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "laser",
};

const bloco = () =>
  renderToStaticMarkup(
    <BlocoAjuste aba={null} aoDefinirPapel={() => undefined} aoDefinirControle={() => undefined} />,
  );

describe("foco e campo de papel", () => {
  it("o anel de foco das opcoes usa o acento", () => {
    expect(regra(folha("HistoricoDialog"), ".caixa:focus-visible")).toContain("var(--accent)");
    expect(regra(folha("Sidebar"), ".campoAjuste:focus-visible")).toContain("var(--accent)");
  });

  it("o campo de papel tem a medida, a fonte e a seta de um select", () => {
    const campo = regra(folha("Sidebar"), ".campoAjuste");
    expect(campo).toContain("height: 32px");
    expect(campo).toContain("padding: 0 28px 0 10px");
    expect(campo).toContain("font-size: 13px");
    expect(campo).toContain("appearance: none");
    expect(bloco()).toContain("lucide-chevron-down");
  });
});

describe("dicas fixas das opcoes", () => {
  it("a opcao do driver traz a dica ligada ao controle", () => {
    const tela = renderToStaticMarkup(<OpcaoInversao impressora={FILA} />);
    const id = /aria-describedby="([^"]+)"/.exec(tela)?.[1];
    expect(id).toBeTruthy();
    expect(tela).toContain(`id="${id}"`);
    expect(tela).toContain(traduzir("pt-BR", "inversao.dica"));
  });

  it("a folha de controle traz a dica ligada ao controle, sem o aviso do canto se estiver desligada", () => {
    const tela = bloco();
    const id = /aria-describedby="([^"]+)"/.exec(tela)?.[1];
    expect(id).toBeTruthy();
    expect(tela).toContain(`id="${id}"`);
    expect(tela).toContain(traduzir("pt-BR", "controle.titulo"));
    expect(tela).not.toContain(traduzir("pt-BR", "controle.canto"));
  });

  const ABA_LIGADA = { controle: true, papelDeAjuste: null } as unknown as Aba;
  const ligado = (semMarca: boolean) =>
    renderToStaticMarkup(
      <BlocoAjuste
        aba={ABA_LIGADA}
        semMarca={semMarca}
        aoDefinirPapel={() => undefined}
        aoDefinirControle={() => undefined}
      />,
    );

  it("com a opcao ligada, a dica ganha o aviso do canto", () => {
    expect(ligado(false)).toContain(traduzir("pt-BR", "controle.canto"));
    expect(ligado(false)).not.toContain(traduzir("pt-BR", "controle.semMarca"));
  });

  it("sem marca neste envio, avisa ligado ao controle e nao fala do canto", () => {
    const tela = ligado(true);
    expect(tela).toContain(traduzir("pt-BR", "controle.semMarca"));
    expect(tela).not.toContain(traduzir("pt-BR", "controle.canto"));
    const ids = /aria-describedby="([^"]+)"/.exec(tela)?.[1]?.split(" ") ?? [];
    expect(ids).toHaveLength(2);
    for (const id of ids) expect(tela).toContain(`id="${id}"`);
  });

  it("com cartao na tela, as dicas somem so da vista: continuam descrevendo o controle", () => {
    const oculta = regra(folha("Sidebar"), ".lateral[data-com-cartao] [data-dica-opcao]");
    expect(oculta).toContain("clip-path: inset(50%)");
    expect(oculta).not.toContain("display: none");
    expect(bloco()).toContain("data-dica-opcao");
    expect(renderToStaticMarkup(<OpcaoInversao impressora={FILA} />)).toContain("data-dica-opcao");
  });

  it("a dica tem 11 px no tom secundario", () => {
    const dica = regra(folha("HistoricoDialog"), ".dicaOpcao");
    expect(dica).toContain("font-size: 11px");
    expect(dica).toContain("var(--fg-muted)");
  });

  it("os textos das dicas existem nos dois idiomas", () => {
    expect(traduzir("pt-BR", "inversao.dica")).toBe("Ligue se a última página sair primeiro.");
    expect(traduzir("en", "inversao.dica")).toBe("Turn on if the last page prints first.");
  });
});

describe("guardar historico", () => {
  const vista = (estado: "lendo" | "ligada" | "desligada" | "ilegivel", falhou = false) =>
    renderToStaticMarkup(
      <VistaGuardarHistorico estado={estado} falhou={falhou} aoMudar={() => undefined} />,
    );
  const DESLIGADA = traduzir("pt-BR", "historico.guardar.desligada");
  const MARCADA = /<input[^>]*checked/;

  it("ligada, a caixa fica marcada e a frase nao aparece", () => {
    const tela = vista("ligada");
    expect(tela).toContain(traduzir("pt-BR", "historico.guardar.rotulo"));
    expect(tela).toMatch(MARCADA);
    expect(tela).not.toContain(DESLIGADA);
  });

  it("desligada, diz em uma frase que nada novo entra e que a lista continua", () => {
    const tela = vista("desligada");
    expect(tela).not.toMatch(MARCADA);
    expect(tela).toContain(DESLIGADA);
    expect(tela).toContain("aria-describedby");
    expect(DESLIGADA).toBe("Novos envios não são registrados, e a lista continua aqui até ser apagada.");
    expect(traduzir("en", "historico.guardar.desligada")).toBe(
      "New prints are not recorded, and the list stays here until it is deleted.",
    );
  });

  it("falha ao gravar a escolha aparece em palavras e com o icone de alerta", () => {
    const tela = vista("ligada", true);
    expect(tela).toContain('role="alert"');
    expect(tela).toContain(traduzir("pt-BR", "inversao.falha"));
    expect(tela).toContain("lucide-circle-alert");
  });

  it("sem conseguir ler a escolha, a caixa fica desabilitada e diz o que vale", () => {
    const tela = vista("ilegivel");
    expect(tela).toMatch(/<input[^>]*disabled/);
    expect(tela).toContain(traduzir("pt-BR", "historico.guardar.falhaLeitura"));
    expect(tela).toContain("lucide-circle-alert");
    expect(traduzir("en", "historico.guardar.falhaLeitura")).toBe(
      "This option could not be read. Until then, no new print is recorded.",
    );
  });
});

describe("falha ao gravar uma opcao", () => {
  it("o icone usa a cor de alerta, e o texto continua dizendo o que houve", () => {
    expect(regra(folha("HistoricoDialog"), ".iconeFalha")).toContain("var(--status-offline-text)");
  });
});

describe("seletor do gesto ao gravar", () => {
  const seletor = (gravando: boolean, demorando: boolean) =>
    renderToStaticMarkup(
      <SeletorDoGesto valor="jato" gravando={gravando} demorando={demorando} aoEscolher={() => undefined} />,
    );

  it("gravando, nao fica desabilitado: o foco de quem usa o teclado continua ali", () => {
    expect(seletor(true, false)).not.toMatch(/<select[^>]*disabled/);
    expect(seletor(true, false)).toContain('aria-busy="true"');
    expect(seletor(false, false)).toContain('aria-busy="false"');
  });

  it("quando demora, o titulo e o indicador dizem que esta gravando", () => {
    const tela = seletor(true, true);
    expect(tela).toContain(`title="${traduzir("pt-BR", "modo.seletor.gravando")}"`);
    expect(tela).toContain("lucide-loader-circle");
    expect(seletor(true, false)).toContain("lucide-chevron-down");
  });
});

describe("posicao da seta e rodape da janela curta", () => {
  it("a seta e o indicador ficam centrados no seletor", () => {
    expect(regra(folha("Sidebar"), ".setaSeletor")).toContain("top: 9px");
    expect(regra(folha("Sidebar"), ".indicadorGravacao")).toContain("top: 8px");
  });

  it("os campos do rodape rolam por conta propria, com altura limitada pela janela", () => {
    const css = folha("Sidebar");
    const curta = css.slice(css.indexOf("@media (max-height: 560px)"));
    expect(regra(curta, ".camposDoRodape")).toMatch(/max-height: calc\(100vh - \d+px\)/);
  });
});

describe("resumo abaixo das folhas por lote", () => {
  let estado = reduzirAbas(estadoInicial, { tipo: "abrir", documento: { caminho: "a.pdf", nome: "a.pdf" } });
  const id = estado.abas[0]?.id ?? -1;
  estado = reduzirAbas(estado, { tipo: "paginas_contadas", id, total: 8 });
  const aba = estado.abas[0] as Aba;
  const campo = (variosPapeis: boolean) =>
    renderToStaticMarkup(
      <CampoLote aba={aba} aoDefinir={() => undefined} nota={null} soAutomaticas={false} variosPapeis={variosPapeis} />,
    );

  it("com um papel so, conta os lotes; com mais de um, fala so das paginas", () => {
    expect(campo(false)).toMatch(/8 pág · 1 lote/);
    expect(campo(true)).toContain("8 pág");
    expect(campo(true)).not.toMatch(/lote<\/p>/);
  });
});
