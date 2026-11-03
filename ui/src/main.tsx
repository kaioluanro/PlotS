import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { avisarMontagem } from "./abertura/passagem";
import { App } from "./App";
import { idiomaAtual } from "./i18n";
import { acompanharTemaDoSistema } from "./theme";
import "./theme.css";

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

avisarMontagem();
