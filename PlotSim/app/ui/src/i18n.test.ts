import { describe, expect, it } from "vitest";
import { traducoes, traduzir, type Chave } from "./i18n";

// Os CSS de módulo não voltam como texto sob o teste: a folha é lida do disco.
interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;
const estilos = fs.readFileSync(new URL("./App.module.css", import.meta.url), "utf8");

const chaves = Object.keys(traducoes["pt-BR"]) as Chave[];

function marcadores(texto: string): string[] {
  return [...texto.matchAll(/\{(\w+)\}/g)].map((m) => m[1] ?? "").sort();
}

describe("contraste do metadado da linha", () => {
  it("na linha ativa ou sob o ponteiro, o modo usa o texto mais forte", () => {
    // O texto tênue fica abaixo de 4,5:1 sobre o fundo da linha ativa no tema escuro.
    expect(estilos).toMatch(/\.item:hover \.itemModo,\s*\.itemAtivo \.itemModo\s*\{\s*color: var\(--fg-muted\)/);
  });
});

describe("textos da janela", () => {
  it("os dois idiomas têm as mesmas chaves", () => {
    expect(Object.keys(traducoes.en).sort()).toEqual([...chaves].sort());
  });

  it("nenhum texto está vazio", () => {
    for (const chave of chaves) {
      expect(traducoes["pt-BR"][chave].trim(), chave).not.toBe("");
      expect(traducoes.en[chave].trim(), chave).not.toBe("");
    }
  });

  it("os dois idiomas usam os mesmos marcadores", () => {
    for (const chave of chaves) {
      expect(marcadores(traducoes.en[chave]), chave).toEqual(marcadores(traducoes["pt-BR"][chave]));
    }
  });

  it("os estados pedidos estão escritos como na especificação", () => {
    expect(traduzir("pt-BR", "estado.ociosa")).toBe("Ociosa");
    expect(traduzir("pt-BR", "estado.recebendo")).toBe("Recebendo");
    expect(traduzir("pt-BR", "estado.aguardando_verso")).toBe("Aguardando verso");
    expect(traduzir("pt-BR", "rodape.virar")).toBe("Virar folha");
    expect(traduzir("pt-BR", "rodape.recolocar")).toBe("Recolocar");
    expect(traduzir("pt-BR", "rodape.limpar")).toBe("Limpar bandeja");
    expect(traduzir("pt-BR", "rodape.resumo", { certas: "3", total: "3" })).toBe(
      "3 de 3 folhas corretas",
    );
  });

  it("marcador sem valor continua visível em vez de sumir", () => {
    expect(traduzir("en", "folha.titulo")).toBe("Sheet {n}");
  });
});
