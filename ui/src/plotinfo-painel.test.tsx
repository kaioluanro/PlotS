import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type {
  ConfiguracaoDeTinta,
  PerfilDeTinta,
  ResultadoDaAnalise,
  ResultadoDaPagina,
} from "./analise-tinta";
import {
  ID_GAVETA_PLOTINFO,
  PainelDeTinta,
  type ConteudoDaTinta,
} from "./components/PlotInfoGaveta";
import { traduzir } from "./i18n";
import type { Aba } from "./tabs";

const PERFIL: PerfilDeTinta = {
  id: "tanque-generico",
  nome: "Tanque de tinta genérico",
  moeda: "BRL",
  volumes_ml: [65, 65, 65, 65],
  rendimentos_paginas: [6500, 6500, 6500, 4500],
  precos_ml: [0.77, 0.77, 0.77, 1.11],
  fatores_qualidade: [0.6, 1, 1.3],
};

const CONFIGURACAO: ConfiguracaoDeTinta = {
  perfil: PERFIL.id,
  qualidade: "normal",
  dpi: 150,
  modo: "simples",
  cinzas_so_preto: true,
  tamanho_da_imagem: "a4",
};

const PAGINA: ResultadoDaPagina = {
  numero: 1,
  largura_mm: 210,
  altura_mm: 297,
  cobertura_pct: [10, 0, 0, 0],
  tinta_ml: [0.1, 0, 0, 0],
  custo: [0.1, 0, 0, 0],
  total_ml: 0.1,
  total_custo: 0.1,
  em_branco: false,
};

const PAGINA_EM_BRANCO: ResultadoDaPagina = {
  numero: 2,
  largura_mm: 210,
  altura_mm: 297,
  cobertura_pct: [0, 0, 0, 0],
  tinta_ml: [0, 0, 0, 0],
  custo: [0, 0, 0, 0],
  total_ml: 0,
  total_custo: 0,
  em_branco: true,
};

const RESULTADO: ResultadoDaAnalise = {
  id: "job-1",
  arquivo: {
    nome: "documento.pdf",
    paginas: 10,
    largura_mm: null,
    altura_mm: null,
    dpi: 150,
    modo_usado: "simples",
  },
  perfil: PERFIL.id,
  moeda: "BRL",
  qualidade: "normal",
  copias: 12,
  documento_inteiro: false,
  selecao: {
    paginas: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
    quantidade: 12,
    total_com_copias: 144,
    tinta: [4, 3, 2, 1],
    total_ml: 10,
    custo: [4, 3, 2, 1],
    total_custo: 10,
    cartucho: {
      percentual_consumido: 40,
      copias_ate_esvaziar: 50,
      canal_que_acaba_primeiro: "magenta",
    },
  },
  documento: {
    tinta: [4, 3, 2, 1],
    total_ml: 10,
    custo: [4, 3, 2, 1],
    total_custo: 10,
    cobertura_media: [10, 10, 10, 10],
    ml_medio_por_pagina: 1,
    pagina_mais_cara: {
      numero: 7,
      tinta: [1, 1, 1, 1],
      custo: [1, 1, 1, 1],
    },
  },
};

const ABA: Aba = {
  id: 1,
  documento: { caminho: "x.pdf", nome: "documento.pdf", tipo: "pdf" },
  impressaoEmAndamento: false,
  impressorasMarcadas: [],
  intervalo: null,
  face: "frente",
  frenteEnviada: null,
  frentesProntas: [],
  copias: { frente: 2, verso: 3 },
  paginas: { estado: "conhecido", total: 10 },
  cartoes: [],
};

function tinta(mudanca: Partial<ConteudoDaTinta> = {}): ConteudoDaTinta {
  return {
    situacao: "pronto",
    erro: null,
    aviso: null,
    mensagem: null,
    exportando: false,
    resultado: RESULTADO,
    paginas: new Map([
      [1, PAGINA],
      [2, PAGINA_EM_BRANCO],
    ]),
    progresso: null,
    recalculando: false,
    painel: { paginasTexto: "", copiasTexto: "12" },
    copias: 12,
    configuracao: CONFIGURACAO,
    perfilAtivo: PERFIL,
    perfis: [PERFIL],
    avisoDosPerfis: null,
    definirPaginas: () => undefined,
    definirCopias: () => undefined,
    tentarDeNovo: () => undefined,
    exportar: async () => undefined,
    ...mudanca,
  };
}

function painel(
  estado: Partial<ConteudoDaTinta> = {},
  opcoes: { aba?: Aba | null; paginaAtual?: number | null } = {},
): string {
  return renderToStaticMarkup(
    <PainelDeTinta
      aba={opcoes.aba === undefined ? ABA : opcoes.aba}
      paginaAtual={opcoes.paginaAtual === undefined ? 1 : opcoes.paginaAtual}
      tinta={tinta(estado)}
      aoFechar={() => undefined}
      aoAbrirEngrenagem={() => undefined}
      engrenagemAberta={false}
    />,
  );
}

interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}

const modulofs = "node:fs";

async function lerCss(nome: string): Promise<string> {
  const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;
  return fs.readFileSync(new URL(nome, import.meta.url), "utf8").replace(/\r\n/g, "\n");
}

function regra(css: string, seletor: string): string {
  const inicio = css.indexOf(`${seletor} {`);
  expect(inicio, seletor).toBeGreaterThanOrEqual(0);
  return css.slice(inicio, css.indexOf("}", inicio));
}

describe("cabecalho e estados da gaveta", () => {
  it("a gaveta tem nome acessivel, configurar e fechar", () => {
    const html = painel();
    expect(html).toContain(`id="${ID_GAVETA_PLOTINFO}"`);
    expect(html).toContain('aria-label="Tinta e custo"');
    expect(html).toContain(`aria-label="${traduzir("pt-BR", "plotinfo.engrenagem")}"`);
    expect(html).toContain(`aria-label="${traduzir("pt-BR", "plotinfo.fechar")}"`);
    expect(html).toContain('aria-haspopup="dialog"');
    expect(html).toContain("lucide-calculator");
    expect(html).toContain("lucide-settings");
  });

  it("sem documento explica o que fazer", () => {
    const html = painel({ situacao: "sem_documento", resultado: null }, { aba: null });
    expect(html).toContain(traduzir("pt-BR", "plotinfo.estado.vazioTitulo"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.estado.vazioTexto"));
    expect(html).not.toContain(traduzir("pt-BR", "plotinfo.secao.arquivo"));
  });

  it("contando paginas diz que esta lendo o documento", () => {
    const html = painel({ situacao: "contando", resultado: null });
    expect(html).toContain(traduzir("pt-BR", "plotinfo.estado.contando"));
  });

  it("documento recusado mostra o motivo em alerta", () => {
    const recusada: Aba = {
      ...ABA,
      paginas: { estado: "recusado", motivo: "o PDF está protegido por senha" },
    };
    const html = painel({ situacao: "recusado", resultado: null }, { aba: recusada });
    expect(html).toContain('role="alert"');
    expect(html).toContain(traduzir("pt-BR", "plotinfo.estado.recusado", {
      motivo: "o PDF está protegido por senha",
    }));
  });

  it("analisando mostra a pagina atual e a barra de 2 px", () => {
    const html = painel({
      situacao: "analisando",
      resultado: null,
      progresso: { feitas: 3, total: 12, selecao: null },
    });
    expect(html).toContain("Analisando página 3 de 12");
    expect(html).toContain('role="progressbar"');
    expect(html).toContain('aria-valuenow="3"');
    expect(html).toContain('aria-valuemax="12"');
    expect(html).toContain('aria-busy="true"');
  });

  it("a falha nao apaga o ultimo resultado e oferece tentar de novo", () => {
    const html = painel({
      situacao: "erro",
      erro: "Páginas 99 fora do documento.",
    });
    expect(html).toContain('role="alert"');
    expect(html).toContain("Páginas 99 fora do documento.");
    expect(html).toContain(traduzir("pt-BR", "plotinfo.tentarDeNovo"));
    expect(html).toContain("Página 1 de 10");
  });
});

describe("secao arquivo", () => {
  it("mostra nome, paginas, papel, dpi e modo usado", () => {
    const html = painel();
    expect(html).toContain("documento.pdf");
    expect(html).toContain(traduzir("pt-BR", "plotinfo.arquivo.papel"));
    expect(html).toContain("A4");
    expect(html).toContain("150 dpi");
    expect(html).toContain(traduzir("pt-BR", "plotinfo.modo.simples"));
  });
});

describe("secao pagina", () => {
  it("mostra a pagina atual do visualizador, uma copia e os canais", () => {
    const html = painel();
    expect(html).toContain("Página 1 de 10");
    expect(html).toContain(traduzir("pt-BR", "plotinfo.pagina.umaCopia"));
    for (const chave of [
      "plotinfo.canal.ciano",
      "plotinfo.canal.magenta",
      "plotinfo.canal.amarelo",
      "plotinfo.canal.preto",
    ] as const) {
      expect(html).toContain(traduzir("pt-BR", chave));
    }
    expect(html).toContain("10%");
    expect(html).toContain("0,1 mL");
    expect(html).toContain(traduzir("pt-BR", "plotinfo.total"));
  });

  it("pagina em branco tem o aviso e os zeros reais", () => {
    const html = painel({}, { paginaAtual: 2 });
    expect(html).toContain(traduzir("pt-BR", "plotinfo.pagina.emBranco"));
    expect(html).toContain("Página 2 de 10");
    expect(html).toContain("0%");
  });

  it("pagina ainda nao analisada nao inventa numero", () => {
    const html = painel(
      { situacao: "analisando", resultado: null, progresso: { feitas: 1, total: 10, selecao: null } },
      { paginaAtual: 5 },
    );
    expect(html).toContain("Analisando a página 5…");
  });
});

describe("secao paginas a estimar", () => {
  it("os campos sao independentes e o total e paginas vezes copias", () => {
    const casos: ReadonlyArray<[number, string, number, number]> = [
      [1, "2", 2, 2],
      [4, "1", 1, 4],
      [12, "12", 12, 144],
      [10, "5", 5, 50],
    ];
    for (const [paginas, copiasTexto, copias, total] of casos) {
      const esperado = `= ${total} páginas`;
      const resultado: ResultadoDaAnalise = {
        ...RESULTADO,
        copias,
        selecao: {
          ...RESULTADO.selecao,
          paginas: Array.from({ length: paginas }, (_, indice) => indice + 1),
          quantidade: paginas,
          total_com_copias: total,
        },
      };
      const html = painel({
        resultado,
        painel: { paginasTexto: copias === 12 ? "5-16" : "2,5,7,9", copiasTexto },
        copias,
      });
      expect(html, `${paginas} x ${copias}`).toContain(esperado);
      expect(html).toContain(`value="${copias === 12 ? "5-16" : "2,5,7,9"}"`);
      expect(html).toContain(`value="${copiasTexto}"`);
    }
  });

  it("copias fora de 1 a 999 aparecem como erro no campo", () => {
    const html = painel({
      painel: { paginasTexto: "", copiasTexto: "1000" },
      copias: null,
    });
    expect(html).toContain('aria-invalid="true"');
    expect(html).toContain(traduzir("pt-BR", "plotinfo.erro.copias"));
  });

  it("mostra tinta e custo da selecao", () => {
    const html = painel();
    expect(html).toContain(traduzir("pt-BR", "plotinfo.selecao.tinta"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.selecao.custo"));
    expect(html).toContain("10 mL");
  });
});

describe("secao documento e cartucho", () => {
  it("mostra totais por cor, media e pagina mais cara", () => {
    const html = painel();
    expect(html).toContain(traduzir("pt-BR", "plotinfo.documento.umaCopia"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.documento.media"));
    expect(html).toContain("Página mais cara: 7");
  });

  it("mostra o consumo e a cor que acaba primeiro", () => {
    const html = painel();
    expect(html).toContain("Acaba primeiro: Magenta");
    expect(html).toContain("~50 cópias até esvaziar");
    expect(html).toContain("40%");
  });

  it("sem resultado, o documento diz quando os numeros chegam", () => {
    const html = painel({ resultado: null });
    expect(html).toContain(traduzir("pt-BR", "plotinfo.documento.semDados"));
  });
});

describe("rodape", () => {
  it("repete que a estimativa nao altera o envio e nomeia perfil, qualidade e modo", () => {
    const html = painel();
    expect(html).toContain(traduzir("pt-BR", "plotinfo.naoAltera"));
    expect(html).toContain(PERFIL.nome);
    expect(html).toContain(traduzir("pt-BR", "plotinfo.qualidade.normal"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.modo.simples"));
  });

  it("exporta o CSV quando ha resultado e desliga sem ele", () => {
    const comResultado = painel();
    expect(comResultado).toContain(traduzir("pt-BR", "plotinfo.csv"));
    expect(comResultado).toContain("lucide-download");
    const semResultado = painel({ resultado: null });
    expect(semResultado).toContain(traduzir("pt-BR", "plotinfo.csv"));
    expect(semResultado).toContain('disabled=""');
    expect(semResultado).toContain(
      `title="${traduzir("pt-BR", "plotinfo.csv.semResultado")}"`,
    );
  });

  it("enquanto exporta, o botao mostra o estado ocupado", () => {
    const html = painel({ exportando: true });
    expect(html).toContain(traduzir("pt-BR", "plotinfo.csv.exportando"));
    expect(html).toContain('aria-busy="true"');
    expect(html).toContain('disabled=""');
  });

  it("a mensagem de CSV exportado aparece como status, sem depender so da cor", () => {
    const html = painel({ mensagem: "CSV exportado: C:\\saida.csv" });
    expect(html).toContain('role="status"');
    expect(html).toContain("CSV exportado: C:\\saida.csv");
    expect(html).toContain("lucide-check");
  });

  it("anuncia a conclusao num aria-live", () => {
    const html = painel();
    expect(html).toContain('role="status"');
    expect(html).toContain('aria-live="polite"');
    expect(html).toContain("Análise concluída: 144 páginas.");
  });
});

describe("folha da gaveta", () => {
  it("a gaveta tem 320 px e borda de 1 px", async () => {
    const css = await lerCss("./components/PlotInfoGaveta.module.css");
    const gaveta = regra(css, ".gaveta");
    expect(gaveta).toMatch(/flex:\s*0 0 320px/);
    expect(gaveta).toMatch(/width:\s*320px/);
    expect(gaveta).toMatch(/border-left:\s*1px solid var\(--border\)/);
  });

  it("abaixo de 900 px a gaveta passa por cima do documento", async () => {
    const css = await lerCss("./components/PlotInfoGaveta.module.css");
    expect(css).toContain("@media (max-width: 900px)");
    const media = css.slice(css.indexOf("@media (max-width: 900px)"));
    expect(media).toContain("position: absolute");
    expect(media).toContain("box-shadow: var(--shadow-lg)");
  });

  it("a barra de progresso tem 2 px e o titulo de secao e caixa alta", async () => {
    const css = await lerCss("./components/PlotInfoGaveta.module.css");
    expect(regra(css, ".trilho")).toMatch(/height:\s*2px/);
    expect(regra(css, ".tituloSecao")).toMatch(/text-transform:\s*uppercase/);
    expect(regra(css, ".tituloSecao")).toMatch(/font-size:\s*10px/);
  });

  it("usa as cores da tinta e contorno no amarelo, sem estado so por cor", async () => {
    const css = await lerCss("./components/PlotInfoGaveta.module.css");
    expect(css).toContain("background: var(--ink-cyan)");
    expect(css).toContain("background: var(--ink-magenta)");
    expect(css).toContain("background: var(--ink-yellow)");
    expect(css).toContain("background: var(--ink-key)");
    expect(regra(css, ".amarelo")).toMatch(/--ink-yellow-outline/);
    expect(regra(css, ".ponto")).toMatch(/--ink-outline/);
  });

  it("alvos de 28 px e foco visivel de 2 px", async () => {
    const css = await lerCss("./components/PlotInfoGaveta.module.css");
    const botao = regra(css, ".botaoIcone");
    expect(botao).toMatch(/width:\s*28px/);
    expect(botao).toMatch(/height:\s*28px/);
    expect(css).toContain(":focus-visible");
    expect(css).toContain("@media (forced-colors: active)");
  });
});