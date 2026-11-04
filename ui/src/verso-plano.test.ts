import { describe, expect, it } from "vitest";
import type { Cartao, ResumoDoEnvio } from "./cartoes";
import { conteudoDoVerso, planoDoVerso } from "./face-do-envio";
import type { Impressora } from "./impressoras";
import { textoDoPapelAjustado } from "./opcoes-envio";
import { estadoInicial, reduzirAbas, type Aba, type EstadoAbas } from "./tabs";

const SALA: Impressora = {
  id: "fila:Sala",
  nome: "Sala",
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "laser",
};

const A4 = { formato: "a4", larguraMm: 210, alturaMm: 297 } as const;

function cartao(mudancas: Partial<Cartao> = {}): Cartao {
  return {
    aba: 1,
    impressora: SALA.id,
    nome: SALA.nome,
    simulada: false,
    lote: 2,
    total: 2,
    papel: A4,
    estado: { tipo: "aguardando" },
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    alcance: "so_frentes",
    folhasDoLote: 4,
    faixaDoLote: { lote: 1, inicio: 1, fim: 8 },
    copias: 1,
    ...mudancas,
  };
}

function comAba(): EstadoAbas {
  let estado = reduzirAbas(estadoInicial, {
    tipo: "abrir",
    documento: { caminho: "C:\\a.pdf", nome: "a.pdf" },
  });
  estado = reduzirAbas(estado, { tipo: "paginas_contadas", id: 1, total: 24 });
  return reduzirAbas(estado, { tipo: "alternar_impressora", id: 1, impressora: SALA.id });
}

const abaDe = (estado: EstadoAbas) => estado.abas[0] as Aba;

const RESUMO_DO_TRABALHO: ResumoDoEnvio = {
  paginas: 8,
  lotes: 1,
  folhas: 4,
  maiorMaco: 4,
  papeis: [A4],
  paresMistos: [],
  assinado: false,
  fluxos: [{ automatico: false, inicio: 1, fim: 8 }],
  faixas: [{ lote: 1, inicio: 1, fim: 8 }],
};

describe("confirmacao do verso em espera", () => {
  it("mostra os numeros do trabalho enviado, nao os da aba alterada depois da frente", () => {
    let estado = comAba();
    estado = reduzirAbas(estado, {
      tipo: "frente_enviada",
      id: 1,
      frente: { trecho: { inicio: 1, fim: 8 }, papel: "a4" },
    });
    // Depois de a frente sair, a pessoa mexe no intervalo, no papel e nas copias da aba.
    estado = reduzirAbas(estado, {
      tipo: "definir_intervalo",
      id: 1,
      intervalo: { inicio: "9", fim: "16" },
    });
    estado = reduzirAbas(estado, { tipo: "definir_papel_ajuste", id: 1, papel: "a5" });
    estado = reduzirAbas(estado, { tipo: "definir_copias", id: 1, face: "verso", valor: 2 });
    const aba = abaDe(estado);

    const plano = planoDoVerso(aba, [cartao()]);
    // O lote ja planejado manda: trecho, papel e copias da frente enviada.
    expect(plano.trecho).toEqual({ inicio: 1, fim: 8 });
    expect(plano.papel).toBe("a4");
    expect(plano.copias).toBe(1);
    expect(plano.leitura).toMatchObject({ estado: "trecho", inicio: 1, fim: 8, total: 24 });

    const dialogo = conteudoDoVerso(
      [SALA],
      [],
      RESUMO_DO_TRABALHO,
      plano.leitura,
      textoDoPapelAjustado(plano.papel, null),
      false,
      "pt-BR",
      { frente: plano.copias, verso: plano.copias },
    );
    expect(dialogo.linhas).toContain("8 pág · 4 folhas");
    expect(dialogo.linhas).toContain("Trecho: páginas 1 a 8 de 24");
    expect(dialogo.linhas).toContain("Papel: A4");
    expect(dialogo.linhas?.join(" ")).not.toContain("9 a 16");
    expect(dialogo.linhas?.join(" ")).not.toContain("× 2");
  });

  it("o documento inteiro enviado continua inteiro, mesmo depois de a aba virar um trecho", () => {
    let estado = comAba();
    // A frente saiu com o documento inteiro e sem papel escolhido: `null` e o valor do envio.
    estado = reduzirAbas(estado, {
      tipo: "frente_enviada",
      id: 1,
      frente: { trecho: null, papel: null },
    });
    estado = reduzirAbas(estado, {
      tipo: "definir_intervalo",
      id: 1,
      intervalo: { inicio: "9", fim: "16" },
    });
    estado = reduzirAbas(estado, { tipo: "definir_papel_ajuste", id: 1, papel: "a5" });
    estado = reduzirAbas(estado, { tipo: "definir_copias", id: 1, face: "verso", valor: 2 });
    const aba = abaDe(estado);

    const plano = planoDoVerso(aba, [cartao({ copias: 4 })]);
    expect(plano.trecho).toBeNull();
    expect(plano.papel).toBeNull();
    expect(plano.copias).toBe(4);
    expect(plano.leitura).toMatchObject({ estado: "inteiro", total: 24 });

    const dialogo = conteudoDoVerso(
      [SALA],
      [],
      RESUMO_DO_TRABALHO,
      plano.leitura,
      textoDoPapelAjustado(plano.papel, null),
      false,
      "pt-BR",
      { frente: plano.copias, verso: plano.copias },
    );
    const linhas = dialogo.linhas?.join(" ") ?? "";
    expect(linhas).not.toContain("9 a 16");
    expect(linhas).not.toContain("A5");
    expect(linhas).toContain("Cópias: 4");
  });

  it("sem trabalho em espera, o verso novo segue a aba, como antes", () => {
    const aba = abaDe(comAba());
    const plano = planoDoVerso(aba, []);
    expect(plano.trecho).toBeNull();
    expect(plano.papel).toBeNull();
    expect(plano.copias).toBe(1);
    expect(plano.leitura).toMatchObject({ estado: "inteiro", total: 24 });
  });
});
