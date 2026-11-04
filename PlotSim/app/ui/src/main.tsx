import "@plots/theme.css";
import { acompanharTemaDoSistema } from "@plots/theme";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { interfaceMontou } from "./backend";
import { idiomaAtual } from "./i18n";

document.documentElement.lang = idiomaAtual;
acompanharTemaDoSistema();

const raiz = document.getElementById("root");
if (raiz) {
  createRoot(raiz).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

interfaceMontou().catch(() => {
  // A reserva por prazo mostra a janela mesmo sem esta resposta.
});
