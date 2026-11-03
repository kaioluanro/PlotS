import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import { linhasDoEnvio, type Cartao } from "./cartoes";
import { CartaoImpressao } from "./components/CartaoImpressao";
import { AjusteDaLinha, AvisoDoAjuste, LinhaImpressora } from "./components/Sidebar";
import { conteudoDaDivisao } from "./confirmacao";
import { traducoes, traduzir } from "./i18n";
import type { Impressora } from "./impressoras";
import {
  ajustesSemLinha,
  aoVoltarOFoco,
  avisosDepoisDaLeitura,
  avisosSemLinha,
  conteudoDoVoltarAoPadrao,
  linhaDoAjusteNoEnvio,
  registrarFilasComAjuste,
  semFilasQueSairam,
  type CodigoAvisoDoAjuste,
} from "./preferencias";

interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;
const folhaDaLateral = fs.readFileSync(
  new URL("./components/Sidebar.module.css", import.meta.url),
  "utf8",
);

function regra(seletor: string): string {
  const inicio = folhaDaLateral.indexOf(`${seletor} {`);
  expect(inicio).toBeGreaterThanOrEqual(0);
  return folhaDaLateral.slice(inicio, folhaDaLateral.indexOf("}", inicio));
}

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

describe("textos das preferencias", () => {
  const ESPERADOS: readonly (readonly [keyof (typeof traducoes)["en"], string, string])[] = [
    ["preferencias.titulo", "Preferências do driver, só no PlotS", "Driver preferences, PlotS only"],
    [
      "preferencias.rotulo",
      "Preferências do driver de {nome}, só no PlotS",
      "Driver preferences for {nome}, PlotS only",
    ],
    ["preferencias.salvo", "Ajuste só no PlotS", "PlotS-only settings"],
    [
      "preferencias.salvo.titulo",
      "O PlotS usa este ajuste do driver nesta impressora e não altera o padrão do Windows.",
      "PlotS uses these driver settings for this printer and does not change the Windows default.",
    ],
    [
      "preferencias.aberta.nota",
      "Vale só no PlotS, a partir do próximo envio. O PlotS continua decidindo papel, cópias, escala, orientação, páginas por folha e faces.",
      "Applies only in PlotS, from the next job. PlotS still decides paper, copies, scale, orientation, pages per sheet and sides.",
    ],
    ["preferencias.voltarDialogo.titulo", "Voltar {nome} ao padrão?", "Return {nome} to default?"],
    [
      "preferencias.voltarDialogo.texto",
      "O ajuste salvo no PlotS será apagado. Para tê-lo de novo, refaça na janela do driver.",
      "The settings saved in PlotS will be deleted. To get them back, redo them in the driver window.",
    ],
    ["preferencias.voltarDialogo.manter", "Manter ajuste", "Keep settings"],
    ["preferencias.voltarDialogo.confirmar", "Usar padrão", "Use default"],
    ["preferencias.aviso.dispensar", "Dispensar aviso de {nome}", "Dismiss warning for {nome}"],
    [
      "preferencias.envio.linha",
      "Driver: ajuste só no PlotS. Papel e faces seguem o PlotS.",
      "Driver: PlotS-only settings. Paper and sides follow PlotS.",
    ],
    [
      "preferencias.cartao.recusado",
      "Enviado com o padrão da impressora: o driver não aceitou o ajuste salvo.",
      "Sent with the printer default: the driver did not accept the saved settings.",
    ],
  ];

  it.each(ESPERADOS)("%s nos dois idiomas", (chave, pt, en) => {
    expect(traducoes["pt-BR"][chave]).toBe(pt);
    expect(traducoes.en[chave]).toBe(en);
  });
});

describe("anatomia da linha", () => {
  function posicoes(html: string, trechos: readonly string[]): number[] {
    return trechos.map((trecho) => {
      const posicao = html.indexOf(trecho);
      expect(posicao, trecho).toBeGreaterThanOrEqual(0);
      return posicao;
    });
  }

  it("a ordem do Tab em fila real e caixa, engrenagem e seletor", () => {
    const [caixa, engrenagem, seletor] = posicoes(lateral([FILA]), [
      'type="checkbox"',
      'aria-haspopup="dialog"',
      "<select",
    ]);
    expect(caixa).toBeLessThan(engrenagem!);
    expect(engrenagem).toBeLessThan(seletor!);
  });

  it("o nome vem antes da engrenagem, na mesma fileira", () => {
    const html = lateral([FILA]);
    const [nome, engrenagem] = posicoes(html, ['title="Sala"', 'aria-haspopup="dialog"']);
    expect(nome).toBeLessThan(engrenagem!);
    expect(html.slice(nome, engrenagem)).not.toContain("</div>");
  });

  it("o seletor fica em uma fileira propria, abaixo do nome", () => {
    const html = lateral([FILA]);
    const [engrenagem, seletor] = posicoes(html, ['aria-haspopup="dialog"', "<select"]);
    expect(html.slice(engrenagem, seletor)).toContain("</div>");
  });

  it("a simulada continua com o modo na fileira do nome e sem seletor", () => {
    const html = lateral([SIMULADA]);
    expect(html).not.toContain("<select");
    expect(html).not.toContain('aria-haspopup="dialog"');
    const nome = html.indexOf(`title="${SIMULADA.nome}"`);
    const modo = html.indexOf("Laser", nome);
    expect(modo).toBeGreaterThan(nome);
    expect(html.slice(nome, modo)).toContain("</label>");
    expect(html.slice(nome, modo)).not.toContain("</div>");
  });

  it("a engrenagem em repouso nao se diz ocupada", () => {
    expect(lateral([FILA])).toContain('aria-busy="false"');
  });

  it("o seletor tem 108 px e a fileira mantem a reserva de quebra", () => {
    expect(regra(".seletorCaixa")).toContain("width: 108px");
    expect(regra(".fileiraDoModo")).toContain("flex-wrap: wrap");
  });
});

describe("linha do ajuste salvo", () => {
  const html = (duasFaces: boolean) =>
    renderToStaticMarkup(createElement(AjusteDaLinha, { nome: "Sala", duasFaces, aoVoltar: sem }));

  it("o botao descreve o ajuste por aria-describedby, sem depender do title", () => {
    const marcado = html(false);
    const id = /aria-describedby="([^"]+)"/.exec(marcado)?.[1];
    expect(id).toBeTruthy();
    const alvo = marcado.indexOf(`id="${id}"`);
    expect(alvo).toBeGreaterThanOrEqual(0);
    const conteudo = marcado.slice(alvo, marcado.indexOf("<button"));
    expect(conteudo).toContain(`>${traduzir("pt-BR", "preferencias.salvo.titulo")}<`);
  });

  it("so avisa da face por vez quando o campo publico do ajuste diz duas faces", () => {
    const aviso = traduzir("pt-BR", "preferencias.salvo.duasFaces");
    expect(aviso).toContain("uma face por vez");
    expect(traducoes.en["preferencias.salvo.duasFaces"]).toContain("one side at a time");
    expect(html(true)).toContain(aviso);
    expect(html(false)).not.toContain(aviso);
  });
});

describe("foco e acabamento na folha da lateral", () => {
  it("o anel de foco de Usar padrao tem 2 px, sem folga, e a coluna afasta o texto vizinho", () => {
    expect(regra(".voltarAoPadrao:focus-visible")).toContain("outline-offset: 0");
    expect(regra(".voltarAoPadrao:focus-visible")).toContain("outline-width: 2px");
    expect(regra(".ajusteSalvo")).toContain("column-gap: 6px");
    expect(regra(".ajusteSalvo")).toContain("margin: 2px -4px 0 0");
  });

  it("os dois textos do ajuste ficam em 11/16", () => {
    for (const seletor of [".ajusteSalvoTexto", ".voltarAoPadrao"]) {
      expect(regra(seletor)).toContain("font-size: 11px");
      expect(regra(seletor)).toContain("line-height: 16px");
    }
  });

  it("pressionar encolhe a engrenagem e Usar padrao", () => {
    expect(folhaDaLateral).toMatch(/\.engrenagem:active,\s+\.voltarAoPadrao:active \{\s+transform: scale\(0\.97\)/);
  });

  it("a sombra entra na transicao", () => {
    for (const seletor of [".engrenagem", ".voltarAoPadrao"]) {
      expect(regra(seletor)).toContain("box-shadow var(--anim-fast)");
    }
  });

  it("nas cores forcadas o icone do aviso usa a cor do texto do sistema", () => {
    const inicio = folhaDaLateral.indexOf("@media (forced-colors: active)");
    const forcadas = folhaDaLateral.slice(inicio);
    expect(forcadas).toMatch(/\.iconeAviso[^{]*\{\s+color: CanvasText/);
  });
});

describe("aviso do ajuste", () => {
  const html = (naLinha: boolean) =>
    renderToStaticMarkup(
      createElement(AvisoDoAjuste, {
        fila: "Sala",
        codigo: "recusado",
        naLinha,
        aoDispensar: sem,
      }),
    );

  it.each([true, false])("tem botao de dispensar com o nome da impressora (na linha: %s)", (naLinha) => {
    const marcado = html(naLinha);
    expect(marcado).toContain('role="alert"');
    expect(marcado).toContain(`aria-label="${traduzir("pt-BR", "preferencias.aviso.dispensar", { nome: "Sala" })}"`);
    expect(marcado).toContain("<button");
  });

  it("o botao e um alvo de 24 px", () => {
    expect(regra(".dispensarAviso")).toContain("width: 24px");
    expect(regra(".dispensarAviso")).toContain("height: 24px");
  });
});

describe("avisos dispensados e filas fora da lista", () => {
  it("a leitura nao recria o aviso de corrompido dispensado", () => {
    const novos = avisosDepoisDaLeitura(new Map(), ["Velha", "Nova"], new Set(["Velha"]));
    expect([...novos]).toEqual([["Nova", "corrompido"]]);
  });

  it("fila que sumiu nunca sobe ao topo", () => {
    const avisos = new Map<string, CodigoAvisoDoAjuste>([
      ["Sumiu", "fila_sumiu"],
      ["Fora", "recusado"],
    ]);
    expect(avisosSemLinha(avisos, [FILA])).toEqual([["Fora", "recusado"]]);
  });

  it("o aviso de fila que sumiu sai quando a fila deixa a lista, e o resto fica", () => {
    const avisos = new Map<string, CodigoAvisoDoAjuste>([
      ["Sumiu", "fila_sumiu"],
      ["Sala", "fila_sumiu"],
      ["Fora", "recusado"],
    ]);
    expect([...semFilasQueSairam(avisos, [FILA])].sort()).toEqual([
      ["Fora", "recusado"],
      ["Sala", "fila_sumiu"],
    ]);
  });

  it("sem nada a tirar, os avisos continuam o mesmo objeto", () => {
    const avisos = new Map<string, CodigoAvisoDoAjuste>([["Sala", "fila_sumiu"]]);
    expect(semFilasQueSairam(avisos, [FILA])).toBe(avisos);
  });

  it("ajuste de fila fora da lista continua alcancavel; simulada de mesmo nome nao conta", () => {
    const salvas = new Set(["Sala", "Fora", SIMULADA.nome]);
    expect(ajustesSemLinha(salvas, [FILA, SIMULADA])).toEqual(["Fora", SIMULADA.nome]);
  });
});

describe("confirmacao de Usar padrao", () => {
  it("nomeia a impressora, confirma pelo botao de destino e devolve o foco a origem", () => {
    const conteudo = conteudoDoVoltarAoPadrao("Sala");
    expect(traduzir("pt-BR", conteudo.titulo, conteudo.parametros)).toBe("Voltar Sala ao padrão?");
    expect(traduzir("en", conteudo.titulo, conteudo.parametros)).toBe("Return Sala to default?");
    expect(conteudo.manter).toBe("preferencias.voltarDialogo.manter");
    expect(conteudo.fechar).toBe("preferencias.voltarDialogo.confirmar");
    expect(conteudo.focoNaOrigemAoConfirmar).toBe(true);
  });

  function origemFalsa() {
    const ouvintes = new Set<() => void>();
    return {
      addEventListener: (_: string, ouvinte: () => void) => ouvintes.add(ouvinte),
      removeEventListener: (_: string, ouvinte: () => void) => ouvintes.delete(ouvinte),
      focar: () => [...ouvintes].forEach((ouvinte) => ouvinte()),
      ouvintes,
    };
  }

  it("a acao espera o foco voltar a origem e roda uma vez so", () => {
    const origem = origemFalsa();
    let vezes = 0;
    aoVoltarOFoco(origem as unknown as HTMLElement, () => (vezes += 1), 10_000);
    expect(vezes).toBe(0);
    origem.focar();
    origem.focar();
    expect(vezes).toBe(1);
    expect(origem.ouvintes.size).toBe(0);
  });

  it("sem o evento de foco, o prazo garante a acao", async () => {
    const origem = origemFalsa();
    let vezes = 0;
    aoVoltarOFoco(origem as unknown as HTMLElement, () => (vezes += 1), 1);
    await new Promise((pronto) => setTimeout(pronto, 20));
    expect(vezes).toBe(1);
  });
});

describe("confirmacao do envio real", () => {
  afterEach(() => registrarFilasComAjuste(new Set()));

  const RESUMO = {
    paginas: 2,
    lotes: 1,
    papeis: [{ formato: "a4", larguraMm: 210, alturaMm: 297 }],
    paresMistos: [],
    assinado: false,
  };

  it("a linha do ajuste vem logo depois do papel", () => {
    const { linhas } = linhasDoEnvio([FILA], RESUMO, "pt-BR", new Set(["Sala"]));
    expect(linhas).toEqual([
      "Sala",
      "2 páginas em 1 lote",
      "Papel: A4",
      "Driver: ajuste só no PlotS. Papel e faces seguem o PlotS.",
    ]);
  });

  it("sem ajuste, o dialogo fica como era", () => {
    const { linhas } = linhasDoEnvio([FILA], RESUMO, "en", new Set(["Outra"]));
    expect(linhas).toEqual(["Sala", "2 pages in 1 batch", "Paper: A4"]);
  });

  it("simulada nao tem driver, mesmo com o mesmo nome de uma fila com ajuste", () => {
    expect(linhaDoAjusteNoEnvio([SIMULADA], new Set([SIMULADA.nome]), "pt-BR")).toBeNull();
  });

  it("na divisao, a mesma frase vai para as observacoes", () => {
    registrarFilasComAjuste(new Set(["Sala"]));
    const conteudo = conteudoDaDivisao(
      [FILA, { ...FILA, id: "fila:Outra", nome: "Outra" }],
      {
        ...RESUMO,
        lotes: 2,
        divisao: [
          { impressora: "fila:Sala", lotes: [1], viraParaLer: true },
          { impressora: "fila:Outra", lotes: [2], viraParaLer: true },
        ],
        paginasDoDocumento: 4,
        faixas: [
          { lote: 1, inicio: 1, fim: 2 },
          { lote: 2, inicio: 3, fim: 4 },
        ],
      },
      null,
      "pt-BR",
    );
    expect(conteudo.notas).toContain("Driver: ajuste só no PlotS. Papel e faces seguem o PlotS.");
  });
});

describe("recusa no cartao", () => {
  const cartao = (ajusteRecusado: boolean): Cartao => ({
    aba: 1,
    impressora: "fila:Sala",
    nome: "Sala",
    simulada: false,
    lote: 1,
    total: 1,
    papel: null,
    estado: { tipo: "imprimindo", passada: "frente" },
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    ajusteRecusado,
  });
  const html = (ajusteRecusado: boolean) =>
    renderToStaticMarkup(
      createElement(CartaoImpressao, {
        cartao: cartao(ajusteRecusado),
        aoComandar: sem,
        aoPedirCancelar: sem,
        aoDispensar: sem,
      }),
    );

  it("diz com que ajuste o trabalho saiu, com icone e sem caixa", () => {
    const marcado = html(true);
    expect(marcado).toContain(traduzir("pt-BR", "preferencias.cartao.recusado"));
    expect(marcado).toContain("<svg");
  });

  it("sem recusa, o cartao nao fala do ajuste", () => {
    expect(html(false)).not.toContain(traduzir("pt-BR", "preferencias.cartao.recusado"));
  });
});
