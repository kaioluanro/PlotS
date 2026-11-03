import { invoke } from "@tauri-apps/api/core";
import type { PDFDocumentProxy } from "pdfjs-dist";
import { escalaDoCanvas, tamanhoSeguro, totalSeguro, type TamanhoPagina } from "./contas";
import type { Desenho } from "./fila";
import { DadosEmbutidos, URL_DO_DESENHO, URL_DO_NUCLEO } from "./recursos";

/** Imagem acima disso nao e desenhada: uma imagem hostil nao esgota a memoria do worker. */
const PIXELS_MAXIMOS_DE_IMAGEM = 200_000_000;

/** Quanto se espera o worker encerrar sozinho antes de derruba-lo. */
const PRAZO_PARA_ENCERRAR_MS = 3000;

export interface PdfAberto {
  documento: PDFDocumentProxy;
  total: number;
  primeiraPagina: TamanhoPagina;
  /** Solta os workers e toda a memoria do documento. */
  fechar(): void;
}

/** O backend so entrega PDF que passou pela barreira da impressao; a recusa traz o motivo. */
export function lerDocumento(caminho: string): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("ler_documento", { caminho });
}

/** Erro para quem desistiu de um desenho antes de ele comecar. */
function cancelamento(): Error {
  const erro = new Error("desenho cancelado");
  erro.name = "RenderingCancelledException";
  return erro;
}

/**
 * Abre os bytes no pdf.js, que os transfere ao worker sem copia. Se `aberto` disser que ja nao ha
 * interesse, solta tudo e rejeita. O pdf.js nao recebe senha: um PDF que pedir uma e recusado.
 */
export async function abrirPdf(bytes: ArrayBuffer, aberto: () => boolean): Promise<PdfAberto> {
  if (!aberto()) throw cancelamento();
  const pdfjs = await import("pdfjs-dist");
  // O modulo demora: a aba pode ter saido nesse meio tempo, e nada e criado para ela.
  if (!aberto()) throw cancelamento();
  pdfjs.GlobalWorkerOptions.workerSrc = URL_DO_NUCLEO;
  pdfjs.GlobalWorkerOptions.rendererSrc = URL_DO_DESENHO;

  // Worker proprio, para poder derruba-lo mesmo que um PDF hostil o deixe sem responder.
  const worker = new pdfjs.PDFWorker({ verbosity: pdfjs.VerbosityLevel.ERRORS });
  const tarefa = pdfjs.getDocument({
    data: new Uint8Array(bytes),
    worker,
    BinaryDataFactory: DadosEmbutidos,
    useWorkerFetch: false,
    useSystemFonts: true,
    maxImageSize: PIXELS_MAXIMOS_DE_IMAGEM,
    verbosity: pdfjs.VerbosityLevel.ERRORS,
  });

  const fechar = () => {
    const derrubar = () => worker.destroy();
    const prazo = setTimeout(derrubar, PRAZO_PARA_ENCERRAR_MS);
    tarefa
      .destroy()
      .catch(() => undefined)
      .finally(() => {
        clearTimeout(prazo);
        derrubar();
      });
  };

  try {
    const documento = await tarefa.promise;
    if (!aberto()) throw cancelamento();
    const total = totalSeguro(documento.numPages);
    if (total === null) throw new Error("contagem de paginas sem sentido");
    const pagina = await documento.getPage(1);
    if (!aberto()) throw cancelamento();
    const base = pagina.getViewport({ scale: 1 });
    return {
      documento,
      total,
      primeiraPagina: tamanhoSeguro(base.width, base.height),
      fechar,
    };
  } catch (erro) {
    fechar();
    throw erro;
  }
}

/** Mede a pagina, com a rotacao do proprio PDF. */
export async function medirPagina(
  documento: PDFDocumentProxy,
  numero: number,
): Promise<TamanhoPagina> {
  const pagina = await documento.getPage(numero);
  const base = pagina.getViewport({ scale: 1 });
  return tamanhoSeguro(base.width, base.height);
}

/**
 * Desenha a pagina no canvas, com a rotacao do proprio PDF e sem passar do limite de memoria. O
 * desenho roda no worker quando o sistema permite; nos dois casos a janela continua respondendo.
 */
export function desenharPagina(
  documento: PDFDocumentProxy,
  numero: number,
  canvas: HTMLCanvasElement,
  pixelsPorPonto: number,
): Desenho {
  let cancelado = false;
  let cancelarTarefa: (() => void) | null = null;
  const concluido = documento.getPage(numero).then((pagina) => {
    if (cancelado) throw cancelamento();
    const base = pagina.getViewport({ scale: 1 });
    const escala = escalaDoCanvas(base.width, base.height, pixelsPorPonto);
    const viewport = pagina.getViewport({ scale: escala });
    canvas.width = Math.max(Math.floor(viewport.width), 1);
    canvas.height = Math.max(Math.floor(viewport.height), 1);
    const tarefa = pagina.render({ canvas, viewport });
    cancelarTarefa = () => tarefa.cancel();
    return tarefa.promise;
  });
  return {
    concluido,
    cancelar() {
      cancelado = true;
      cancelarTarefa?.();
    },
  };
}
