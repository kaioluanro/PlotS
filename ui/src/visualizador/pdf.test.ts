import { beforeEach, describe, expect, it, vi } from "vitest";

const pdfjs = vi.hoisted(() => ({
  GlobalWorkerOptions: { workerSrc: "", rendererSrc: "" },
  VerbosityLevel: { ERRORS: 0 },
  PDFWorker: vi.fn(),
  getDocument: vi.fn(),
}));

vi.mock("pdfjs-dist", () => pdfjs);
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { abrirPdf } from "./pdf";

// Documento falso: se a abertura chegar ao fim, a interessada que saiu ainda e conferida antes.
function tarefaComDocumento() {
  const documento = {
    numPages: 1,
    getPage: () =>
      Promise.resolve({ getViewport: () => ({ width: 595, height: 842 }) }),
  };
  return {
    promise: Promise.resolve(documento),
    destroy: () => Promise.resolve(),
  };
}

describe("abrirPdf e a interessada que ja saiu", () => {
  beforeEach(() => {
    pdfjs.PDFWorker.mockReset();
    pdfjs.getDocument.mockReset();
    pdfjs.PDFWorker.mockImplementation(function () {
      return { destroy: () => undefined };
    });
    pdfjs.getDocument.mockImplementation(tarefaComDocumento);
  });

  it("nao cria worker nem entrega os bytes quando a aba ja saiu antes do carregamento", async () => {
    const aberto = () => false;

    await expect(abrirPdf(new ArrayBuffer(8), aberto)).rejects.toMatchObject({
      name: "RenderingCancelledException",
    });
    expect(pdfjs.PDFWorker).not.toHaveBeenCalled();
    expect(pdfjs.getDocument).not.toHaveBeenCalled();
  });

  it("confere de novo depois de importar o pdf.js, antes de criar o worker", async () => {
    // A primeira conferencia passa; a aba sai enquanto o modulo do pdf.js carrega.
    let consultas = 0;
    const aberto = () => {
      consultas += 1;
      return consultas < 2;
    };

    await expect(abrirPdf(new ArrayBuffer(8), aberto)).rejects.toMatchObject({
      name: "RenderingCancelledException",
    });
    expect(pdfjs.PDFWorker).not.toHaveBeenCalled();
    expect(pdfjs.getDocument).not.toHaveBeenCalled();
  });
});
