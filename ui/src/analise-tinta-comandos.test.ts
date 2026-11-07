import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ConfiguracaoDeTinta, PerfilDeTinta } from "./analise-tinta";
import {
  EVENTOS_DA_TINTA,
  apagarPerfilDeTinta,
  cancelarAnalise,
  configuracaoDeTinta,
  escolherCaminhoCsv,
  definirConfiguracaoDeTinta,
  exportarCsv,
  iniciarAnalise,
  nomeDoCsv,
  ouvirAnaliseDeTinta,
  perfisDeTinta,
  recalcularAnalise,
  salvarPerfilDeTinta,
} from "./analise-tinta";

const tauri = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  save: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: tauri.listen }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: tauri.save }));

const PERFIL: PerfilDeTinta = {
  id: "tanque-generico",
  nome: "Tanque de tinta genérico",
  moeda: "BRL",
  volumes_ml: [65, 65, 65, 65],
  rendimentos_paginas: [6500, 6500, 6500, 4500],
  precos_ml: [0.77, 0.77, 0.77, 1.11],
  fatores_qualidade: [0.6, 1, 1.3],
};

const CONFIGURACAO: ConfiguracaoDeTinta = {
  perfil: PERFIL.id,
  qualidade: "normal",
  dpi: 150,
  modo: "simples",
  cinzas_so_preto: true,
  tamanho_da_imagem: "a4",
};

function canais(c: number): Record<string, number> {
  return { c, m: 0, y: 0, k: 0, total: c };
}

const PAGINA = {
  id: "job-1",
  pagina: 1,
  larguraMm: 210,
  alturaMm: 297,
  cobertura: canais(10),
  tinta: canais(0.1),
  custo: canais(0.1),
};

const RESULTADO = {
  totalDePaginas: 10,
  modoUsado: "simples",
  dpi: 150,
  perfil: PERFIL.id,
  moeda: "BRL",
  qualidade: "normal",
  copias: 1,
  documentoInteiro: false,
  selecao: {
    paginas: [2, 5, 7, 9],
    quantidade: 4,
    totalComCopias: 4,
    tinta: canais(0.4),
    custo: canais(0.4),
    cartucho: {
      percentualConsumido: 1,
      copiasAteEsvaziar: 100,
      canalQueAcabaPrimeiro: 0,
    },
  },
  documento: {
    tinta: canais(1),
    custo: canais(1),
    coberturaMedia: canais(10),
    mlMedioPorPagina: 0.1,
    paginaMaisCara: { numero: 1, tinta: canais(0.3), custo: canais(0.3) },
  },
};

beforeEach(() => {
  tauri.invoke.mockReset();
  tauri.listen.mockReset();
  tauri.save.mockReset();
});

describe("comandos de analise", () => {
  it("iniciar_analise leva caminho, selecao, copias, configuracao e pagina atual", async () => {
    tauri.invoke.mockResolvedValue("job-1");
    await expect(
      iniciarAnalise("x.pdf", "2,5,7,9", 1, CONFIGURACAO, 3),
    ).resolves.toBe("job-1");
    expect(tauri.invoke).toHaveBeenCalledWith("iniciar_analise", {
      caminho: "x.pdf",
      paginas: "2,5,7,9",
      copias: 1,
      configuracao: CONFIGURACAO,
      paginaAtual: 3,
    });
  });

  it("paginas vazias vao como null, e nao como string vazia", async () => {
    tauri.invoke.mockResolvedValue("job-1");
    await iniciarAnalise("x.pdf", null, 5, CONFIGURACAO, null);
    expect(tauri.invoke).toHaveBeenCalledWith(
      "iniciar_analise",
      expect.objectContaining({ paginas: null, paginaAtual: null }),
    );
  });

  it("recalcular_analise usa o mesmo id e a configuracao nova", async () => {
    tauri.invoke.mockResolvedValue(undefined);
    await recalcularAnalise("job-1", CONFIGURACAO, "5-16", 12);
    expect(tauri.invoke).toHaveBeenCalledWith("recalcular_analise", {
      id: "job-1",
      configuracao: CONFIGURACAO,
      paginas: "5-16",
      copias: 12,
    });
  });

  it("cancelar_analise leva so o id e devolve se o trabalho ainda estava vivo", async () => {
    tauri.invoke.mockResolvedValue(true);
    await expect(cancelarAnalise("job-1")).resolves.toBe(true);
    expect(tauri.invoke).toHaveBeenCalledWith("cancelar_analise", { id: "job-1" });
  });
});

describe("comandos de perfil e configuracao", () => {
  it("le e normaliza perfis", async () => {
    tauri.invoke.mockResolvedValue({ perfis: [PERFIL], aviso: null });
    await expect(perfisDeTinta()).resolves.toEqual({ perfis: [PERFIL], aviso: null });
    expect(tauri.invoke).toHaveBeenCalledWith("perfis_de_tinta");
  });

  it("salva e apaga perfis pelo id", async () => {
    tauri.invoke.mockResolvedValue({ perfis: [PERFIL], aviso: null });
    await salvarPerfilDeTinta(PERFIL);
    expect(tauri.invoke).toHaveBeenCalledWith("salvar_perfil_de_tinta", { perfil: PERFIL });
    await apagarPerfilDeTinta(PERFIL.id);
    expect(tauri.invoke).toHaveBeenCalledWith("apagar_perfil_de_tinta", { id: PERFIL.id });
  });

  it("le e grava a configuracao", async () => {
    tauri.invoke.mockResolvedValue({ configuracao: CONFIGURACAO, aviso: null });
    await expect(configuracaoDeTinta()).resolves.toEqual({
      configuracao: CONFIGURACAO,
      aviso: null,
    });
    expect(tauri.invoke).toHaveBeenCalledWith("configuracao_de_tinta");
    await definirConfiguracaoDeTinta(CONFIGURACAO);
    expect(tauri.invoke).toHaveBeenCalledWith("definir_configuracao_de_tinta", {
      configuracao: CONFIGURACAO,
    });
  });
});

describe("CSV", () => {
  it("exportar_csv leva o id e o caminho escolhido", async () => {
    tauri.invoke.mockResolvedValue(undefined);
    await exportarCsv("job-1", "C:\\saida\\tinta.csv");
    expect(tauri.invoke).toHaveBeenCalledWith("exportar_csv", {
      id: "job-1",
      caminho: "C:\\saida\\tinta.csv",
    });
  });

  it("o nome sugerido nao leva separador de caminho e troca a extensao", () => {
    expect(nomeDoCsv("relatório/final.pdf")).toBe("tinta-relatório_final.csv");
    expect(nomeDoCsv("")).toBe("tinta-documento.csv");
  });

  it("o seletor de CSV abre com o nome sugerido e o filtro", async () => {
    tauri.save.mockResolvedValue("C:\\saida\\tinta.csv");
    await expect(escolherCaminhoCsv("x.pdf")).resolves.toBe("C:\\saida\\tinta.csv");
    expect(tauri.save).toHaveBeenCalledWith(
      expect.objectContaining({
        defaultPath: "tinta-x.csv",
        filters: [{ name: "Planilha CSV", extensions: ["csv"] }],
      }),
    );
  });

  it("cancelar o seletor devolve null e nao exporta", async () => {
    tauri.save.mockResolvedValue(null);
    await expect(escolherCaminhoCsv("x.pdf")).resolves.toBeNull();
    expect(tauri.invoke).not.toHaveBeenCalled();
  });
});

describe("eventos da analise", () => {
  it("assina os quatro nomes antes de disparar e desassina todos ao fechar", async () => {
    const ouvintes = {
      aoProgresso: vi.fn(),
      aoPagina: vi.fn(),
      aoFim: vi.fn(),
      aoFalha: vi.fn(),
    };
    const registrados = new Map<string, (mensagem: { payload: unknown }) => void>();
    const retirados: string[] = [];
    tauri.listen.mockImplementation(
      async (evento: string, tratar: (mensagem: { payload: unknown }) => void) => {
        registrados.set(evento, tratar);
        return () => retirados.push(evento);
      },
    );

    const cancelar = await ouvirAnaliseDeTinta(ouvintes, () => "job-1", "x.pdf");
    expect(tauri.listen).toHaveBeenCalledTimes(4);
    for (const nome of Object.values(EVENTOS_DA_TINTA)) {
      expect(registrados.has(nome)).toBe(true);
    }

    registrados.get(EVENTOS_DA_TINTA.progresso)?.({ payload: { id: "job-1", feitas: 1, total: 4, selecao: null } });
    expect(ouvintes.aoProgresso).toHaveBeenCalledWith({ feitas: 1, total: 4, selecao: null });

    registrados.get(EVENTOS_DA_TINTA.pagina)?.({ payload: PAGINA });
    expect(ouvintes.aoPagina).toHaveBeenCalledWith(
      expect.objectContaining({ numero: 1, em_branco: false }),
    );

    registrados.get(EVENTOS_DA_TINTA.fim)?.({ payload: { id: "job-1", resultado: RESULTADO } });
    expect(ouvintes.aoFim).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "job-1",
        arquivo: expect.objectContaining({ nome: "x.pdf" }),
        selecao: expect.objectContaining({ quantidade: 4 }),
      }),
    );

    registrados.get(EVENTOS_DA_TINTA.falha)?.({ payload: { id: "job-1", codigo: "cancelado", detalhe: null } });
    expect(ouvintes.aoFalha).toHaveBeenCalledWith("A análise foi cancelada.");

    cancelar();
    expect(retirados.sort()).toEqual(Object.values(EVENTOS_DA_TINTA).sort());
  });

  it("resultado invalido no fim vira falha, nao some", async () => {
    const ouvintes = {
      aoProgresso: vi.fn(),
      aoPagina: vi.fn(),
      aoFim: vi.fn(),
      aoFalha: vi.fn(),
    };
    const registrados = new Map<string, (mensagem: { payload: unknown }) => void>();
    tauri.listen.mockImplementation(
      async (evento: string, tratar: (mensagem: { payload: unknown }) => void) => {
        registrados.set(evento, tratar);
        return () => undefined;
      },
    );
    await ouvirAnaliseDeTinta(ouvintes, () => "job-1", "x.pdf");
    registrados.get(EVENTOS_DA_TINTA.fim)?.({ payload: { id: "job-1", resultado: {} } });
    expect(ouvintes.aoFim).not.toHaveBeenCalled();
    expect(ouvintes.aoFalha).toHaveBeenCalledWith(
      expect.stringContaining("incompleta"),
    );
  });

  it("ignora evento de outro trabalho", async () => {
    const ouvintes = {
      aoProgresso: vi.fn(),
      aoPagina: vi.fn(),
      aoFim: vi.fn(),
      aoFalha: vi.fn(),
    };
    const registrados = new Map<string, (mensagem: { payload: unknown }) => void>();
    tauri.listen.mockImplementation(
      async (evento: string, tratar: (mensagem: { payload: unknown }) => void) => {
        registrados.set(evento, tratar);
        return () => undefined;
      },
    );
    await ouvirAnaliseDeTinta(ouvintes, () => "job-1", "x.pdf");
    registrados.get(EVENTOS_DA_TINTA.fim)?.({ payload: { id: "job-2", resultado: RESULTADO } });
    expect(ouvintes.aoFim).not.toHaveBeenCalled();
  });
});