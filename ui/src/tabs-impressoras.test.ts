import { describe, expect, it } from "vitest";
import type { Cartao } from "./cartoes";
import { estadoInicial, reduzirAbas, type EstadoAbas } from "./tabs";

function comAbas(...nomes: string[]): EstadoAbas {
  return nomes.reduce(
    (estado, nome) =>
      reduzirAbas(estado, { tipo: "abrir", documento: { caminho: `C:\\pdfs\\${nome}`, nome } }),
    estadoInicial,
  );
}

function idDa(estado: EstadoAbas, posicao: number): number {
  const aba = estado.abas[posicao];
  if (!aba) throw new Error(`sem aba na posicao ${posicao}`);
  return aba.id;
}

function abaDe(estado: EstadoAbas, id: number) {
  return estado.abas.find((aba) => aba.id === id);
}

describe("selecao de impressoras por aba", () => {
  it("aba nova comeca sem nenhuma marcada, com lote padrao e contando paginas", () => {
    const aba = comAbas("a.pdf").abas[0];
    expect(aba?.impressorasMarcadas).toEqual([]);
    expect(aba?.tamanhoLote).toBe(10);
    expect(aba?.paginas).toEqual({ estado: "contando" });
  });

  it("cada aba guarda as proprias marcas, sem vazar para a outra", () => {
    let estado = comAbas("a.pdf", "b.pdf");
    const [a, b] = [idDa(estado, 0), idDa(estado, 1)];
    estado = reduzirAbas(estado, { tipo: "alternar_impressora", id: a, impressora: "fila:X" });
    estado = reduzirAbas(estado, { tipo: "alternar_impressora", id: b, impressora: "fila:Y" });
    estado = reduzirAbas(estado, { tipo: "alternar_impressora", id: b, impressora: "simulada:gaveta" });
    expect(abaDe(estado, a)?.impressorasMarcadas).toEqual(["fila:X"]);
    expect(abaDe(estado, b)?.impressorasMarcadas).toEqual(["fila:Y", "simulada:gaveta"]);
    estado = reduzirAbas(estado, { tipo: "trocar" });
    expect(abaDe(estado, estado.ativa ?? -1)?.impressorasMarcadas).toEqual(["fila:X"]);
  });

  it("marcar de novo desmarca, sem repetir", () => {
    let estado = comAbas("a.pdf");
    const id = idDa(estado, 0);
    estado = reduzirAbas(estado, { tipo: "alternar_impressora", id, impressora: "fila:X" });
    estado = reduzirAbas(estado, { tipo: "alternar_impressora", id, impressora: "fila:X" });
    expect(abaDe(estado, id)?.impressorasMarcadas).toEqual([]);
  });

  it("aba desconhecida nao muda nada", () => {
    const estado = comAbas("a.pdf");
    expect(reduzirAbas(estado, { tipo: "alternar_impressora", id: 99, impressora: "x" })).toBe(estado);
    expect(reduzirAbas(estado, { tipo: "definir_lote", id: 99, tamanho: 5 })).toBe(estado);
    expect(reduzirAbas(estado, { tipo: "paginas_contadas", id: 99, total: 5 })).toBe(estado);
    expect(reduzirAbas(estado, { tipo: "paginas_recusadas", id: 99, motivo: "x" })).toBe(estado);
  });
});

describe("lote e paginas por aba", () => {
  it("o lote e de cada aba e nunca sai do intervalo", () => {
    let estado = comAbas("a.pdf", "b.pdf");
    const [a, b] = [idDa(estado, 0), idDa(estado, 1)];
    estado = reduzirAbas(estado, { tipo: "definir_lote", id: a, tamanho: 25 });
    expect(abaDe(estado, a)?.tamanhoLote).toBe(25);
    expect(abaDe(estado, b)?.tamanhoLote).toBe(10);
    estado = reduzirAbas(estado, { tipo: "definir_lote", id: a, tamanho: 500 });
    expect(abaDe(estado, a)?.tamanhoLote).toBe(50);
    estado = reduzirAbas(estado, { tipo: "definir_lote", id: a, tamanho: 0 });
    expect(abaDe(estado, a)?.tamanhoLote).toBe(1);
    const antes = estado;
    estado = reduzirAbas(estado, { tipo: "definir_lote", id: a, tamanho: Number.NaN });
    expect(estado).toBe(antes);
  });

  it("guarda o total ou o motivo da recusa na propria aba", () => {
    let estado = comAbas("a.pdf", "b.pdf");
    const [a, b] = [idDa(estado, 0), idDa(estado, 1)];
    estado = reduzirAbas(estado, { tipo: "paginas_contadas", id: a, total: 24 });
    estado = reduzirAbas(estado, { tipo: "paginas_recusadas", id: b, motivo: "criptografado" });
    expect(abaDe(estado, a)?.paginas).toEqual({ estado: "conhecido", total: 24 });
    expect(abaDe(estado, b)?.paginas).toEqual({ estado: "recusado", motivo: "criptografado" });
  });

  it("resposta que chega depois de a aba fechar e ignorada", () => {
    let estado = comAbas("a.pdf");
    const id = idDa(estado, 0);
    estado = reduzirAbas(estado, { tipo: "pedir_fechamento", id });
    const fechada = estado;
    expect(reduzirAbas(estado, { tipo: "paginas_contadas", id, total: 3 })).toBe(fechada);
  });
});

describe("documento da aba mantem a identidade ao mexer em marcas e cartoes", () => {
  it("cartao em andamento e troca de marcas deixam o mesmo objeto de documento, que chaveia o estado de leitura", () => {
    let estado = comAbas("a.pdf");
    const id = idDa(estado, 0);
    const antes = abaDe(estado, id)?.documento;
    const cartao: Cartao = {
      aba: id,
      impressora: "simulada:gaveta",
      nome: "gaveta",
      simulada: true,
      lote: 1,
      total: 3,
      papel: null,
      estado: { tipo: "preparando" },
      recolocar: null,
      trocaDePapel: null,
      resultado: null,
      sequencia: null,
      viraParaLer: false,
    };
    estado = reduzirAbas(estado, { tipo: "cartao", cartao });
    estado = reduzirAbas(estado, { tipo: "alternar_impressora", id, impressora: "simulada:gaveta" });
    expect(abaDe(estado, id)?.documento).toBe(antes);
  });
});
