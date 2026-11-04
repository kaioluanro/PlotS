import { describe, expect, it } from "vitest";
import { notaValeNaSelecao, selecaoCoincide, textoDaFalhaDoGrupo, textoDasAusentes, textoDosIgnorados } from "./grupos";
import { traducoes, traduzir } from "./i18n";

// Os CSS de modulo nao voltam como texto sob o teste: a folha e lida do disco.
interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;

describe("textos dos grupos", () => {
  it("cada recusa do backend tem a sua frase", () => {
    const frases = [
      "nome_vazio",
      "nome_grande",
      "nome_invalido",
      "nome_repetido",
      "sem_impressoras",
      "muitas_impressoras",
      "muitos_grupos",
      "inexistente",
      "leitura",
      "gravacao",
    ].map((codigo) => textoDaFalhaDoGrupo(codigo, "en"));
    expect(new Set(frases).size).toBe(frases.length);
    expect(textoDaFalhaDoGrupo("nome_repetido", "pt-BR")).toBe(
      traducoes["pt-BR"]["grupo.erro.nomeRepetido"],
    );
  });

  it("o que vier de fora nunca aparece como veio", () => {
    const hostil = "<img src=x> C:\\Users\\alguem\\perfis.toml";
    for (const falha of [hostil, 7, null, undefined, { codigo: "leitura" }]) {
      expect(textoDaFalhaDoGrupo(falha, "pt-BR")).toBe(traduzir("pt-BR", "grupo.erro.gravacao"));
    }
  });

  it("diz quais impressoras do grupo nao existem mais", () => {
    expect(textoDasAusentes([], "pt-BR")).toBeNull();
    expect(textoDasAusentes(["Sala", "Copa"], "pt-BR")).toBe(
      "Não existem mais e foram ignoradas: Sala, Copa.",
    );
    expect(textoDasAusentes(["Sala"], "en")).toBe("No longer exist and were ignored: Sala.");
  });
});

describe("grupo escolhido na lista", () => {
  const aplicado = { impressoras: ["a", "b"] };

  it("continua escolhido com a mesma selecao, em qualquer ordem", () => {
    expect(selecaoCoincide(aplicado, ["b", "a"])).toBe(true);
  });

  it("volta ao convite se a selecao mudar", () => {
    expect(selecaoCoincide(aplicado, ["a"])).toBe(false);
    expect(selecaoCoincide(aplicado, ["a", "c"])).toBe(false);
  });
});

describe("grupos ignorados do arquivo", () => {
  it("avisa so a contagem", () => {
    expect(textoDosIgnorados(0, "pt-BR")).toBeNull();
    expect(textoDosIgnorados(1, "en")).toBe(
      "1 saved group could not be read and was ignored. It stays in the file.",
    );
    expect(textoDosIgnorados(3, "pt-BR")).toContain("3 grupos salvos");
  });
});

describe("notas da linha de grupos", () => {
  const nota = { texto: "x", aba: 1, impressoras: ["a", "b"] };
  const css = fs
    .readFileSync(new URL("./components/BlocoGrupos.module.css", import.meta.url), "utf8")
    .replaceAll("\r\n", "\n");

  it("valem na mesma aba e na mesma selecao, em qualquer ordem", () => {
    expect(notaValeNaSelecao(nota, 1, ["b", "a"])).toBe(true);
  });

  it("deixam de valer em outra aba, com outra marcacao ou sem nota", () => {
    expect(notaValeNaSelecao(nota, 2, ["a", "b"])).toBe(false);
    expect(notaValeNaSelecao(nota, 1, ["a"])).toBe(false);
    expect(notaValeNaSelecao(nota, 1, ["a", "c"])).toBe(false);
    expect(notaValeNaSelecao(null, 1, ["a", "b"])).toBe(false);
  });

  it("o anuncio de apagado aparece so acima de 560 px e sem texto nao ocupa lugar", () => {
    const curto = css.slice(css.lastIndexOf("@media (max-height: 560px)"));
    expect(curto).toMatch(/\.anuncio\s*\{[^}]*position: absolute;[^}]*clip-path: inset\(50%\);/);
    expect(css).toMatch(/\.anuncio:empty\s*\{[^}]*position: absolute;[^}]*clip-path: inset\(50%\);/);
  });

  it("o botao de salvar sem grupos tem o tamanho das acoes do nome, sem perder para o botao comum", () => {
    const regra = /\.grupos \.salvarGrupo\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(regra).toContain("min-height: 28px");
    expect(regra).toContain("font-size: 11px");
    expect(regra).toContain("border-radius: var(--radius-sm)");
    const acao = /\.acoesDoNome \.acao\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(acao).toContain("min-height: 28px");
  });
});
