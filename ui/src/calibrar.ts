/** Nome do executavel que quem compila o projeto recebe. */
export const PROGRAMA = "plots-cli";

/** Fim de todo comando: quem escreve o comando na tela mantem esta parte sem quebra de linha. */
export const SAIDA_NA_PASTA_ATUAL = "--saida .";

/** Toda aspa simples que o PowerShell reconhece; dentro de uma cadeia, cada uma vale dobrada. */
const ASPAS_SIMPLES = /['\u2018\u2019\u201A\u201B]/g;
const CONTROLE = /[\u0000-\u001F\u007F]/;

/**
 * Comando para colar no PowerShell, na pasta do executavel, com o nome exato da impressora.
 *
 * Aspas simples deixam `$` e crase sem efeito. Elas nao bastam quando o nome tem aspas duplas: o
 * PowerShell 5.1 as entrega sem escape ao executavel, que as descarta. Nesse caso `--%` passa o
 * resto da linha sem interpretar, e a aspa vai dobrada: com `\"` o PowerShell acharia que a cadeia
 * acabou e aceitaria um `|` do nome como comando. `--%` termina na quebra de linha, entao um nome
 * com caractere de controle fica sempre entre aspas simples, onde a quebra nao encerra o comando.
 *
 * Devolve `null` quando o nome tem aspas duplas e `%`: `--%` expande `%VAR%`, sem escape possivel,
 * entao nenhuma forma entrega esse nome com seguranca.
 */
export function comandoDeCalibrar(nome: string): string | null {
  const executavel = `.\\${PROGRAMA}`;
  if (nome.includes('"')) {
    if (nome.includes("%")) return null;
    if (!CONTROLE.test(nome)) {
      const escapado = nome.replace(/(\\*)"/g, '$1$1""').replace(/\\+$/, "$&$&");
      return `${executavel} --% calibrar "${escapado}" ${SAIDA_NA_PASTA_ATUAL}`;
    }
  }
  return `${executavel} calibrar '${nome.replace(ASPAS_SIMPLES, "$&$&")}' ${SAIDA_NA_PASTA_ATUAL}`;
}
