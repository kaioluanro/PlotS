import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AjusteSemLinha } from "./components/Sidebar";
import { traducoes, traduzir } from "./i18n";
import type { Impressora } from "./impressoras";
import { ajustesSemLinha, conteudoDoApagarAjuste, linhaDoAjusteNoEnvio } from "./preferencias";

const fila = (nome: string): Impressora => ({
  id: `fila:${nome}`,
  nome,
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "laser",
});

const SIMULADA: Impressora = {
  id: "simulada:gaveta",
  nome: "Simulada — Gaveta",
  tipo: "simulada",
  nivel: "pronta",
  condicoes: [],
  modo: "laser",
};

describe("ajuste do driver na confirmacao com varios destinos", () => {
  it("nomeia so as filas com ajuste, sem cortar a lista", () => {
    const nomes = ["Sala", "Recepcao", "Almoxarifado", "Diretoria"];
    const destinos = [...nomes.map(fila), fila("Sem ajuste")];
    const linha = linhaDoAjusteNoEnvio(destinos, new Set(nomes), "pt-BR");
    expect(linha).toBe(
      "Ajuste do driver só no PlotS: Sala, Recepcao, Almoxarifado, Diretoria. Papel e faces seguem o PlotS.",
    );
    expect(linha).not.toContain("Sem ajuste");
    expect(linhaDoAjusteNoEnvio(destinos, new Set(["Sala"]), "en")).toBe(
      "PlotS-only driver settings: Sala. Paper and sides follow PlotS.",
    );
  });

  it("uma simulada junto conta como destino, mas nao entra na lista", () => {
    expect(linhaDoAjusteNoEnvio([fila("Sala"), SIMULADA], new Set(["Sala"]), "pt-BR")).toBe(
      "Ajuste do driver só no PlotS: Sala. Papel e faces seguem o PlotS.",
    );
  });

  it("com um destino, a frase continua a mesma", () => {
    expect(linhaDoAjusteNoEnvio([fila("Sala")], new Set(["Sala"]), "pt-BR")).toBe(
      traduzir("pt-BR", "preferencias.envio.linha"),
    );
  });

  it("sem ajuste em nenhum destino, nao ha linha", () => {
    expect(linhaDoAjusteNoEnvio([fila("Sala"), fila("Outra")], new Set(), "pt-BR")).toBeNull();
  });
});

describe("textos do ajuste salvo", () => {
  it("o aviso de duas faces diz o que o driver esta pedindo e o que o PlotS faz", () => {
    expect(traducoes["pt-BR"]["preferencias.salvo.duasFaces"]).toBe(
      "O driver está em duas faces, mas o PlotS imprime uma por vez.",
    );
    expect(traducoes.en["preferencias.salvo.duasFaces"]).toBe(
      "The driver is set to two sides, but PlotS prints one at a time.",
    );
  });

  it("a fila fora da lista diz que o ajuste segue salvo", () => {
    expect(traduzir("pt-BR", "preferencias.salvo.semFila", { nome: "Sala" })).toBe(
      "A impressora Sala não está na lista. O ajuste dela segue salvo no PlotS.",
    );
    expect(traduzir("en", "preferencias.salvo.semFila", { nome: "Sala" })).toBe(
      "The printer Sala is not in the list. Its settings remain saved in PlotS.",
    );
  });
});

describe("ajuste de uma fila fora da lista", () => {
  it("o bloco tem o x de dispensar e o botao de apagar", () => {
    const html = renderToStaticMarkup(
      createElement(AjusteSemLinha, {
        fila: "Sala",
        aoApagar: () => undefined,
        aoDispensar: () => undefined,
      }),
    );
    expect(html).toContain(traduzir("pt-BR", "preferencias.salvo.semFila", { nome: "Sala" }));
    expect(html).toContain(`>${traduzir("pt-BR", "preferencias.apagar")}<`);
    expect(html).toContain(
      `aria-label="${traduzir("pt-BR", "preferencias.aviso.dispensar", { nome: "Sala" })}"`,
    );
  });

  it("o dialogo fala em apagar, e o botao dele tambem", () => {
    const conteudo = conteudoDoApagarAjuste("Sala");
    expect(conteudo.fechar).toBe("preferencias.apagar");
    expect(traducoes["pt-BR"]["preferencias.apagar"]).toBe("Apagar ajuste");
    expect(traducoes.en["preferencias.apagar"]).toBe("Delete settings");
    expect(conteudo.parametros.nome).toBe("Sala");
  });

  it("o que a pessoa dispensou nesta sessao sai do topo; o resto fica", () => {
    const salvas = new Set(["Sala", "Depois"]);
    expect(ajustesSemLinha(salvas, [], new Set(["Sala"]))).toEqual(["Depois"]);
    expect(ajustesSemLinha(salvas, [])).toEqual(["Sala", "Depois"]);
  });
});
