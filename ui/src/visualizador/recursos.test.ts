import { describe, expect, it } from "vitest";
import { chavesDosArquivos } from "./recursos";

describe("arquivos embutidos no bundle", () => {
  const chaves = chavesDosArquivos();

  it("nao embute a fonte Liberation, que sai sob GPLv2 sem versao posterior", () => {
    expect(chaves.filter((chave) => /LiberationSans/i.test(chave))).toEqual([]);
  });

  it("mantem as tabelas de CMap de todas as linguas que o pdf.js usa", () => {
    expect(chaves.some((chave) => chave.endsWith("/cmaps/UniGB-UCS2-H.bcmap"))).toBe(true);
    expect(chaves.some((chave) => chave.endsWith("/cmaps/UniJIS-UCS2-H.bcmap"))).toBe(true);
  });

  it("mantem os decodificadores de JPEG 2000 e JBIG2", () => {
    expect(chaves.some((chave) => chave.endsWith("/wasm/openjpeg.wasm"))).toBe(true);
    expect(chaves.some((chave) => chave.endsWith("/wasm/jbig2.wasm"))).toBe(true);
  });

  it("nao embute o decodificador de cores ICC, que o pdf.js nao usa sem wasmUrl", () => {
    expect(chaves.filter((chave) => chave.includes("qcms_bg"))).toEqual([]);
  });
});
