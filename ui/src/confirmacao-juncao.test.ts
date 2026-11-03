import { describe, expect, it } from "vitest";
import { cartaoEmAndamento, estadoDoEnvio, rodapeCedeAoCartao, type Cartao, type EstadoCartao, type FaixaDoLote } from "./cartoes";
import {
  conteudoDaDivisao,
  conteudoDaReimpressao,
  divisaoDoResumo,
  envioComDivisao,
  juncaoDaAba,
  ordemDaPilha,
  type ResumoComDivisao,
} from "./confirmacao";
import { traduzir } from "./i18n";
import type { Impressora } from "./impressoras";
import { estadoInicial, reduzirAbas, type Aba, type ParteDaDivisao } from "./tabs";

const faixa = (lote: number): FaixaDoLote => ({ lote, inicio: lote * 4 - 3, fim: lote * 4 });

function parte(nome: string, lotes: number[], viraParaLer: boolean | null): ParteDaDivisao {
  return { impressora: nome, nome, lotes, viraParaLer, faixas: lotes.map(faixa) };
}

const JATO = parte("Jato", [1, 3, 5], false);
const LASER = parte("Laser", [2, 4, 6], true);
const A4 = { formato: "a4", larguraMm: 210, alturaMm: 297 };

const SIMULADA = (id: string, nome: string): Impressora =>
  ({ id, nome, tipo: "simulada", nivel: "pronta", condicoes: [], modo: "jato" }) as Impressora;
const FILA = (nome: string): Impressora =>
  ({ id: `fila:${nome}`, nome, tipo: "fila", nivel: "pronta", condicoes: [], modo: "laser" }) as Impressora;

function resumo(divisao: ResumoComDivisao["divisao"], lotes = 6): ResumoComDivisao {
  return {
    paginas: lotes * 4,
    lotes,
    papeis: [A4],
    paresMistos: [],
    assinado: false,
    divisao,
    paginasDoDocumento: lotes * 4,
    faixas: Array.from({ length: lotes }, (_, i) => faixa(i + 1)),
  };
}

describe("ordem da pilha", () => {
  it("uma linha por lote, em ordem, com as paginas que vieram do backend", () => {
    const ordem = ordemDaPilha([LASER, JATO], 24, [A4], "pt-BR");
    expect(ordem.cabecalhoDoGesto).toBe(true);
    expect(ordem.linhas.map((linha) => linha.lote)).toEqual([1, 2, 3, 4, 5, 6]);
    expect(ordem.linhas[1]).toEqual({
      lote: 2,
      paginas: "págs. 5–8",
      impressora: "Laser",
      gesto: "vire",
    });
    expect(ordem.linhas[0]?.gesto).toBe("não vire");
    expect(ordem.comGesto).toBe(true);
    expect(ordem.frases).toEqual([]);
    expect(ordem.resumo).toBe("24 páginas em 6 lotes · A4");
  });

  it("todos com o mesmo gesto: a coluna some e fica uma frase so", () => {
    const viram = ordemDaPilha([parte("A", [1], true), parte("B", [2], true)], 8, [A4], "pt-BR");
    expect(viram.comGesto).toBe(false);
    expect(viram.linhas.every((linha) => linha.gesto === null)).toBe(true);
    expect(viram.frases).toEqual(["Ao terminar, vire cada maço antes de empilhar."]);
    expect(viram.cabecalhoDoGesto).toBe(false);
    const naoViram = ordemDaPilha([parte("A", [1], false), parte("B", [2], false)], 8, [A4], "en");
    expect(naoViram.frases).toEqual(["When finished, do not flip the stacks."]);
  });

  it("perfil que nao afirma o gesto nao ganha uma ordem de virar", () => {
    const misturado = ordemDaPilha([parte("M", [1], null), parte("B", [2], true)], 8, [A4], "en");
    expect(misturado.linhas[0]?.gesto).toBe("check");
    expect(misturado.frases[0]).toContain("as in the calibration");
    const nenhum = ordemDaPilha([parte("M", [1], null), parte("N", [2], null)], 8, [A4], "pt-BR");
    expect(nenhum.comGesto).toBe(false);
    expect(nenhum.frases[0]).toContain("como na calibração");
  });

  it("todos os lotes aparecem, sem reticencias", () => {
    const impares = Array.from({ length: 50 }, (_, i) => i * 2 + 1);
    const ordem = ordemDaPilha(
      [parte("A", impares, false), parte("B", impares.map((n) => n + 1), false)],
      400,
      [A4],
      "pt-BR",
    );
    expect(ordem.linhas).toHaveLength(100);
    expect(JSON.stringify(ordem)).not.toContain("…");
  });

  it("pagina unica nao vira faixa, e trecho mantem os numeros do documento", () => {
    const parcial: ParteDaDivisao = {
      impressora: "A",
      nome: "A",
      lotes: [1, 2],
      viraParaLer: false,
      faixas: [
        { lote: 1, inicio: 3, fim: 6 },
        { lote: 2, inicio: 7, fim: 7 },
      ],
    };
    const ordem = ordemDaPilha([parcial], 5, [A4], "en");
    expect(ordem.linhas.map((linha) => linha.paginas)).toEqual(["pp. 3–6", "p. 7"]);
  });
});

describe("confirmacao da divisao", () => {
  const destinos = [SIMULADA("simulada:traseira", "T"), SIMULADA("simulada:gaveta", "G")];
  const duas = resumo([
    { impressora: "simulada:traseira", lotes: [1, 3, 5], viraParaLer: false },
    { impressora: "simulada:gaveta", lotes: [2, 4, 6], viraParaLer: true },
  ]);

  it("mostra a pilha no lugar das linhas por impressora e do paragrafo de empilhar", () => {
    const conteudo = conteudoDaDivisao(destinos, duas, null, "pt-BR");
    expect(conteudo.titulo).toBe("modo.envio.titulo");
    expect(conteudo.linhas).toEqual([]);
    expect(conteudo.pilha?.linhas).toHaveLength(6);
    expect(conteudo.pilha?.resumo).toBe("24 páginas em 6 lotes · A4");
    const notas = conteudo.notas ?? [];
    expect(notas).toContain(traduzir("pt-BR", "modo.juntar.separar"));
    expect(notas).toContain("Em cada maço, a página de número menor fica por cima.");
    expect(notas.join(" ")).not.toContain("Empilhe");
  });

  it("com trecho, a linha do trecho vem antes e os numeros sao os do documento", () => {
    const conteudo = conteudoDaDivisao(
      destinos,
      {
        ...duas,
        paginas: 12,
        lotes: 3,
        paginasDoDocumento: 20,
        faixas: [
          { lote: 1, inicio: 3, fim: 6 },
          { lote: 2, inicio: 7, fim: 10 },
          { lote: 3, inicio: 11, fim: 14 },
        ],
        divisao: [
          { impressora: "simulada:traseira", lotes: [1, 3], viraParaLer: false },
          { impressora: "simulada:gaveta", lotes: [2], viraParaLer: false },
        ],
      },
      { estado: "trecho", inicio: 3, fim: 14, total: 20 },
      "pt-BR",
    );
    expect(conteudo.linhas).toEqual(["Trecho: páginas 3 a 14 de 20"]);
    expect(conteudo.pilha?.linhas.map((linha) => linha.paginas)).toEqual([
      "págs. 3–6",
      "págs. 7–10",
      "págs. 11–14",
    ]);
    expect(conteudo.pilha?.resumo).toBe("12 páginas em 3 lotes · A4");
  });

  it("avisa antes de enviar quem fica sem lote", () => {
    const conteudo = conteudoDaDivisao(
      [...destinos, FILA("Sobra")],
      resumo(
        [
          { impressora: "simulada:traseira", lotes: [1], viraParaLer: false },
          { impressora: "simulada:gaveta", lotes: [2], viraParaLer: false },
          { impressora: "fila:Sobra", lotes: [], viraParaLer: true },
        ],
        2,
      ),
      null,
      "pt-BR",
    );
    expect(conteudo.notas).toContain("Sobra fica sem lote e não recebe nada.");
    expect(conteudo.pilha?.linhas.some((linha) => linha.impressora === "Sobra")).toBe(false);
  });

  it("um lote so: sem dividir, separar nem empilhar", () => {
    const conteudo = conteudoDaDivisao(
      [FILA("Sala"), FILA("Copa")],
      resumo(
        [
          { impressora: "fila:Sala", lotes: [1], viraParaLer: false },
          { impressora: "fila:Copa", lotes: [], viraParaLer: false },
        ],
        1,
      ),
      null,
      "pt-BR",
    );
    expect(conteudo.titulo).toBe("modo.unico.titulo");
    expect(conteudo.fechar).toBe("modo.unico.confirmar");
    expect(traduzir("pt-BR", conteudo.texto, conteudo.parametros)).toBe(
      "Só há 1 lote: Sala imprime tudo. Copa fica de fora.",
    );
    expect(conteudo.pilha).toBeUndefined();
    const tudo = [...(conteudo.linhas ?? []), ...(conteudo.notas ?? [])].join(" ");
    expect(tudo).not.toMatch(/Dividir|separado|Empilhe/);
  });

  it("so com simuladas nao cita uma fila real que nao existe", () => {
    const conteudo = conteudoDaDivisao(destinos, duas, null, "pt-BR");
    expect(conteudo.notas).not.toContain(traduzir("pt-BR", "cartao.envio.simuladas"));
  });

  it("com fila real, o nome dela aparece so na pilha", () => {
    const conteudo = conteudoDaDivisao(
      [FILA("Sala"), SIMULADA("simulada:gaveta", "G")],
      resumo(
        [
          { impressora: "fila:Sala", lotes: [1], viraParaLer: true },
          { impressora: "simulada:gaveta", lotes: [2], viraParaLer: true },
        ],
        2,
      ),
      null,
      "pt-BR",
    );
    expect(conteudo.linhas?.some((linha) => linha.includes("Sala"))).toBe(false);
    expect(conteudo.pilha?.linhas[0]?.impressora).toBe("Sala");
  });

  it("impressora que o backend citou e a interface nao conhece fica fora", () => {
    expect(
      divisaoDoResumo(
        destinos,
        resumo([{ impressora: "fila:Outra", lotes: [1], viraParaLer: null }]),
        "pt-BR",
      ),
    ).toEqual([]);
  });
});

function cartao(impressora: string, estado: EstadoCartao, lote: number): Cartao {
  return {
    aba: 1,
    impressora,
    nome: impressora,
    simulada: true,
    lote,
    total: 3,
    papel: null,
    estado,
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
  };
}

function abaComCartoes(...cartoes: Cartao[]): Aba {
  let estado = reduzirAbas(estadoInicial, {
    tipo: "abrir",
    documento: { caminho: "a.pdf", nome: "a.pdf" },
  });
  estado = reduzirAbas(estado, {
    tipo: "divisao_enviada",
    id: 1,
    divisao: { partes: [JATO, LASER], papeis: [A4] },
  });
  for (const item of cartoes) estado = reduzirAbas(estado, { tipo: "cartao", cartao: item });
  const aba = estado.abas[0];
  if (!aba) throw new Error("sem aba");
  return aba;
}

describe("fim do trabalho", () => {
  const CONCLUIDO: EstadoCartao = { tipo: "concluido" };

  it("a juncao fala do que fazer agora, sem o cabecalho da coluna de gesto", () => {
    const misturado = juncaoDaAba(
      abaComCartoes(cartao("Jato", CONCLUIDO, 3), cartao("Laser", CONCLUIDO, 3)),
      "pt-BR",
    );
    expect(misturado?.pilha.comGesto).toBe(true);
    expect(misturado?.pilha.cabecalhoDoGesto).toBe(false);
    const frases = (partes: ParteDaDivisao[], idioma: "pt-BR" | "en") =>
      ordemDaPilha(partes, 8, [A4], idioma, "juncao").frases;
    const viram = [parte("A", [1], true), parte("B", [2], true)];
    const naoViram = [parte("A", [1], false), parte("B", [2], false)];
    const conferem = [parte("A", [1], null), parte("B", [2], null)];
    expect(frases(viram, "pt-BR")).toEqual(["Vire cada maço antes de empilhar."]);
    expect(frases(viram, "en")).toEqual(["Flip each stack before piling them up."]);
    expect(frases(naoViram, "pt-BR")).toEqual(["Não vire os maços."]);
    expect(frases(naoViram, "en")).toEqual(["Do not flip the stacks."]);
    expect(frases(conferem, "pt-BR")).toEqual([
      "Deixe a primeira folha de cada maço por cima, como na calibração.",
    ]);
    expect(frases(conferem, "en")).toEqual([
      "Keep the first sheet of each stack on top, as in the calibration.",
    ]);
  });

  it("os titulos da juncao e da lista sao curtos e nao falam em terminar", () => {
    expect(traduzir("pt-BR", "modo.juntar.titulo")).toBe("Como juntar os maços");
    expect(traduzir("en", "modo.juntar.titulo")).toBe("How to join the stacks");
    expect(traduzir("pt-BR", "modo.pilha.titulo")).toBe("Pilha, de cima para baixo");
    expect(traduzir("en", "modo.pilha.titulo")).toBe("Stack order, top to bottom");
  });

  it("a dica da divisao cabe em uma linha", () => {
    expect(traduzir("pt-BR", "modo.divisao.dica")).toBe("Cada uma imprime parte; junte os maços.");
    expect(traduzir("en", "modo.divisao.dica")).toBe("Each prints a part; you join the stacks.");
  });

  it("so aparece quando as duas terminaram", () => {
    const uma = abaComCartoes(
      cartao("Jato", CONCLUIDO, 3),
      cartao("Laser", { tipo: "aguardando" }, 2),
    );
    expect(juncaoDaAba(uma, "pt-BR")).toBeNull();
    const duas = abaComCartoes(cartao("Jato", CONCLUIDO, 3), cartao("Laser", CONCLUIDO, 3));
    const juncao = juncaoDaAba(duas, "pt-BR");
    expect(juncao?.faltas).toEqual([]);
    expect(juncao?.pilha.linhas).toHaveLength(6);
    expect(juncao?.pilha.resumo).toBe("24 páginas em 6 lotes · A4");
    expect(juncao?.conferencia).toContain("número menor");
  });

  it("no espelho nao ha juncao", () => {
    const aba = { ...abaComCartoes(cartao("Jato", CONCLUIDO, 3)), divisao: null };
    expect(juncaoDaAba(aba, "pt-BR")).toBeNull();
  });

  it("um lote so nao tem o que juntar", () => {
    let estado = reduzirAbas(estadoInicial, {
      tipo: "abrir",
      documento: { caminho: "a.pdf", nome: "a.pdf" },
    });
    estado = reduzirAbas(estado, {
      tipo: "divisao_enviada",
      id: 1,
      divisao: { partes: [parte("Jato", [1], false)] },
    });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao("Jato", CONCLUIDO, 1) });
    const aba = estado.abas[0];
    if (!aba) throw new Error("sem aba");
    expect(juncaoDaAba(aba, "pt-BR")).toBeNull();
  });

  it("erro ainda sem resposta segura a juncao; cancelado diz o que faltou e tira da pilha", () => {
    const motivo = { codigo: "fila", texto: "" };
    expect(
      juncaoDaAba(
        abaComCartoes(
          cartao("Jato", CONCLUIDO, 3),
          cartao("Laser", { tipo: "erro", motivo }, 2),
        ),
        "pt-BR",
      ),
    ).toBeNull();
    const juncao = juncaoDaAba(
      abaComCartoes(cartao("Jato", CONCLUIDO, 3), cartao("Laser", { tipo: "cancelado" }, 2)),
      "pt-BR",
    );
    expect(juncao?.faltas).toEqual([
      "Faltam os lotes 4, 6 da Laser (págs. 13–16, 21–24). O documento fica incompleto: imprima essas páginas de novo.",
    ]);
    const lotes = juncao?.pilha.linhas.map((linha) => linha.lote);
    expect(lotes).toEqual([1, 2, 3, 5]);
    expect(juncao?.pilha.resumo).toBe("16 páginas em 4 lotes · A4");
  });

  it("nada impresso em lugar nenhum: nao mostra como juntar", () => {
    const juncao = juncaoDaAba(
      abaComCartoes(
        cartao("Jato", { tipo: "cancelado" }, 1),
        cartao("Laser", { tipo: "cancelado" }, 1),
      ),
      "en",
    );
    expect(juncao).toBeNull();
  });

  it("documento que mudou: impressora livre, sem juncao e com o envio de volta", () => {
    const mudou: EstadoCartao = { tipo: "erro", motivo: { codigo: "documento_mudou", texto: "" } };
    const item = { ...cartao("Jato", mudou, 0), total: 0 };
    expect(cartaoEmAndamento(item)).toBe(false);
    const aba = abaComCartoes(item, { ...cartao("Laser", mudou, 0), total: 0 });
    expect(juncaoDaAba(aba, "pt-BR")).toBeNull();
    expect(envioComDivisao(estadoDoEnvio(aba, [], new Set()), aba, [], new Set()).semDestino).toBe(
      undefined,
    );
  });

  it("a falta de um lote so usa o singular", () => {
    const juncao = juncaoDaAba(
      abaComCartoes(cartao("Jato", CONCLUIDO, 3), cartao("Laser", { tipo: "cancelado" }, 3)),
      "en",
    );
    expect(juncao?.faltas).toEqual([
      "Batch 6 from Laser is missing (pp. 21–24). The document is incomplete: print those pages again.",
    ]);
  });

  it("cartao dispensado continua contando o que faltava nele", () => {
    let estado = reduzirAbas(estadoInicial, {
      tipo: "abrir",
      documento: { caminho: "a.pdf", nome: "a.pdf" },
    });
    estado = reduzirAbas(estado, {
      tipo: "divisao_enviada",
      id: 1,
      divisao: { partes: [JATO, LASER] },
    });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao("Jato", CONCLUIDO, 3) });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao("Laser", { tipo: "cancelado" }, 3) });
    estado = reduzirAbas(estado, { tipo: "dispensar_cartao", id: 1, impressora: "Laser" });
    const aba = estado.abas[0];
    if (!aba) throw new Error("sem aba");
    expect(juncaoDaAba(aba, "pt-BR")?.faltas[0]).toContain("Falta o lote 6 da Laser");
    estado = reduzirAbas(estado, { tipo: "dispensar_cartao", id: 1, impressora: "Jato" });
    const vazia = estado.abas[0];
    if (!vazia) throw new Error("sem aba");
    expect(juncaoDaAba(vazia, "pt-BR")).toBeNull();
  });
});

describe("botao de envio na divisao", () => {
  const A = FILA("A");
  const B = FILA("B");
  const C = FILA("C");

  function aba(marcadas: string[], modo: "espelho" | "divisao", cartoes: Cartao[] = []): Aba {
    return {
      ...abaComCartoes(...cartoes),
      impressorasMarcadas: marcadas,
      modo,
      paginas: { estado: "conhecido", total: 24 },
    };
  }

  const envio = (alvo: Aba, lista: Impressora[], ocupadas: string[]) => {
    const conjunto = new Set(ocupadas);
    return envioComDivisao(estadoDoEnvio(alvo, lista, conjunto, "pt-BR"), alvo, lista, conjunto, "pt-BR");
  };

  it("com divisao em curso nesta aba, desabilita e explica", () => {
    const alvo = aba(["fila:C"], "espelho", [cartao("Jato", { tipo: "aguardando" }, 1)]);
    const resultado = envio(alvo, [C], []);
    expect(resultado.desabilitado).toBe(true);
    expect(resultado.semDestino).toBe(true);
    expect(resultado.motivo).toBe("A divisão deste documento ainda não terminou.");
    expect(resultado.rotulo).not.toContain("(1)");
  });

  it("divisao terminada nao segura o proximo envio", () => {
    const alvo = aba(["fila:C"], "espelho", [cartao("Jato", { tipo: "concluido" }, 3)]);
    expect(envio(alvo, [C], []).desabilitado).toBe(false);
  });

  it("em divisao, com uma ocupada e a outra livre, nao oferece enviar so a uma", () => {
    const alvo = { ...aba(["fila:A", "fila:B"], "divisao"), divisao: null };
    const resultado = envio(alvo, [A, B], ["fila:A"]);
    expect(resultado.desabilitado).toBe(true);
    expect(resultado.semDestino).toBeUndefined();
    expect(resultado.rotulo).toBe("Imprimir frente");
    expect(resultado.motivo).toBe("Divisão: A está ocupada. Aguarde ou desmarque-a.");
  });

  describe("o rodape na altura curta", () => {
    const EM_CURSO: EstadoCartao = { tipo: "aguardando" };
    const FIM: EstadoCartao = { tipo: "concluido" };
    const cede = (alvo: Aba, lista: Impressora[], ocupadas: string[]) =>
      rodapeCedeAoCartao(envio(alvo, lista, ocupadas), alvo, lista, new Set(ocupadas));

    it("sai com a divisao em curso nesta aba", () => {
      const alvo = aba(["fila:C"], "espelho", [cartao("Jato", EM_CURSO, 1)]);
      expect(cede(alvo, [C], [])).toBe(true);
    });

    it("fica com uma marcada ocupada e a outra livre, com cartao em andamento", () => {
      const alvo = { ...aba(["fila:A", "fila:B"], "divisao", [cartao("fila:A", EM_CURSO, 1)]), divisao: null };
      expect(cede(alvo, [A, B], ["fila:A"])).toBe(false);
    });

    it("fica quando os cartoes da aba ja encerraram", () => {
      const alvo = { ...aba(["fila:A", "fila:B"], "divisao", [cartao("fila:A", FIM, 3)]), divisao: null };
      expect(cede(alvo, [A, B], ["fila:A"])).toBe(false);
    });

    it("fica quando todas as marcadas estao ocupadas por outra aba", () => {
      const alvo = { ...aba(["fila:A", "fila:B"], "espelho", [cartao("fila:A", FIM, 3)]), divisao: null };
      expect(cede(alvo, [A, B], ["fila:A", "fila:B"])).toBe(false);
    });

    it("sai quando toda marcada tem cartao em andamento nesta aba", () => {
      const alvo = {
        ...aba(["fila:A", "fila:B"], "espelho", [cartao("fila:A", EM_CURSO, 1), cartao("fila:B", EM_CURSO, 1)]),
        divisao: null,
      };
      expect(cede(alvo, [A, B], ["fila:A", "fila:B"])).toBe(true);
    });

    it("fica sem marcada e sem aba", () => {
      const alvo = { ...aba([], "espelho", [cartao("fila:A", EM_CURSO, 1)]), divisao: null };
      expect(cede(alvo, [A, B], ["fila:A"])).toBe(false);
      const botao = estadoDoEnvio(null, [A], new Set(), "pt-BR");
      expect(rodapeCedeAoCartao(botao, null, [A], new Set())).toBe(false);
    });
  });

  it("fora da conta por outro motivo tem a sua frase", () => {
    const semPerfil = { ...FILA("Sala"), modo: "sem_perfil" } as Impressora;
    const alvo = { ...aba(["fila:A", "fila:Sala"], "divisao"), divisao: null };
    const resultado = envio(alvo, [A, semPerfil], []);
    expect(resultado.desabilitado).toBe(true);
    expect(resultado.motivo).toBe("Divisão: Sala (sem perfil) não pode receber. Resolva ou desmarque.");
  });

  it("duas ocupadas ficam no plural", () => {
    const alvo = { ...aba(["fila:A", "fila:B", "fila:C"], "divisao"), divisao: null };
    expect(envio(alvo, [A, B, C], ["fila:A", "fila:B"]).motivo).toBe(
      "Divisão: A, B estão ocupadas. Aguarde ou desmarque-as.",
    );
  });

  it("desmarcada a ocupada, o seletor sai e o envio a uma so volta a valer", () => {
    const alvo = { ...aba(["fila:B"], "divisao"), divisao: null };
    const resultado = envio(alvo, [A, B], ["fila:A"]);
    expect(resultado.desabilitado).toBe(false);
    expect(resultado.rotulo).toBe("Imprimir frente (1)");
    expect(resultado.semDestino).toBeUndefined();
  });

  it("em espelho, uma ocupada continua so fora da conta", () => {
    const alvo = { ...aba(["fila:A", "fila:B"], "espelho"), divisao: null };
    const resultado = envio(alvo, [A, B], ["fila:A"]);
    expect(resultado.desabilitado).toBe(false);
    expect(resultado.rotulo).toBe("Imprimir frente (1)");
  });
});

describe("reimprimir na divisao", () => {
  it("o titulo traz o numero do lote no documento", () => {
    const item: Cartao = {
      ...cartao("Laser", { tipo: "erro", motivo: { codigo: "fila", texto: "" } }, 2),
      lotesNoDocumento: [2, 4, 6],
    };
    const conteudo = conteudoDaReimpressao("Laser", 2, 3, item);
    expect(conteudo.titulo).toBe("modo.reimprimir.titulo");
    expect(conteudo.parametros).toEqual({ nome: "Laser", n: "4" });
  });

  it("sem divisao continua como antes", () => {
    const item = cartao("Laser", { tipo: "erro", motivo: { codigo: "fila", texto: "" } }, 2);
    const conteudo = conteudoDaReimpressao("Laser", 2, 3, item);
    expect(conteudo.titulo).toBe("cartao.reimprimirDialogo.titulo");
    expect(conteudo.parametros).toEqual({ nome: "Laser", n: "2", total: "3" });
  });
});
