import { describe, expect, it } from "vitest";
import dialogoFonte from "./components/ConfirmDialog.tsx?raw";
import cartaoFonte from "./components/CartaoImpressao.tsx?raw";
import {
  acoesDoCartao,
  cartaoEmAndamento,
  cartaoInterrompido,
  cartaoRecusado,
  cartoesRecusados,
  comCartao,
  estadoDoEnvio,
  frasesDoCartao,
  frasesDoGesto,
  impressorasOcupadas,
  linhasDoCartao,
  linhasDoEnvio,
  situacaoDaAba,
  nomeDoCartao,
  nomeDoPapel,
  precisaConfirmar,
  resultadoDoCartao,
  rotuloDaFrente,
  rotuloDoVerso,
  sequenciaDoCartao,
  textoDoEstadoDoCartao,
  textoDoMotivo,
  tomDoCartao,
  type Cartao,
  type EstadoCartao,
  type ResumoDoEnvio,
} from "./cartoes";
import { conteudoDaReimpressao } from "./confirmacao";
import { traduzir, traducoes, type Idioma } from "./i18n";
import type { Impressora } from "./impressoras";
import { estadoInicial, reduzirAbas, type Aba, type EstadoAbas } from "./tabs";

const IDIOMAS: readonly Idioma[] = ["pt-BR", "en"];

function fila(id: string, mudancas: Partial<Impressora> = {}): Impressora {
  return {
    id: `fila:${id}`,
    nome: id,
    tipo: "fila",
    nivel: "pronta",
    condicoes: [],
    modo: "jato",
    ...mudancas,
  };
}

const TRASEIRA: Impressora = {
  id: "simulada:traseira",
  nome: "Simulada — Traseira",
  tipo: "simulada",
  nivel: "pronta",
  condicoes: [],
  modo: "jato",
};
const GAVETA: Impressora = { ...TRASEIRA, id: "simulada:gaveta", modo: "laser" };

function abrirAba(estado: EstadoAbas, nome: string): EstadoAbas {
  return reduzirAbas(estado, {
    tipo: "abrir",
    documento: { caminho: `C:\\${nome}`, nome },
  });
}

function abaPronta(marcadas: readonly Impressora[]): Aba {
  let estado = abrirAba(estadoInicial, "a.pdf");
  const id = estado.ativa ?? -1;
  estado = reduzirAbas(estado, { tipo: "paginas_contadas", id, total: 6 });
  for (const impressora of marcadas) {
    estado = reduzirAbas(estado, { tipo: "alternar_impressora", id, impressora: impressora.id });
  }
  const aba = estado.abas[0];
  if (!aba) throw new Error("sem aba");
  return aba;
}

function cartao(estado: EstadoCartao, mudancas: Partial<Cartao> = {}): Cartao {
  return {
    aba: 1,
    impressora: "fila:A",
    nome: "A",
    simulada: false,
    lote: 1,
    total: 2,
    papel: { formato: "a4", larguraMm: 210, alturaMm: 297 },
    estado,
    recolocar: { lado: "baixo", virada: "nao_virar", textoDoPerfil: null },
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    ...mudancas,
  };
}

const NENHUMA = new Set<string>();

describe("botao de enviar a frente", () => {
  it("sem documento fica desabilitado com o motivo", () => {
    const botao = estadoDoEnvio(null, [TRASEIRA], NENHUMA, "pt-BR");
    expect(botao.desabilitado).toBe(true);
    expect(botao.motivo).toBe("Abra um PDF para imprimir.");
    expect(botao.rotulo).toBe("Imprimir frentes");
  });

  it("sem impressora marcada fica desabilitado com o motivo", () => {
    const botao = estadoDoEnvio(abaPronta([]), [fila("A")], NENHUMA, "en");
    expect(botao.desabilitado).toBe(true);
    expect(botao.motivo).toBe("Select at least one printer.");
  });

  it("conta so quem pode receber e diz quem ficou de fora", () => {
    const boa = fila("Boa");
    const semPerfil = fila("Sem", { modo: "sem_perfil" });
    const lista = [boa, semPerfil, fila("Livre")];
    const botao = estadoDoEnvio(abaPronta([boa, semPerfil]), lista, NENHUMA, "pt-BR");
    expect(botao.desabilitado).toBe(false);
    expect(botao.rotulo).toBe("Imprimir frentes em 1");
    expect(botao.destinos).toEqual([boa]);
    expect(botao.motivo).toBe("Fora da contagem: Sem (sem perfil).");
  });

  it.each([
    [{ modo: "sem_perfil" }, "(no profile)"],
    [{ modo: "desconhecido" }, "(profile not read)"],
    [{ nivel: "problema", condicoes: [{ codigo: "offline" }] }, "(Offline)"],
    [{ recusa: "pede_arquivo" }, "(asks for a file name)"],
    [{ recusa: "fax" }, "(it is a fax)"],
  ] as const)("%j nao conta e o motivo aparece", (mudancas, trecho) => {
    const ruim = fila("Ruim", mudancas as Partial<Impressora>);
    const botao = estadoDoEnvio(abaPronta([ruim]), [ruim], NENHUMA, "en");
    expect(botao.desabilitado).toBe(true);
    expect(botao.destinos).toEqual([]);
    expect(botao.motivo).toContain(trecho);
  });

  it("perfil personalizado e fila com atencao contam", () => {
    const personalizada = fila("P", { modo: "personalizado", nivel: "atencao" });
    const botao = estadoDoEnvio(abaPronta([personalizada]), [personalizada], NENHUMA, "pt-BR");
    expect(botao.destinos).toEqual([personalizada]);
    expect(botao.motivo).toBeNull();
  });

  it("PDF ainda contando ou recusado nao deixa enviar", () => {
    const aba = abaPronta([TRASEIRA]);
    const contando = estadoDoEnvio(
      { ...aba, paginas: { estado: "contando" } },
      [TRASEIRA],
      NENHUMA,
      "pt-BR",
    );
    expect(contando.desabilitado).toBe(true);
    expect(contando.motivo).toContain("contagem");
    const recusado = estadoDoEnvio(
      { ...aba, paginas: { estado: "recusado", motivo: "x" } },
      [TRASEIRA],
      NENHUMA,
      "en",
    );
    expect(recusado.desabilitado).toBe(true);
    expect(recusado.motivo).toContain("refused");
  });

  it("impressora com trabalho em andamento em qualquer aba nao conta", () => {
    const a = fila("A");
    const botao = estadoDoEnvio(abaPronta([a]), [a], new Set([a.id]), "pt-BR");
    expect(botao.desabilitado).toBe(true);
    expect(botao.motivo).toBe("Em andamento nos cartões acima.");
  });

  it("fila marcada que saiu da lista nao conta", () => {
    const a = fila("A");
    const botao = estadoDoEnvio(abaPronta([a, fila("Sumiu")]), [a], NENHUMA, "pt-BR");
    expect(botao.rotulo).toBe("Imprimir frentes em 1");
  });

  it("muitas de fora nao estouram o espaco do motivo", () => {
    const ruins = ["A", "B", "C", "D"].map((nome) => fila(nome, { modo: "sem_perfil" }));
    const botao = estadoDoEnvio(abaPronta(ruins), ruins, NENHUMA, "pt-BR");
    expect(botao.motivo).toContain("e mais 2");
    expect(botao.motivo).not.toContain("D (");
  });

  it("so com simuladas o botao fala em simular e nao pede confirmacao", () => {
    for (const [idioma, rotulo] of [
      ["pt-BR", "Simular frentes em 2"],
      ["en", "Simulate fronts on 2"],
    ] as const) {
      const botao = estadoDoEnvio(abaPronta([TRASEIRA, GAVETA]), [TRASEIRA, GAVETA], NENHUMA, idioma);
      expect(botao.rotulo).toBe(rotulo);
      expect(precisaConfirmar(botao.destinos)).toBe(false);
    }
  });

  it("qualquer fila real pede confirmacao e o botao nao fala em simular", () => {
    const a = fila("A");
    const botao = estadoDoEnvio(abaPronta([a, TRASEIRA]), [a, TRASEIRA], NENHUMA, "pt-BR");
    expect(botao.rotulo).toBe("Imprimir frentes em 2");
    expect(precisaConfirmar(botao.destinos)).toBe(true);
  });
});

describe("confirmacao do envio", () => {
  const resumo: ResumoDoEnvio = {
    paginas: 6,
    lotes: 2,
    papeis: [
      { formato: "a4", larguraMm: 210, alturaMm: 297 },
      { formato: "carta", larguraMm: 216, alturaMm: 279 },
    ],
    paresMistos: [[1, 2]],
    assinado: true,
  };

  it("lista as impressoras reais, as paginas, o papel e as observacoes", () => {
    const { linhas, notas } = linhasDoEnvio([fila("Sala"), TRASEIRA], resumo, "pt-BR");
    expect(linhas).toEqual(["Sala", "6 pág", "Papel: A4, Carta"]);
    expect(notas.join(" ")).toContain("mistura retrato e paisagem");
    expect(notas.join(" ")).toContain("assinatura digital");
    expect(notas.join(" ")).toContain("2 ou 3 folhas");
    expect(notas.join(" ")).toContain("simuladas");
    expect(notas.join(" ")).toContain("mais de um papel");
  });

  it("em ingles, com um so trecho e sem simulada, nao cita lote", () => {
    const { linhas, notas } = linhasDoEnvio(
      [fila("Room", { modo: "personalizado" })],
      {
        paginas: 2,
        lotes: 1,
        papeis: [{ formato: "carta", larguraMm: 216, alturaMm: 279 }],
        paresMistos: [],
        assinado: false,
      },
      "en",
    );
    expect(linhas).toEqual(["Room", "2 pages", "Paper: Letter"]);
    expect(notas).toEqual([]);
  });
});

describe("estado do cartao", () => {
  const CASOS: readonly [EstadoCartao, string, string, string][] = [
    [{ tipo: "imprimindo", passada: "frente" }, "Enviando à fila", "Sending to the queue", "ativo"],
    [{ tipo: "aguardando" }, "Aguardando recolocação", "Waiting for you to reload the stack", "espera"],
    [{ tipo: "concluido" }, "Concluído", "Done", "ativo"],
    [{ tipo: "erro", motivo: { codigo: "fila", texto: "x" } }, "Erro", "Error", "problema"],
  ];

  it.each(CASOS)("%j tem texto nos dois idiomas e um tom", (estado, pt, en, tom) => {
    expect(textoDoEstadoDoCartao(cartao(estado), "pt-BR")).toBe(pt);
    expect(textoDoEstadoDoCartao(cartao(estado), "en")).toBe(en);
    expect(tomDoCartao(cartao(estado))).toBe(tom);
  });

  it("a simulacao nunca diz que esta imprimindo", () => {
    const simulado = cartao({ tipo: "imprimindo", passada: "frente" }, { simulada: true });
    expect(textoDoEstadoDoCartao(simulado, "pt-BR")).toBe("Simulando");
    expect(textoDoEstadoDoCartao(simulado, "en")).toBe("Simulating");
  });

  it("so concluido e cancelado saem do andamento", () => {
    expect(cartaoEmAndamento(cartao({ tipo: "aguardando" }))).toBe(true);
    expect(cartaoEmAndamento(cartao({ tipo: "erro", motivo: { codigo: "x", texto: "" } }))).toBe(true);
    expect(cartaoEmAndamento(cartao({ tipo: "concluido" }))).toBe(false);
    expect(cartaoEmAndamento(cartao({ tipo: "cancelado" }))).toBe(false);
  });
});

describe("frases do cartao", () => {
  it("aguardando traz a espera da folha e o gesto, com o lado e a segunda linha", () => {
    const frases = frasesDoCartao(cartao({ tipo: "aguardando" }), "pt-BR");
    expect(frases).toEqual([
      "Os versos do trecho anterior terminaram ou saíram da fila.",
      "Tire esse maço e guarde-o à parte.",
      "Não vire nem gire o maço.",
      "Sem mudar a ordem das folhas, coloque na entrada com a face impressa para baixo.",
      "Com o maço na entrada, clique em Imprimir versos.",
    ]);
  });

  it("em ingles a frase do gesto tambem vem completa", () => {
    const frases = frasesDoGesto(
      { lado: "baixo", virada: "borda_de_cima", textoDoPerfil: null },
      "en",
    );
    expect(frases).toEqual([
      "Flip the stack over its top edge, like a notepad.",
      "Then, without changing the sheet order, place it in the feed with the printed side facing down.",
    ]);
  });

  it("perfil personalizado nao afirma lado", () => {
    const frases = frasesDoGesto(
      { lado: null, virada: null, textoDoPerfil: "Vire pela lateral." },
      "pt-BR",
    );
    expect(frases).toEqual([
      "Pegue o maço como saiu, não embaralhe, e recoloque do mesmo jeito que na calibração.",
      "Vire pela lateral.",
    ]);
    expect(frases.join(" ")).not.toMatch(/para (cima|baixo)/);
    expect(frasesDoGesto({ lado: null, virada: null, textoDoPerfil: null }, "en")).toHaveLength(1);
  });

  it("a saida final da laser pede para virar antes de ler, e a do jato nao", () => {
    const fim = { tipo: "concluido" } as const;
    expect(frasesDoCartao(cartao(fim, { viraParaLer: true }), "pt-BR").join(" ")).toContain(
      "Vire o maço antes de ler as folhas.",
    );
    expect(frasesDoCartao(cartao(fim, { viraParaLer: true }), "en").join(" ")).toContain(
      "Flip the stack before reading",
    );
    expect(frasesDoCartao(cartao(fim), "pt-BR").join(" ")).not.toContain("vire o maço");
  });

  it("troca de papel aparece antes do proximo trecho", () => {
    const proximo = cartao(
      { tipo: "proximo_lote" },
      { trocaDePapel: { formato: "a5", larguraMm: 148, alturaMm: 210 } },
    );
    expect(frasesDoCartao(proximo, "pt-BR").join(" ")).toContain(
      "Troque o papel para A5 e retire as folhas do trecho anterior.",
    );
    expect(frasesDoCartao(proximo, "en").join(" ")).toContain("Change the paper to A5");
    expect(frasesDoCartao(cartao({ tipo: "proximo_lote" }), "pt-BR").join(" ")).not.toContain(
      "Troque",
    );
  });

  it("a simulacao diz que nada foi a impressora", () => {
    const frases = frasesDoCartao(cartao({ tipo: "aguardando" }, { simulada: true }), "pt-BR");
    expect(frases[0]).toContain("nada foi à impressora");
    expect(frases.join(" ")).not.toContain("saiu da fila");
  });

  it.each(IDIOMAS)("%s nunca diz que algo foi impresso", (idioma) => {
    const estados: EstadoCartao[] = [
      { tipo: "imprimindo", passada: "frente" },
      { tipo: "imprimindo", passada: "verso" },
      { tipo: "aguardando" },
      { tipo: "proximo_lote" },
      { tipo: "concluido" },
      { tipo: "cancelado" },
    ];
    const proibido = idioma === "en" ? /\bwas printed\b|\bprinted\.$/i : /\bimpress[oa]s?\.$|foi impress/i;
    for (const estado of estados) {
      for (const frase of frasesDoCartao(cartao(estado), idioma)) {
        expect(frase).not.toMatch(proibido);
      }
    }
    const chaves = Object.entries(traducoes[idioma]).filter(([chave]) =>
      /^cartao\.(concluido|cancelado|estado|frente|verso)/.test(chave),
    );
    for (const [, texto] of chaves) {
      expect(texto).not.toMatch(idioma === "en" ? /\bprinted\b/i : /\bimpress[oa]\b/i);
    }
  });

  it("o resultado da simulacao segue o texto do terminal", () => {
    const fim = cartao({ tipo: "concluido" }, { resultado: { corretas: 3, folhas: 3 } });
    expect(resultadoDoCartao(fim, "pt-BR")).toBe("3 de 3 folhas corretas (na simulação)");
    expect(resultadoDoCartao(fim, "en")).toBe("3 of 3 sheets correct (in the simulation)");
    expect(resultadoDoCartao(cartao({ tipo: "concluido" }), "pt-BR")).toBeNull();
  });

  it("uma so folha usa o singular no resultado", () => {
    const fim = cartao({ tipo: "concluido" }, { resultado: { corretas: 1, folhas: 1 } });
    expect(resultadoDoCartao(fim, "pt-BR")).toBe("1 de 1 folha correta (na simulação)");
    expect(resultadoDoCartao(fim, "en")).toBe("1 of 1 sheet correct (in the simulation)");
    const zero = cartao({ tipo: "concluido" }, { resultado: { corretas: 0, folhas: 1 } });
    expect(resultadoDoCartao(zero, "pt-BR")).toBe("0 de 1 folha correta (na simulação)");
  });

  it("a sequencia diz a ordem, a pagina em branco e o giro", () => {
    const face = (numero: number | null, girada: boolean) => ({ numero, girada });
    const comSequencia = cartao(
      { tipo: "aguardando" },
      {
        sequencia: {
          frente: [face(6, false), face(4, false), face(2, false)],
          verso: [face(5, true), face(3, true), face(null, true)],
        },
      },
    );
    expect(sequenciaDoCartao(comSequencia, "pt-BR")).toEqual([
      "Frente: 6, 4, 2",
      "Verso: 5, 3, branca (girado 180°)",
    ]);
    expect(sequenciaDoCartao(comSequencia, "en")[1]).toBe("Back: 5, 3, blank (turned 180°)");
    expect(sequenciaDoCartao(cartao({ tipo: "aguardando" }), "en")).toEqual([]);
  });

  it("nomes do papel seguem o idioma e o formato fora da tabela usa as medidas", () => {
    const carta = { formato: "carta", larguraMm: 216, alturaMm: 279 };
    expect(nomeDoPapel(carta, "pt-BR")).toBe("Carta");
    expect(nomeDoPapel(carta, "en")).toBe("Letter");
    expect(nomeDoPapel({ formato: "outro", larguraMm: 100.4, alturaMm: 200 }, "en")).toBe(
      "100 × 200 mm",
    );
  });

  it("o nome da simulada e traduzido e o da fila fica como o sistema deu", () => {
    const simulada = cartao({ tipo: "aguardando" }, { simulada: true, impressora: "simulada:gaveta" });
    expect(nomeDoCartao(simulada, "en")).toBe("Simulated — Tray");
    expect(nomeDoCartao(cartao({ tipo: "aguardando" }), "en")).toBe("A");
  });
});

describe("motivos", () => {
  it("em portugues vale a mensagem exata, com inicial maiuscula", () => {
    expect(
      textoDoMotivo({ codigo: "fila", texto: "a impressora está sem papel" }, "pt-BR"),
    ).toBe("A impressora está sem papel.");
    expect(textoDoMotivo({ codigo: "driver", texto: "o driver recusou" }, "pt-BR")).toBe(
      "O driver recusou",
    );
  });

  it("em ingles traduz as frases da fila e mantem o texto do driver", () => {
    expect(
      textoDoMotivo(
        {
          codigo: "fila",
          texto: "a impressora está sem papel; o trabalho deu erro (Paper tray 2)",
        },
        "en",
      ),
    ).toBe("The printer is out of paper; the job failed (Paper tray 2).");
  });

  it("papel acabado, atolado e impressora sem resposta sao frases diferentes nos dois idiomas", () => {
    const casos = [
      ["a impressora está sem papel", "A impressora está sem papel.", "The printer is out of paper."],
      ["o papel atolou na impressora", "O papel atolou na impressora.", "The paper is jammed in the printer."],
      ["a impressora não responde", "A impressora não responde.", "The printer is not responding."],
    ] as const;
    for (const [texto, pt, en] of casos) {
      expect(textoDoMotivo({ codigo: "fila", texto }, "pt-BR")).toBe(pt);
      expect(textoDoMotivo({ codigo: "fila", texto }, "en")).toBe(en);
    }
  });

  it("em ingles nao mostra mensagem de envio em portugues", () => {
    const texto = textoDoMotivo({ codigo: "papel", texto: "o driver não aplicou o papel" }, "en");
    expect(texto).toBe("The driver did not accept the paper size of this job.");
    expect(textoDoMotivo({ codigo: "novo", texto: "algo" }, "en")).toBe(
      "The system refused a step of the job.",
    );
  });

  it("sem texto usa a frase do codigo nos dois idiomas", () => {
    for (const idioma of IDIOMAS) {
      expect(textoDoMotivo({ codigo: "interrompido", texto: "" }, idioma).length).toBeGreaterThan(20);
    }
  });
});

describe("acoes do cartao", () => {
  it("o verso aparece desde a frente, mas so libera aguardando a recolocacao", () => {
    for (const estado of [
      { tipo: "imprimindo", passada: "frente" },
      { tipo: "aviso", passada: "frente", motivo: { codigo: "fila", texto: "x" }, respondido: false },
    ] as const) {
      expect(acoesDoCartao(cartao(estado)).verso).toEqual({ visivel: true, liberado: false });
    }
    expect(acoesDoCartao(cartao({ tipo: "aguardando" })).verso).toEqual({
      visivel: true,
      liberado: true,
    });
    expect(acoesDoCartao(cartao({ tipo: "imprimindo", passada: "verso" })).verso).toEqual({
      visivel: true,
      liberado: false,
    });
  });

  it("aviso oferece cancelar e continuar; respondido, so cancelar", () => {
    const motivo = { codigo: "fila", texto: "x" };
    const novo = acoesDoCartao(cartao({ tipo: "aviso", passada: "frente", motivo, respondido: false }));
    expect([novo.continuar, novo.cancelar]).toEqual([true, true]);
    const respondido = acoesDoCartao(
      cartao({ tipo: "aviso", passada: "frente", motivo, respondido: true }),
    );
    expect([respondido.continuar, respondido.cancelar]).toEqual([false, true]);
  });

  it("erro oferece reimprimir o lote, salvo se a preparacao nem comecou", () => {
    const erro = { tipo: "erro", motivo: { codigo: "fila", texto: "x" } } as const;
    expect(acoesDoCartao(cartao(erro)).reimprimir).toBe(true);
    expect(acoesDoCartao(cartao(erro, { total: 0 })).reimprimir).toBe(false);
    expect(acoesDoCartao(cartao(erro, { total: 0 })).cancelar).toBe(true);
  });

  it("proximo lote oferece a frente e o fim nao oferece nada", () => {
    expect(acoesDoCartao(cartao({ tipo: "proximo_lote" })).frente).toBe(true);
    const fim = acoesDoCartao(cartao({ tipo: "concluido" }));
    expect(fim).toEqual({
      frente: false,
      pedirVerso: false,
      verso: { visivel: false, liberado: false },
      reimprimir: false,
      continuar: false,
      cancelar: false,
      dispensar: true,
    });
  });

  it("com impressora simulada os botoes falam em simular", () => {
    const simulado = cartao({ tipo: "aguardando" }, { simulada: true });
    expect(rotuloDoVerso(simulado, "pt-BR")).toBe("Simular versos");
    expect(rotuloDoVerso(simulado, "en")).toBe("Simulate backs");
    expect(rotuloDaFrente(simulado, "pt-BR")).toBe("Simular frentes");
    expect(rotuloDoVerso(cartao({ tipo: "aguardando" }), "pt-BR")).toBe("Imprimir versos");
    expect(rotuloDaFrente(cartao({ tipo: "aguardando" }), "en")).toBe("Print fronts");
  });

  it("perder o contato com o trabalho vira erro com motivo", () => {
    const perdido = cartaoInterrompido(cartao({ tipo: "aguardando" }));
    expect(perdido.estado.tipo).toBe("erro");
    expect(frasesDoCartao(perdido, "en")[0]).toContain("interrupted");
    expect(frasesDoCartao(perdido, "pt-BR")[0]).toContain("interrompido");
  });
});

describe("cartoes por documento", () => {
  function duasAbas(): { estado: EstadoAbas; a: number; b: number } {
    const estado = abrirAba(abrirAba(estadoInicial, "a.pdf"), "b.pdf");
    const [a, b] = estado.abas;
    if (!a || !b) throw new Error("sem abas");
    return { estado, a: a.id, b: b.id };
  }

  it("cada aba guarda os proprios cartoes e o trabalho continua na aba de tras", () => {
    const { estado: inicio, a, b } = duasAbas();
    let estado = reduzirAbas(inicio, {
      tipo: "cartao",
      cartao: cartao({ tipo: "imprimindo", passada: "frente" }, { aba: a, impressora: "x" }),
    });
    estado = reduzirAbas(estado, {
      tipo: "cartao",
      cartao: cartao({ tipo: "aguardando" }, { aba: b, impressora: "y" }),
    });
    expect(estado.ativa).toBe(b);
    estado = reduzirAbas(estado, {
      tipo: "cartao",
      cartao: cartao({ tipo: "aguardando" }, { aba: a, impressora: "x" }),
    });
    const [abaA, abaB] = estado.abas;
    expect(abaA?.cartoes.map((c) => [c.impressora, c.estado.tipo])).toEqual([["x", "aguardando"]]);
    expect(abaB?.cartoes.map((c) => c.impressora)).toEqual(["y"]);
  });

  it("cartao de aba que ja fechou nao muda nada", () => {
    const { estado } = duasAbas();
    expect(
      reduzirAbas(estado, { tipo: "cartao", cartao: cartao({ tipo: "aguardando" }, { aba: 99 }) }),
    ).toBe(estado);
  });

  it("a impressao em andamento acompanha o estado real dos trabalhos", () => {
    const { estado: inicio, a } = duasAbas();
    const com = (estado: EstadoAbas, novo: EstadoCartao, impressora: string) =>
      reduzirAbas(estado, { tipo: "cartao", cartao: cartao(novo, { aba: a, impressora }) });
    let estado = com(inicio, { tipo: "imprimindo", passada: "frente" }, "x");
    expect(estado.abas[0]?.impressaoEmAndamento).toBe(true);
    estado = com(estado, { tipo: "aguardando" }, "y");
    estado = com(estado, { tipo: "concluido" }, "x");
    expect(estado.abas[0]?.impressaoEmAndamento).toBe(true);
    estado = com(estado, { tipo: "cancelado" }, "y");
    expect(estado.abas[0]?.impressaoEmAndamento).toBe(false);
  });

  it("fechar a aba com trabalho pede confirmacao, e o pedido cai quando o trabalho acaba", () => {
    const { estado: inicio, a } = duasAbas();
    let estado = reduzirAbas(inicio, {
      tipo: "cartao",
      cartao: cartao({ tipo: "aguardando" }, { aba: a }),
    });
    estado = reduzirAbas(estado, { tipo: "pedir_fechamento", id: a });
    expect(estado.aguardandoConfirmacao).toBe(a);
    estado = reduzirAbas(estado, {
      tipo: "cartao",
      cartao: cartao({ tipo: "concluido" }, { aba: a }),
    });
    expect(estado.aguardandoConfirmacao).toBeNull();
  });

  it("um novo trabalho na mesma impressora fica no lugar do cartao antigo", () => {
    const antigo = cartao({ tipo: "concluido" });
    const outro = cartao({ tipo: "concluido" }, { impressora: "outra" });
    const novo = cartao({ tipo: "preparando" });
    expect(comCartao([antigo, outro], novo)).toEqual([novo, outro]);
  });

  it("impressora ocupada vale entre as abas e sai ao terminar", () => {
    const { estado: inicio, a, b } = duasAbas();
    let estado = reduzirAbas(inicio, {
      tipo: "cartao",
      cartao: cartao({ tipo: "aguardando" }, { aba: a, impressora: "x" }),
    });
    estado = reduzirAbas(estado, {
      tipo: "cartao",
      cartao: cartao({ tipo: "concluido" }, { aba: b, impressora: "y" }),
    });
    expect([...impressorasOcupadas(estado.abas)]).toEqual(["x"]);
  });
});
function abaCom(cartoes: readonly Cartao[]): Aba {
  return { ...abaPronta([]), cartoes };
}

function duasAbas(): { estado: EstadoAbas; a: number; b: number } {
  const estado = abrirAba(abrirAba(estadoInicial, "a.pdf"), "b.pdf");
  const [a, b] = estado.abas;
  if (!a || !b) throw new Error("sem abas");
  return { estado, a: a.id, b: b.id };
}

describe("cartao de thread morta nao ocupa a impressora", () => {
  it("perder o contato libera a impressora e o cartao pode ser dispensado", () => {
    const perdido = cartaoInterrompido(cartao({ tipo: "aguardando" }));
    expect(cartaoEmAndamento(perdido)).toBe(false);
    expect(impressorasOcupadas([abaCom([perdido])]).size).toBe(0);
    const acoes = acoesDoCartao(perdido);
    expect(acoes.dispensar).toBe(true);
    expect(acoes.reimprimir).toBe(false);
  });

  it("dispensar tira o cartao da aba e a impressora volta a contar", () => {
    const { estado: inicio, a } = duasAbas();
    let estado = reduzirAbas(inicio, {
      tipo: "cartao",
      cartao: cartaoInterrompido(cartao({ tipo: "aguardando" }, { aba: a, impressora: "x" })),
    });
    estado = reduzirAbas(estado, { tipo: "dispensar_cartao", id: a, impressora: "x" });
    expect(estado.abas[0]?.cartoes).toEqual([]);
    expect(impressorasOcupadas(estado.abas).size).toBe(0);
  });

  it("recusa de inicio vira cartao com o motivo ocupada, nos dois idiomas", () => {
    const recusado = cartaoRecusado(7, fila("A"));
    expect(recusado.estado).toEqual({ tipo: "erro", motivo: { codigo: "ocupada", texto: "" } });
    expect(cartaoEmAndamento(recusado)).toBe(false);
    expect(frasesDoCartao(recusado, "pt-BR")[0]).toBe("A impressora já tem um trabalho em andamento.");
    expect(frasesDoCartao(recusado, "en")[0]).toBe("The printer already has a job in progress.");
  });

  it("encerrados podem ser dispensados e os que acabaram nao oferecem reimprimir", () => {
    expect(acoesDoCartao(cartao({ tipo: "concluido" })).dispensar).toBe(true);
    expect(acoesDoCartao(cartao({ tipo: "cancelado" })).dispensar).toBe(true);
    expect(acoesDoCartao(cartao({ tipo: "aguardando" })).dispensar).toBe(false);
  });
});

describe("rotulo e contagem pelos destinos", () => {
  it("uma simulada valida com uma fila invalida marcadas diz simular e nao pede confirmacao", () => {
    const ruim = fila("Ruim", { modo: "sem_perfil" });
    const botao = estadoDoEnvio(abaPronta([TRASEIRA, ruim]), [TRASEIRA, ruim], NENHUMA, "pt-BR");
    expect(botao.destinos).toEqual([TRASEIRA]);
    expect(botao.rotulo).toBe("Simular frentes em 1");
    expect(precisaConfirmar(botao.destinos)).toBe(false);
  });

  it("marcadas todas ocupadas dizem uma linha so, nos dois idiomas", () => {
    const a = fila("A");
    const ocupadas = new Set([a.id]);
    expect(estadoDoEnvio(abaPronta([a]), [a], ocupadas, "pt-BR").motivo).toBe(
      "Em andamento nos cartões acima.",
    );
    expect(estadoDoEnvio(abaPronta([a]), [a], ocupadas, "en").motivo).toBe(
      "In progress in the cards above.",
    );
  });
});

describe("texto do gesto na jato e na lateral", () => {
  it("o gesto vem antes da colocacao, nos dois idiomas", () => {
    const borda = { lado: "baixo", virada: "borda_de_cima", textoDoPerfil: null } as const;
    expect(frasesDoGesto(borda, "pt-BR")).toEqual([
      "Vire o maço pela borda de cima, como um bloco de notas.",
      "Depois, sem mudar a ordem das folhas, coloque na entrada com a face impressa para baixo.",
    ]);
    expect(frasesDoGesto(borda, "en")).toEqual([
      "Flip the stack over its top edge, like a notepad.",
      "Then, without changing the sheet order, place it in the feed with the printed side facing down.",
    ]);
    expect(frasesDoGesto({ ...borda, virada: "lateral" }, "pt-BR")[0]).toBe(
      "Vire o maço pela lateral, como a página de um livro.",
    );
    expect(frasesDoGesto({ ...borda, virada: "girar" }, "en")[0]).toBe(
      "Turn the stack 180°, without flipping it.",
    );
    expect(frasesDoGesto({ ...borda, virada: "nao_virar" }, "en")[0]).toBe(
      "Do not flip or turn the stack.",
    );
    expect(frasesDoGesto({ ...borda, virada: "nao_virar" }, "pt-BR")).toEqual([
      "Não vire nem gire o maço.",
      "Sem mudar a ordem das folhas, coloque na entrada com a face impressa para baixo.",
    ]);
  });

  it("em toda combinacao que vira ou gira, o gesto vem primeiro e a colocacao depois", () => {
    for (const lado of ["cima", "baixo"]) {
      for (const virada of ["borda_de_cima", "lateral", "girar", "nao_virar"]) {
        for (const idioma of IDIOMAS) {
          const frases = frasesDoGesto({ lado, virada, textoDoPerfil: null }, idioma);
          expect(frases).toHaveLength(2);
          const [gesto, colocacao] = frases;
          expect(gesto).not.toMatch(/face impressa|printed side/);
          expect(colocacao).toMatch(
            idioma === "en"
              ? new RegExp(`printed side facing ${lado === "cima" ? "up" : "down"}\\.$`)
              : new RegExp(`face impressa para ${lado}\\.$`),
          );
        }
      }
    }
  });

  it("o perfil personalizado em ingles nao mostra o texto do perfil em portugues", () => {
    expect(
      frasesDoGesto({ lado: null, virada: null, textoDoPerfil: "Vire pela lateral." }, "en"),
    ).toEqual([
      "Take the stack as it came out, do not shuffle it, and put it back the same way as in the calibration.",
    ]);
  });
});

describe("frases do cartao nos novos textos", () => {
  it("aguardando real termina com o clique em imprimir verso", () => {
    const frases = frasesDoCartao(cartao({ tipo: "aguardando" }), "pt-BR");
    expect(frases.at(-1)).toBe("Com o maço na entrada, clique em Imprimir versos.");
    expect(frasesDoCartao(cartao({ tipo: "aguardando" }), "en").at(-1)).toBe(
      "With the stack in the feed, click Print backs.",
    );
  });

  it("aguardando simulado mostra o proximo passo real e manda conferir", () => {
    const simulado = cartao({ tipo: "aguardando" }, { simulada: true });
    const frases = frasesDoCartao(simulado, "pt-BR");
    expect(frases).toContain("Na impressora real, o próximo passo seria:");
    expect(frases.at(-1)).toBe("Clique em Simular versos para conferir.");
    expect(frasesDoCartao(simulado, "en")).toContain("On a real printer, the next step would be:");
    expect(frasesDoCartao(simulado, "en").at(-1)).toBe("Click Simulate backs to check.");
  });

  it("frente real em envio mostra o que a fila faz com as frentes, como texto", () => {
    expect(frasesDoCartao(cartao({ tipo: "imprimindo", passada: "frente" }), "pt-BR")).toEqual([
      "As frentes estão indo para a fila.",
    ]);
  });

  it("proximo trecho diz o estado novo e o que fazer, sem a palavra lote", () => {
    const sem = cartao({ tipo: "proximo_lote" });
    expect(textoDoEstadoDoCartao(sem, "pt-BR")).toBe("Pronto para o próximo trecho");
    expect(textoDoEstadoDoCartao(sem, "en")).toBe("Ready for the next section");
    expect(frasesDoCartao(sem, "pt-BR")).toEqual([
      "As frentes deste trecho terminaram ou saíram da fila.",
      "Retire as folhas do trecho anterior e coloque papel em branco.",
      "Depois, clique em Imprimir frente.",
    ]);
    const en = frasesDoCartao(sem, "en").join(" ");
    expect(en).toContain("section");
    expect(en).not.toMatch(/set of sheets/);
  });

  it("preparando tem tom ativo, uma frase e nenhum botao de verso", () => {
    const preparando = cartao({ tipo: "preparando" });
    expect(tomDoCartao(preparando)).toBe("ativo");
    expect(frasesDoCartao(preparando, "pt-BR")).toEqual(["Preparando as páginas para enviar."]);
    expect(frasesDoCartao(preparando, "en")).toEqual(["Preparing the pages to send."]);
    expect(acoesDoCartao(preparando).verso.visivel).toBe(false);
  });

  it("verso em envio aparece desabilitado para o clique duplo nao cair em outra acao", () => {
    expect(acoesDoCartao(cartao({ tipo: "imprimindo", passada: "verso" })).verso).toEqual({
      visivel: true,
      liberado: false,
    });
  });

  it("a frase de cada linha diz o papel dela", () => {
    const linhas = linhasDoCartao(cartao({ tipo: "aguardando" }), "pt-BR");
    expect(linhas.map((linha) => linha.papel)).toEqual([
      "ressalva",
      "contexto",
      "gesto",
      "gesto",
      "acao",
    ]);
  });
});

describe("motivos da fila com ponto final", () => {
  it("em portugues a frase da fila termina com ponto", () => {
    expect(textoDoMotivo({ codigo: "fila", texto: "a impressora está sem papel" }, "pt-BR")).toBe(
      "A impressora está sem papel.",
    );
  });

  it("em ingles a frase traduzida termina com ponto", () => {
    expect(textoDoMotivo({ codigo: "fila", texto: "o trabalho deu erro" }, "en")).toBe(
      "The job failed.",
    );
  });
});

describe("textos novos da interface", () => {
  it("estado e termos do envio nos dois idiomas", () => {
    expect(textoDoEstadoDoCartao(cartao({ tipo: "imprimindo", passada: "frente" }), "en")).toBe(
      "Sending to the queue",
    );
    expect(textoDoEstadoDoCartao(cartao({ tipo: "imprimindo", passada: "frente" }), "pt-BR")).toBe(
      "Enviando à fila",
    );
    expect(textoDoEstadoDoCartao(cartao({ tipo: "aguardando" }), "en")).toBe(
      "Waiting for you to reload the stack",
    );
    expect(traducoes.en["cartao.envio.voltar"]).toBe("Cancel");
    expect(traducoes.en["cartao.virarParaLer"]).toBe("Flip the stack before reading the sheets.");
    expect(traducoes["pt-BR"]["cartao.virarParaLer"]).toBe("Vire o maço antes de ler as folhas.");
  });
});

describe("lista de linhas sem chave repetida", () => {
  it("os componentes de dialogo e de cartao nao usam o texto da linha como chave", () => {
    expect(dialogoFonte).not.toMatch(/key=\{linha\}/);
    expect(dialogoFonte).not.toMatch(/key=\{nota\}/);
    expect(cartaoFonte).not.toMatch(/key=\{frase\}/);
    expect(cartaoFonte).not.toMatch(/key=\{linha\}/);
  });
});

describe("situacao da aba na barra de titulo", () => {
  function abaCom(...cartoes: Cartao[]): Aba {
    return { ...abaPronta([]), cartoes };
  }

  it("pedir acao vale antes de estar em curso, e aba sem trabalho nao tem ponto", () => {
    expect(situacaoDaAba(abaCom(cartao({ tipo: "imprimindo", passada: "frente" })))).toBe("em_curso");
    expect(
      situacaoDaAba(
        abaCom(
          cartao({ tipo: "imprimindo", passada: "frente" }, { impressora: "x" }),
          cartao({ tipo: "aguardando" }, { impressora: "y" }),
        ),
      ),
    ).toBe("pede_acao");
    expect(situacaoDaAba(abaCom(cartao({ tipo: "concluido" })))).toBeNull();
    expect(situacaoDaAba(abaCom())).toBeNull();
  });

  it("os rotulos da aba dizem o que pedem nos dois idiomas", () => {
    expect(traducoes["pt-BR"]["barra.aba.aguardando"]).toBe("{nome}, aguardando você");
    expect(traducoes.en["barra.aba.aguardando"]).toBe("{nome}, waiting for you");
    expect(traducoes["pt-BR"]["barra.aba.emCurso"]).toBe("{nome}, em andamento");
    expect(traducoes.en["barra.aba.emCurso"]).toBe("{nome}, in progress");
  });
});

describe("recusas do inicio e reimpressao", () => {
  it("so os destinos que o backend nao aceitou viram cartao de recusa", () => {
    const a = fila("A");
    const b = fila("B");
    const recusados = cartoesRecusados(3, [a, b], [a.id]);
    expect(recusados.map((cartao) => cartao.impressora)).toEqual([b.id]);
    expect(recusados[0]?.aba).toBe(3);
    expect(recusados[0]?.estado).toEqual({ tipo: "erro", motivo: { codigo: "ocupada", texto: "" } });
    expect(cartoesRecusados(3, [a], [a.id, "outra"])).toEqual([]);
  });

  it("reimprimir diz quantas folhas saem de novo, e so a face do trabalho", () => {
    const frentes = cartao({ tipo: "erro", motivo: { codigo: "fila", texto: "" } }, {
      folhasDoLote: 12,
      alcance: "so_frentes",
    });
    const pt = conteudoDaReimpressao("Fila A", frentes, "pt-BR");
    expect(traduzir("pt-BR", pt.titulo, pt.parametros)).toBe("Reimprimir este trecho em Fila A?");
    expect(traduzir("pt-BR", pt.texto, pt.parametros)).toBe(
      "São 12 folhas. O que restar sai da fila e as frentes são enviadas de novo. Deixe na entrada só folhas em branco: tire as folhas já impressas deste trecho.",
    );
    expect(traduzir("pt-BR", pt.fechar)).toBe("Reimprimir");
    const versos = conteudoDaReimpressao("Fila A", { ...frentes, alcance: "so_versos", folhasDoLote: 1 }, "en");
    expect(traduzir("en", versos.texto, versos.parametros)).toBe(
      "That is 1 sheet. What is left leaves the queue, and the backs are sent again. Load a stack with the fronts ready and no backs.",
    );
  });
});

describe("simulacao em envio explica o verso desabilitado", () => {
  it("a frente e o verso simulados dizem que nada foi a impressora, nos dois idiomas", () => {
    const frente = cartao({ tipo: "imprimindo", passada: "frente" }, { simulada: true });
    const verso = cartao({ tipo: "imprimindo", passada: "verso" }, { simulada: true });
    expect(frasesDoCartao(frente, "pt-BR")).toEqual(["As frentes estão sendo simuladas."]);
    expect(frasesDoCartao(verso, "pt-BR")).toEqual(["Os versos estão sendo simulados. Nada foi à impressora."]);
    expect(frasesDoCartao(verso, "en")).toEqual(["The backs are being simulated. Nothing went to the printer."]);
    expect(acoesDoCartao(verso).verso).toEqual({ visivel: true, liberado: false });
  });
});
