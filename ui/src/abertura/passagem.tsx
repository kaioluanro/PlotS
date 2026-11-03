import { invoke } from "@tauri-apps/api/core";
import { createRoot } from "react-dom/client";
import { Avisos } from "./Avisos";
import "./passagem.css";
import { avisosDoToast, type AvisoAbertura } from "./textos";

const TEMPO_DO_FADE_MS = 220;
// Se a resposta nunca chegar, a interface aparece sozinha: nunca fica invisivel para sempre.
const TEMPO_MAXIMO_MS = 20_000;

function dentroDoTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function revelar() {
  const raiz = document.documentElement;
  const reduzido = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduzido) {
    raiz.classList.remove("abertura-oculta");
    return;
  }
  raiz.classList.add("abertura-entrando");
  requestAnimationFrame(() => raiz.classList.remove("abertura-oculta"));
  window.setTimeout(() => raiz.classList.remove("abertura-entrando"), TEMPO_DO_FADE_MS + 80);
}

function mostrarAvisos(recebidos: AvisoAbertura[]) {
  const avisos = avisosDoToast(recebidos);
  if (avisos.length === 0) return;
  const lugar = document.createElement("div");
  document.body.append(lugar);
  const raiz = createRoot(lugar);
  raiz.render(
    <Avisos
      avisos={avisos}
      aoDispensar={() => {
        raiz.unmount();
        lugar.remove();
      }}
    />,
  );
}

/**
 * Chamada depois da primeira montagem da interface: avisa o programa, espera a janela principal
 * ser mostrada e so entao a revela, com o aviso do que falhou na abertura, se houver.
 */
export function avisarMontagem(): void {
  if (!dentroDoTauri()) return;
  document.documentElement.classList.add("abertura-oculta");
  const reserva = window.setTimeout(revelar, TEMPO_MAXIMO_MS);
  window.setTimeout(() => {
    invoke<{ avisos: AvisoAbertura[] }>("interface_montou")
      .then(({ avisos }) => {
        window.clearTimeout(reserva);
        revelar();
        mostrarAvisos(avisos);
      })
      .catch(() => {
        window.clearTimeout(reserva);
        revelar();
      });
  }, 0);
}
