import urlDoDesenho from "pdfjs-dist/build/pdf.renderer.min.mjs?url";
import urlDoNucleo from "pdfjs-dist/build/pdf.worker.min.mjs?url";

/** Os dois workers do pdf.js saem do proprio bundle: um interpreta o PDF, o outro desenha. */
export const URL_DO_NUCLEO = urlDoNucleo;
export const URL_DO_DESENHO = urlDoDesenho;

type Carregadores = Record<string, () => Promise<unknown>>;

// A politica de seguranca da janela nao permite `fetch`, nem para o proprio bundle. Por isso as
// tabelas e fontes vem embutidas em modulos que so carregam quando o PDF precisa deles.
const tabelas: Carregadores = import.meta.glob("/node_modules/pdfjs-dist/cmaps/*.bcmap", {
  query: "?url&inline",
  import: "default",
});
// So as fontes Foxit: a Liberation Sans sai sob GPLv2 sem versao posterior, e fica de fora. Com
// useSystemFonts, so Symbol e ZapfDingbats usam estes dados; as demais base-14 vem da fonte do sistema.
const fontes: Carregadores = import.meta.glob("/node_modules/pdfjs-dist/standard_fonts/Foxit*.pfb", {
  query: "?url&inline",
  import: "default",
});
// O decodificador de cores ICC (qcms) fica de fora: sem wasmUrl o pdf.js nao o carrega.
const decodificadores: Carregadores = import.meta.glob("/node_modules/pdfjs-dist/wasm/{jbig2,openjpeg}.wasm", {
  query: "?url&inline",
  import: "default",
});

const PASTAS: Readonly<Record<string, readonly [string, Carregadores]>> = {
  cMapUrl: ["cmaps", tabelas],
  standardFontDataUrl: ["standard_fonts", fontes],
  wasmUrl: ["wasm", decodificadores],
};

/** Caminhos que o bundle embute: a conferencia de licenca dos arquivos usa esta lista nos testes. */
export function chavesDosArquivos(): readonly string[] {
  return [...Object.keys(tabelas), ...Object.keys(fontes), ...Object.keys(decodificadores)];
}

function bytesDaUrl(url: string): Uint8Array {
  const virgula = url.indexOf(",");
  const binario = atob(virgula >= 0 ? url.slice(virgula + 1) : url);
  const bytes = new Uint8Array(binario.length);
  for (let indice = 0; indice < binario.length; indice += 1) {
    bytes[indice] = binario.charCodeAt(indice);
  }
  return bytes;
}

/** Entrega ao pdf.js os arquivos auxiliares que ele pede. Um arquivo que nao existe vira rejeicao. */
export class DadosEmbutidos {
  async fetch({ kind, filename }: { kind: string; filename: string }): Promise<Uint8Array> {
    const [pasta, carregadores] = PASTAS[kind] ?? [];
    const carregar = carregadores?.[`/node_modules/pdfjs-dist/${pasta}/${filename}`];
    if (!carregar) throw new Error(`arquivo auxiliar desconhecido: ${filename}`);
    const url = await carregar();
    if (typeof url !== "string") throw new Error(`arquivo auxiliar invalido: ${filename}`);
    return bytesDaUrl(url);
  }
}
