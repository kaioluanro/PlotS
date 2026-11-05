export type Tema = "dark" | "gray";

export function aplicarTema(tema: Tema): void {
  document.documentElement.setAttribute("data-theme", tema);
}

/** Segue o tema do sistema, agora e nas proximas trocas; a preferencia clara usa o cinza, e sem ela fica o escuro. */
export function acompanharTemaDoSistema(): void {
  const consulta = window.matchMedia("(prefers-color-scheme: light)");
  const atualizar = () => aplicarTema(consulta.matches ? "gray" : "dark");
  atualizar();
  consulta.addEventListener("change", atualizar);
}
