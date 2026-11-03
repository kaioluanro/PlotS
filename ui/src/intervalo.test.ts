import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { EstadoDoEnvio } from "./cartoes";
import { traducoes } from "./i18n";
import {
  ariaDoCampo,
  envioComIntervalo,
  ESPERA_PARADA,
  leituraDoErro,
  lerIntervalo,
  limparCampo,
  linhasComTrecho,
  motivoDosCamposFechados,
  paginasDoResumo,
  passoNoCampo,
  textoDoErro,
  trechoDoEnvio,
  valoresDosCampos,
  type EstadoDaEspera,
  type LeituraIntervalo,
} from "./intervalo";
import { criarEsperaDoErro, ESPERA_DO_ERRO_MS } from "./intervalo-espera";
import { estadoInicial, reduzirAbas, type Aba, type EstadoAbas, type PaginasDaAba } from "./tabs";

const DOZE: PaginasDaAba = { estado: "conhecido", total: 12 };

function ler(inicio: string, fim: string, paginas: PaginasDaAba = DOZE) {
  return lerIntervalo(paginas, { inicio, fim });
}

function comAba(nome = "a.pdf"): EstadoAbas {
  return reduzirAbas(estadoInicial, {
    tipo: "abrir",
    documento: { caminho: `C:\\pdfs\\${nome}`, nome },
  });
}

function contada(total = 12): EstadoAbas {
  return reduzirAbas(comAba(), { tipo: "paginas_contadas", id: 1, total });
}

function aba(estado: EstadoAbas, id = 1): Aba {
  const achada = estado.abas.find((candidata) => candidata.id === id);
  if (!achada) throw new Error("sem aba");
  return achada;
}

describe("leitura dos campos do intervalo", () => {
  it("sem rascunho vale o documento inteiro", () => {
    expect(lerIntervalo(DOZE, null)).toEqual({ estado: "inteiro", total: 12 });
  });

  it("sem total conhecido nao ha o que ler", () => {
    expect(lerIntervalo(null, null).estado).toBe("indisponivel");
    expect(lerIntervalo({ estado: "contando" }, { inicio: "1", fim: "2" }).estado).toBe(
      "indisponivel",
    );
    expect(lerIntervalo({ estado: "recusado", motivo: "x" }, null).estado).toBe("indisponivel");
  });

  it("le um trecho, inclusive de uma pagina so, nas duas pontas", () => {
    expect(ler("3", "8")).toEqual({ estado: "trecho", inicio: 3, fim: 8, total: 12 });
    expect(ler("1", "1")).toMatchObject({ estado: "trecho", inicio: 1, fim: 1 });
    expect(ler("12", "12")).toMatchObject({ estado: "trecho", inicio: 12, fim: 12 });
    expect(ler(" 03 ", "008")).toMatchObject({ estado: "trecho", inicio: 3, fim: 8 });
  });

  it("de 1 ate o total e o documento inteiro", () => {
    expect(ler("1", "12")).toEqual({ estado: "inteiro", total: 12 });
    expect(ler("01", "12").estado).toBe("inteiro");
  });

  it("campo vazio aponta qual falta", () => {
    expect(ler("", "8")).toMatchObject({ erro: "vazio", campos: ["inicio"] });
    expect(ler("3", "  ")).toMatchObject({ erro: "vazio", campos: ["fim"] });
    expect(ler("", "")).toMatchObject({ erro: "vazio", campos: ["inicio", "fim"] });
  });

  it("recusa o que nao e numero inteiro", () => {
    for (const texto of ["a", "-1", "3.5", "1e3", "3-8", "٣", "+2", "0x10"]) {
      expect(ler(texto, "8"), texto).toMatchObject({ erro: "naoNumero", campos: ["inicio"] });
    }
  });

  it("zero e o que passa do total ficam fora, sem perder a conta em numero enorme", () => {
    expect(ler("0", "8")).toMatchObject({ erro: "fora", campos: ["inicio"] });
    expect(ler("3", "13")).toMatchObject({ erro: "fora", campos: ["fim"] });
    expect(ler("0", "13")).toMatchObject({ erro: "fora", campos: ["inicio", "fim"] });
    for (const enorme of ["99999999999999999999999999", "9".repeat(400)]) {
      expect(ler("1", enorme)).toMatchObject({ erro: "fora", campos: ["fim"] });
    }
    expect(ler("000000000000000000000000003", "8")).toMatchObject({ estado: "trecho", inicio: 3 });
  });

  it("fim antes do inicio marca os dois campos", () => {
    expect(ler("8", "3")).toMatchObject({ erro: "invertido", campos: ["inicio", "fim"] });
  });
});

describe("o que o intervalo muda na interface", () => {
  it("cada erro tem uma explicacao nos dois idiomas", () => {
    const casos = [ler("", "8"), ler("a", "8"), ler("3", "13"), ler("8", "3")];
    for (const idioma of ["pt-BR", "en"] as const) {
      const textos = casos.map((caso) => textoDoErro(caso, idioma));
      expect(new Set(textos).size).toBe(casos.length);
      for (const texto of textos) expect(texto).toBeTruthy();
    }
    expect(textoDoErro(ler("3", "13"), "pt-BR")).toContain("12");
    expect(textoDoErro(ler("3", "13"), "en")).toContain("12");
    expect(textoDoErro(ler("3", "8"))).toBeNull();
  });

  it("com uma pagina so, o erro nao fala em paginas no plural", () => {
    const uma: PaginasDaAba = { estado: "conhecido", total: 1 };
    expect(textoDoErro(ler("2", "2", uma), "pt-BR")).toBe(traducoes["pt-BR"]["intervalo.erro.foraUma"]);
    expect(textoDoErro(ler("2", "2", uma), "en")).toBe(traducoes.en["intervalo.erro.foraUma"]);
  });

  it("so um trecho de verdade vai ao backend", () => {
    expect(trechoDoEnvio(ler("3", "8"))).toEqual({ inicio: 3, fim: 8 });
    expect(trechoDoEnvio(ler("1", "12"))).toBeNull();
    expect(trechoDoEnvio(lerIntervalo(DOZE, null))).toBeNull();
    expect(trechoDoEnvio(ler("8", "3"))).toBeNull();
  });

  it("o resumo do lote conta so o trecho e some enquanto ha erro", () => {
    expect(paginasDoResumo(ler("3", "8"))).toBe(6);
    expect(paginasDoResumo(ler("5", "5"))).toBe(1);
    expect(paginasDoResumo(lerIntervalo(DOZE, null))).toBe(12);
    expect(paginasDoResumo(ler("8", "3"))).toBeNull();
    expect(paginasDoResumo(lerIntervalo(null, null))).toBeNull();
  });

  it("intervalo por corrigir desabilita o envio e explica; valido nao muda nada", () => {
    const envio: EstadoDoEnvio = {
      rotulo: "Imprimir frente (1)",
      motivo: null,
      desabilitado: false,
      destinos: [],
    };
    const estado = reduzirAbas(contada(), {
      tipo: "definir_intervalo",
      id: 1,
      intervalo: { inicio: "8", fim: "3" },
    });
    expect(envioComIntervalo(envio, aba(estado), ESPERA_PARADA, "pt-BR")).toEqual({
      ...envio,
      desabilitado: true,
      motivo: traducoes["pt-BR"]["intervalo.botao.invalido"],
    });
    expect(envioComIntervalo(envio, aba(estado), ESPERA_PARADA, "en").motivo).toBe(
      traducoes.en["intervalo.botao.invalido"],
    );
    expect(envioComIntervalo(envio, aba(contada()))).toBe(envio);
    expect(envioComIntervalo(envio, null)).toBe(envio);
  });

  it("os campos ficam fechados sem documento, contando ou com o PDF recusado", () => {
    expect(motivoDosCamposFechados(null, "pt-BR")).toBe(
      traducoes["pt-BR"]["intervalo.fechado.semDocumento"],
    );
    expect(motivoDosCamposFechados(aba(comAba()), "en")).toBe(
      traducoes.en["intervalo.fechado.contando"],
    );
    const recusada = reduzirAbas(comAba(), { tipo: "paginas_recusadas", id: 1, motivo: "x" });
    expect(motivoDosCamposFechados(aba(recusada), "pt-BR")).toBe(
      traducoes["pt-BR"]["intervalo.fechado.recusado"],
    );
    expect(motivoDosCamposFechados(aba(contada()))).toBeNull();
  });

  it("os campos mostram o documento inteiro ate alguem escrever, e nada antes da contagem", () => {
    expect(valoresDosCampos(null)).toEqual({ inicio: "", fim: "" });
    expect(valoresDosCampos(aba(comAba()))).toEqual({ inicio: "", fim: "" });
    expect(valoresDosCampos(aba(contada()))).toEqual({ inicio: "1", fim: "12" });
    const escrita = reduzirAbas(contada(), {
      tipo: "definir_intervalo",
      id: 1,
      intervalo: { inicio: "", fim: "abc" },
    });
    expect(valoresDosCampos(aba(escrita))).toEqual({ inicio: "", fim: "abc" });
  });

  it("sair do campo limpa so o que ja e numero", () => {
    expect(limparCampo(" 03 ")).toBe("3");
    expect(limparCampo("000")).toBe("0");
    expect(limparCampo("abc")).toBe("abc");
    expect(limparCampo("")).toBe("");
  });

  it("as setas andam dentro do documento", () => {
    const valores = { inicio: "3", fim: "8" };
    expect(passoNoCampo(valores, "inicio", 1, 12)).toEqual({ inicio: "4", fim: "8" });
    expect(passoNoCampo(valores, "fim", -1, 12)).toEqual({ inicio: "3", fim: "7" });
    expect(passoNoCampo({ inicio: "1", fim: "12" }, "inicio", -1, 12).inicio).toBe("1");
    expect(passoNoCampo({ inicio: "1", fim: "12" }, "fim", 1, 12).fim).toBe("12");
    expect(passoNoCampo({ inicio: "", fim: "abc" }, "fim", -1, 12).fim).toBe("11");
    expect(passoNoCampo({ inicio: "", fim: "abc" }, "inicio", 1, 12).inicio).toBe("2");
    expect(passoNoCampo({ inicio: "9".repeat(30), fim: "1" }, "inicio", 1, 12).inicio).toBe("12");
  });

  it("a confirmacao diz o trecho antes da contagem", () => {
    const linhas = ["Sala", "6 páginas em 1 lote", "Papel: A4"];
    expect(linhasComTrecho(linhas, ler("3", "8"), "pt-BR")).toEqual([
      "Sala",
      "Trecho: páginas 3 a 8 de 12",
      "6 páginas em 1 lote",
      "Papel: A4",
    ]);
    expect(linhasComTrecho(linhas, ler("5", "5"), "en")[1]).toBe("Range: page 5 of 12");
    expect(linhasComTrecho(linhas, lerIntervalo(DOZE, null), "pt-BR")).toEqual(linhas);
  });
});

describe("intervalo no redutor das abas", () => {
  it("aba nova comeca sem intervalo", () => {
    expect(aba(comAba()).intervalo).toBeNull();
  });

  it("guarda o que foi escrito, mesmo incompleto, sem travar a digitacao", () => {
    for (const intervalo of [
      { inicio: "3", fim: "" },
      { inicio: "", fim: "" },
      { inicio: "x", fim: "99" },
    ]) {
      const estado = reduzirAbas(contada(), { tipo: "definir_intervalo", id: 1, intervalo });
      expect(aba(estado).intervalo).toEqual(intervalo);
    }
  });

  it("o documento inteiro escrito por extenso volta a ser nenhum intervalo", () => {
    const trecho = reduzirAbas(contada(), {
      tipo: "definir_intervalo",
      id: 1,
      intervalo: { inicio: "3", fim: "8" },
    });
    const inteiro = reduzirAbas(trecho, {
      tipo: "definir_intervalo",
      id: 1,
      intervalo: { inicio: "1", fim: "12" },
    });
    expect(aba(inteiro).intervalo).toBeNull();
    const limpo = reduzirAbas(trecho, { tipo: "definir_intervalo", id: 1, intervalo: null });
    expect(aba(limpo).intervalo).toBeNull();
  });

  it("cada aba guarda o seu", () => {
    let estado = reduzirAbas(contada(), {
      tipo: "abrir",
      documento: { caminho: "C:\\pdfs\\b.pdf", nome: "b.pdf" },
    });
    estado = reduzirAbas(estado, { tipo: "paginas_contadas", id: 2, total: 5 });
    estado = reduzirAbas(estado, {
      tipo: "definir_intervalo",
      id: 1,
      intervalo: { inicio: "3", fim: "8" },
    });
    estado = reduzirAbas(estado, {
      tipo: "definir_intervalo",
      id: 2,
      intervalo: { inicio: "2", fim: "2" },
    });
    estado = reduzirAbas(estado, { tipo: "ativar", id: 1 });
    expect(aba(estado, 1).intervalo).toEqual({ inicio: "3", fim: "8" });
    expect(aba(estado, 2).intervalo).toEqual({ inicio: "2", fim: "2" });
  });

  it("antes da contagem ou com o PDF recusado, o pedido nao muda nada", () => {
    const intervalo = { inicio: "3", fim: "8" };
    const contando = reduzirAbas(comAba(), { tipo: "definir_intervalo", id: 1, intervalo });
    expect(aba(contando).intervalo).toBeNull();
    const recusada = reduzirAbas(
      reduzirAbas(comAba(), { tipo: "paginas_recusadas", id: 1, motivo: "x" }),
      { tipo: "definir_intervalo", id: 1, intervalo },
    );
    expect(aba(recusada).intervalo).toBeNull();
  });

  it("aba que ja fechou fica como esta", () => {
    const estado = contada();
    expect(
      reduzirAbas(estado, {
        tipo: "definir_intervalo",
        id: 99,
        intervalo: { inicio: "3", fim: "8" },
      }),
    ).toBe(estado);
  });
});

describe("o erro espera a digitacao acabar", () => {
  const ENVIO: EstadoDoEnvio = {
    rotulo: "Imprimir",
    desabilitado: false,
    motivo: null,
    destinos: [],
  };
  const DUZENTAS: PaginasDaAba = { estado: "conhecido", total: 200 };
  const INVERTIDO = ler("100", "20", DUZENTAS);
  const VAZIO = ler("100", "", DUZENTAS);
  const VALIDO = ler("10", "20", DUZENTAS);
  let estado: EstadoDaEspera;
  let espera: ReturnType<typeof criarEsperaDoErro>;

  function abaCom(inicio: string, fim: string): Aba {
    return aba(
      reduzirAbas(contada(200), { tipo: "definir_intervalo", id: 1, intervalo: { inicio, fim } }),
    );
  }

  const textoNaTela = (leitura: LeituraIntervalo) => {
    const naTela = leituraDoErro(leitura, estado);
    return naTela === null ? null : textoDoErro(naTela, "pt-BR");
  };

  const textoDe = (erro: "invertido" | "vazio") => traducoes["pt-BR"][`intervalo.erro.${erro}`];

  const invertidoNaTela = () => {
    espera.digitar(VALIDO, INVERTIDO);
    vi.advanceTimersByTime(ESPERA_DO_ERRO_MS);
  };

  beforeEach(() => {
    vi.useFakeTimers();
    estado = ESPERA_PARADA;
    espera = criarEsperaDoErro((novo) => {
      estado = novo;
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("o erro e o motivo so aparecem depois do tempo sem digitar", () => {
    const alvo = abaCom("100", "20");
    espera.digitar(VALIDO, INVERTIDO);
    expect(textoNaTela(INVERTIDO)).toBeNull();
    expect(envioComIntervalo(ENVIO, alvo, estado).motivo).toBeNull();
    vi.advanceTimersByTime(ESPERA_DO_ERRO_MS - 1);
    expect(textoNaTela(INVERTIDO)).toBeNull();
    vi.advanceTimersByTime(1);
    expect(textoNaTela(INVERTIDO)).toBe(textoDe("invertido"));
    expect(envioComIntervalo(ENVIO, alvo, estado, "pt-BR").motivo).toBe(
      traducoes["pt-BR"]["intervalo.botao.invalido"],
    );
  });

  it("cada tecla recomeca a contagem", () => {
    espera.digitar(VALIDO, INVERTIDO);
    vi.advanceTimersByTime(ESPERA_DO_ERRO_MS - 1);
    espera.digitar(INVERTIDO, INVERTIDO);
    vi.advanceTimersByTime(ESPERA_DO_ERRO_MS - 1);
    expect(textoNaTela(INVERTIDO)).toBeNull();
    vi.advanceTimersByTime(1);
    expect(textoNaTela(INVERTIDO)).toBe(textoDe("invertido"));
  });

  it("texto invalido desabilita o botao na hora, antes do erro aparecer", () => {
    espera.digitar(VALIDO, INVERTIDO);
    expect(envioComIntervalo(ENVIO, abaCom("100", "20"), estado).desabilitado).toBe(true);
  });

  it("texto valido deixa o botao ativo na hora, mesmo no meio da digitacao", () => {
    espera.digitar(VALIDO, VALIDO);
    expect(estado.digitando).toBe(true);
    expect(envioComIntervalo(ENVIO, abaCom("10", "20"), estado)).toBe(ENVIO);
  });

  it("sair do campo ou Enter mostra o erro sem esperar", () => {
    espera.digitar(VALIDO, INVERTIDO);
    espera.concluir();
    expect(textoNaTela(INVERTIDO)).toBe(textoDe("invertido"));
    expect(envioComIntervalo(ENVIO, abaCom("100", "20"), estado, "en").motivo).toBe(
      traducoes.en["intervalo.botao.invalido"],
    );
    // A contagem antiga ja nao muda nada depois disso.
    expect(vi.getTimerCount()).toBe(0);
  });

  it("ao ficar valido, o erro e o motivo somem na hora, sem esperar o tempo", () => {
    invertidoNaTela();
    espera.digitar(INVERTIDO, VALIDO);
    expect(textoNaTela(VALIDO)).toBeNull();
    expect(envioComIntervalo(ENVIO, abaCom("10", "20"), estado)).toBe(ENVIO);
  });

  it("de invertido para vazio antes do tempo, o erro na tela nao muda", () => {
    invertidoNaTela();
    espera.digitar(INVERTIDO, VAZIO);
    vi.advanceTimersByTime(ESPERA_DO_ERRO_MS - 1);
    expect(textoNaTela(VAZIO)).toBe(textoDe("invertido"));
    expect(leituraDoErro(VAZIO, estado)?.campos).toEqual(["inicio", "fim"]);
  });

  it("o erro parado sobrevive a varias teclas e atualiza depois do tempo", () => {
    invertidoNaTela();
    espera.digitar(INVERTIDO, VAZIO);
    espera.digitar(VAZIO, INVERTIDO);
    espera.digitar(INVERTIDO, VAZIO);
    expect(textoNaTela(VAZIO)).toBe(textoDe("invertido"));
    vi.advanceTimersByTime(ESPERA_DO_ERRO_MS);
    expect(textoNaTela(VAZIO)).toBe(textoDe("vazio"));
    expect(leituraDoErro(VAZIO, estado)?.campos).toEqual(["fim"]);
  });

  it("sair do campo ou Enter atualiza o erro parado na hora", () => {
    invertidoNaTela();
    espera.digitar(INVERTIDO, VAZIO);
    espera.concluir();
    expect(textoNaTela(VAZIO)).toBe(textoDe("vazio"));
    expect(leituraDoErro(VAZIO, estado)?.campos).toEqual(["fim"]);
  });

  it("o motivo continua na tela enquanto o erro esta parado", () => {
    invertidoNaTela();
    espera.digitar(INVERTIDO, VAZIO);
    expect(envioComIntervalo(ENVIO, abaCom("100", ""), estado, "pt-BR")).toMatchObject({
      desabilitado: true,
      motivo: traducoes["pt-BR"]["intervalo.botao.invalido"],
    });
  });

  it("depois de corrigir, um erro novo volta a esperar", () => {
    invertidoNaTela();
    espera.digitar(INVERTIDO, VALIDO);
    espera.digitar(VALIDO, INVERTIDO);
    expect(textoNaTela(INVERTIDO)).toBeNull();
  });

  it("encerrar nao deixa contagem rodando", () => {
    espera.digitar(VALIDO, VALIDO);
    espera.encerrar();
    expect(vi.getTimerCount()).toBe(0);
  });

  it("o motivo de antes continua valendo enquanto se digita", () => {
    const comMotivo = { ...ENVIO, desabilitado: true, motivo: "sem impressora" };
    espera.digitar(VALIDO, INVERTIDO);
    expect(envioComIntervalo(comMotivo, abaCom("100", "20"), estado).motivo).toBe("sem impressora");
  });
});

describe("o campo para o leitor de tela", () => {
  it("com numero, diz o numero; com outro texto, le o texto", () => {
    expect(ariaDoCampo("3", 12)).toEqual({
      "aria-valuemax": 12,
      "aria-valuenow": 3,
      "aria-valuetext": undefined,
    });
    expect(ariaDoCampo("x", 12)).toEqual({
      "aria-valuemax": 12,
      "aria-valuenow": undefined,
      "aria-valuetext": "x",
    });
    expect(ariaDoCampo("", 12)["aria-valuenow"]).toBeUndefined();
    expect(ariaDoCampo("", 12)["aria-valuetext"]).toBe("");
    expect(ariaDoCampo("  ", 12)["aria-valuenow"]).toBeUndefined();
    expect(ariaDoCampo("99999999999999999999", 12)["aria-valuenow"]).toBeUndefined();
  });

  it("sem total, nao anuncia maximo", () => {
    expect(ariaDoCampo("", 0)["aria-valuemax"]).toBeUndefined();
  });
});

describe("textos do motivo do botao", () => {
  it("fala das paginas nos dois idiomas", () => {
    expect(traducoes["pt-BR"]["intervalo.botao.invalido"]).toBe(
      "Corrija as páginas para imprimir.",
    );
    expect(traducoes.en["intervalo.botao.invalido"]).toBe("Fix the pages to print.");
  });
});
