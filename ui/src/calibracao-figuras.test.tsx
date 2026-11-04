import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { OPCOES, type FiguraDaResposta, type Pergunta } from "./calibracao";
import { Respostas } from "./components/CalibracaoDialog";
import { CalibracaoFigura } from "./components/CalibracaoFigura";

const PERGUNTAS = Object.keys(OPCOES) as Pergunta[];

function desenho(figura: FiguraDaResposta): string {
  return renderToStaticMarkup(<CalibracaoFigura figura={figura} />);
}

/** O que sobra do desenho sem as classes: duas respostas nao podem diferir so no estilo. */
function forma(figura: FiguraDaResposta): string {
  return desenho(figura).replace(/ class="[^"]*"/g, "");
}

describe("cada resposta na tela", () => {
  const sem = () => undefined;
  const tela = (pergunta: Pergunta, valor?: string) =>
    renderToStaticMarkup(<Respostas pergunta={pergunta} valor={valor} aoResponder={sem} aoConfirmar={sem} />);

  it.each(PERGUNTAS)("%s junta figura, escolha e frase no mesmo rotulo clicavel", (pergunta) => {
    const html = tela(pergunta);
    const rotulos = [...html.matchAll(/<label.*?<\/label>/g)].map((achado) => achado[0]);
    expect(rotulos).toHaveLength(OPCOES[pergunta].length);
    OPCOES[pergunta].forEach((opcao, indice) => {
      const rotulo = rotulos[indice] ?? "";
      expect(rotulo).toContain(`data-figura="${opcao.figura}"`);
      expect(rotulo).toContain('type="radio"');
      expect(rotulo).toContain(`value="${opcao.valor}"`);
    });
    expect(html).toContain('role="radiogroup"');
    expect(html).toContain("aria-labelledby");
  });

  it("a escolhida fica marcada na propria caixa de escolha, nao so na cor", () => {
    const html = tela("par", "nao");
    expect(html.match(/checked=""/g)).toHaveLength(1);
    expect(html).toMatch(/value="nao" checked=""|checked="" value="nao"/);
  });
});

describe("figuras das respostas", () => {
  it.each(PERGUNTAS)("%s tem uma figura desenhada para cada resposta", (pergunta) => {
    for (const opcao of OPCOES[pergunta]) {
      const html = desenho(opcao.figura);
      expect(html).toContain(`data-figura="${opcao.figura}"`);
      expect(html).toMatch(/<(rect|path)/);
    }
  });

  it.each(PERGUNTAS)("%s tem respostas que diferem na forma, nao so na cor", (pergunta) => {
    const formas = OPCOES[pergunta].map((opcao) => forma(opcao.figura));
    expect(new Set(formas).size).toBe(formas.length);
  });

  it("todas sao decorativas para o leitor de tela", () => {
    for (const pergunta of PERGUNTAS) {
      for (const opcao of OPCOES[pergunta]) {
        const html = desenho(opcao.figura);
        expect(html).toContain('aria-hidden="true"');
        expect(html).not.toContain("<title");
      }
    }
  });

  it("nenhuma tem cor propria: tudo vem dos tokens da interface", () => {
    for (const pergunta of PERGUNTAS) {
      for (const opcao of OPCOES[pergunta]) {
        expect(desenho(opcao.figura)).not.toMatch(/#[0-9a-f]{3,8}\b|rgb\(/i);
      }
    }
  });

  it("cada figura mostra o que a resposta diz", () => {
    expect(desenho("par_certo")).toContain(">2<");
    expect(desenho("par_errado")).toContain(">4<");
    expect(desenho("par_certo")).toContain('data-selo="certo"');
    expect(desenho("par_errado")).toContain('data-selo="errado"');
    expect(desenho("setas_iguais").match(/data-seta="cima"/g)).toHaveLength(2);
    expect(desenho("setas_opostas")).toContain('data-seta="baixo"');
    expect(desenho("setas_iguais")).toContain('data-gesto="borda_longa"');
    expect(desenho("lado_por_cima").match(/<text/g)).toHaveLength(2);
    expect(desenho("leitura_sem_virar")).not.toContain('data-gesto="virar"');
    expect(desenho("leitura_virando")).toContain('data-gesto="virar"');
    expect(desenho("saida_escondida")).toContain('data-marca="canto"');
    expect(desenho("saida_escondida")).not.toContain("<text");
    expect(desenho("primeira_nao_reparei")).toContain(">?<");
  });
});
