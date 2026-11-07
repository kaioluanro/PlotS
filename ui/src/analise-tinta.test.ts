import { describe, expect, it } from "vitest";
import {
  EVENTOS_DA_TINTA,
  configuracaoPadrao,
  formatarMl,
  formatarMoeda,
  formatarPercentual,
  lerCopiasDoPainel,
  mensagemDeAviso,
  motivoDaFalha,
  normalizarConfiguracao,
  normalizarEstadoDaConfiguracao,
  normalizarEstadoDosPerfis,
  normalizarPagina,
  normalizarPerfil,
  normalizarProgresso,
  normalizarResultado,
  resumoDaBarra,
  rotuloDoPapel,
  textoDePaginas,
  totalDePaginas,
  type ConfiguracaoDeTinta,
  type PerfilDeTinta,
  type ResultadoDaAnalise,
} from "./analise-tinta";

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
  perfil: "tanque-generico",
  qualidade: "normal",
  dpi: 150,
  modo: "simples",
  cinzas_so_preto: true,
  tamanho_da_imagem: "a4",
};

function perfilCru(mudanca: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: PERFIL.id,
    nome: PERFIL.nome,
    moeda: PERFIL.moeda,
    volumes_ml: [...PERFIL.volumes_ml],
    rendimentos_paginas: [...PERFIL.rendimentos_paginas],
    precos_ml: [...PERFIL.precos_ml],
    fatores_qualidade: [...PERFIL.fatores_qualidade],
    ...mudanca,
  };
}

function canaisCru(c: number, total = c): Record<string, number> {
  return { c, m: 0, y: 0, k: 0, total };
}

function paginaCru(mudanca: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: "job-1",
    pagina: 1,
    larguraMm: 210,
    alturaMm: 297,
    cobertura: canaisCru(10),
    tinta: canaisCru(0.1),
    custo: canaisCru(0.1),
    ...mudanca,
  };
}

function cartuchoCru(mudanca: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    percentualConsumido: 12.5,
    copiasAteEsvaziar: 50,
    canalQueAcabaPrimeiro: 1,
    ...mudanca,
  };
}

function resumoCru(mudanca: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    paginas: [2, 5, 7, 9],
    quantidade: 4,
    totalComCopias: 4,
    tinta: canaisCru(0.4),
    custo: canaisCru(0.4),
    cartucho: cartuchoCru(),
    ...mudanca,
  };
}

function documentoCru(mudanca: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    tinta: canaisCru(1.2),
    custo: canaisCru(1.2),
    coberturaMedia: canaisCru(10),
    mlMedioPorPagina: 0.12,
    paginaMaisCara: {
      numero: 7,
      tinta: canaisCru(0.3),
      custo: canaisCru(0.3),
    },
    ...mudanca,
  };
}

function resultadoCru(mudanca: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    totalDePaginas: 10,
    modoUsado: "simples",
    dpi: 150,
    perfil: PERFIL.id,
    moeda: "BRL",
    qualidade: "normal",
    copias: 1,
    documentoInteiro: false,
    selecao: resumoCru(),
    documento: documentoCru(),
    ...mudanca,
  };
}

describe("eventos e comandos da tinta", () => {
  it("assina exatamente os quatro nomes combinados com a interface", () => {
    expect(EVENTOS_DA_TINTA).toEqual({
      progresso: "plotinfo://progresso",
      pagina: "plotinfo://pagina",
      fim: "plotinfo://fim",
      falha: "plotinfo://falha",
    });
  });
});

describe("leitura de perfil", () => {
  it("aceita o perfil completo do backend", () => {
    expect(normalizarPerfil(perfilCru())).toEqual(PERFIL);
  });

  it("recusa identificador vazio, moeda minuscula e vetor de tamanho errado", () => {
    expect(normalizarPerfil(perfilCru({ id: " " }))).toBeNull();
    expect(normalizarPerfil(perfilCru({ moeda: "brl" }))).toBeNull();
    expect(normalizarPerfil(perfilCru({ volumes_ml: [65, 65, 65] }))).toBeNull();
    expect(normalizarPerfil(perfilCru({ fatores_qualidade: [0.6, 1] }))).toBeNull();
  });

  it("recusa volume zero, rendimento zero e preco negativo", () => {
    expect(normalizarPerfil(perfilCru({ volumes_ml: [0, 65, 65, 65] }))).toBeNull();
    expect(normalizarPerfil(perfilCru({ rendimentos_paginas: [1, 0, 1, 1] }))).toBeNull();
    expect(normalizarPerfil(perfilCru({ precos_ml: [1, -1, 1, 1] }))).toBeNull();
    expect(normalizarPerfil(perfilCru({ fatores_qualidade: [0.6, 0, 1.3] }))).toBeNull();
  });

  it("recusa identificador e nome acima dos limites do arquivo", () => {
    expect(normalizarPerfil(perfilCru({ id: "x".repeat(65) }))).toBeNull();
    expect(normalizarPerfil(perfilCru({ nome: "x".repeat(201) }))).toBeNull();
  });
});

describe("leitura da configuracao", () => {
  it("aceita a configuracao completa", () => {
    expect(normalizarConfiguracao(CONFIGURACAO)).toEqual(CONFIGURACAO);
  });

  it("recusa dpi fora da lista, qualidade e modo desconhecidos", () => {
    expect(normalizarConfiguracao({ ...CONFIGURACAO, dpi: 200 })).toBeNull();
    expect(normalizarConfiguracao({ ...CONFIGURACAO, qualidade: "maxima" })).toBeNull();
    expect(normalizarConfiguracao({ ...CONFIGURACAO, modo: "rgb" })).toBeNull();
    expect(normalizarConfiguracao({ ...CONFIGURACAO, cinzas_so_preto: "sim" })).toBeNull();
  });

  it("estado com configuracao ausente cai nos valores iniciais", () => {
    const estado = normalizarEstadoDaConfiguracao({ aviso: "arquivo_corrompido" });
    expect(estado.configuracao).toEqual(configuracaoPadrao(""));
    expect(estado.aviso).toBe("arquivo_corrompido");
  });
});

describe("leitura de pagina", () => {
  it("aceita a pagina plana e calcula o total dos canais quando ele falta", () => {
    const pagina = normalizarPagina(
      paginaCru({
        tinta: { c: 0.1, m: 0, y: 0, k: 0 },
        custo: { c: 0.1, m: 0, y: 0, k: 0 },
      }),
    );
    expect(pagina).toEqual({
      numero: 1,
      largura_mm: 210,
      altura_mm: 297,
      cobertura_pct: [10, 0, 0, 0],
      tinta_ml: [0.1, 0, 0, 0],
      custo: [0.1, 0, 0, 0],
      total_ml: 0.1,
      total_custo: 0.1,
      em_branco: false,
    });
  });

  it("pagina em branco e 0,0% real, sem inventar cobertura", () => {
    const pagina = normalizarPagina(
      paginaCru({
        cobertura: canaisCru(0, 0),
        tinta: canaisCru(0, 0),
        custo: canaisCru(0, 0),
      }),
    );
    expect(pagina?.em_branco).toBe(true);
    expect(pagina?.cobertura_pct).toEqual([0, 0, 0, 0]);
    expect(pagina?.total_ml).toBe(0);
  });

  it("recusa numero fora do documento e cobertura fora de 0 a 100", () => {
    expect(normalizarPagina(paginaCru({ pagina: 0 }))).toBeNull();
    expect(
      normalizarPagina(paginaCru({ cobertura: { c: 101, m: 0, y: 0, k: 0 } })),
    ).toBeNull();
    expect(normalizarPagina(paginaCru({ larguraMm: 0 }))).toBeNull();
  });
});

describe("leitura do resultado", () => {
  it("aceita o resumo completo e o identificador do trabalho", () => {
    const resultado = normalizarResultado(resultadoCru(), "job-1", "reserva.pdf");
    expect(resultado?.id).toBe("job-1");
    expect(resultado?.arquivo.nome).toBe("reserva.pdf");
    expect(resultado?.arquivo.paginas).toBe(10);
    expect(resultado?.selecao.paginas).toEqual([2, 5, 7, 9]);
    expect(resultado?.selecao.quantidade).toBe(4);
    expect(resultado?.selecao.total_com_copias).toBe(4);
    expect(resultado?.documento.pagina_mais_cara?.numero).toBe(7);
    expect(resultado?.selecao.cartucho.canal_que_acaba_primeiro).toBe("magenta");
    expect(resultado?.selecao.cartucho.percentual_consumido).toBe(12.5);
    expect(resultado?.documento_inteiro).toBe(false);
  });

  it("usa o nome da aba como nome do arquivo", () => {
    expect(normalizarResultado(resultadoCru(), "job-1", "reserva.pdf")?.arquivo.nome).toBe(
      "reserva.pdf",
    );
    expect(normalizarResultado(resultadoCru(), "job-1", "")?.arquivo.nome).toBe("—");
  });

  it("recusa resultado sem cartucho ou com resumo torto", () => {
    expect(
      normalizarResultado(resultadoCru({ selecao: resumoCru({ cartucho: undefined }) }), "job-1"),
    ).toBeNull();
    expect(
      normalizarResultado(
        resultadoCru({ selecao: resumoCru({ tinta: { c: -1, m: 0, y: 0, k: 0 } }) }),
        "job-1",
      ),
    ).toBeNull();
    expect(
      normalizarResultado(resultadoCru({ documento: documentoCru({ tinta: null }) }), "job-1"),
    ).toBeNull();
  });

  it("leva o total com copias que o backend calculou", () => {
    const resultado = normalizarResultado(
      resultadoCru({ selecao: resumoCru({ totalComCopias: 144 }) }),
      "job-1",
      "reserva.pdf",
    );
    expect(resultado?.selecao.total_com_copias).toBe(144);
  });
});

describe("aviso do arquivo de perfis", () => {
  it("traduz o codigo do backend e mantem um codigo desconhecido visivel", () => {
    expect(mensagemDeAviso(null)).toBeNull();
    expect(mensagemDeAviso("arquivo_corrompido")).toBe(
      "O arquivo de perfis está corrompido; os padrões valem.",
    );
    expect(mensagemDeAviso("codigo_novo")).toBe(
      "O arquivo de perfis foi lido com um aviso: codigo_novo.",
    );
  });
});

describe("progresso e falha", () => {
  it("aceita progresso dentro do total e recusa feitas maior que total", () => {
    const progresso = normalizarProgresso({ feitas: 3, total: 12, selecao: null });
    expect(progresso).toEqual({ feitas: 3, total: 12, selecao: null });
    expect(normalizarProgresso({ feitas: 13, total: 12, selecao: null })).toBeNull();
    expect(normalizarProgresso({ feitas: 3, total: 0, selecao: null })).toBeNull();
  });

  it("a selecao chega junto do progresso assim que as paginas dela ficam prontas", () => {
    const progresso = normalizarProgresso({
      feitas: 3,
      total: 12,
      selecao: resumoCru({ totalComCopias: 12 }),
    });
    expect(progresso?.selecao?.quantidade).toBe(4);
    expect(progresso?.selecao?.total_com_copias).toBe(12);
    expect(progresso?.selecao?.cartucho.canal_que_acaba_primeiro).toBe("magenta");
  });

  it("mostra o motivo do backend e traduz os codigos conhecidos", () => {
    expect(motivoDaFalha("Disco cheio")).toBe("Disco cheio");
    expect(motivoDaFalha({ detalhe: "Página ilegível" })).toBe("Página ilegível");
    expect(motivoDaFalha({ codigo: "copias_invalidas" })).toBe(
      "As cópias precisam ser um número de 1 a 999.",
    );
    expect(motivoDaFalha({ codigo: "perfil_nao_encontrado" })).toBe(
      "O perfil de tinta não foi encontrado.",
    );
    expect(motivoDaFalha({ codigo: "renderizacao", detalhe: "sem memória" })).toBe(
      "sem memória",
    );
    expect(motivoDaFalha({ codigo: "desconhecido" })).toBe(
      "A estimativa falhou. Tente de novo.",
    );
  });
});

describe("campos do painel", () => {
  it("vazio vale o documento inteiro, e o texto vai aparado para o backend", () => {
    expect(textoDePaginas("")).toBeNull();
    expect(textoDePaginas("   ")).toBeNull();
    expect(textoDePaginas(" 2,5,7,9 ")).toBe("2,5,7,9");
    expect(textoDePaginas("5-16")).toBe("5-16");
  });

  it("copias aceitam 1 a 999 e recusam 0, 1000, vazio e fracao", () => {
    expect(lerCopiasDoPainel("1")).toBe(1);
    expect(lerCopiasDoPainel("999")).toBe(999);
    expect(lerCopiasDoPainel("0")).toBeNull();
    expect(lerCopiasDoPainel("1000")).toBeNull();
    expect(lerCopiasDoPainel("")).toBeNull();
    expect(lerCopiasDoPainel("1,5")).toBeNull();
  });

  it("total e paginas selecionadas vezes copias", () => {
    expect(totalDePaginas(1, 2)).toBe(2);
    expect(totalDePaginas(4, 1)).toBe(4);
    expect(totalDePaginas(12, 12)).toBe(144);
    expect(totalDePaginas(10, 5)).toBe(50);
  });

  it("total recusa copias fora da faixa e paginas fora do limite", () => {
    expect(totalDePaginas(1, 0)).toBeNull();
    expect(totalDePaginas(1, 1000)).toBeNull();
    expect(totalDePaginas(1, 1.5)).toBeNull();
    expect(totalDePaginas(-1, 1)).toBeNull();
    expect(totalDePaginas(100_001, 1)).toBeNull();
  });
});

describe("numeros e rotulos", () => {
  it("formata mL, porcentagem e moeda no idioma pedido", () => {
    expect(formatarMl(0.34, "pt-BR")).toBe("0,34 mL");
    expect(formatarPercentual(10, "pt-BR")).toBe("10%");
    const reais = formatarMoeda(1.12, "BRL", "pt-BR");
    expect(reais).toContain("R$");
    expect(reais).toContain("1,12");
    expect(formatarMoeda(1.12, "BRL", "en")).toContain("1.12");
  });

  it("reconhece A4 nas duas orientacoes e mostra a medida quando nao conhece", () => {
    expect(rotuloDoPapel(210, 297, "pt-BR")).toBe("A4");
    expect(rotuloDoPapel(297, 210, "pt-BR")).toBe("A4");
    expect(rotuloDoPapel(215.9, 279.4, "pt-BR")).toBe("Carta");
    expect(rotuloDoPapel(100, 100, "pt-BR")).toBe("100 × 100 mm");
  });

  it("o resumo da barra leva o total com copias, os mL e o custo da selecao", () => {
    const resultado = normalizarResultado(resultadoCru(), "job-1") as ResultadoDaAnalise;
    expect(resumoDaBarra(resultado)).toEqual({
      totalPaginas: 4,
      ml: 0.4,
      valor: 0.4,
      moeda: "BRL",
    });
  });
});

describe("estado dos perfis", () => {
  it("mantem os perfis validos e descarta os invalidos, com o aviso do arquivo", () => {
    const estado = normalizarEstadoDosPerfis({
      perfis: [perfilCru(), perfilCru({ id: "" })],
      aviso: "perfil_ignorado",
    });
    expect(estado.perfis.map((perfil) => perfil.id)).toEqual(["tanque-generico"]);
    expect(estado.aviso).toBe("perfil_ignorado");
  });
});

