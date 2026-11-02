import { describe, expect, it } from "vitest";
import {
  caminhoDoAviso,
  confirmarLote,
  conteudoAbaixoDoLote,
  contarLotes,
  estadoDoBotao,
  impressorasValidas,
  interpretarLote,
  liberaMenuDeContexto,
  LOTE_MAXIMO,
  LOTE_MINIMO,
  LOTE_PADRAO,
  resumoDoLote,
  textoDoEstado,
  type Impressora,
} from "./impressoras";

function impressora(id: string): Impressora {
  return {
    id,
    nome: id,
    tipo: "fila",
    nivel: "pronta",
    condicoes: [],
    modo: "sem_perfil",
  };
}

describe("campo de lote", () => {
  it("usa 10 folhas como padrao, dentro de 1 a 50", () => {
    expect([LOTE_MINIMO, LOTE_PADRAO, LOTE_MAXIMO]).toEqual([1, 10, 50]);
  });

  it("aceita enquanto digita so inteiros dentro do intervalo", () => {
    expect(interpretarLote("1")).toBe(1);
    expect(interpretarLote("50")).toBe(50);
    expect(interpretarLote(" 25 ")).toBe(25);
    for (const texto of ["", "0", "51", "-3", "1e1", "10.5", "abc", "٣", "99999999999999999999999"]) {
      expect(interpretarLote(texto), texto).toBeNull();
    }
  });

  it("ao confirmar leva ao limite mais proximo ou volta ao valor atual", () => {
    expect(confirmarLote("0", 10)).toBe(1);
    expect(confirmarLote("51", 10)).toBe(50);
    expect(confirmarLote("99999999999999999999999", 10)).toBe(50);
    expect(confirmarLote("", 7)).toBe(7);
    expect(confirmarLote("abc", 7)).toBe(7);
    expect(confirmarLote("-3", 7)).toBe(7);
    expect(confirmarLote("23", 7)).toBe(23);
  });
});

describe("resumo do lote", () => {
  it("conta folhas de duas paginas, nao paginas", () => {
    expect(contarLotes(24, 10)).toBe(2);
    expect(contarLotes(20, 10)).toBe(1);
    expect(contarLotes(21, 10)).toBe(2);
    expect(contarLotes(1, 10)).toBe(1);
    expect(contarLotes(100_000, 1)).toBe(50_000);
  });

  it("recusa numeros sem sentido sem inventar um total", () => {
    expect(contarLotes(0, 10)).toBe(0);
    expect(contarLotes(10, 0)).toBe(0);
    expect(contarLotes(Number.NaN, 10)).toBe(0);
  });

  it("escreve o resumo nos dois idiomas, com singular", () => {
    expect(resumoDoLote(24, 10, "pt-BR")).toBe("24 pág · 2 lotes");
    expect(resumoDoLote(1, 10, "pt-BR")).toBe("1 pág · 1 lote");
    expect(resumoDoLote(24, 10, "en")).toBe("24 pages · 2 batches");
    expect(resumoDoLote(1, 10, "en")).toBe("1 page · 1 batch");
  });
});

describe("botao de imprimir", () => {
  it("fica desabilitado em qualquer contagem e diz por que", () => {
    for (const marcadas of [0, 1, 2, 50]) {
      const botao = estadoDoBotao(marcadas, "pt-BR");
      expect(botao.desabilitado).toBe(true);
      expect(botao.titulo).toContain("ainda não está disponível");
    }
  });

  it("mostra a contagem so quando ha impressora marcada", () => {
    expect(estadoDoBotao(0, "pt-BR").rotulo).toBe("Imprimir frente");
    expect(estadoDoBotao(2, "pt-BR").rotulo).toBe("Imprimir frente (2)");
    expect(estadoDoBotao(2, "en").rotulo).toBe("Print front (2)");
  });

  it("nao conta fila marcada que saiu da lista", () => {
    const lista = [impressora("fila:A"), impressora("simulada:traseira")];
    expect(impressorasValidas(["fila:A", "fila:Sumiu"], lista)).toEqual(["fila:A"]);
    expect(impressorasValidas([], lista)).toEqual([]);
  });
});

describe("estado da impressora", () => {
  it("traduz as condicoes e mantem o codigo das desconhecidas", () => {
    const pronta = impressora("fila:A");
    expect(textoDoEstado(pronta, "pt-BR")).toBe("Pronta");
    expect(textoDoEstado(pronta, "en")).toBe("Ready");
    const varias: Impressora = {
      ...pronta,
      condicoes: [{ codigo: "offline" }, { codigo: "sem_papel" }],
    };
    expect(textoDoEstado(varias, "pt-BR")).toBe("Offline, sem papel");
    expect(textoDoEstado(varias, "en")).toBe("Offline, out of paper");
    const outra: Impressora = {
      ...pronta,
      condicoes: [{ codigo: "outro", detalhe: "0x08000000" }],
    };
    expect(textoDoEstado(outra, "pt-BR")).toContain("0x08000000");
    const alimentacao: Impressora = { ...pronta, condicoes: [{ codigo: "alimentacao_manual" }] };
    expect(textoDoEstado(alimentacao, "pt-BR")).toBe("Alimentação manual");
    expect(textoDoEstado(alimentacao, "en")).toBe("Manual feed");
    const futura: Impressora = { ...pronta, condicoes: [{ codigo: "algo_novo" }] };
    expect(textoDoEstado(futura, "en")).toBe("Unknown state");
  });
});

describe("menu de contexto", () => {
  const alvo = (achou: boolean) => ({ closest: () => (achou ? {} : null) });

  it("so aparece onde a interface pediu", () => {
    expect(liberaMenuDeContexto(alvo(true))).toBe(true);
    expect(liberaMenuDeContexto(alvo(false))).toBe(false);
    expect(liberaMenuDeContexto(null)).toBe(false);
    expect(liberaMenuDeContexto({})).toBe(false);
  });
});

describe("caminho no aviso", () => {
  const caminho = "C:\\Users\\a\\plots\\perfis.toml";

  it.each(["perfis_invalido", "perfis_grande", "perfis_leitura"])(
    "%s mostra o arquivo sobre o qual o texto manda agir",
    (codigo) => {
      expect(caminhoDoAviso({ codigo, caminho })).toBe(caminho);
    },
  );

  it.each(["perfis_versao", "perfis_pasta", "filas"])(
    "%s nao manda agir no arquivo e nao mostra caminho",
    (codigo) => {
      expect(caminhoDoAviso({ codigo, caminho })).toBeNull();
    },
  );

  it("sem caminho conhecido nao mostra nada", () => {
    expect(caminhoDoAviso({ codigo: "perfis_invalido" })).toBeNull();
    expect(caminhoDoAviso({ codigo: "perfis_invalido", caminho: "" })).toBeNull();
  });
});

describe("abaixo do campo de lote", () => {
  it("o erro ocupa o lugar do resumo, qualquer que seja o estado do PDF", () => {
    expect(conteudoAbaixoDoLote(true, { estado: "conhecido", total: 24 })).toBe("erro");
    expect(conteudoAbaixoDoLote(true, { estado: "contando" })).toBe("erro");
    expect(conteudoAbaixoDoLote(true, { estado: "recusado", motivo: "x" })).toBe("erro");
  });

  it("sem erro mostra o resumo da aba, e sem aba nao mostra nada", () => {
    expect(conteudoAbaixoDoLote(false, { estado: "conhecido", total: 24 })).toBe("resumo");
    expect(conteudoAbaixoDoLote(false, null)).toBe("nada");
  });
});
