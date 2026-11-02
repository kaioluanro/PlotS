import { t, type Chave } from "./i18n";

const CHAVES_POR_CODIGO: ReadonlyMap<string, Chave> = new Map<string, Chave>([
  ["nao_encontrado", "erro.nao_encontrado"],
  ["nao_e_arquivo", "erro.nao_e_arquivo"],
  ["nao_e_pdf", "erro.nao_e_pdf"],
  ["inacessivel", "erro.inacessivel"],
]);

/** Traduz a recusa vinda do backend; qualquer outra falha vira uma mensagem generica. */
export function mensagemDeErro(erro: unknown): string {
  if (typeof erro === "object" && erro !== null && "codigo" in erro && "nome" in erro) {
    const { codigo, nome } = erro;
    if (typeof codigo === "string" && typeof nome === "string") {
      const chave = CHAVES_POR_CODIGO.get(codigo);
      if (chave) return t(chave, { nome });
    }
  }
  return t("erro.desconhecido");
}
