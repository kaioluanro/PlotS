import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { LinhaImpressora } from "./components/Sidebar";
import { t } from "./i18n";
import type { Impressora } from "./impressoras";

// A lista de impressoras nao mostra mais os cartoes: eles viraram avisos no topo.
import sidebarFonte from "./components/Sidebar.tsx?raw";

const FILA: Impressora = {
  id: "fila:A",
  nome: "Impressora A",
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "laser",
};

function linha(impressora: Impressora) {
  return renderToStaticMarkup(
    <ul>
      <LinhaImpressora
        impressora={impressora}
        marcada={false}
        desabilitada={false}
        idExplicacao={undefined}
        idNotaDoModelo="nota"
        gravando={undefined}
        falha={null}
        aoAlternar={() => undefined}
        aoEscolherModelo={() => undefined}
        aoPedirSubstituicao={() => undefined}
        aoCalibrar={() => undefined}
        comAjuste={false}
        duasFaces={false}
        janelaAberta={false}
        avisoDoAjuste={null}
        aoAbrirPreferencias={() => undefined}
        aoAbrirOpcoes={() => undefined}
        aoPedirVolta={() => undefined}
        aoDispensarAviso={() => undefined}
      />
    </ul>,
  );
}

describe("lista de impressoras", () => {
  it("nao importa nem desenha os cartoes de impressao", () => {
    expect(sidebarFonte).not.toMatch(/import \{ CartaoImpressao \}/);
    expect(sidebarFonte).not.toMatch(/<CartaoImpressao/);
  });

  it("fila com seletor tem o botao Calibrar ao lado da engrenagem, com o nome acessivel", () => {
    const markup = linha(FILA);
    expect(markup).toContain(`aria-label="${t("calibracao.titulo", { nome: "Impressora A" })}"`);
    expect(markup).toContain(`title="${t("modo.calibrar")}"`);
  });

  it("a chave de boca fica ao lado da engrenagem e abre o painel, sem marcar a impressora", () => {
    const markup = linha(FILA);
    expect(markup).toContain(`aria-label="${t("opcoes.rotulo", { nome: "Impressora A" })}"`);
    expect(markup).toContain(`title="${t("opcoes.titulo")}"`);
    expect(markup).toContain('aria-haspopup="dialog"');
    // A chave nao usa `disabled`: o foco de quem usa o teclado continua nela.
    const chave = markup.slice(markup.indexOf(`title="${t("opcoes.titulo")}"`));
    expect(chave.slice(0, chave.indexOf(">"))).not.toContain("disabled");
  });

  it("simulada nao ganha o botao da engrenagem nem o Calibrar extra da fila", () => {
    const simulada: Impressora = { ...FILA, id: "simulada:traseira", nome: "Simulada", tipo: "simulada" };
    const markup = linha(simulada);
    expect(markup).not.toContain(`title="${t("preferencias.titulo")}"`);
    expect(markup).not.toContain(`title="${t("opcoes.titulo")}"`);
  });
});
