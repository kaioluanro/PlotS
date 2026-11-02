import { describe, expect, it } from "vitest";
import { t, traduzir } from "./i18n";
import { abrirLote, mensagemDoLote } from "./lote";

const recusa = (nome: string) => ({ codigo: "nao_encontrado", nome });

describe("mensagemDoLote", () => {
  it("sem falha e sem corte nao ha mensagem", () => {
    expect(mensagemDoLote([], false)).toBeNull();
  });

  it("uma falha mostra a mensagem dela", () => {
    expect(mensagemDoLote([recusa("a.pdf")], false)).toBe(t("erro.nao_encontrado", { nome: "a.pdf" }));
  });

  it("varias falhas mostram a contagem", () => {
    expect(mensagemDoLote([recusa("a.pdf"), recusa("b.pdf"), recusa("c.pdf")], false)).toBe(
      t("erro.varios", { n: "3" }),
    );
  });

  it("avisa do corte, sozinho ou junto das falhas", () => {
    expect(mensagemDoLote([], true)).toBe(t("lote.cortado"));
    const junto = mensagemDoLote([recusa("a.pdf")], true);
    expect(junto).toContain(t("erro.nao_encontrado", { nome: "a.pdf" }));
    expect(junto).toContain(t("lote.cortado"));
  });
});

describe("texto das varias falhas junto do aviso de corte", () => {
  it("em portugues, cada frase termina no proprio ponto", () => {
    const combinado = `${traduzir("pt-BR", "erro.varios", { n: "3" })} ${traduzir("pt-BR", "lote.cortado")}`;
    expect(combinado).toBe("3 arquivos não abriram. Só os primeiros 32 arquivos foram abertos.");
  });

  it("em ingles, cada frase termina no proprio ponto", () => {
    const combinado = `${traduzir("en", "erro.varios", { n: "3" })} ${traduzir("en", "lote.cortado")}`;
    expect(combinado).toBe("3 files did not open. Only the first 32 files were opened.");
  });

  it("a mensagem do lote junta as duas frases no idioma atual", () => {
    expect(mensagemDoLote([recusa("a.pdf"), recusa("b.pdf"), recusa("c.pdf")], true)).toBe(
      `${t("erro.varios", { n: "3" })} ${t("lote.cortado")}`,
    );
  });
});

describe("abrirLote", () => {
  const abrir = async (caminho: string) => {
    if (caminho.includes("ruim")) throw recusa(caminho);
    return { caminho, nome: caminho };
  };

  it("um sucesso depois de uma falha nao apaga o erro", async () => {
    const abertos: string[] = [];
    const erro = await abrirLote(
      { caminhos: ["ruim.pdf", "bom.pdf"], cortado: false },
      abrir,
      (documento) => abertos.push(documento.nome),
    );
    expect(abertos).toEqual(["bom.pdf"]);
    expect(erro).toBe(t("erro.nao_encontrado", { nome: "ruim.pdf" }));
  });

  it("abre na ordem recebida e conta todas as falhas", async () => {
    const abertos: string[] = [];
    const erro = await abrirLote(
      { caminhos: ["a.pdf", "ruim1.pdf", "b.pdf", "ruim2.pdf"], cortado: false },
      abrir,
      (documento) => abertos.push(documento.nome),
    );
    expect(abertos).toEqual(["a.pdf", "b.pdf"]);
    expect(erro).toBe(t("erro.varios", { n: "2" }));
  });

  it("lote limpo nao deixa mensagem", async () => {
    expect(await abrirLote({ caminhos: ["a.pdf"], cortado: false }, abrir, () => undefined)).toBeNull();
  });
});
