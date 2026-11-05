import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { ConteudoConfirmacao } from "./confirmacao";
import { TituloDoDialogo } from "./components/ConfirmDialog";

const CONTEUDO: ConteudoConfirmacao = {
  titulo: "face.envio.versoTitulo",
  texto: "face.envio.versoTexto",
  consequencia: null,
  manter: "cartao.envio.voltar",
  fechar: "face.envio.versoConfirmar",
  parametros: {},
  ehJanela: false,
  focoNaOrigemAoConfirmar: true,
};

describe("titulo do dialogo", () => {
  it("com atencao, o triangulo vem antes do texto e fica fora do nome acessivel", () => {
    const markup = renderToStaticMarkup(
      <TituloDoDialogo conteudo={{ ...CONTEUDO, atencao: true }} />,
    );
    expect(markup.startsWith("<svg")).toBe(true);
    expect(markup).toContain('aria-hidden="true"');
    expect(markup).toContain("Imprimir os versos?");
  });

  it("sem atencao, o titulo e so o texto", () => {
    const markup = renderToStaticMarkup(<TituloDoDialogo conteudo={CONTEUDO} />);
    expect(markup).not.toContain("<svg");
    expect(markup).toContain("Imprimir os versos?");
  });
});
