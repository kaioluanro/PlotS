import { describe, expect, it } from "vitest";
import { EXTENSOES_DE_DOCUMENTO, filtroDoSeletor } from "./backend";
import { t, traducoes } from "./i18n";
import { estadoInicial, reduzirAbas } from "./tabs";

describe("abertura de documentos", () => {
  it("o seletor oferece PDF e todas as imagens aceitas", () => {
    expect([...EXTENSOES_DE_DOCUMENTO]).toEqual([
      "pdf",
      "png",
      "jpg",
      "jpeg",
      "bmp",
      "gif",
      "tif",
      "tiff",
      "webp",
    ]);
  });

  it("o filtro usa a lista inteira e o nome traduzido", () => {
    const filtro = filtroDoSeletor();
    expect(filtro.extensions).toEqual([...EXTENSOES_DE_DOCUMENTO]);
    expect(filtro.name).toBe(t("seletor.filtro"));
    expect(traducoes["pt-BR"]["seletor.filtro"]).toBe("PDF e imagens");
    expect(traducoes.en["seletor.filtro"]).toBe("PDF and images");
  });

  it("a aba guarda o tipo do documento", () => {
    const estado = reduzirAbas(estadoInicial, {
      tipo: "abrir",
      documento: { caminho: "faixas.png", nome: "faixas.png", tipo: "imagem" },
    });
    expect(estado.abas[0]?.documento.tipo).toBe("imagem");
  });

  it("o estado vazio e o botao mencionam imagem nos dois idiomas", () => {
    expect(traducoes["pt-BR"]["documento.vazioTexto"]).toContain("imagem");
    expect(traducoes.en["documento.vazioTexto"]).toContain("image");
    expect(traducoes["pt-BR"]["documento.vazioTitulo"]).toBe("Nenhum documento aberto");
    expect(traducoes.en["documento.vazioTitulo"]).toBe("No document open");
    expect(traducoes["pt-BR"]["documento.abrir"]).toBe("Abrir arquivo");
    expect(traducoes.en["documento.abrir"]).toBe("Open file");
  });

  it("os textos gerais falam em documento, nao em PDF", () => {
    const chaves = [
      "barra.abrir",
      "lateral.semDocumento",
      "folhas.recusado",
      "visualizador.carregando",
      "visualizador.erro.texto",
      "cartao.botao.semDocumento",
      "cartao.botao.recusado",
      "intervalo.fechado.semDocumento",
      "intervalo.fechado.recusado",
    ] as const;
    for (const idioma of ["pt-BR", "en"] as const) {
      for (const chave of chaves) {
        expect(traducoes[idioma][chave].toLowerCase(), `${idioma}:${chave}`).not.toContain(
          "pdf",
        );
      }
    }
    expect(traducoes["pt-BR"]["barra.abrir"]).toBe("Abrir arquivo");
    expect(traducoes.en["barra.abrir"]).toBe("Open file");
    expect(traducoes["pt-BR"]["folhas.recusado"]).toBe("Documento recusado");
    expect(traducoes.en["folhas.recusado"]).toBe("Document refused");
  });

  it("o PDF continua nomeado onde ele e a condicao real", () => {
    expect(traducoes["pt-BR"]["erro.pdf_invalido"]).toContain("PDF");
    expect(traducoes.en["erro.pdf_invalido"]).toContain("PDF");
    expect(traducoes["pt-BR"]["erro.criptografado"]).toContain("PDF");
    expect(traducoes.en["erro.criptografado"]).toContain("PDF");
    expect(traducoes["pt-BR"]["cartao.envio.assinado"]).toContain("assinatura digital");
    expect(traducoes.en["cartao.envio.assinado"]).toContain("digital signature");
  });

  it("a imagem invalida orienta a recuperar o arquivo nos dois idiomas", () => {
    expect(traducoes["pt-BR"]["erro.imagem_invalida"]).toContain(
      "Salve uma nova cópia pelo programa de origem ou reduza a imagem e abra de novo.",
    );
    expect(traducoes.en["erro.imagem_invalida"]).toContain(
      "Save a new copy from the original program or reduce the image and open it again.",
    );
    expect(traducoes["pt-BR"]["erro.imagem_invalida"]).toContain("10 mil quadros");
    expect(traducoes.en["erro.imagem_invalida"]).toContain("10,000 frames");
  });
});
