import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AjusteDaLinha, LinhaImpressora } from "./components/Sidebar";
import { traducoes, traduzir, type Idioma } from "./i18n";
import type { Impressora } from "./impressoras";
import {
  avisosDepoisDaLeitura,
  avisosSemLinha,
  codigoDaFalha,
  comAviso,
  semAviso,
  temPreferencias,
  textoDoAvisoDoAjuste,
  type CodigoAvisoDoAjuste,
} from "./preferencias";

const IDIOMAS: readonly Idioma[] = ["pt-BR", "en"];

const FILA: Impressora = {
  id: "fila:Sala",
  nome: "Sala",
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "laser",
};

const SIMULADA: Impressora = {
  id: "simulada:gaveta",
  nome: "Simulada — Gaveta",
  tipo: "simulada",
  nivel: "pronta",
  condicoes: [],
  modo: "laser",
};

const sem = () => undefined;

function lateral(impressoras: readonly Impressora[]): string {
  return impressoras
    .map((impressora) =>
      renderToStaticMarkup(
        createElement(LinhaImpressora, {
          impressora,
          marcada: false,
          desabilitada: true,
          idExplicacao: undefined,
          idNotaDoModelo: "nota",
          gravando: undefined,
          falha: null,
          aoAlternar: sem,
          aoEscolherModelo: sem,
          aoPedirSubstituicao: sem,
          aoCalibrar: sem,
          comAjuste: false,
          duasFaces: false,
          janelaAberta: false,
          avisoDoAjuste: null,
          comCartaoNaTela: false,
          aoAbrirPreferencias: sem,
          aoPedirVolta: sem,
          aoDispensarAviso: sem,
        }),
      ),
    )
    .join("");
}

const ROTULO_DA_ENGRENAGEM = "aria-haspopup=\"dialog\"";

function vezes(texto: string, trecho: string): number {
  return texto.split(trecho).length - 1;
}

describe("engrenagem das preferencias", () => {
  it("so a fila real tem preferencias", () => {
    expect(temPreferencias(FILA)).toBe(true);
    expect(temPreferencias(SIMULADA)).toBe(false);
  });

  it("aparece em cada fila real e em nenhuma simulada", () => {
    expect(vezes(lateral([SIMULADA]), ROTULO_DA_ENGRENAGEM)).toBe(0);
    expect(vezes(lateral([FILA, SIMULADA]), ROTULO_DA_ENGRENAGEM)).toBe(1);
    const duas = lateral([FILA, { ...FILA, id: "fila:Outra", nome: "Outra" }, SIMULADA]);
    expect(vezes(duas, ROTULO_DA_ENGRENAGEM)).toBe(2);
  });

  it("fica fora de qualquer rotulo: clicar nela nao marca a impressora", () => {
    const html = lateral([FILA]);
    const antes = html.slice(0, html.indexOf(ROTULO_DA_ENGRENAGEM));
    expect(antes).not.toBe(html);
    expect(vezes(antes, "<label")).toBe(vezes(antes, "</label>"));
  });

  it("tem nome acessivel e dica", () => {
    const html = lateral([FILA]);
    expect(html).toContain(`aria-label="${traduzir("pt-BR", "preferencias.rotulo", { nome: "Sala" })}"`);
    expect(html).toContain(`title="${traduzir("pt-BR", "preferencias.titulo")}"`);
  });

  it("continua disponivel sem documento aberto, quando a marcacao fica desligada", () => {
    const html = lateral([FILA]);
    const botao = html.slice(html.lastIndexOf("<button", html.indexOf(ROTULO_DA_ENGRENAGEM)));
    expect(botao.slice(0, botao.indexOf(">"))).not.toContain("disabled");
  });

  it("o texto pedido esta nos dois idiomas", () => {
    expect(traduzir("pt-BR", "preferencias.titulo")).toBe("Preferências do driver, só no PlotS");
    expect(traduzir("en", "preferencias.titulo")).toBe("Driver preferences, PlotS only");
  });
});

describe("linha do ajuste salvo", () => {
  const html = renderToStaticMarkup(createElement(AjusteDaLinha, { nome: "Sala", duasFaces: false, aoVoltar: sem }));

  it("diz em texto que ha ajuste, nao so por cor", () => {
    expect(html).toContain(traduzir("pt-BR", "preferencias.salvo"));
  });

  it("oferece voltar ao padrao com nome acessivel da impressora", () => {
    expect(html).toContain(traduzir("pt-BR", "preferencias.voltar"));
    expect(html).toContain(traduzir("pt-BR", "preferencias.voltar.rotulo", { nome: "Sala" }));
    expect(html).toContain("<button");
  });
});

describe("avisos do ajuste", () => {
  const CODIGOS: readonly CodigoAvisoDoAjuste[] = [
    "fila_sumiu",
    "nao_abriu",
    "nao_gravou",
    "grande",
    "recusado",
    "corrompido",
  ];

  it.each(IDIOMAS)("%s nomeia a impressora em todo aviso", (idioma) => {
    for (const codigo of CODIGOS) {
      expect(textoDoAvisoDoAjuste(codigo, "Sala", idioma)).toContain("Sala");
    }
  });

  it.each(IDIOMAS)("%s diz que o envio usou o padrao quando o driver recusa", (idioma) => {
    expect(textoDoAvisoDoAjuste("recusado", "Sala", idioma)).toMatch(
      idioma === "en" ? /default/ : /padrão/,
    );
  });

  it("todo texto novo existe nos dois idiomas", () => {
    const chaves = Object.keys(traducoes["pt-BR"]).filter((chave) => chave.startsWith("preferencias."));
    expect(chaves.length).toBeGreaterThan(0);
    for (const chave of chaves) {
      expect(traducoes.en[chave as keyof (typeof traducoes)["en"]]).toBeTruthy();
    }
  });

  it("recusa do backend vira o codigo dela, e o resto conta como falha ao abrir", () => {
    expect(codigoDaFalha({ codigo: "fila_sumiu", fila: "Sala" })).toBe("fila_sumiu");
    expect(codigoDaFalha({ codigo: "outra coisa" })).toBe("nao_abriu");
    expect(codigoDaFalha("texto")).toBe("nao_abriu");
    expect(codigoDaFalha(null)).toBe("nao_abriu");
    expect(codigoDaFalha(undefined)).toBe("nao_abriu");
  });

  it("a leitura soma os corrompidos, e o aviso antigo continua depois de o arquivo ser regravado", () => {
    const atuais = new Map<string, CodigoAvisoDoAjuste>([
      ["Sala", "recusado"],
      ["Velha", "corrompido"],
    ]);
    const novos = avisosDepoisDaLeitura(atuais, ["Nova", "Sala"]);
    expect([...novos].sort()).toEqual([
      ["Nova", "corrompido"],
      ["Sala", "recusado"],
      ["Velha", "corrompido"],
    ]);
  });

  it("aviso de impressora fora da lista vai para o topo; a simulada de mesmo nome nao conta", () => {
    const avisos = new Map<string, CodigoAvisoDoAjuste>([
      ["Sala", "recusado"],
      ["Fora", "nao_gravou"],
      [SIMULADA.nome, "corrompido"],
    ]);
    expect(avisosSemLinha(avisos, [FILA, SIMULADA])).toEqual([
      ["Fora", "nao_gravou"],
      [SIMULADA.nome, "corrompido"],
    ]);
  });

  it("agir de novo na impressora tira o aviso dela e deixa o das outras", () => {
    const avisos = comAviso(comAviso(new Map(), "Sala", "grande"), "Outra", "recusado");
    expect([...semAviso(avisos, "Sala").keys()]).toEqual(["Outra"]);
    expect(semAviso(avisos, "Ninguem")).toBe(avisos);
  });
});
