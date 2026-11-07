import { beforeEach, describe, expect, it, vi } from "vitest";

const tauri = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));

import { iniciarImpressao } from "./trabalhos";

describe("iniciar impressao", () => {
  beforeEach(() => {
    tauri.invoke.mockReset();
    tauri.invoke.mockResolvedValue([]);
  });

  it("manda a opcao de uma copia por vez junto com as copias", async () => {
    await iniciarImpressao(1, "x.pdf", ["fila:Sala"], null, "frente", null, null, 3, false);
    expect(tauri.invoke).toHaveBeenCalledWith(
      "iniciar_impressao",
      expect.objectContaining({ copias: 3, umaCopiaPorVez: false }),
    );
  });

  it("o padrao da interface e mandar ligado", async () => {
    await iniciarImpressao(1, "x.pdf", ["fila:Sala"], null, "frente");
    expect(tauri.invoke).toHaveBeenCalledWith(
      "iniciar_impressao",
      expect.objectContaining({ copias: 1, umaCopiaPorVez: true }),
    );
  });
});