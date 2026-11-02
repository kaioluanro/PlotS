export type AcaoAtalho = "abrir" | "fechar" | "trocar" | "lateral";

export interface TeclaPressionada {
  key: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

export function interpretarAtalho(tecla: TeclaPressionada): AcaoAtalho | null {
  if (!tecla.ctrlKey || tecla.shiftKey || tecla.altKey) return null;
  switch (tecla.key.toLowerCase()) {
    case "o":
    case "t":
      return "abrir";
    case "w":
      return "fechar";
    case "tab":
      return "trocar";
    case "b":
      return "lateral";
    default:
      return null;
  }
}

/** Teclas do navegador embutido que recarregariam a pagina, perdendo as abas, ou a imprimiriam. */
export function deveBloquearTecla(tecla: TeclaPressionada): boolean {
  if (tecla.key === "F5") return true;
  if (!tecla.ctrlKey || tecla.altKey) return false;
  const letra = tecla.key.toLowerCase();
  return letra === "r" || (letra === "p" && !tecla.shiftKey);
}
