import { describe, expect, it } from "vitest";
import {
  anuncioDoPasso,
  eUltimaPergunta,
  estadoInicial,
  filaNaoServe,
  fraseDoPasso,
  INSTRUCAO_DA_PERGUNTA,
  instrucaoDoPasso,
  leituraFinal,
  numeroDoPasso,
  OPCOES,
  pedeConfirmacaoAoDesistir,
  perguntasDaRodada,
  proximaRodada,
  reduzir,
  chavesDoAnuncio,
  resumoDasRespostas,
  respostasParaAvaliar,
  RODADAS_MAXIMAS,
  temPassadaNaFila,
  trocaPoeCalibracaoEmDuvida,
  TEXTO_DA_MUDANCA,
  TEXTO_DA_PERGUNTA,
  type Acao,
  type Estado,
  type PerfilDaRodada,
  type Pergunta,
} from "./calibracao";
import type { Cartao, EstadoCartao } from "./cartoes";
import { traducoes } from "./i18n";
import { estadoDoSeletor, pedeConfirmacao, type Impressora } from "./impressoras";

const FILA: Impressora = {
  id: "fila:Sala",
  nome: "Sala",
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "sem_perfil",
};

const PERFIL: PerfilDaRodada = {
  frenteInversa: true,
  versoInverso: true,
  versoGirado: true,
  virada: "borda_de_cima",
  borda: "curta",
  driverInverte: false,
};

function cartao(estado: EstadoCartao): Cartao {
  return {
    aba: 1,
    impressora: FILA.id,
    nome: FILA.nome,
    simulada: false,
    lote: 1,
    total: 1,
    papel: null,
    estado,
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
  };
}

function aplicar(estado: Estado, ...acoes: Acao[]): Estado {
  return acoes.reduce(reduzir, estado);
}

const responder = (pergunta: Pergunta, valor: string): Acao[] => [
  { tipo: "responder", pergunta, valor },
  { tipo: "continuar" },
];

function ateAsPerguntas(inicio: Estado = estadoInicial(FILA)): Estado {
  const frente = aplicar(
    inicio,
    { tipo: "seguir_para_confirmar" },
    { tipo: "rodada_iniciada", perfil: PERFIL, rodada: inicio.rodada },
    { tipo: "cartao", cartao: cartao({ tipo: "aguardando" }) },
  );
  const recolocar = frente.passo === "saida" ? aplicar(frente, ...responder("saida", "cima")) : frente;
  return aplicar(recolocar, { tipo: "verso_pedido" }, { tipo: "cartao", cartao: cartao({ tipo: "concluido" }) });
}

describe("ponto de partida", () => {
  it.each([
    ["sem_perfil", "jato"],
    ["personalizado", "jato"],
    ["calibrada", "jato"],
    ["jato", "jato"],
    ["laser", "laser"],
  ] as const)("perfil %s parte do modelo %s", (modo, modelo) => {
    expect(estadoInicial({ ...FILA, modo }).modelo).toBe(modelo);
  });

  it("a simulada de gaveta parte da laser, e a automatica nao tem gesto", () => {
    expect(estadoInicial({ ...FILA, id: "simulada:gaveta", tipo: "simulada", modo: "laser" }).modelo).toBe("laser");
    expect(estadoInicial({ ...FILA, modo: "automatico" }).automatica).toBe(true);
  });
});

describe("perguntas de cada rodada", () => {
  it("impressao por cima encerra as perguntas", () => {
    expect(perguntasDaRodada(false, {})).toEqual(["lado", "leitura", "par", "setas"]);
    expect(perguntasDaRodada(false, { lado: "por_cima" })).toEqual(["lado"]);
  });

  it("o que olha o maco inteiro vem antes de mandar pegar uma folha", () => {
    expect(perguntasDaRodada(false, { leitura: "nao_fica" })).toEqual(["lado", "leitura", "primeira", "par", "setas"]);
    expect(perguntasDaRodada(true, {})).toEqual(["leitura", "par", "setas"]);
  });

  it("so o maco invertido pergunta qual folha saiu primeiro", () => {
    expect(perguntasDaRodada(false, { leitura: "virando" })).not.toContain("primeira");
    expect(perguntasDaRodada(false, { leitura: "nao_fica" })).toContain("primeira");
  });

  it("a automatica confere virada e ordem, sem perguntar lado nem driver", () => {
    expect(perguntasDaRodada(true, { leitura: "nao_fica" })).toEqual(["leitura", "par", "setas"]);
  });
});

describe("fluxo", () => {
  it("anda do preparo ao fim, um passo de cada vez", () => {
    let estado = estadoInicial(FILA);
    expect(numeroDoPasso(estado.passo)).toBe(1);
    estado = reduzir(estado, { tipo: "seguir_para_confirmar" });
    expect(estado.passo).toBe("confirmar");
    expect(numeroDoPasso(estado.passo)).toBe(2);
    estado = reduzir(estado, { tipo: "rodada_iniciada", perfil: PERFIL, rodada: 1 });
    expect(estado.passo).toBe("frente");
    expect(temPassadaNaFila(estado)).toBe(true);
    estado = reduzir(estado, { tipo: "cartao", cartao: cartao({ tipo: "imprimindo", passada: "frente" }) });
    expect(estado.passo).toBe("frente");
    estado = reduzir(estado, { tipo: "cartao", cartao: cartao({ tipo: "aguardando" }) });
    expect(estado.passo).toBe("saida");
    expect(temPassadaNaFila(estado)).toBe(false);
    estado = aplicar(estado, ...responder("saida", "baixo"));
    expect(estado.passo).toBe("recolocar");
    expect(estado.saiuParaCima).toBe(false);
    estado = reduzir(estado, { tipo: "verso_pedido" });
    expect(estado.passo).toBe("verso");
    estado = reduzir(estado, { tipo: "cartao", cartao: cartao({ tipo: "concluido" }) });
    expect(estado.passo).toBe("perguntas");
    expect(estado.pergunta).toBe("lado");
    expect(numeroDoPasso(estado.passo)).toBe(4);
  });

  it("como a folha sai so e perguntado uma vez: e fato da impressora", () => {
    const certa = aplicar(
      ateAsPerguntas(),
      ...responder("lado", "por_cima"),
      { tipo: "avaliada", avaliacao: { certo: false, mudancas: ["virada_nao_virar"], proximo: PERFIL } },
    );
    expect(certa.passo).toBe("ajustar");
    const segunda = aplicar(
      certa,
      { tipo: "rodada_iniciada", perfil: PERFIL, rodada: 2 },
      { tipo: "cartao", cartao: cartao({ tipo: "aguardando" }) },
    );
    expect(segunda.passo).toBe("recolocar");
    expect(segunda.saiuParaCima).toBe(true);
    expect(segunda.rodada).toBe(2);
  });

  it("sem resposta nao avanca, e voltar reaproveita o que foi respondido", () => {
    const inicio = ateAsPerguntas();
    expect(reduzir(inicio, { tipo: "continuar" }).pergunta).toBe("lado");
    const adiante = aplicar(inicio, ...responder("lado", "em_branco"));
    expect(adiante.pergunta).toBe("leitura");
    const voltou = reduzir(adiante, { tipo: "voltar" });
    expect(voltou.pergunta).toBe("lado");
    expect(voltou.respostas.lado).toBe("em_branco");
    expect(reduzir(voltou, { tipo: "voltar" }).pergunta).toBe("lado");
  });

  it("trocar a resposta anterior apaga as que deixaram de valer", () => {
    const respondida = aplicar(
      ateAsPerguntas(),
      ...responder("lado", "em_branco"),
      ...responder("leitura", "nao_fica"),
      ...responder("primeira", "3"),
      ...responder("par", "sim"),
      { tipo: "responder", pergunta: "setas", valor: "nao" },
    );
    expect(eUltimaPergunta(respondida)).toBe(true);
    const sem = reduzir(respondida, { tipo: "responder", pergunta: "leitura", valor: "virando" });
    expect(sem.respostas.primeira).toBeUndefined();
    const porCima = reduzir(respondida, { tipo: "responder", pergunta: "lado", valor: "por_cima" });
    expect(porCima.respostas).toEqual({ lado: "por_cima" });
  });

  it("as respostas viram o que o backend espera, sem chutar o que falta", () => {
    const inicio = ateAsPerguntas();
    expect(respostasParaAvaliar(inicio)).toBeNull();
    const faltaUma = aplicar(inicio, ...responder("lado", "em_branco"), ...responder("leitura", "nao_fica"));
    expect(respostasParaAvaliar(faltaUma)).toBeNull();
    const toda = aplicar(
      faltaUma,
      ...responder("primeira", "nao_reparei"),
      ...responder("par", "sim"),
      { tipo: "responder", pergunta: "setas", valor: "nao" },
    );
    expect(respostasParaAvaliar(toda)).toEqual({
      porCima: false,
      parCerto: true,
      setasIguais: false,
      leitura: "nao_fica",
      primeiraASair: null,
    });
    expect(
      respostasParaAvaliar(reduzir(toda, { tipo: "responder", pergunta: "primeira", valor: "1" }))?.primeiraASair,
    ).toBe(1);
    const porCima = reduzir(inicio, { tipo: "responder", pergunta: "lado", valor: "por_cima" });
    expect(respostasParaAvaliar(porCima)?.porCima).toBe(true);
  });

  it("depois da ultima rodada errada, diz que nao se encaixou", () => {
    const errada = { certo: false, mudancas: ["verso_girado"], proximo: PERFIL };
    let estado = ateAsPerguntas();
    for (let rodada = 1; rodada < RODADAS_MAXIMAS; rodada += 1) {
      estado = reduzir(ateAsPerguntas({ ...estado, rodada }), { tipo: "avaliada", avaliacao: errada });
      expect(estado.passo).toBe("ajustar");
      expect(proximaRodada(estado)).toEqual({ perfil: PERFIL, rodada: rodada + 1 });
    }
    estado = ateAsPerguntas({ ...estado, rodada: RODADAS_MAXIMAS });
    estado = reduzir(estado, { tipo: "avaliada", avaliacao: errada });
    expect(estado.passo).toBe("nao_encaixou");
    expect(proximaRodada(estado)).toBeNull();
  });

  it("resposta certa encerra em qualquer rodada e guarda como ler o maco", () => {
    const certa = { certo: true, mudancas: [], proximo: PERFIL };
    let estado = aplicar(ateAsPerguntas(), ...responder("lado", "em_branco"), ...responder("leitura", "virando"));
    estado = aplicar(estado, ...responder("par", "sim"), { tipo: "responder", pergunta: "setas", valor: "sim" });
    estado = reduzir(estado, { tipo: "avaliada", avaliacao: certa });
    expect(estado.passo).toBe("certo");
    expect(leituraFinal(estado)).toBe("virando");
    expect(reduzir(estado, { tipo: "gravada" }).passo).toBe("gravada");
    expect(reduzir(estado, { tipo: "falha_ao_gravar" }).recusa).toBe("falha_ao_gravar");
  });

  it("erro na fila nao avanca, e tentar de novo volta a confirmacao", () => {
    const frente = aplicar(estadoInicial(FILA), {
      tipo: "rodada_iniciada",
      perfil: PERFIL,
      rodada: 1,
    });
    const erro = reduzir(frente, {
      tipo: "cartao",
      cartao: cartao({ tipo: "erro", motivo: { codigo: "fila", texto: "offline" } }),
    });
    expect(erro.passo).toBe("frente");
    expect(temPassadaNaFila(erro)).toBe(false);
    expect(reduzir(erro, { tipo: "refazer" }).passo).toBe("confirmar");
  });

  it("impressora ocupada recusa sem sair do passo", () => {
    const estado = aplicar(estadoInicial(FILA), { tipo: "seguir_para_confirmar" }, { tipo: "recusada", recusa: "ocupada" });
    expect(estado.passo).toBe("confirmar");
    expect(estado.recusa).toBe("ocupada");
  });

  it.each(["falha_envio", "falha_perfil"] as const)("%s nao se confunde com impressora ocupada", (recusa) => {
    const estado = aplicar(estadoInicial(FILA), { tipo: "seguir_para_confirmar" }, { tipo: "recusada", recusa });
    expect(estado.passo).toBe("confirmar");
    expect(estado.recusa).toBe(recusa);
  });

  it("verso que nao foi aceito volta a recolocar, com o alerta de envio", () => {
    const estado = aplicar(ateAsPerguntas(), { tipo: "verso_pedido" }, { tipo: "verso_falhou" });
    expect(estado.passo).toBe("recolocar");
    expect(estado.recusa).toBe("falha_envio");
    expect(reduzir(estado, { tipo: "verso_pedido" }).recusa).toBeNull();
  });

  it("so a fila que pede arquivo e dada como sem conserto", () => {
    const comMotivo = (codigo: string) =>
      reduzir(estadoInicial(FILA), { tipo: "cartao", cartao: cartao({ tipo: "erro", motivo: { codigo, texto: "" } }) });
    expect(filaNaoServe(comMotivo("fila_pede_arquivo"))).toBe(true);
    expect(filaNaoServe(comMotivo("fila"))).toBe(false);
    expect(filaNaoServe(estadoInicial(FILA))).toBe(false);
  });

  it("so a tela certa de uma fila real pede confirmacao ao desistir", () => {
    const certo = { ...estadoInicial(FILA), passo: "certo" as const };
    expect(pedeConfirmacaoAoDesistir(certo, false)).toBe(true);
    expect(pedeConfirmacaoAoDesistir(certo, true)).toBe(false);
    expect(pedeConfirmacaoAoDesistir(estadoInicial(FILA), false)).toBe(false);
    expect(pedeConfirmacaoAoDesistir({ ...certo, passo: "gravada" }, false)).toBe(false);
  });

  it("o anuncio de cada passo diz a pergunta ou a conclusao", () => {
    expect(fraseDoPasso(estadoInicial(FILA))).toBeNull();
    expect(fraseDoPasso(ateAsPerguntas())).toBe("calibracao.pergunta.lado");
    expect(fraseDoPasso({ ...estadoInicial(FILA), passo: "certo" })).toBe("calibracao.certo.titulo");
    expect(fraseDoPasso({ ...estadoInicial(FILA), passo: "conferida", conclusao: "nao_vira_pela_borda_longa" })).toBe(
      "calibracao.conferida.borda",
    );
  });

  it("cartao fora de passada nao muda de passo", () => {
    const estado = reduzir(estadoInicial(FILA), { tipo: "cartao", cartao: cartao({ tipo: "concluido" }) });
    expect(estado.passo).toBe("preparar");
  });

  it("a automatica vai direto as perguntas e termina na conferencia", () => {
    const estado = aplicar(
      estadoInicial({ ...FILA, modo: "automatico" }),
      { tipo: "rodada_iniciada", perfil: PERFIL, rodada: 1 },
      { tipo: "cartao", cartao: cartao({ tipo: "concluido" }) },
    );
    expect(estado.pergunta).toBe("leitura");
    const fim = reduzir(estado, { tipo: "conferida", conclusao: "ordem_nao_serve" });
    expect(fim.passo).toBe("conferida");
    expect(fim.conclusao).toBe("ordem_nao_serve");
  });
});

describe("textos das perguntas", () => {
  const idiomas = ["pt-BR", "en"] as const;
  const perguntas = Object.keys(OPCOES) as Pergunta[];

  it("cada resposta tem a sua figura, sem repetir", () => {
    const figuras = perguntas.flatMap((pergunta) => OPCOES[pergunta].map((opcao) => opcao.figura));
    expect(new Set(figuras).size).toBe(figuras.length);
    for (const pergunta of perguntas) expect(OPCOES[pergunta].length).toBeGreaterThanOrEqual(2);
  });

  it.each(idiomas)("%s tem frase para cada pergunta e cada mudanca, sem nome de marca", (idioma) => {
    for (const pergunta of perguntas) {
      expect(traducoes[idioma][TEXTO_DA_PERGUNTA[pergunta]].length).toBeGreaterThan(10);
    }
    for (const chave of TEXTO_DA_MUDANCA.values()) expect(traducoes[idioma][chave].length).toBeGreaterThan(5);
    const todos = Object.entries(traducoes[idioma])
      .filter(([chave]) => chave.startsWith("calibracao."))
      .map(([, texto]) => texto)
      .join(" ");
    expect(todos).not.toMatch(/\b(HP|Brother|Epson|Canon)\b/);
  });
});

describe("linha da fila calibrada", () => {
  it("avisa ao trocar a compensacao do driver, e so em fila calibrada", () => {
    expect(trocaPoeCalibracaoEmDuvida("calibrada", false, true)).toBe(true);
    expect(trocaPoeCalibracaoEmDuvida("calibrada", true, false)).toBe(true);
    expect(trocaPoeCalibracaoEmDuvida("calibrada", true, true)).toBe(false);
    expect(trocaPoeCalibracaoEmDuvida("laser", false, true)).toBe(false);
  });

  it("mostra Calibrada e pede confirmacao para trocar por um modelo", () => {
    const seletor = estadoDoSeletor({ ...FILA, modo: "calibrada" }, undefined, "pt-BR");
    expect(seletor?.valor).toBe("calibrada");
    expect(seletor?.opcoes[0]?.rotulo).toBe("Calibrada");
    expect(estadoDoSeletor({ ...FILA, modo: "calibrada" }, undefined, "en")?.opcoes[0]?.rotulo).toBe("Calibrated");
    expect(pedeConfirmacao("calibrada", undefined)).toBe(true);
    expect(pedeConfirmacao("calibrada", "jato")).toBe(false);
  });
});

describe("correcao depois da tela certa", () => {
  const manual = {
    ...estadoInicial(FILA),
    passo: "certo" as const,
    respostas: { lado: "em_branco", leitura: "sem_virar", par: "sim", setas: "nao" } as const,
  };

  it("corrigir volta a ultima pergunta e mantem as respostas anteriores", () => {
    const voltou = reduzir({ ...manual, respostas: { ...manual.respostas } }, { tipo: "corrigir" });
    expect(voltou.passo).toBe("perguntas");
    expect(voltou.pergunta).toBe("setas");
    expect(voltou.respostas).toEqual(manual.respostas);
  });

  it("a pergunta de volta vale para a rodada com primeira folha, sem perder a resposta", () => {
    const comPrimeira = { ...manual, respostas: { ...manual.respostas, leitura: "nao_fica" as const, primeira: "1" as const } };
    expect(reduzir(comPrimeira, { tipo: "corrigir" }).pergunta).toBe("setas");
  });

  it("corrigir fora da tela certa nao muda nada", () => {
    const perguntas = { ...manual, passo: "perguntas" as const, pergunta: "par" as const };
    expect(reduzir(perguntas, { tipo: "corrigir" })).toEqual(perguntas);
  });

  it("o resumo traz as respostas da rodada, na ordem das perguntas", () => {
    expect(resumoDasRespostas(manual)).toEqual([
      { rotulo: "calibracao.resumo.lado", valor: "calibracao.resposta.emBranco" },
      { rotulo: "calibracao.resumo.leitura", valor: "calibracao.resposta.semVirar" },
      { rotulo: "calibracao.resumo.par", valor: "calibracao.resposta.parCerto" },
      { rotulo: "calibracao.resumo.setas", valor: "calibracao.resposta.setasOpostas" },
    ]);
  });

  it("uma falha do julgamento tem recusa propria, e nao a de gravar", () => {
    const perguntas = { ...manual, passo: "perguntas" as const, pergunta: "setas" as const };
    const recusado = reduzir(perguntas, { tipo: "falha_julgamento" });
    expect(recusado.recusa).toBe("falha_julgamento");
    expect(recusado.passo).toBe("perguntas");
  });

  it("continuar com a pergunta fora da lista nao pula para a primeira", () => {
    const fora = {
      ...estadoInicial(FILA),
      passo: "perguntas" as const,
      pergunta: "primeira" as const,
      respostas: { leitura: "sem_virar" as const, primeira: "1" as const },
    };
    expect(reduzir(fora, { tipo: "continuar" })).toEqual(fora);
  });

  it("a instrucao e a pergunta de lado, leitura e setas somam o texto que ja existia", () => {
    expect(INSTRUCAO_DA_PERGUNTA.leitura).toBe("calibracao.instrucao.leitura");
    expect(instrucaoDoPasso({ ...manual, passo: "perguntas", pergunta: "par" })).toBe("calibracao.instrucao.par");
    expect(instrucaoDoPasso({ ...manual, passo: "perguntas", pergunta: "leitura" })).toBe("calibracao.instrucao.leitura");
  });

  it("sem o valor guardado na calibracao, trocar a compensacao nao fica em duvida", () => {
    expect(trocaPoeCalibracaoEmDuvida("calibrada", undefined, true)).toBe(false);
    expect(trocaPoeCalibracaoEmDuvida("calibrada", undefined, false)).toBe(false);
  });
});

describe("anuncio do passo", () => {
  it("na recusa da fila que pede arquivo, o leitor de tela nao anuncia o progresso", () => {
    expect(anuncioDoPasso("Passo 2 de 5", ["Imprimir frente"], true)).toBe("");
  });

  it("fora da recusa, o progresso vem antes dos textos do passo", () => {
    expect(anuncioDoPasso("Passo 2 de 5", [], false)).toBe("Passo 2 de 5");
    expect(anuncioDoPasso("Passo 4 de 5", ["Sem mudar a ordem.", "Quando o 1 aparece?"], false)).toBe(
      "Passo 4 de 5. Sem mudar a ordem. Quando o 1 aparece?",
    );
  });
});

const PERFIL_TESTE: PerfilDaRodada = {
  frenteInversa: true,
  versoInverso: true,
  versoGirado: false,
  virada: "lateral",
  borda: "curta",
  driverInverte: false,
};

describe("julgamento que da certo limpa a recusa anterior", () => {
  it("depois de uma falha do julgamento, o resultado que chega tira a mensagem", () => {
    const comFalha = {
      ...estadoInicial(FILA),
      passo: "perguntas" as const,
      pergunta: "setas" as const,
      recusa: "falha_julgamento" as const,
    };
    const certo = reduzir(comFalha, {
      tipo: "avaliada",
      avaliacao: { proximo: PERFIL_TESTE, mudancas: [], certo: true },
    });
    expect(certo.passo).toBe("certo");
    expect(certo.recusa).toBeNull();
    const conferida = reduzir(comFalha, { tipo: "conferida", conclusao: "certa" });
    expect(conferida.recusa).toBeNull();
  });
});

describe("rodada 4: recusa limpa, par, resposta antiga e anuncio", () => {
  it("uma chamada nova tira a mensagem anterior, e a falha seguinte volta a aparecer", () => {
    const falhou = reduzir(estadoInicial(FILA), { tipo: "recusada", recusa: "falha_ao_gravar" });
    const nova = reduzir(falhou, { tipo: "chamada_iniciada" });
    expect(nova.recusa).toBeNull();
    expect(reduzir(nova, { tipo: "falha_ao_gravar" }).recusa).toBe("falha_ao_gravar");
  });

  it("uma primeira folha que a rodada nao pergunta nao entra na avaliacao", () => {
    const r = respostasParaAvaliar({
      ...estadoInicial(FILA),
      respostas: { lado: "em_branco", leitura: "sem_virar", par: "sim", setas: "sim", primeira: "1" },
    });
    expect(r?.primeiraASair).toBeNull();
  });

  it("o anuncio de desistir traz a pergunta e a explicacao", () => {
    expect(chavesDoAnuncio({ ...estadoInicial(FILA), passo: "certo" }, true)).toEqual([
      "calibracao.desistir.pergunta",
      "calibracao.desistir.explicacao",
    ]);
  });

  it("a trava de envio e de gravacao tem texto de andamento", () => {
    expect(traducoes["pt-BR"]["calibracao.ocupacao.enviando"]).toBe("Enviando para a impressora…");
    expect(traducoes.en["calibracao.ocupacao.gravando"]).toBe("Saving the profile…");
  });
});
