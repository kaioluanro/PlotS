import { describe, expect, it } from "vitest";
import { traducoes, traduzir, type Chave, type Idioma } from "./i18n";
import { nomeDeExibicao, textoDoAviso, type Impressora } from "./impressoras";

const IDIOMAS: readonly Idioma[] = ["pt-BR", "en"];

describe("texto dos modelos", () => {
  it.each(IDIOMAS)("%s diz que o modelo e ponto de partida a conferir com folhas", (idioma) => {
    expect(traduzir(idioma, "modo.seletor.nota")).toMatch(idioma === "en" ? /2 or 3 sheets/ : /2 ou 3 folhas/);
  });

  it("os nomes dos modelos nao trazem marca", () => {
    expect(traduzir("pt-BR", "modo.jato")).toBe("Jato de tinta");
    expect(traduzir("pt-BR", "modo.laser")).toBe("Laser");
    expect(traduzir("en", "modo.jato")).toBe("Inkjet");
    expect(traduzir("en", "modo.laser")).toBe("Laser");
  });

  it.each(IDIOMAS)("%s diz qual fila nao gravou e o que fazer", (idioma) => {
    const texto = textoDoAviso({ codigo: "modelo_nao_gravado", detalhe: "Sala" }, idioma);
    expect(texto).toContain("Sala");
    expect(texto).toMatch(idioma === "en" ? /try again\.$/ : /tente de novo\.$/);
  });
});

describe("avisos de perfil dizem como sair do problema", () => {
  it.each([
    ["perfis_invalido", "pt-BR", /Restaure uma cópia ou remova o arquivo e calibre de novo/],
    ["perfis_invalido", "en", /Restore a copy or remove the file and calibrate again/],
    ["perfis_grande", "pt-BR", /Restaure uma cópia ou remova o arquivo e calibre de novo/],
    ["perfis_grande", "en", /Restore a copy or remove the file and calibrate again/],
    ["perfis_versao", "pt-BR", /Atualize o PlotS/],
    ["perfis_versao", "en", /Update PlotS/],
    ["perfis_pasta", "pt-BR", /^O Windows não informou .*Entre de novo no Windows e reabra o PlotS\.$/],
    ["perfis_pasta", "en", /^Windows did not report .*Sign in to Windows again and open PlotS again\.$/],
  ] as const)("%s em %s", (codigo, idioma, esperado) => {
    expect(textoDoAviso({ codigo }, idioma)).toMatch(esperado);
  });
});

describe("vocabulario da interface", () => {
  // O dialogo de fechamento fala da fila de impressao do Windows, nao do nome de uma impressora.
  const FALAM_DA_FILA_DO_WINDOWS = new Set<string>([
    "aviso.filas",
    "aviso.filasCodigo",
    "confirmar.consequencia",
    "confirmar.consequenciaVarios",
  ]);

  it.each(IDIOMAS)("%s chama de impressora o que aparece na tela", (idioma) => {
    const palavra = idioma === "en" ? /\bqueues?\b/i : /\bfilas?\b/i;
    const encontrados = Object.entries(traducoes[idioma])
      .filter(([chave, texto]) => !FALAM_DA_FILA_DO_WINDOWS.has(chave) && palavra.test(texto))
      .map(([chave]) => chave);
    expect(encontrados).toEqual([]);
  });

  it.each(IDIOMAS)("%s diz perfil, e nao modo, ao falar da configuracao da impressora", (idioma) => {
    const palavra = idioma === "en" ? /\bmodes?\b/i : /\bmodos?\b/i;
    const encontrados = Object.entries(traducoes[idioma])
      .filter(
        ([chave, texto]) =>
          (chave.startsWith("calibrar.") ||
            chave.startsWith("aviso.perfis_") ||
            chave === "modo.desconhecido.titulo") &&
          palavra.test(texto),
      )
      .map(([chave]) => chave);
    expect(encontrados).toEqual([]);
  });

  it("o ingles usa Tray no nome simulado", () => {
    const simulada: Impressora = {
      id: "simulada:gaveta",
      nome: "Simulada — Gaveta",
      tipo: "simulada",
      nivel: "pronta",
      condicoes: [],
      modo: "laser",
    };
    expect(nomeDeExibicao(simulada, "en")).toBe("Simulated — Tray");
    expect(JSON.stringify(traducoes.en)).not.toMatch(/drawer/i);
  });
});

describe("PDF protegido", () => {
  const casos: ReadonlyArray<[Idioma, string]> = [
    [
      "pt-BR",
      "a.pdf foi recusado: o PDF está protegido por senha. Se você tiver permissão para isso, salve uma cópia sem senha e abra de novo.",
    ],
    [
      "en",
      "a.pdf was refused: the PDF is password protected. If you are allowed to, save a copy without a password and open it again.",
    ],
  ];
  it.each(casos)("%s condiciona a copia a ter permissao", (idioma, esperado) => {
    const chave: Chave = "erro.criptografado";
    expect(traduzir(idioma, chave, { nome: "a.pdf" })).toBe(esperado);
  });
});

describe("dialogo de calibrar", () => {
  it.each([
    ["pt-BR", /na pasta atual/],
    ["en", /in the current folder/],
  ] as const)("%s diz onde o arquivo fica", (idioma, esperado) => {
    expect(traduzir(idioma, "calibrar.passos")).toMatch(esperado);
  });

  it.each([
    ["pt-BR", /No PowerShell, use:$/, /rode-o na pasta do executável plots-cli, gerado ao compilar o projeto\.$/],
    ["en", /In PowerShell, use:$/, /run it in the folder of the plots-cli executable, created when you build the project\.$/],
  ] as const)("%s diz onde rodar o comando", (idioma, intro, instalacao) => {
    expect(traduzir(idioma, "calibrar.intro")).toMatch(intro);
    expect(traduzir(idioma, "calibrar.instalacao")).toMatch(instalacao);
  });
});

describe("impressora sem comando seguro", () => {
  it.each([
    [
      "pt-BR",
      "O nome desta impressora tem aspas duplas e %, e o PlotS não consegue montar um comando seguro para o PowerShell. Renomeie a impressora nas configurações do Windows e use Atualizar lista.",
    ],
    [
      "en",
      "This printer's name has double quotes and %, and PlotS cannot build a safe PowerShell command for it. Rename the printer in Windows settings, then use Refresh list.",
    ],
  ] as const)("%s manda renomear a impressora", (idioma, esperado) => {
    expect(traduzir(idioma, "calibrar.semComando")).toBe(esperado);
  });
});

describe("dialogo de fechamento", () => {
  it.each([
    ["pt-BR", "na fila de impressão"],
    ["en", "in the print queue"],
  ] as const)("%s nomeia a fila de impressao no singular e no plural", (idioma, trecho) => {
    expect(traduzir(idioma, "confirmar.consequencia")).toContain(trecho);
    expect(traduzir(idioma, "confirmar.consequenciaVarios")).toContain(trecho);
  });
});
