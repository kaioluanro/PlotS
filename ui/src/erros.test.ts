import { describe, expect, it } from "vitest";
import { mensagemDeErro } from "./erros";
import { t } from "./i18n";

describe("mensagemDeErro", () => {
  it("usa o texto de cada codigo, com o nome do arquivo", () => {
    for (const codigo of [
      "nao_encontrado",
      "nao_e_arquivo",
      "nao_suportado",
      "inacessivel",
      "pdf_invalido",
      "imagem_invalida",
      "criptografado",
      "grande",
    ] as const) {
      expect(mensagemDeErro({ codigo, nome: "a b.pdf" })).toBe(
        t(`erro.${codigo}`, { nome: "a b.pdf" }),
      );
    }
  });

  it("cai na mensagem generica quando o formato nao e o esperado", () => {
    const generica = t("erro.desconhecido");
    expect(mensagemDeErro("falhou")).toBe(generica);
    expect(mensagemDeErro(null)).toBe(generica);
    expect(mensagemDeErro({ codigo: "outro", nome: "a.pdf" })).toBe(generica);
    expect(mensagemDeErro({ codigo: "constructor", nome: "a.pdf" })).toBe(generica);
    expect(mensagemDeErro({ codigo: "nao_suportado", nome: 1 })).toBe(generica);
  });
});
