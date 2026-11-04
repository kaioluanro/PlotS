import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { EstadoDoEnvio } from "./cartoes";
import { GrupoDeEnvio } from "./components/GrupoDeEnvio";
import { traduzir } from "./i18n";

interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;

const ESTADO: EstadoDoEnvio = {
  rotulo: "Imprimir frentes em 2",
  motivo: null,
  desabilitado: false,
  destinos: [],
  rotuloAcessivel: "Imprimir frentes em 2 impressoras",
};

function grupo(mudancas: { copiasValidas?: boolean; botao?: Partial<EstadoDoEnvio> }) {
  return renderToStaticMarkup(
    <GrupoDeEnvio
      blocoFace={<p>bloco face</p>}
      campoCopias={<p>campo copias</p>}
      botao={{ ...ESTADO, ...mudancas.botao }}
      copiasValidas={mudancas.copiasValidas ?? true}
      aoImprimir={() => undefined}
    />,
  );
}

describe("grupo de envio: quantidade invalida", () => {
  it("desabilita o envio e diz o que corrigir, no rodape, em pt-BR", () => {
    const markup = grupo({ copiasValidas: false });
    expect(markup).toMatch(/<button[^>]*aria-disabled="true"/);
    expect(markup).not.toMatch(/<button[^>]*\sdisabled=""/);
    expect(markup).toContain(traduzir("pt-BR", "copias.invalida"));
    expect(markup).toContain('role="alert"');
  });

  it("o erro substitui o motivo que estaria embaixo do botao", () => {
    const markup = grupo({ copiasValidas: false, botao: { motivo: "Motivo que nao deve aparecer" } });
    expect(markup).not.toContain("Motivo que nao deve aparecer");
    expect(markup).toContain(traduzir("pt-BR", "copias.invalida"));
  });

  it("com a quantidade valida, o envio segue o estado dele e nao mostra erro", () => {
    const markup = grupo({ copiasValidas: true });
    expect(markup).not.toMatch(/<button[^>]*aria-disabled/);
    expect(markup).not.toContain('role="alert"');
  });
});

describe("grupo de envio: envio em andamento", () => {
  it("fica inerte mas focavel, para o foco voltar a ele depois de confirmar pelo teclado", () => {
    const markup = grupo({
      botao: { desabilitado: true, motivo: traduzir("pt-BR", "cartao.botao.emAndamento") },
    });
    expect(markup).toMatch(/<button[^>]*aria-disabled="true"/);
    expect(markup).not.toMatch(/<button[^>]*\sdisabled=""/);
    expect(markup).toContain("Em andamento. Veja os avisos.");
  });
});

describe("grupo de envio: nome por extenso", () => {
  it("o botao leva o nome com a contagem por extenso, e o rotulo visivel continua", () => {
    const markup = grupo({});
    expect(markup).toContain('aria-label="Imprimir frentes em 2 impressoras"');
    expect(markup).toContain("Imprimir frentes em 2<");
  });
});

describe("campo de quantidade: entrada sem sair com texto velho", () => {
  it("nao restaura o valor ao sair do campo", async () => {
    const fonte = fs.readFileSync(new URL("./components/CampoCopias.tsx", import.meta.url), "utf8");
    expect(fonte).not.toContain("aoSair");
    expect(fonte).not.toContain("onBlur");
  });

  it("o erro do campo nao aparece dentro dele: o texto fica no rodape", () => {
    const fonte = fs.readFileSync(new URL("./components/CampoCopias.tsx", import.meta.url), "utf8");
    expect(fonte).not.toContain("role=\"alert\"");
  });
});
