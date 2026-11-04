import { describe, expect, it } from "vitest";
import { estadoDoEnvio, type Cartao, type EstadoCartao } from "./cartoes";
import { acaoDoEnvio, envioComEspera, esperasDoRodape, esperaPedeConfirmacao } from "./rodape-espera";
import type { Impressora } from "./impressoras";
import type { Aba } from "./tabs";

function fila(id: string): Impressora {
  return {
    id: `fila:${id}`,
    nome: id,
    tipo: "fila",
    nivel: "pronta",
    condicoes: [],
    modo: "jato",
  };
}

function cartao(aba: number, impressora: Impressora, estado: EstadoCartao, alcance: Cartao["alcance"] = "so_frentes"): Cartao {
  return {
    aba,
    impressora: impressora.id,
    nome: impressora.nome,
    simulada: false,
    lote: 1,
    total: 2,
    papel: null,
    estado,
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    alcance,
  };
}

function aba(face: "frente" | "verso", marcadas: readonly Impressora[], cartoes: readonly Cartao[]): Aba {
  return {
    id: 1,
    impressorasMarcadas: marcadas.map((impressora) => impressora.id),
    face,
    cartoes,
    paginas: { estado: "conhecido", total: 4 },
    frentesProntas: [],
  } as unknown as Aba;
}

const A = fila("A");
const B = fila("B");

describe("rodape com trabalho que espera a pessoa", () => {
  it("frente: a impressora em proximo lote recebe o proximo trecho", () => {
    const aberta = aba("frente", [A], [cartao(1, A, { tipo: "proximo_lote" })]);
    const espera = esperasDoRodape(aberta, "frente", [A]);
    expect(espera.itens.map((item) => item.comando)).toEqual(["enviar_frente"]);
    expect(espera.itens[0]?.cartao.impressora).toBe(A.id);
  });

  it("verso: a impressora aguardando recebe o verso, mesmo com alcance so de frentes", () => {
    const aberta = aba("verso", [A], [cartao(1, A, { tipo: "aguardando" }, "so_frentes")]);
    const espera = esperasDoRodape(aberta, "verso", [A]);
    expect(espera.itens.map((item) => item.comando)).toEqual(["enviar_verso"]);
  });

  it("frente nao manda o verso de uma impressora aguardando, e a lista a deixa de fora", () => {
    const aberta = aba("frente", [A], [cartao(1, A, { tipo: "aguardando" })]);
    const espera = esperasDoRodape(aberta, "frente", [A]);
    expect(espera.itens).toHaveLength(0);
    expect(espera.fora.map((impressora) => impressora.id)).toEqual([A.id]);
  });

  it("verso nao manda a frente de uma impressora em proximo lote", () => {
    const aberta = aba("verso", [A], [cartao(1, A, { tipo: "proximo_lote" })]);
    expect(esperasDoRodape(aberta, "verso", [A]).itens).toHaveLength(0);
  });

  it("um trabalho de outra aba nunca entra: o verso de um documento nao sai pela aba de outro", () => {
    const aberta = aba("verso", [A], [cartao(2, A, { tipo: "aguardando" }, "so_versos")]);
    expect(esperasDoRodape(aberta, "verso", [A]).itens).toHaveLength(0);
  });

  it("impressora que nao esta marcada nao entra, mesmo esperando", () => {
    const aberta = aba("frente", [A], [cartao(1, B, { tipo: "proximo_lote" })]);
    expect(esperasDoRodape(aberta, "frente", [A]).itens).toHaveLength(0);
  });

  it("sem aba nao ha espera", () => {
    expect(esperasDoRodape(null, "frente", [A]).itens).toHaveLength(0);
  });

  it("sem espera, o envio segue como estava", () => {
    const aberta = aba("frente", [A], []);
    const base = estadoDoEnvio(aberta, [A], new Set());
    const espera = esperasDoRodape(aberta, "frente", [A]);
    expect(envioComEspera(base, espera, "pt-BR")).toBe(base);
  });

  it("com espera, o botao fica habilitado mesmo que o envio comum estivesse parado", () => {
    const aberta = aba("frente", [A], [cartao(1, A, { tipo: "proximo_lote" })]);
    const base = estadoDoEnvio(aberta, [A], new Set([A.id]));
    expect(base.desabilitado).toBe(true);
    const envio = envioComEspera(base, esperasDoRodape(aberta, "frente", [A]), "pt-BR");
    expect(envio.desabilitado).toBe(false);
    expect(envio.motivo).toContain("A");
  });

  it("com uma impressora de fora da acao, o motivo diz quem ficou de fora", () => {
    const aberta = aba("frente", [A, B], [cartao(1, A, { tipo: "proximo_lote" })]);
    const base = estadoDoEnvio(aberta, [A, B], new Set([A.id]));
    const envio = envioComEspera(base, esperasDoRodape(aberta, "frente", [A, B]), "pt-BR");
    expect(envio.desabilitado).toBe(false);
    expect(envio.motivo).toContain("B");
  });

  it("o rotulo diz quantas impressoras recebem a acao, em pt-BR e em ingles", () => {
    const aberta = aba("verso", [A, B], [
      cartao(1, A, { tipo: "aguardando" }),
      cartao(1, B, { tipo: "aguardando" }),
    ]);
    const base = estadoDoEnvio(aberta, [A, B], new Set([A.id, B.id]));
    const espera = esperasDoRodape(aberta, "verso", [A, B]);
    const pt = envioComEspera(base, espera, "pt-BR");
    const en = envioComEspera(base, espera, "en");
    expect(pt.rotulo).toContain("2");
    expect(en.rotulo).toContain("2");
    expect(pt.rotulo).not.toBe(en.rotulo);
  });
  it("o verso que espera sempre passa pela confirmacao; a frente do proximo trecho nao", () => {
    const verso = aba("verso", [A], [cartao(1, A, { tipo: "aguardando" })]);
    expect(esperaPedeConfirmacao(esperasDoRodape(verso, "verso", [A]))).toBe(true);
    const frente = aba("frente", [A], [cartao(1, A, { tipo: "proximo_lote" })]);
    expect(esperaPedeConfirmacao(esperasDoRodape(frente, "frente", [A]))).toBe(false);
  });

  it("o portao diz confirmar o verso em espera e comandar a frente direto", () => {
    const verso = aba("verso", [A], [cartao(1, A, { tipo: "aguardando" })]);
    const acaoVerso = acaoDoEnvio(esperasDoRodape(verso, "verso", [A]));
    expect(acaoVerso.tipo).toBe("confirmar");
    expect(acaoVerso.tipo === "confirmar" ? acaoVerso.itens : []).toHaveLength(1);

    const frente = aba("frente", [A], [cartao(1, A, { tipo: "proximo_lote" })]);
    const acaoFrente = acaoDoEnvio(esperasDoRodape(frente, "frente", [A]));
    expect(acaoFrente.tipo).toBe("comandar");
    expect(acaoFrente.tipo === "comandar" ? acaoFrente.itens[0]?.comando : null).toBe("enviar_frente");

    const semEspera = aba("frente", [A], []);
    expect(acaoDoEnvio(esperasDoRodape(semEspera, "frente", [A]))).toEqual({ tipo: "enviar" });
  });

  it("proximo lote: o botao diz Continuar frentes, e a simulada Continuar simulacao", () => {
    const aberta = aba("frente", [A], [cartao(1, A, { tipo: "proximo_lote" })]);
    const base = estadoDoEnvio(aberta, [A], new Set([A.id]));
    const espera = esperasDoRodape(aberta, "frente", [A]);
    expect(envioComEspera(base, espera, "pt-BR").rotulo).toBe("Continuar frentes em 1");
    expect(envioComEspera(base, espera, "en").rotulo).toBe("Continue fronts on 1");
    const simulada = aba("frente", [A], [{ ...cartao(1, A, { tipo: "proximo_lote" }), simulada: true }]);
    const esperaSim = esperasDoRodape(simulada, "frente", [A]);
    expect(envioComEspera(base, esperaSim, "pt-BR").rotulo).toBe("Continuar simulação em 1");
    expect(envioComEspera(base, esperaSim, "en").rotulo).toBe("Continue simulation on 1");
  });

  it("proximo lote com troca de papel: a dica persistente manda trocar o papel", () => {
    const papel = { formato: "a5", larguraMm: 148, alturaMm: 210 };
    const aberta = aba("frente", [A], [{ ...cartao(1, A, { tipo: "proximo_lote" }), trocaDePapel: papel }]);
    const base = estadoDoEnvio(aberta, [A], new Set([A.id]));
    const envio = envioComEspera(base, esperasDoRodape(aberta, "frente", [A]), "pt-BR");
    expect(envio.motivo).toBe("Troque o papel para A5. Depois, continua o próximo trecho em A.");
    const en = envioComEspera(base, esperasDoRodape(aberta, "frente", [A]), "en");
    expect(en.motivo).toBe("Change the paper to A5. This then continues the next section on A.");
  });

  it("proximo lote sem troca: a dica diz antes para retirar as folhas", () => {
    const aberta = aba("frente", [A], [cartao(1, A, { tipo: "proximo_lote" })]);
    const base = estadoDoEnvio(aberta, [A], new Set([A.id]));
    const envio = envioComEspera(base, esperasDoRodape(aberta, "frente", [A]), "pt-BR");
    expect(envio.motivo).toBe(
      "Retire as folhas do trecho anterior e coloque papel em branco. Continua o próximo trecho em A.",
    );
  });
});
