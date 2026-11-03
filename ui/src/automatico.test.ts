import { describe, expect, it } from "vitest";
import { acoesDoCartao, cartaoEmAndamento, linhasDoCartao, linhasDoEnvio, precisaConfirmar, type Cartao } from "./cartoes";
import type { Impressora } from "./impressoras";
import { conteudoDoEnvio } from "./confirmacao";

const impressora: Impressora = { id: "simulada:automatica", nome: "Simulada — Automática", tipo: "simulada", nivel: "pronta", condicoes: [], modo: "automatico", duasFaces: true };
const cartao: Cartao = { aba: 1, impressora: impressora.id, nome: impressora.nome, simulada: true, lote: 1, total: 1, papel: null, estado: { tipo: "imprimindo", passada: "duas_faces" }, recolocar: null, trocaDePapel: null, resultado: null, sequencia: null, viraParaLer: false, loteAutomatico: true };

describe("fluxo automatico", () => {
  it("nao oferece verso durante o envio de duas faces", () => {
    expect(acoesDoCartao(cartao).verso.visivel).toBe(false);
    expect(linhasDoCartao(cartao, "pt-BR").map((linha) => linha.texto)).toContain("Imprimindo as duas faces automaticamente.");
  });

  it("a recusa do driver libera a impressora e orienta a escolha manual", () => {
    const recusado: Cartao = { ...cartao, estado: { tipo: "erro", motivo: { codigo: "duas_faces_recusadas", texto: "" } } };
    expect(cartaoEmAndamento(recusado)).toBe(false);
    expect(linhasDoCartao(recusado, "en")[0]?.texto).toContain("Choose a manual model");
  });

  it("ao trocar do automatico para o manual, informa que o trecho automatico terminou", () => {
    const proximo: Cartao = { ...cartao, loteAutomatico: false, anteriorAutomatico: true, estado: { tipo: "proximo_lote" } };
    expect(linhasDoCartao(proximo, "pt-BR")[0]?.texto).toBe("O trecho automático terminou.");
  });

  it("confirma os trechos automaticos e manuais mesmo na simulada", () => {
    expect(precisaConfirmar([impressora])).toBe(true);
    const { notas } = linhasDoEnvio([impressora], { paginas: 4, lotes: 2, papeis: [], paresMistos: [], assinado: false, fluxos: [{ automatico: true, inicio: 1, fim: 2 }, { automatico: false, inicio: 3, fim: 4 }] }, "pt-BR");
    expect(notas).toContain("Frente e verso automático, sem gesto: págs. 1–2.");
    expect(notas).toContain("Frente e verso manual, com gesto: págs. 3–4.");
    expect(conteudoDoEnvio([], notas, true).fechar).toBe("modo.automatico.envioConfirmar");
  });
});
