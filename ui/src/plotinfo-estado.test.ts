import { describe, expect, it } from "vitest";
import type { PerfilDeTinta } from "./analise-tinta";
import {
  ESTADO_INICIAL_DO_PAINEL,
  configuracaoValida,
  errosDoRascunho,
  idDePerfil,
  perfilDoRascunho,
  reduzirPainelDeTinta,
  rascunhoDoPerfil,
  rascunhoNovo,
  totalDoPainel,
} from "./plotinfo-estado";
import { copiasDaFace, estadoInicial, reduzirAbas, type Documento } from "./tabs";

const PERFIL: PerfilDeTinta = {
  id: "tanque-generico",
  nome: "Tanque de tinta genérico",
  moeda: "BRL",
  volumes_ml: [65, 65, 65, 65],
  rendimentos_paginas: [6500, 6500, 6500, 4500],
  precos_ml: [0.77, 0.77, 0.77, 1.11],
  fatores_qualidade: [0.6, 1, 1.3],
};

const DOCUMENTO: Documento = { caminho: "x.pdf", nome: "x.pdf", tipo: "pdf" };

function documentoComEnvioConhecido() {
  let estado = reduzirAbas(estadoInicial, { tipo: "abrir", documento: DOCUMENTO });
  estado = reduzirAbas(estado, { tipo: "paginas_contadas", id: 1, total: 10 });
  estado = reduzirAbas(estado, {
    tipo: "definir_intervalo",
    id: 1,
    intervalo: { inicio: "2", fim: "4" },
  });
  estado = reduzirAbas(estado, { tipo: "definir_copias", id: 1, face: "frente", valor: 3 });
  estado = reduzirAbas(estado, { tipo: "definir_face", id: 1, face: "verso" });
  return estado;
}

describe("campos do painel nao tocam no estado da aba", () => {
  it("mudar paginas e copias do painel deixa intervalo, face e copias do envio intactos", () => {
    const abas = documentoComEnvioConhecido();
    const antes = structuredClone(abas);
    const aba = abas.abas[0];
    expect(aba).toBeDefined();
    if (aba === undefined) return;

    let painel = ESTADO_INICIAL_DO_PAINEL;
    painel = reduzirPainelDeTinta(painel, { tipo: "paginas", valor: "2,5,7,9" });
    painel = reduzirPainelDeTinta(painel, { tipo: "copias", valor: "12" });
    painel = reduzirPainelDeTinta(painel, { tipo: "paginas", valor: "5-16" });

    expect(abas).toEqual(antes);
    expect(aba.intervalo).toEqual({ inicio: "2", fim: "4" });
    expect(aba.face).toBe("verso");
    expect(copiasDaFace(aba, "frente")).toBe(3);
    expect(copiasDaFace(aba, "verso")).toBe(3);
    expect(painel).toEqual({ paginasTexto: "5-16", copiasTexto: "12" });
  });

  it("o redutor do painel so muda os dois campos que pertencem a ele", () => {
    let painel = ESTADO_INICIAL_DO_PAINEL;
    painel = reduzirPainelDeTinta(painel, { tipo: "paginas", valor: "1" });
    expect(painel).toEqual({ paginasTexto: "1", copiasTexto: "1" });
    painel = reduzirPainelDeTinta(painel, { tipo: "copias", valor: "2" });
    expect(painel).toEqual({ paginasTexto: "1", copiasTexto: "2" });
    painel = reduzirPainelDeTinta(painel, { tipo: "reiniciar" });
    expect(painel).toEqual(ESTADO_INICIAL_DO_PAINEL);
  });
});

describe("identificador do perfil", () => {
  it("vira um slug sem acentos e sem repetir", () => {
    expect(idDePerfil("Tanque de tinta", [])).toBe("tanque-de-tinta");
    expect(idDePerfil("Perfil Ação", [])).toBe("perfil-acao");
    expect(idDePerfil("Tanque", ["tanque"])).toBe("tanque-2");
    expect(idDePerfil("Tanque", ["tanque", "tanque-2"])).toBe("tanque-3");
    expect(idDePerfil("", [])).toBe("perfil");
  });

  it("respeita o limite de 64 bytes do arquivo", () => {
    const longo = "a".repeat(80);
    expect(idDePerfil(longo, []).length).toBeLessThanOrEqual(64);
    expect(idDePerfil(longo, [idDePerfil(longo, [])])).not.toBe(idDePerfil(longo, []));
  });
});

describe("rascunho do perfil", () => {
  it("mostra os numeros com virgula e volta para numeros ao salvar", () => {
    const rascunho = rascunhoDoPerfil(PERFIL);
    expect(rascunho.volumes_ml).toEqual(["65", "65", "65", "65"]);
    expect(rascunho.fatores_qualidade).toEqual(["0,6", "1", "1,3"]);
    expect(perfilDoRascunho(rascunho, [PERFIL.id], PERFIL.id)).toEqual(PERFIL);
  });

  it("aceita virgula e ponto nos campos decimais", () => {
    const rascunho = { ...rascunhoDoPerfil(PERFIL), precos_ml: ["0,75", "1.25", "2", "3"] as const };
    const perfil = perfilDoRascunho(rascunho, [PERFIL.id], PERFIL.id);
    expect(perfil?.precos_ml).toEqual([0.75, 1.25, 2, 3]);
  });

  it("marca nome vazio, moeda errada, volume zero e rendimento fracionario", () => {
    const rascunho = {
      ...rascunhoDoPerfil(PERFIL),
      nome: " ",
      moeda: "brl",
      volumes_ml: ["0", "65", "65", "65"] as const,
      rendimentos_paginas: ["6500", "1,5", "6500", "4500"] as const,
      fatores_qualidade: ["0,6", "0", "1,3"] as const,
    };
    const erros = errosDoRascunho(rascunho, [PERFIL.id], PERFIL.id);
    expect(erros.algum).toBe(true);
    expect(erros.nome).toBe(true);
    expect(erros.moeda).toBe(true);
    expect(erros.volumes[0]).toBe(true);
    expect(erros.rendimentos[1]).toBe(true);
    expect(erros.fatores[1]).toBe(true);
    expect(perfilDoRascunho(rascunho, [PERFIL.id], PERFIL.id)).toBeNull();
  });

  it("recusa identificador repetido de outro perfil", () => {
    const rascunho = { ...rascunhoNovo("Outro", [PERFIL.id]), id: PERFIL.id };
    expect(errosDoRascunho(rascunho, [PERFIL.id], null).id).toBe(true);
    expect(errosDoRascunho(rascunho, [PERFIL.id], PERFIL.id).id).toBe(false);
  });
});

describe("configuracao e total do painel", () => {
  it("a configuracao so vale com um perfil existente e dpi aceito", () => {
    const configuracao = {
      perfil: PERFIL.id,
      qualidade: "normal" as const,
      dpi: 150,
      modo: "simples" as const,
      cinzas_so_preto: true,
      tamanho_da_imagem: "a4" as const,
    };
    expect(configuracaoValida(configuracao, [PERFIL])).toBe(true);
    expect(configuracaoValida({ ...configuracao, perfil: "sumiu" }, [PERFIL])).toBe(false);
    expect(configuracaoValida({ ...configuracao, dpi: 200 }, [PERFIL])).toBe(false);
  });

  it("o total do painel usa a contagem do backend e as copias digitadas", () => {
    expect(totalDoPainel(4, "1")).toBe(4);
    expect(totalDoPainel(12, "12")).toBe(144);
    expect(totalDoPainel(10, "5")).toBe(50);
    expect(totalDoPainel(4, "0")).toBeNull();
    expect(totalDoPainel(4, "1000")).toBeNull();
  });
});