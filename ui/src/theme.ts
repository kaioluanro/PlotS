export type Tema = "dark" | "light";

export function aplicarTema(tema: Tema): void {
  document.documentElement.setAttribute("data-theme", tema);
}

/** Segue o tema do sistema, agora e nas proximas trocas; sem preferencia clara, fica no escuro. */
export function acompanharTemaDoSistema(): void {
  const consulta = window.matchMedia("(prefers-color-scheme: light)");
  const atualizar = () => aplicarTema(consulta.matches ? "light" : "dark");
  atualizar();
  consulta.addEventListener("change", atualizar);
}
