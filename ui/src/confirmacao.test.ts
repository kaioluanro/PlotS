import { describe, expect, it } from "vitest";
import { conteudoDaConfirmacao } from "./confirmacao";
import { traducoes } from "./i18n";
import { estadoInicial, reduzirAbas, type EstadoAbas } from "./tabs";

function comImpressao(...nomes: string[]): EstadoAbas {
  let estado = estadoInicial;
  for (const nome of nomes) {
    estado = reduzirAbas(estado, { tipo: "abrir", documento: { caminho: nome, nome } });
    estado = reduzirAbas(estado, { tipo: "marcar_impressao", id: estado.proximoId - 1, emAndamento: true });
  }
  return estado;
}

describe("conteudoDaConfirmacao", () => {
  it("sem pedido nao ha dialogo", () => {
    expect(conteudoDaConfirmacao(comImpressao("a.pdf"))).toBeNull();
  });

  it("ao fechar a aba usa o nome do documento e a consequencia", () => {
    const estado = comImpressao("a.pdf");
    const pedido = reduzirAbas(estado, { tipo: "pedir_fechamento", id: estado.ativa ?? -1 });
    const conteudo = conteudoDaConfirmacao(pedido);
    expect(conteudo?.titulo).toBe("confirmar.titulo");
    expect(conteudo?.texto).toBe("confirmar.texto");
    expect(conteudo?.parametros).toEqual({ nome: "a.pdf" });
    expect(conteudo?.consequencia).toBe("confirmar.consequencia");
    expect(conteudo?.manter).toBe("confirmar.manter");
    expect(conteudo?.ehJanela).toBe(false);
  });

  it("ao fechar a janela com um documento usa o nome dele", () => {
    const pedido = reduzirAbas(comImpressao("a.pdf"), { tipo: "pedir_fechamento_janela" });
    const conteudo = conteudoDaConfirmacao(pedido);
    expect(conteudo?.titulo).toBe("confirmar.janela.titulo");
    expect(conteudo?.texto).toBe("confirmar.texto");
    expect(conteudo?.parametros).toEqual({ nome: "a.pdf" });
    expect(conteudo?.consequencia).toBe("confirmar.consequencia");
    expect(conteudo?.manter).toBe("confirmar.janela.manter");
    expect(conteudo?.ehJanela).toBe(true);
  });

  it("ao fechar a janela com varios documentos usa a contagem", () => {
    const pedido = reduzirAbas(comImpressao("a.pdf", "b.pdf", "c.pdf"), { tipo: "pedir_fechamento_janela" });
    const conteudo = conteudoDaConfirmacao(pedido);
    expect(conteudo?.texto).toBe("confirmar.janela.textoVarios");
    expect(conteudo?.parametros).toEqual({ n: "3" });
    expect(conteudo?.consequencia).toBe("confirmar.consequenciaVarios");
  });
});

describe("textos da confirmacao", () => {
  it("a consequencia esta escrita nos dois idiomas", () => {
    expect(traducoes["pt-BR"]["confirmar.consequencia"]).toBe(
      "O que já foi enviado continua na fila, mas o PlotS deixa de acompanhar este documento e o verso não poderá ser impresso por aqui.",
    );
    expect(traducoes.en["confirmar.consequencia"]).toBe(
      "Anything already sent stays in the queue, but PlotS stops tracking this document and the back side can no longer be printed from here.",
    );
  });

  it("nomeia o documento em ingles do jeito pedido", () => {
    expect(traducoes.en["confirmar.texto"]).toBe("{nome} is still printing.");
  });

  it("a acao segura da janela e nomeada nos dois idiomas", () => {
    expect(traducoes["pt-BR"]["confirmar.janela.manter"]).toBe("Manter janela aberta");
    expect(traducoes["pt-BR"]["confirmar.janela.titulo"]).toBe("Fechar janela com impressão em andamento?");
  });
});
