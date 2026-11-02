import { mensagemDeErro } from "./erros";
import { t } from "./i18n";
import type { Documento } from "./tabs";

export interface LoteCaminhos {
  caminhos: string[];
  cortado: boolean;
}

/** Resume o lote em uma frase: a falha unica, a contagem de varias, e o aviso de corte. */
export function mensagemDoLote(falhas: readonly unknown[], cortado: boolean): string | null {
  const partes: string[] = [];
  const [primeira] = falhas;
  if (falhas.length === 1) partes.push(mensagemDeErro(primeira));
  if (falhas.length > 1) partes.push(t("erro.varios", { n: String(falhas.length) }));
  if (cortado) partes.push(t("lote.cortado"));
  return partes.length > 0 ? partes.join(" ") : null;
}

/** Abre os caminhos em ordem; devolve a mensagem final, sem que um sucesso apague uma falha. */
export async function abrirLote(
  lote: LoteCaminhos,
  abrir: (caminho: string) => Promise<Documento>,
  aoAbrir: (documento: Documento) => void,
): Promise<string | null> {
  const falhas: unknown[] = [];
  for (const caminho of lote.caminhos) {
    try {
      aoAbrir(await abrir(caminho));
    } catch (falha) {
      falhas.push(falha);
    }
  }
  return mensagemDoLote(falhas, lote.cortado);
}
