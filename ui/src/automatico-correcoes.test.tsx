import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { linhasDoCartao, linhasDoEnvio, type Cartao, type EstadoCartao } from "./cartoes";
import { CampoLote } from "./components/Sidebar";
import { conteudoDaDivisao } from "./confirmacao";
import { resumoDoLote, type Impressora } from "./impressoras";
import { traduzir } from "./i18n";
import { conteudoDoApagarAjuste } from "./preferencias";
import { estadoInicial, reduzirAbas, type Aba } from "./tabs";

const SALA: Impressora = {
  id: "fila:Sala",
  nome: "Sala",
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "automatico",
  duasFaces: true,
};
const RECEPCAO: Impressora = { ...SALA, id: "fila:Recepção", nome: "Recepção" };
const GAVETA: Impressora = { ...SALA, id: "fila:Gaveta", nome: "Gaveta", modo: "laser", duasFaces: false };
const SIM_AUTOMATICA: Impressora = {
  ...SALA,
  id: "simulada:automatica",
  nome: "Simulada — Automática",
  tipo: "simulada",
};
const SIM_TRASEIRA: Impressora = {
  ...SALA,
  id: "simulada:traseira",
  nome: "Simulada — Traseira",
  tipo: "simulada",
  modo: "jato",
  duasFaces: false,
};

const MISTO = {
  paginas: 6,
  lotes: 2,
  papeis: [],
  paresMistos: [],
  assinado: false,
  lotesAutomaticos: 1,
  fluxos: [
    { automatico: true, inicio: 1, fim: 4 },
    { automatico: false, inicio: 5, fim: 6 },
  ],
};

function cartao(estado: EstadoCartao, mudancas: Partial<Cartao> = {}): Cartao {
  return {
    aba: 1,
    impressora: SALA.id,
    nome: SALA.nome,
    simulada: false,
    lote: 1,
    total: 1,
    papel: null,
    estado,
    recolocar: { lado: "baixo", virada: "borda_de_cima", textoDoPerfil: null },
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    loteAutomatico: true,
    temAutomatico: true,
    ...mudancas,
  };
}

const textos = (item: Cartao, idioma: "pt-BR" | "en" = "pt-BR") =>
  linhasDoCartao(item, idioma).map((linha) => linha.texto);

function abaComPaginas(paginas: number): Aba {
  const aberta = reduzirAbas(estadoInicial, {
    tipo: "abrir",
    documento: { caminho: "C:\\a.pdf", nome: "a.pdf" },
  });
  return { ...(aberta.abas[0] as Aba), tamanhoLote: 1, paginas: { estado: "conhecido", total: paginas } };
}

describe("contagem no rodape", () => {
  it("so com automaticas mostra as paginas, sem lotes", () => {
    expect(resumoDoLote(8, 1, "pt-BR", true)).toBe("8 pág");
    expect(resumoDoLote(8, 1, "en", true)).toBe("8 pages");
  });

  it("com impressora manual junto, a contagem de lotes continua", () => {
    expect(resumoDoLote(8, 1, "pt-BR", false)).toBe("8 pág · 4 lotes");
    expect(resumoDoLote(8, 1, "pt-BR")).toBe("8 pág · 4 lotes");
  });

  it("o campo passa a escolha adiante", () => {
    const so = renderToStaticMarkup(
      <CampoLote aba={abaComPaginas(8)} aoDefinir={() => undefined} nota={null} soAutomaticas />,
    );
    const junto = renderToStaticMarkup(
      <CampoLote aba={abaComPaginas(8)} aoDefinir={() => undefined} nota={null} soAutomaticas={false} />,
    );
    expect(so).toContain(">8 pág<");
    expect(junto).toContain(">8 pág · 4 lotes<");
  });
});

describe("linhas de trecho na confirmacao", () => {
  it("dizem o gesto de cada trecho antes de pedir papel normal", () => {
    const { notas } = linhasDoEnvio([SALA], MISTO, "pt-BR", new Set());
    expect(notas.slice(0, 3)).toEqual([
      "Frente e verso automático, sem gesto: págs. 1–4.",
      "Frente e verso manual, com gesto: págs. 5–6.",
      "Use papel normal nos trechos automáticos.",
    ]);
    const ingles = linhasDoEnvio([SALA], MISTO, "en", new Set()).notas;
    expect(ingles.slice(0, 3)).toEqual([
      "Automatic duplex, no handling: pp. 1–4.",
      "Manual duplex, with handling: pp. 5–6.",
      "Use plain paper for automatic sections.",
    ]);
  });
});

describe("nome da fila nas linhas de trecho", () => {
  it("junto de uma manual, usa o nome da automatica", () => {
    const { notas } = linhasDoEnvio([SALA, GAVETA], MISTO, "pt-BR", new Set());
    expect(notas).toContain("Em Sala, págs. 1–4: frente e verso automático.");
    expect(notas).toContain("Em Sala, págs. 5–6: frente e verso manual.");
    expect(notas.join(" ")).not.toContain("Na impressora automática");
    const ingles = linhasDoEnvio([SALA, GAVETA], MISTO, "en", new Set()).notas;
    expect(ingles).toContain("On Sala, pp. 1–4: automatic duplex.");
    expect(ingles).toContain("On Sala, pp. 5–6: manual duplex.");
  });

  it("com duas automaticas, lista os nomes", () => {
    const { notas } = linhasDoEnvio([SALA, RECEPCAO, GAVETA], MISTO, "pt-BR", new Set());
    expect(notas).toContain("Em Sala e Recepção, págs. 1–4: frente e verso automático.");
    const so = linhasDoEnvio([SALA, RECEPCAO], MISTO, "en", new Set()).notas;
    expect(so).toContain("On Sala and Recepção, pp. 5–6: manual duplex.");
  });
});

describe("frase final do trabalho automatico", () => {
  const concluido: EstadoCartao = { tipo: "concluido" };

  it("com dois ou mais lotes, fala de cada maco", () => {
    expect(textos(cartao(concluido, { lote: 2, total: 2 }))).toContain(
      "Em cada maço, se a página de número menor não estiver em cima, vire o maço.",
    );
    expect(textos(cartao(concluido, { lote: 3, total: 3 }), "en")).toContain(
      "In each stack, if the lowest page number is not on top, flip that stack.",
    );
  });

  it("com lote unico, mantem a frase de um maco so", () => {
    const frases = textos(cartao(concluido));
    expect(frases).toContain("Se a primeira página não estiver em cima, vire o maço.");
    expect(frases.join(" ")).not.toContain("Em cada maço");
  });
});

describe("recusa do driver", () => {
  const recusa: EstadoCartao = { tipo: "erro", motivo: { codigo: "duas_faces_recusadas", texto: "" } };

  it("no meio do trabalho, nao diz que nada foi enviado", () => {
    const frases = textos(cartao(recusa, { lote: 3, total: 3 }));
    expect(frases.join(" ")).not.toContain("Nada foi enviado");
    expect(frases).toContain(
      "O driver recusou duas faces. Este trabalho não saiu; os lotes anteriores saíram. Escolha um modelo manual.",
    );
    expect(textos(cartao(recusa, { lote: 2, total: 2 }), "en")).toContain(
      "The driver rejected two-sided printing. This job did not print; the earlier batches did. Choose a manual model.",
    );
  });

  it("no primeiro lote, continua dizendo que nada saiu", () => {
    expect(textos(cartao(recusa, { lote: 1, total: 3 }))).toContain(
      "O driver recusou duas faces. Nada foi enviado. Escolha um modelo manual.",
    );
  });
});

describe("aviso de simulacao", () => {
  const resumo = {
    ...MISTO,
    paginasDoDocumento: 6,
    faixas: [
      { lote: 1, inicio: 1, fim: 4 },
      { lote: 2, inicio: 5, fim: 6 },
    ],
  };
  const AVISO = "Simulação: nada vai à impressora.";

  it("aparece na divisao so com simuladas", () => {
    const conteudo = conteudoDaDivisao(
      [SIM_AUTOMATICA, SIM_TRASEIRA],
      {
        ...resumo,
        divisao: [
          { impressora: SIM_AUTOMATICA.id, lotes: [1], viraParaLer: null },
          { impressora: SIM_TRASEIRA.id, lotes: [2], viraParaLer: true },
        ],
      },
      null,
      "pt-BR",
    );
    expect(conteudo.notas ?? []).toContain(AVISO);
  });

  it("aparece no lote unico so com simuladas", () => {
    const conteudo = conteudoDaDivisao(
      [SIM_AUTOMATICA, SIM_TRASEIRA],
      {
        ...resumo,
        lotes: 1,
        faixas: [{ lote: 1, inicio: 1, fim: 6 }],
        divisao: [
          { impressora: SIM_AUTOMATICA.id, lotes: [1], viraParaLer: null },
          { impressora: SIM_TRASEIRA.id, lotes: [], viraParaLer: true },
        ],
      },
      null,
      "pt-BR",
    );
    expect(conteudo.titulo).toBe("modo.unico.titulo");
    expect(conteudo.notas ?? []).toContain(AVISO);
  });

  it("com fila real recebendo, nao aparece", () => {
    const conteudo = conteudoDaDivisao(
      [SALA, SIM_TRASEIRA],
      {
        ...resumo,
        divisao: [
          { impressora: SALA.id, lotes: [1], viraParaLer: null },
          { impressora: SIM_TRASEIRA.id, lotes: [2], viraParaLer: true },
        ],
      },
      null,
      "pt-BR",
    );
    expect(conteudo.notas ?? []).not.toContain(AVISO);
  });
});

describe("nota de folhas por lote", () => {
  it("o campo aponta para a nota, que continua na pagina mesmo escondida", () => {
    const html = renderToStaticMarkup(
      <CampoLote aba={abaComPaginas(8)} aoDefinir={() => undefined} nota="modo.automatico.lote" soAutomaticas />,
    );
    const descrita = /aria-describedby="([^"]+)"/.exec(html)?.[1] ?? "";
    expect(descrita).not.toBe("");
    const paragrafo = new RegExp(`<p id="${descrita}"[^>]*>([^<]+)</p>`).exec(html);
    expect(paragrafo?.[1]).toBe(traduzir("pt-BR", "modo.automatico.lote"));
  });

  it("sem nota, nao aponta para nada", () => {
    const html = renderToStaticMarkup(
      <CampoLote aba={abaComPaginas(8)} aoDefinir={() => undefined} nota={null} soAutomaticas={false} />,
    );
    expect(html).not.toContain("aria-describedby");
  });
});

describe("dialogo do ajuste de fila fora da lista", () => {
  it("o titulo fala em apagar, nos dois idiomas", () => {
    const conteudo = conteudoDoApagarAjuste("Sala");
    expect(traduzir("pt-BR", conteudo.titulo, conteudo.parametros)).toBe("Apagar o ajuste de Sala?");
    expect(traduzir("en", conteudo.titulo, conteudo.parametros)).toBe("Delete the settings for Sala?");
  });
});

describe("cartao mais alto que a lista", () => {
  it("o estado para abaixo da faixa do nome, e as acoes abaixo dos dois", async () => {
    const modulofs = "node:fs";
    const fs = (await import(/* @vite-ignore */ modulofs)) as {
      readFileSync(caminho: URL, codificacao: "utf8"): string;
    };
    const folha = fs.readFileSync(new URL("./components/CartaoImpressao.module.css", import.meta.url), "utf8");
    const estado = /\n\.estado \{([^}]*)\}/.exec(folha)?.[1] ?? "";
    expect(estado).toMatch(/position: sticky/);
    expect(estado).toMatch(/top: var\(--altura-da-faixa/);
    expect(folha).toMatch(/scroll-margin-top: calc\(var\(--altura-da-faixa, 26px\) \+ var\(--altura-do-estado/);
  });
});

describe("medidas da lateral", () => {
  async function folhaDaLateral(): Promise<string> {
    const modulofs = "node:fs";
    const fs = (await import(/* @vite-ignore */ modulofs)) as {
      readFileSync(caminho: URL, codificacao: "utf8"): string;
    };
    return fs.readFileSync(new URL("./components/Sidebar.module.css", import.meta.url), "utf8");
  }
  const regra = (folha: string, seletor: string) =>
    new RegExp(`(?:^|\\n)${seletor.replace(".", "\\.")} \\{([^}]*)\\}`).exec(folha)?.[1] ?? "";

  it("seletores tem 28 px e o rotulo do gesto 12 px", async () => {
    const folha = await folhaDaLateral();
    expect(regra(folha, ".seletor")).toMatch(/min-height: 28px/);
    expect(regra(folha, ".gestoRotulo")).toMatch(/font-size: 12px/);
  });

  it("a lista e o rodape seguram o texto escondido dentro deles", async () => {
    const folha = await folhaDaLateral();
    expect(regra(folha, ".rolagem")).toMatch(/position: relative/);
    expect(regra(folha, ".camposDoRodape")).toMatch(/position: relative/);
  });
});
