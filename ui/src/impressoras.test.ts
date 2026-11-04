import { describe, expect, it } from "vitest";
import {
  caminhoDoAviso,
  liberaMenuDeContexto,
  contarFolhas,
  resumoDoDocumento,
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

describe("resumo do documento", () => {
  it("conta folhas de duas paginas, nos dois idiomas e com singular", () => {
    expect(contarFolhas(24)).toBe(12);
    expect(contarFolhas(23)).toBe(12);
    expect(resumoDoDocumento(24, "pt-BR")).toBe("24 pág · 12 folhas");
    expect(resumoDoDocumento(1, "en")).toBe("1 page · 1 sheet");
  });

  it("com o plano em maos, o rodape usa as folhas do plano e nao a metade das paginas", () => {
    // Papel misto: 10 paginas, a 5 de outro tamanho, saem em 6 folhas.
    expect(resumoDoDocumento(10, "pt-BR", 1, 6)).toBe("10 pág · 6 folhas");
    expect(resumoDoDocumento(10, "en", 2, 6)).toBe("10 pages × 2 · 12 sheets");
    expect(resumoDoDocumento(10, "pt-BR")).toBe("10 pág · 5 folhas");
  });

  it("numero sem sentido nao inventa folhas", () => {
    expect(contarFolhas(0)).toBe(0);
    expect(contarFolhas(-3)).toBe(0);
    expect(contarFolhas(1.5)).toBe(0);
    expect(contarFolhas(Number.NaN)).toBe(0);
  });
});
