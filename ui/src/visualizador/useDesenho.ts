import type { PDFDocumentProxy } from "pdfjs-dist";
import { useEffect, useRef, useState } from "react";
import { foiCancelamento } from "./contas";
import type { FilaDeDesenho, Prioridade } from "./fila";
import { desenharPagina } from "./pdf";

export type EstadoDoDesenho = "carregando" | "pronto" | "erro";

interface Parametros {
  documento: PDFDocumentProxy;
  fila: FilaDeDesenho;
  numero: number;
  pixelsPorPonto: number;
  prioridade: Prioridade;
  /** Espera antes de pedir: quem passa voando pela tela, ou o zoom repetido, nao chega a desenhar. */
  atrasoMs: number;
}

function descartar(canvas: Element | null): void {
  if (canvas instanceof HTMLCanvasElement) {
    canvas.width = 0;
    canvas.height = 0;
  }
}

/**
 * Desenha em um canvas novo e so troca no fim, para o zoom nao piscar: enquanto isso o canvas antigo
 * fica esticado no lugar. Ao desmontar, o canvas e zerado, e a memoria dele volta na hora.
 */
export function useDesenho({
  documento,
  fila,
  numero,
  pixelsPorPonto,
  prioridade,
  atrasoMs,
}: Parametros) {
  const caixa = useRef<HTMLElement>(null);
  const [estado, setEstado] = useState<EstadoDoDesenho>("carregando");

  useEffect(() => {
    let ativo = true;
    let desistir: (() => void) | null = null;
    const novo = document.createElement("canvas");
    const espera = setTimeout(() => {
      desistir = fila.pedir(prioridade, () => {
        const desenho = desenharPagina(documento, numero, novo, pixelsPorPonto);
        desenho.concluido.then(
          () => {
            if (!ativo) return;
            const alvo = caixa.current;
            descartar(alvo?.firstElementChild ?? null);
            alvo?.replaceChildren(novo);
            setEstado("pronto");
          },
          (falha: unknown) => {
            if (ativo && !foiCancelamento(falha)) setEstado("erro");
          },
        );
        return desenho;
      });
    }, atrasoMs);

    return () => {
      ativo = false;
      clearTimeout(espera);
      desistir?.();
      if (caixa.current?.firstElementChild !== novo) descartar(novo);
    };
  }, [documento, fila, numero, pixelsPorPonto, prioridade, atrasoMs]);

  // O ultimo canvas mostrado sai junto com a pagina.
  useEffect(() => {
    const alvo = caixa.current;
    return () => descartar(alvo?.firstElementChild ?? null);
  }, []);

  return { caixa, estado };
}
