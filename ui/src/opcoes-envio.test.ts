import { describe, expect, it } from "vitest";
import { linhasDoEnvio, sequenciaDoCartao, type Cartao, type ResumoDoEnvio } from "./cartoes";
import { conteudoDaDivisao } from "./confirmacao";
import {
  conteudoDoApagarHistorico,
  linhasDasImpressoras,
  nomeDaVelocidade,
  nomeNoHistorico,
  registrosPorData,
  textoDaData,
  textoDaVelocidade,
  textoDoResultado,
  textoDoTamanho,
  textoDoTempo,
  type RegistroDoHistorico,
} from "./historico";
import { traduzir } from "./i18n";
import type { Impressora } from "./impressoras";
import {
  envioSemMarca,
  mostraOpcaoDoDriver,
  nomeDoPapelDeAjuste,
  notasDasOpcoes,
  textoDaEscala,
  textoDoFator,
  textoDoPapelAjustado,
  type OpcoesDaNota,
} from "./opcoes-envio";
import { estadoInicial, reduzirAbas } from "./tabs";

function impressora(id: string, nome: string, extra: Partial<Impressora> = {}): Impressora {
  return {
    id,
    nome,
    tipo: id.startsWith("fila:") ? "fila" : "simulada",
    nivel: "pronta",
    condicoes: [],
    modo: "laser",
    ...extra,
  };
}

const TRASEIRA = impressora("simulada:traseira", "Simulada — Traseira", { modo: "jato" });
const GAVETA = impressora("simulada:gaveta", "Simulada — Gaveta");
const FILA = impressora("fila:Sala", "Sala");

const REGISTRO: RegistroDoHistorico = {
  arquivo: "contrato.pdf",
  data: 1_700_000_000,
  impressoras: ["fila:Sala"],
  paginas: 6,
  lotes: 2,
  resultado: "concluido",
  lotes_faltando: 0,
};

describe("opcoes por aba", () => {
  it("comecam desligadas e cada aba guarda as suas", () => {
    let estado = reduzirAbas(estadoInicial, {
      tipo: "abrir",
      documento: { caminho: "a.pdf", nome: "a.pdf" },
    });
    estado = reduzirAbas(estado, { tipo: "abrir", documento: { caminho: "b.pdf", nome: "b.pdf" } });
    const [a, b] = estado.abas;
    expect(a?.controle).toBe(false);
    expect(a?.papelDeAjuste).toBeNull();
    estado = reduzirAbas(estado, { tipo: "definir_controle", id: a?.id ?? -1, ligado: true });
    estado = reduzirAbas(estado, { tipo: "definir_papel_ajuste", id: b?.id ?? -1, papel: "carta" });
    expect(estado.abas.map((aba) => aba.controle)).toEqual([true, false]);
    expect(estado.abas.map((aba) => aba.papelDeAjuste)).toEqual([null, "carta"]);
    estado = reduzirAbas(estado, { tipo: "definir_controle", id: a?.id ?? -1, ligado: false });
    expect(estado.abas[0]?.controle).toBe(false);
  });
});

describe("frases da confirmacao", () => {
  const base = { papel: null, modo: "espelho" as const, destinos: [TRASEIRA, GAVETA] };

  it("o papel ajustado vem com a escala, nos dois idiomas", () => {
    expect(textoDoPapelAjustado("a4", [0.94, 0.94], "pt-BR")).toBe("A4, escala 94%");
    expect(textoDoPapelAjustado("carta", [0.94, 0.94], "en")).toBe("Letter, scale 94%");
    expect(textoDoPapelAjustado(null, [1, 1], "pt-BR")).toBeNull();
    expect(textoDoPapelAjustado("a4", null, "pt-BR")).toBeNull();
  });

  it("escala igual depois de arredondar vira um numero so; diferente vira a faixa", () => {
    expect(textoDaEscala([0.941, 0.944])).toBe("94%");
    expect(textoDaEscala([1, 1.41])).toBe("100–141%");
  });

  it("os nomes dos papeis seguem o idioma", () => {
    expect(nomeDoPapelDeAjuste("oficio", "pt-BR")).toBe("Ofício");
    expect(nomeDoPapelDeAjuste("carta", "en")).toBe("Letter");
  });

  it("sem opcao e sem fila real, nada e acrescentado", () => {
    expect(notasDasOpcoes(base, "pt-BR")).toEqual([]);
  });

  const AVISO_PT =
    'Se o driver imprimir a última página primeiro, o maço sai embaralhado. Desligue isso nas preferências dele ou marque "O driver inverte a ordem" na linha da impressora.';
  const AVISO_EN =
    'If the driver prints the last page first, the stack comes out shuffled. Turn that off in its preferences or check "The driver reverses the order" on the printer\'s row.';

  it("o aviso do driver diz como resolver e so aparece com fila real sem a compensacao", () => {
    expect(notasDasOpcoes({ ...base, destinos: [FILA] }, "pt-BR")).toEqual([AVISO_PT]);
    expect(notasDasOpcoes({ ...base, destinos: [FILA, GAVETA] }, "pt-BR")).toEqual([AVISO_PT]);
    expect(notasDasOpcoes({ ...base, destinos: [FILA] }, "en")).toEqual([AVISO_EN]);
  });

  it("com a compensacao ligada em uma so impressora, a nota nao precisa do nome", () => {
    const ligada = { ...FILA, driverInverte: true };
    expect(notasDasOpcoes({ ...base, destinos: [ligada] }, "pt-BR")).toEqual([
      "Ordem compensada: este driver inverte.",
    ]);
    expect(notasDasOpcoes({ ...base, destinos: [ligada] }, "en")).toEqual([
      "Order compensated: this driver reverses.",
    ]);
  });

  it("com varias impressoras, a nota cita quais compensam, e o aviso fica para as outras", () => {
    const sala = { ...FILA, driverInverte: true };
    const copa = impressora("fila:Copa", "Copa", { driverInverte: true });
    const hall = impressora("fila:Hall", "Hall");
    expect(notasDasOpcoes({ ...base, destinos: [sala, copa] }, "pt-BR")).toEqual([
      "Ordem compensada em Sala, Copa: o driver inverte.",
    ]);
    expect(notasDasOpcoes({ ...base, destinos: [sala, hall] }, "en")).toEqual([
      "Order compensated on Sala: the driver reverses.",
      AVISO_EN,
    ]);
  });

  it("tudo pela rota manual, a confirmacao cita a folha de controle; desligada, nada", () => {
    expect(notasDasOpcoes({ ...base, controle: true }, "pt-BR")).toEqual(["Com folha de controle"]);
    expect(notasDasOpcoes({ ...base, controle: true }, "en")).toEqual(["With control sheet"]);
    expect(notasDasOpcoes({ ...base, controle: false }, "pt-BR")).toEqual([]);
  });

  const comparacao = { rapida: "simulada:traseira", lenta: "simulada:gaveta", razao: 2 };
  const TRES = [TRASEIRA, GAVETA, impressora("simulada:bandeja", "Bandeja")];

  it("com tres ou mais impressoras, a velocidade nao fala em duas", () => {
    const espelho = { ...base, destinos: TRES, comparacao };
    const divisao = { ...espelho, modo: "divisao" as const };
    expect(notasDasOpcoes(espelho, "pt-BR")[0]).toMatch(/termina antes\. Cada uma imprime o documento inteiro\.$/);
    expect(notasDasOpcoes(espelho, "en")[0]).toMatch(/finishes first\. Each prints the whole document\.$/);
    expect(notasDasOpcoes(divisao, "pt-BR")[0]).toMatch(/termina antes\. Os lotes seguem em rodízio\.$/);
    expect(notasDasOpcoes(divisao, "en")[0]).toMatch(/finishes first\. Batches are shared in turn\.$/);
  });

  it("na divisao de tres com uma sem lote, recebem duas e a frase volta a falar em duas", () => {
    const partes = TRES.slice(0, 2).map((destino) => ({ impressora: destino.id, faixas: [] }));
    const notas = notasDasOpcoes({ ...base, modo: "divisao", destinos: TRES, comparacao, partes }, "pt-BR");
    expect(notas[0]).toMatch(/As duas recebem lotes em rodízio\.$/);
  });

  it("na divisao, diz que as duas recebem lotes em rodizio e nao pede mais papel", () => {
    expect(notasDasOpcoes({ ...base, modo: "divisao", comparacao }, "pt-BR")).toEqual([
      "Pelo histórico deste PC, Simulada — Traseira imprime cerca de 2x mais rápido que Simulada — Gaveta e termina antes. As duas recebem lotes em rodízio.",
    ]);
    expect(notasDasOpcoes({ ...base, modo: "divisao", comparacao }, "en")[0]).toMatch(
      /^Going by this PC's history, .+ prints about 2x faster than .+ and finishes first\. Both get batches in turn\.$/,
    );
  });

  it("no espelho, usa o mesmo comeco e nao fala de rodizio nem de papel", () => {
    const [nota] = notasDasOpcoes({ ...base, comparacao }, "pt-BR");
    expect(nota).toMatch(/^Pelo histórico deste PC, /);
    expect(nota).not.toMatch(/rodízio|papel/);
    const [emIngles] = notasDasOpcoes({ ...base, comparacao }, "en");
    expect(emIngles).toMatch(/^Going by this PC's history, /);
    expect(emIngles).not.toMatch(/in turn|paper/);
  });

  it("a ordem das notas e fixa: folha de controle, driver, velocidade", () => {
    const notas = notasDasOpcoes(
      { controle: true, comparacao, modo: "divisao", destinos: [FILA, TRASEIRA, GAVETA] },
      "pt-BR",
    );
    expect(notas.map((nota) => nota.slice(0, 8))).toEqual(["Com folh", "Se o dri", "Pelo his"]);
  });

  const AUTOMATICA = impressora("fila:Auto", "Auto", { modo: "automatico", podeInverter: true });
  const FLUXO_AUTO = { automatico: true, inicio: 1, fim: 4 };
  const FLUXO_MANUAL = { automatico: false, inicio: 5, fim: 6 };

  describe("nota da folha de controle pela rota", () => {
    const TUDO = "Com folha de controle";
    const SO_MANUAIS = "Folha de controle só nos trechos manuais.";
    const notaDe = (opcoes: Partial<OpcoesDaNota>, idioma: "pt-BR" | "en" = "pt-BR") =>
      notasDasOpcoes({ ...base, controle: true, ...opcoes }, idioma)[0];

    it("automatica so com papel da rota manual: tudo manual", () => {
      expect(notaDe({ destinos: [AUTOMATICA], fluxos: [FLUXO_MANUAL] })).toBe(TUDO);
    });

    it("automatica com as duas rotas: misto, nos dois idiomas", () => {
      const fluxos = [FLUXO_AUTO, FLUXO_MANUAL];
      expect(notaDe({ destinos: [AUTOMATICA], fluxos })).toBe(SO_MANUAIS);
      expect(notaDe({ destinos: [AUTOMATICA], fluxos }, "en")).toBe("Control sheet on manual sections only.");
    });

    it("manual junto de automatica que so tem papel automatico: misto", () => {
      expect(notaDe({ destinos: [FILA, AUTOMATICA], fluxos: [FLUXO_AUTO] })).toBe(SO_MANUAIS);
    });

    it("so automatico: nenhuma nota", () => {
      const notas = notasDasOpcoes({ ...base, controle: true, destinos: [AUTOMATICA], fluxos: [FLUXO_AUTO] }, "pt-BR");
      expect(notas).toEqual([]);
    });

    it("na divisao, vale o que cada uma recebe", () => {
      const fluxos = [FLUXO_AUTO, FLUXO_MANUAL];
      const so = (inicio: number, fim: number) => [{ impressora: AUTOMATICA.id, faixas: [{ inicio, fim }] }];
      const divisao = { modo: "divisao" as const, destinos: [AUTOMATICA], fluxos };
      expect(notaDe({ ...divisao, partes: so(1, 4) })).toBeUndefined();
      expect(notaDe({ ...divisao, partes: so(5, 6) })).toBe(TUDO);
      expect(notaDe({ ...divisao, partes: so(3, 6) })).toBe(SO_MANUAIS);
    });
  });

  describe("aviso da lateral quando nao sai marca", () => {
    const so = { variosPapeis: false, temPapelManual: false, soPapelAutomatico: true };

    it("com a opcao ligada, so automaticas e so papel automatico: avisa", () => {
      expect(envioSemMarca(true, [AUTOMATICA], so)).toBe(true);
    });

    it("sem a opcao, com outra rota, sem marcadas ou sem leitura: nao avisa", () => {
      expect(envioSemMarca(false, [AUTOMATICA], so)).toBe(false);
      expect(envioSemMarca(true, [AUTOMATICA, FILA], so)).toBe(false);
      expect(envioSemMarca(true, [], so)).toBe(false);
      expect(envioSemMarca(true, [AUTOMATICA], null)).toBe(false);
      expect(envioSemMarca(true, [AUTOMATICA], { soPapelAutomatico: false })).toBe(false);
    });

    it("o texto existe nos dois idiomas", () => {
      expect(traduzir("pt-BR", "controle.semMarca")).toBe("Neste envio não sai marca: o frente e verso automático não usa.");
      expect(traduzir("en", "controle.semMarca")).toBe("No mark on this print: automatic duplex doesn't use it.");
    });
  });

  it("destino so automatico nao ouve falar da ordem do driver, ligada ou nao", () => {
    const fluxos = [FLUXO_AUTO];
    expect(notasDasOpcoes({ ...base, destinos: [AUTOMATICA], fluxos }, "pt-BR")).toEqual([]);
    expect(
      notasDasOpcoes({ ...base, destinos: [{ ...AUTOMATICA, driverInverte: true }], fluxos }, "pt-BR"),
    ).toEqual([]);
  });

  it("com papel na rota manual, a automatica volta a ter o aviso e a compensacao", () => {
    const fluxos = [FLUXO_AUTO, FLUXO_MANUAL];
    expect(notasDasOpcoes({ ...base, destinos: [AUTOMATICA], fluxos }, "pt-BR")).toEqual([AVISO_PT]);
    expect(
      notasDasOpcoes({ ...base, destinos: [{ ...AUTOMATICA, driverInverte: true }], fluxos }, "pt-BR"),
    ).toEqual(["Ordem compensada: este driver inverte."]);
  });

  it("na divisao, vale so o que cada automatica recebe, e quem fica sem lote nao entra", () => {
    const fluxos = [FLUXO_AUTO, FLUXO_MANUAL];
    const so_do_automatico = [{ impressora: "fila:Auto", faixas: [{ inicio: 1, fim: 4 }] }];
    expect(
      notasDasOpcoes({ ...base, modo: "divisao", destinos: [AUTOMATICA, FILA], fluxos, partes: so_do_automatico }, "pt-BR"),
    ).toEqual([]);
    const com_manual = [{ impressora: "fila:Auto", faixas: [{ inicio: 3, fim: 6 }] }];
    expect(
      notasDasOpcoes({ ...base, modo: "divisao", destinos: [AUTOMATICA], fluxos, partes: com_manual }, "pt-BR"),
    ).toEqual([AVISO_PT]);
  });

  it("a opcao do driver so aparece na automatica com papel da rota manual", () => {
    expect(mostraOpcaoDoDriver(AUTOMATICA, false)).toBe(false);
    expect(mostraOpcaoDoDriver(AUTOMATICA, true)).toBe(true);
    expect(mostraOpcaoDoDriver({ ...FILA, podeInverter: true }, false)).toBe(true);
    expect(mostraOpcaoDoDriver(FILA, true)).toBe(false);
  });

  it("sem dados, nenhum numero nem aviso", () => {
    expect(notasDasOpcoes({ ...base, comparacao: null }, "pt-BR")).toEqual([]);
  });

  it("o fator tem uma casa so quando ela diz alguma coisa", () => {
    expect(textoDoFator(2.04, "pt-BR")).toBe("2");
    expect(textoDoFator(1.25, "pt-BR")).toBe("1,3");
    expect(textoDoFator(1.25, "en")).toBe("1.3");
  });

  it("nome hostil de impressora que sumiu fica como texto", () => {
    const hostil = { ...comparacao, rapida: "<img src=x onerror=alert(1)>" };
    expect(notasDasOpcoes({ ...base, comparacao: hostil }, "pt-BR")[0]).toContain("<img src=x");
  });
});

describe("papel, escala e rota na confirmacao", () => {
  const SEM_AJUSTE = new Set<string>();
  const A4 = { formato: "a4", larguraMm: 210, alturaMm: 297 };
  const A5 = { formato: "a5", larguraMm: 148, alturaMm: 210 };
  const AUTO = impressora("fila:Auto", "Auto", { modo: "automatico" });
  const OUTRA = impressora("fila:Outra", "Outra", { modo: "automatico" });
  const COPA = impressora("fila:Copa", "Copa", { modo: "jato" });
  const resumo = (mudancas: Partial<ResumoDoEnvio>): ResumoDoEnvio =>
    ({
      paginas: 4,
      lotes: 1,
      papeis: [A4],
      paresMistos: [],
      assinado: false,
      fluxos: [{ automatico: true, inicio: 1, fim: 4 }],
      lotesAutomaticos: 1,
      ...mudancas,
    }) as ResumoDoEnvio;

  it("uma rota so: papel, escala e rota ficam em uma linha, e o papel nao se repete", () => {
    const { linhas, notas } = linhasDoEnvio([AUTO], resumo({}), "pt-BR", SEM_AJUSTE, { papel: "A4, escala 94%" });
    expect(linhas.filter((linha) => linha.startsWith("Papel"))).toEqual([
      "Papel: A4, escala 94%, frente e verso automático, sem gesto",
    ]);
    expect(notas.join(" ")).not.toMatch(/Frente e verso|Papel:/);
    expect(
      linhasDoEnvio([AUTO], resumo({ fluxos: [{ automatico: false, inicio: 1, fim: 4 }] }), "en", SEM_AJUSTE).linhas,
    ).toContain("Paper: A4, manual duplex, with handling");
  });

  it("sem automatica, a linha nao fala de rota", () => {
    const { linhas } = linhasDoEnvio([COPA], resumo({}), "pt-BR", SEM_AJUSTE, { papel: "A4, escala 94%" });
    expect(linhas).toContain("Papel: A4, escala 94%");
  });

  it("papeis com rotas diferentes ficam em notas, antes de tudo", () => {
    const misto = resumo({
      papeis: [A4, A5],
      fluxos: [
        { automatico: true, inicio: 1, fim: 2 },
        { automatico: false, inicio: 3, fim: 4 },
      ],
    });
    const { linhas, notas } = linhasDoEnvio([AUTO], misto, "pt-BR", SEM_AJUSTE, { notasDoMeio: ["MEIO"] });
    expect(linhas).toContain("Papel: A4, A5");
    expect(notas.slice(0, 3)).toEqual([
      "Frente e verso automático, sem gesto: págs. 1–2.",
      "Frente e verso manual, com gesto: págs. 3–4.",
      "MEIO",
    ]);
    expect(notas.slice(3).length).toBeGreaterThan(0);
  });

  it("modelos diferentes: cada frase diz de quem fala, nos dois idiomas", () => {
    const dados = resumo({ fluxos: [{ automatico: true, inicio: 1, fim: 4 }] });
    expect(linhasDoEnvio([AUTO, COPA], dados, "pt-BR", SEM_AJUSTE).notasDaRota).toEqual([
      "Em Auto, págs. 1–4: frente e verso automático, sem gesto.",
      "Em Copa: frente e verso manual, com gesto.",
    ]);
    expect(linhasDoEnvio([AUTO, COPA], dados, "en", SEM_AJUSTE).notasDaRota).toEqual([
      "On Auto, pp. 1–4: automatic duplex, no handling.",
      "On Copa: manual duplex, with handling.",
    ]);
    expect(linhasDoEnvio([AUTO, OUTRA], dados, "pt-BR", SEM_AJUSTE).notasDaRota).toEqual([]);
  });

  it("na divisao, a escala fica na linha do papel e a ordem das notas se mantem", () => {
    const destinos = [COPA, impressora("fila:Hall", "Hall", { modo: "jato" })];
    const conteudo = conteudoDaDivisao(
      destinos,
      {
        ...resumo({ lotes: 2, fluxos: [] }),
        paginasDoDocumento: 4,
        faixas: [],
        divisao: [
          { impressora: "fila:Copa", lotes: [1], viraParaLer: false },
          { impressora: "fila:Hall", lotes: [2], viraParaLer: false },
        ],
      } as unknown as Parameters<typeof conteudoDaDivisao>[1],
      null,
      "pt-BR",
      { papel: "A4, escala 94%", notasDoMeio: ["MEIO"] },
    );
    expect(conteudo.linhas).toContain("Papel: A4, escala 94%");
    const notas = conteudo.notas ?? [];
    expect(notas[0]).toBe("MEIO");
    expect(conteudo.pilha?.resumo).not.toContain("A4");
  });
});

describe("textos das opcoes", () => {
  it("a dica da folha de controle e uma frase, e o aviso do canto fica a parte", () => {
    expect(traduzir("pt-BR", "controle.titulo")).toBe("Escreve o lote e a folha no canto do verso, em letra pequena.");
    expect(traduzir("en", "controle.titulo")).toBe("Writes the batch and sheet in the corner of the back, in small print.");
    expect(traduzir("pt-BR", "controle.canto")).toBe("Pode ficar sobre o que houver nesse canto.");
    expect(traduzir("en", "controle.canto")).toBe("It may overlap whatever is in that corner.");
  });

  it("a opcao do driver explica o que o PlotS faz", () => {
    expect(traduzir("pt-BR", "inversao.titulo")).toBe(
      "Ligue se este driver imprime a última página primeiro. O PlotS envia cada passada na ordem contrária para compensar.",
    );
    expect(traduzir("en", "inversao.titulo")).toBe(
      "Turn on if this driver prints the last page first. PlotS sends each pass in the opposite order to make up for it.",
    );
  });

  it("a falha ao gravar a opcao diz o que fazer", () => {
    expect(traduzir("pt-BR", "inversao.falha")).toBe("Não foi possível gravar esta opção. Tente de novo.");
    expect(traduzir("en", "inversao.falha")).toBe("This option could not be saved. Try again.");
  });
});

describe("folha de controle no cartao", () => {
  const cartao = (marcas?: string[]) =>
    ({
      sequencia: {
        frente: [{ numero: 2, girada: false }],
        verso: [{ numero: 1, girada: false }],
        marcas,
      },
    }) as unknown as Cartao;

  it("o resultado da simulada registra a marca", () => {
    expect(sequenciaDoCartao(cartao(["Lote 1 · folha 1"]), "pt-BR")).toEqual([
      "Frente: 2",
      "Verso: 1",
      "Marcas: Lote 1: folha 1",
    ]);
  });

  it("as marcas do lote viram uma linha so, na ordem em que saem", () => {
    const marcas = ["Lote 1 · folha 4", "Lote 1 · folha 3", "Lote 1 · folha 2", "Lote 1 · folha 1"];
    expect(sequenciaDoCartao(cartao(marcas), "pt-BR")[2]).toBe("Marcas: Lote 1: folhas 4, 3, 2, 1");
    expect(sequenciaDoCartao(cartao(["Batch 2 · sheet 2", "Batch 2 · sheet 1"]), "en")[2]).toBe(
      "Marks: Batch 2: sheets 2, 1",
    );
  });

  it("marca fora do formato fica como veio, sem perder as outras", () => {
    expect(sequenciaDoCartao(cartao(["Lote 1 · folha 2", "<b>x</b>", "Lote 3 · folha 1"]), "pt-BR")[2]).toBe(
      "Marcas: Lote 1: folha 2; <b>x</b>; Lote 3: folha 1",
    );
  });

  it("desligada, nada muda no cartao", () => {
    expect(sequenciaDoCartao(cartao(), "pt-BR")).toHaveLength(2);
    expect(sequenciaDoCartao(cartao([]), "pt-BR")).toHaveLength(2);
  });
});

describe("historico", () => {
  it("lista do mais novo para o mais antigo, sem mudar a lista recebida", () => {
    const lista = [
      { ...REGISTRO, data: 1, arquivo: "a" },
      { ...REGISTRO, data: 3, arquivo: "b" },
      { ...REGISTRO, data: 3, arquivo: "c" },
    ];
    expect(registrosPorData(lista).map((item) => item.arquivo)).toEqual(["c", "b", "a"]);
    expect(lista[0]?.arquivo).toBe("a");
  });

  it("o resultado sempre vem em palavras, com os lotes que faltam", () => {
    expect(textoDoResultado(REGISTRO, "pt-BR")).toBe("Concluído");
    expect(textoDoResultado({ ...REGISTRO, resultado: "cancelado", lotes_faltando: 1 }, "pt-BR")).toBe(
      "Cancelado · falta 1 lote",
    );
    expect(textoDoResultado({ ...REGISTRO, resultado: "com_erro", lotes_faltando: 3 }, "en")).toBe(
      "With error · 3 batches missing",
    );
  });

  it("diz paginas, trecho e lotes", () => {
    expect(textoDoTamanho(REGISTRO, "pt-BR")).toBe("6 páginas · 2 lotes");
    expect(textoDoTamanho({ ...REGISTRO, paginas: 1, lotes: 1, intervalo: [3, 3] }, "en")).toBe(
      "1 page · pp. 3–3 · 1 batch",
    );
  });

  it("tempo e velocidade legiveis", () => {
    expect(textoDoTempo(90_000, "pt-BR")).toBe("1 min 30 s");
    expect(textoDoTempo(400, "pt-BR")).toBe("1 s");
    expect(textoDaVelocidade(12.54, "pt-BR")).toBe("12,5 folhas/min");
    expect(textoDaVelocidade(12.54, "en")).toBe("12.5 sheets/min");
  });

  it("data absurda vinda do arquivo nao derruba a lista", () => {
    expect(textoDaData(Number.MAX_SAFE_INTEGER, "pt-BR")).toBe("—");
    expect(textoDaData(1_700_000_000, "pt-BR")).toMatch(/2023/);
  });

  it("impressora que sumiu fica com o nome guardado, sem o prefixo", () => {
    expect(nomeNoHistorico("fila:Sala", [FILA])).toBe("Sala");
    expect(nomeNoHistorico("fila:Antiga", [FILA])).toBe("Antiga");
    expect(nomeNoHistorico("simulada:gaveta", [GAVETA])).toBe("Simulada — Gaveta");
  });

  it("cada impressora tem uma linha, com o tempo dela e sem repetir o nome", () => {
    const registro: RegistroDoHistorico = {
      ...REGISTRO,
      impressoras: ["fila:Sala", "fila:Antiga", "simulada:gaveta"],
      tempos: [{ impressora: "fila:Sala", milissegundos: 1000, folhas: 2, resultado: "concluido" }],
    };
    expect(linhasDasImpressoras(registro, [FILA, GAVETA], "pt-BR")).toEqual([
      "Sala · 1 s de impressão",
      "Antiga",
      "Simulada — Gaveta",
    ]);
    expect(linhasDasImpressoras(registro, [FILA], "en")).toEqual([
      "Sala · 1 s of printing",
      "Antiga",
      "simulada:gaveta",
    ]);
  });

  it("arquivo sem tempos, com nome repetido ou tempo de quem nao esta na lista nao perde ninguem", () => {
    expect(linhasDasImpressoras(REGISTRO, [FILA], "pt-BR")).toEqual(["Sala"]);
    expect(
      linhasDasImpressoras(
        {
          ...REGISTRO,
          impressoras: ["fila:Sala", "fila:Sala"],
          tempos: [{ impressora: "fila:Outra", milissegundos: 60_000, folhas: 2, resultado: "concluido" }],
        },
        [FILA],
        "pt-BR",
      ),
    ).toEqual(["Sala", "Outra · 1 min 0 s de impressão"]);
  });

  it("os textos combinados sobre privacidade, em cada idioma", () => {
    expect(traduzir("pt-BR", "historico.local")).toBe(
      "O PlotS guarda esta lista no seu perfil do Windows e não a envia pela internet.",
    );
    expect(traduzir("en", "historico.local")).toBe(
      "PlotS keeps this list in your Windows profile and does not send it over the internet.",
    );
    expect(traduzir("pt-BR", "historico.apagarDialogo.texto")).toBe(
      "A lista inteira será apagada. Os documentos continuam como estão.",
    );
    expect(traduzir("en", "historico.apagarDialogo.texto")).toBe(
      "The whole list will be deleted. Documents stay as they are.",
    );
    expect(traduzir("pt-BR", "historico.velocidades")).toBe("Velocidade medida pelo PlotS");
    expect(traduzir("en", "historico.velocidades")).toBe("Speed measured by PlotS");
  });

  it("com as duas rotas no mesmo envio, a linha soma o tempo que a impressora trabalhou", () => {
    const registro: RegistroDoHistorico = {
      ...REGISTRO,
      tempos: [
        { impressora: "fila:Sala", milissegundos: 30_000, folhas: 10, resultado: "concluido", automatico: true },
        { impressora: "fila:Sala", milissegundos: 31_000, folhas: 2, resultado: "concluido" },
      ],
    };
    expect(linhasDasImpressoras(registro, [FILA], "pt-BR")).toEqual(["Sala · 1 min 1 s de impressão"]);
  });

  it("a velocidade diz a rota quando a mesma impressora tem as duas", () => {
    const manual = { impressora: "fila:Sala", folhas_por_minuto: 10 };
    const automatica = { ...manual, folhas_por_minuto: 40, automatico: true };
    const so_manual = nomeDaVelocidade(manual, [manual], [FILA], "pt-BR");
    expect(so_manual).toBe("Sala");
    expect(nomeDaVelocidade(manual, [manual, automatica], [FILA], "pt-BR")).toBe("Sala · manual");
    expect(nomeDaVelocidade(automatica, [manual, automatica], [FILA], "pt-BR")).toBe("Sala · automático");
    expect(nomeDaVelocidade(automatica, [automatica], [FILA], "en")).toBe("Sala · automatic");
  });

  it("apagar sempre pergunta", () => {
    const conteudo = conteudoDoApagarHistorico();
    expect(conteudo.titulo).toBe("historico.apagarDialogo.titulo");
    expect(conteudo.fechar).toBe("historico.apagarDialogo.confirmar");
  });
});
