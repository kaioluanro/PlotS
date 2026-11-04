import { describe, expect, it } from "vitest";
import type { Cartao } from "./cartoes";
import { conteudoDaConfirmacao, conteudoDoCancelamento, conteudoDoEnvio } from "./confirmacao";
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
      "O que já foi enviado continua na fila de impressão, mas o PlotS deixa de acompanhar este documento.",
    );
    expect(traducoes.en["confirmar.consequencia"]).toBe(
      "Anything already sent stays in the print queue, but PlotS stops tracking this document.",
    );
  });

  it("a nota dos versos diz o que acontece ao fechar, sem contradizer a consequencia", () => {
    expect(traducoes["pt-BR"]["face.fechar.versos"]).toBe(
      "Se fechar agora, o acompanhamento para. Depois, abra o mesmo arquivo e escolha Verso. O PlotS não confere se ele mudou.",
    );
    expect(traducoes.en["face.fechar.versos"]).toBe(
      "If you close now, tracking stops. Later, open the same file and choose Back. PlotS does not check whether it changed.",
    );
  });

  it("nenhuma consequencia de fechar diz que o verso nao sai por aqui", () => {
    for (const idioma of ["pt-BR", "en"] as const) {
      for (const chave of [
        "confirmar.consequencia",
        "confirmar.consequenciaVarios",
        "cartao.fechar.consequenciaEnviando",
      ] as const) {
        expect(traducoes[idioma][chave]).not.toMatch(/não poderá|can no longer/);
      }
    }
  });

  it("nomeia o documento em ingles do jeito pedido", () => {
    expect(traducoes.en["confirmar.texto"]).toBe("The print of {nome} has not finished.");
  });

  it("a acao segura da janela e nomeada nos dois idiomas", () => {
    expect(traducoes["pt-BR"]["confirmar.janela.manter"]).toBe("Manter janela aberta");
    expect(traducoes["pt-BR"]["confirmar.janela.titulo"]).toBe("Fechar janela com impressão em andamento?");
  });
});

describe("texto do fechamento segundo o estado real", () => {
  const trabalho = (simulada: boolean, tipo: "aguardando" | "imprimindo", aba = 1): Cartao => ({
    aba,
    impressora: simulada ? "simulada:gaveta" : "fila:A",
    nome: "A",
    simulada,
    lote: 1,
    total: 1,
    papel: null,
    estado: tipo === "imprimindo" ? { tipo, passada: "frente" } : { tipo },
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
  });

  function comTrabalhos(...cartoes: Cartao[]): EstadoAbas {
    let estado = reduzirAbas(estadoInicial, {
      tipo: "abrir",
      documento: { caminho: "C:\\a.pdf", nome: "a.pdf" },
    });
    for (const cartao of cartoes) estado = reduzirAbas(estado, { tipo: "cartao", cartao });
    return estado;
  }

  it("so simulacao nao fala em fila, nem na aba nem na janela", () => {
    const estado = comTrabalhos(trabalho(true, "aguardando"));
    const aba = conteudoDaConfirmacao(reduzirAbas(estado, { tipo: "pedir_fechamento", id: 1 }));
    expect(aba?.consequencia).toBe("cartao.fechar.consequenciaSimulacao");
    expect(aba?.titulo).toBe("cartao.fechar.tituloSimulacao");
    const janela = conteudoDaConfirmacao(reduzirAbas(estado, { tipo: "pedir_fechamento_janela" }));
    expect(janela?.consequencia).toBe("cartao.fechar.consequenciaSimulacao");
    expect(janela?.titulo).toBe("cartao.fechar.janelaTituloSimulacao");
  });

  it("trabalho real, mesmo junto de simulacao, continua falando da fila", () => {
    const estado = reduzirAbas(
      comTrabalhos(trabalho(false, "aguardando"), trabalho(true, "aguardando")),
      { tipo: "pedir_fechamento", id: 1 },
    );
    expect(conteudoDaConfirmacao(estado)?.consequencia).toBe("confirmar.consequencia");
  });

  it("fechar a janela com passada indo para a fila avisa que ela pode ficar incompleta", () => {
    const estado = reduzirAbas(comTrabalhos(trabalho(false, "imprimindo")), {
      tipo: "pedir_fechamento_janela",
    });
    expect(conteudoDaConfirmacao(estado)?.consequencia).toBe("cartao.fechar.consequenciaEnviando");
    expect(traducoes["pt-BR"]["cartao.fechar.consequenciaEnviando"]).toContain("incompleto");
    expect(traducoes.en["cartao.fechar.consequenciaEnviando"]).toContain("incomplete");
  });
});

describe("dialogos do envio e do cancelamento", () => {
  it("a confirmacao do envio avanca, e o cancelamento pede o tom de perigo", () => {
    const envio = conteudoDoEnvio(["Sala"], ["nota"]);
    expect(envio.confirmaAvancando).toBe(true);
    expect(envio.linhas).toEqual(["Sala"]);
    expect(envio.notas).toEqual(["nota"]);
    const cancelamento = conteudoDoCancelamento("Sala", false);
    expect(cancelamento.confirmaAvancando).toBeUndefined();
    expect(cancelamento.texto).toBe("cartao.cancelarDialogo.texto");
    expect(conteudoDoCancelamento("Sala", true).texto).toBe("cartao.cancelarDialogo.textoSimulado");
  });

  it("o trabalho que ja terminou diz que nada foi cancelado, nos dois idiomas", () => {
    expect(traducoes["pt-BR"]["cartao.cancelarDialogo.jaTerminou.texto"]).toBe(
      "Neste momento, não há trabalho de {nome} para cancelar. Nada foi cancelado.",
    );
    expect(traducoes.en["cartao.cancelarDialogo.jaTerminou.texto"]).toBe(
      "Right now, there is no job on {nome} to cancel. Nothing was canceled.",
    );
  });
});
