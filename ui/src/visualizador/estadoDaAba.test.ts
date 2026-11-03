import { describe, expect, it } from "vitest";
import { guardarEstadoDaAba, lerEstadoDaAba, tamanhosDaLeitura } from "./estadoDaAba";

const carta = { largura: 612, altura: 792 };
const paisagem = { largura: 1000, altura: 500 };
const estado = {
  ancora: { pagina: 7, fracao: 0.25, folga: 0 },
  ajustando: false,
  zoomPedido: 1.5,
  tamanhos: [carta, paisagem],
};

describe("estado da leitura por aba", () => {
  it("devolve a pagina, o zoom, o modo de ajuste e as medidas que a aba guardou", () => {
    const aba = { caminho: "/a.pdf", nome: "a.pdf" };
    guardarEstadoDaAba(aba, estado);
    expect(lerEstadoDaAba(aba)).toEqual(estado);
  });

  it("uma aba nova, mesmo com o mesmo arquivo, comeca sem estado guardado", () => {
    const aba = { caminho: "/a.pdf", nome: "a.pdf" };
    guardarEstadoDaAba(aba, estado);
    expect(lerEstadoDaAba({ caminho: "/a.pdf", nome: "a.pdf" })).toBeUndefined();
  });
});

describe("tamanhos das paginas ao voltar para a aba", () => {
  it("volta com as medidas ja lidas, para o zoom de ajuste nao saltar depois", () => {
    expect(tamanhosDaLeitura(estado, 2, carta)).toEqual([carta, paisagem]);
  });

  it("sem medidas guardadas, ou com outro numero de paginas, repete a primeira pagina", () => {
    expect(tamanhosDaLeitura(undefined, 3, carta)).toEqual([carta, carta, carta]);
    expect(tamanhosDaLeitura(estado, 3, carta)).toEqual([carta, carta, carta]);
  });
});
