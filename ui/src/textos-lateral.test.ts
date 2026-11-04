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
    // Dizem o que a fila do Windows fez com as frentes, e a fila é o assunto dessas frases.
    "face.estado.frentesProntas",
    "face.envio.jaSaiu",
    // O trabalho anterior pode seguir na fila do Windows: a fila é o assunto.
    "face.envio.erroAnterior",
  ]);

  // O cartao diz o que a fila confirmou, porque o PlotS nao sabe mais do que isso sobre o papel.
  it.each(IDIOMAS)("%s chama de impressora o que aparece na tela", (idioma) => {
    const palavra = idioma === "en" ? /\bqueues?\b/i : /\bfilas?\b/i;
    const encontrados = Object.entries(traducoes[idioma])
      .filter(
        ([chave, texto]) =>
          !FALAM_DA_FILA_DO_WINDOWS.has(chave) && !chave.startsWith("cartao.") && palavra.test(texto),
      )
      .map(([chave]) => chave);
    expect(encontrados).toEqual([]);
  });

  it.each(IDIOMAS)("%s diz perfil, e nao modo, ao falar da configuracao da impressora", (idioma) => {
    const palavra = idioma === "en" ? /\bmodes?\b/i : /\bmodos?\b/i;
    const encontrados = Object.entries(traducoes[idioma])
      .filter(
        ([chave, texto]) =>
          (chave.startsWith("calibracao.") ||
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

describe("dialogo de fechamento", () => {
  it.each([
    ["pt-BR", "na fila de impressão"],
    ["en", "in the print queue"],
  ] as const)("%s nomeia a fila de impressao no singular e no plural", (idioma, trecho) => {
    expect(traduzir(idioma, "confirmar.consequencia")).toContain(trecho);
    expect(traduzir(idioma, "confirmar.consequenciaVarios")).toContain(trecho);
  });
});

describe("textos da tela de calibracao", () => {
  it.each([
    ["pt-BR", "calibracao.desistir.pergunta", "Sair sem gravar?"],
    ["en", "calibracao.desistir.pergunta", "Leave without saving?"],
    ["pt-BR", "calibracao.desistir.explicacao", "O perfil da impressora fica como estava. Para calibrar depois, será preciso imprimir o teste de novo."],
    ["en", "calibracao.desistir.explicacao", "The printer's profile stays as it was. To calibrate later, you will need to print the test again."],
    ["pt-BR", "calibracao.instrucao.leitura", "Sem mudar a ordem, olhe o número de cima. Depois vire o maço inteiro e olhe de novo."],
    ["pt-BR", "calibracao.pergunta.leitura", "Quando o 1 aparece por cima?"],
    ["en", "calibracao.instrucao.leitura", "Without changing the order, look at the top number. Then flip the whole stack and look again."],
    ["en", "calibracao.pergunta.leitura", "When is the 1 on top?"],
    ["pt-BR", "calibracao.certo.depoisDeGravar", "Depois de gravar, ao imprimir o verso:"],
    ["en", "calibracao.certo.depoisDeGravar", "After saving, when printing the back:"],
    ["pt-BR", "calibracao.certo.comEstePerfil", "Com este perfil, ao imprimir o verso:"],
    ["en", "calibracao.certo.comEstePerfil", "With this profile, when printing the back:"],
    ["pt-BR", "calibracao.certo.corrigir", "Corrigir uma resposta"],
    ["en", "calibracao.certo.corrigir", "Change an answer"],
    ["pt-BR", "calibracao.julgamentoFalhou", "Não foi possível conferir as respostas. Tente de novo."],
    ["en", "calibracao.julgamentoFalhou", "Could not check the answers. Try again."],
    ["pt-BR", "calibracao.pedeArquivo", "Esta impressora pede o nome de um arquivo. Escolha uma impressora de papel."],
    ["en", "calibracao.pedeArquivo", "This printer asks for a file name. Choose a paper printer."],
    ["pt-BR", "calibracao.driverTrocado", "Esta opção mudou depois da calibração. Se a ordem sair errada, calibre de novo."],
    ["en", "calibracao.driverTrocado", "This option changed after calibration. If the order comes out wrong, calibrate again."],
  ] as const)("%s: %s tem o texto definitivo", (idioma, chave, texto) => {
    expect(traduzir(idioma, chave)).toBe(texto);
  });
});

describe("frase de repare depois do verso", () => {
  it.each([
    ["pt-BR", "Ao imprimir o verso, repare qual número sai primeiro."],
    ["en", "When printing the back, notice which number comes out first."],
  ] as const)("%s nao comeca com Depois", (idioma, texto) => {
    expect(traduzir(idioma, "calibracao.recolocar.repare")).toBe(texto);
    expect(traduzir(idioma, "calibracao.recolocar.repare")).not.toMatch(/^(Depois|Then|After)/);
  });
});

describe("textos da rodada 4", () => {
  it.each([
    ["pt-BR", "calibracao.instrucao.par", "Pegue a folha com o 1 e olhe o outro lado."],
    ["en", "calibracao.instrucao.par", "Take the sheet with the 1 and look at the other side."],
    ["pt-BR", "calibracao.pergunta.par", "O que está nele?"],
    ["en", "calibracao.pergunta.par", "What is on it?"],
    ["pt-BR", "calibracao.resumo.lado", "Segunda impressão:"],
    ["en", "calibracao.resumo.lado", "Second print:"],
    ["pt-BR", "calibracao.resumo.leitura", "1 por cima:"],
    ["en", "calibracao.resumo.leitura", "1 on top:"],
    ["pt-BR", "calibracao.resumo.primeira", "Saiu primeiro:"],
    ["en", "calibracao.resumo.primeira", "Came out first:"],
    ["pt-BR", "calibracao.resumo.par", "Atrás do 1:"],
    ["en", "calibracao.resumo.par", "Behind the 1:"],
    ["pt-BR", "calibracao.resumo.setas", "Setas:"],
    ["en", "calibracao.resumo.setas", "Arrows:"],
    ["pt-BR", "calibracao.ocupacao.enviando", "Enviando para a impressora…"],
    ["en", "calibracao.ocupacao.enviando", "Sending to the printer…"],
    ["pt-BR", "calibracao.ocupacao.gravando", "Gravando o perfil…"],
    ["en", "calibracao.ocupacao.gravando", "Saving the profile…"],
    ["pt-BR", "cartao.colocacao.cima", "Sem mudar a ordem das folhas, coloque na entrada com a face impressa para cima."],
    ["en", "cartao.colocacao.cima", "Without changing the sheet order, place it in the feed with the printed side facing up."],
    ["pt-BR", "cartao.colocacao.baixo", "Sem mudar a ordem das folhas, coloque na entrada com a face impressa para baixo."],
    ["en", "cartao.colocacao.baixo", "Without changing the sheet order, place it in the feed with the printed side facing down."],
  ] as const)("%s: %s", (idioma, chave, texto) => {
    expect(traduzir(idioma, chave)).toBe(texto);
  });
});
