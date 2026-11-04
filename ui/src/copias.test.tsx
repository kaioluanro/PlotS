import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { CampoCopias } from "./components/CampoCopias";
import { conteudoDoVerso } from "./face-do-envio";
import type { ResumoDoEnvio } from "./cartoes";
import type { Impressora } from "./impressoras";
import { copiasDaFace, ehQuantidadeDeCopias, reduzirAbas, type Aba, type EstadoAbas } from "./tabs";

const DOC = { caminho: "x.pdf", paginas: 4 } as unknown as Aba["documento"];

function estadoComAba(): EstadoAbas {
  const base: EstadoAbas = { abas: [], ativa: null, proximoId: 1, aguardandoConfirmacao: null, confirmandoJanela: false };
  return reduzirAbas(base, { tipo: "abrir", documento: DOC });
}

function copiasDe(estado: EstadoAbas) {
  const aba = estado.abas[0] as Aba;
  return { frente: copiasDaFace(aba, "frente"), verso: copiasDaFace(aba, "verso") };
}

describe("quantidade de copias: redutor", () => {
  it("comeca com uma copia em cada face", () => {
    expect(copiasDe(estadoComAba())).toEqual({ frente: 1, verso: 1 });
  });

  it("editar a frente leva o numero ao verso", () => {
    const id = 1;
    const estado = reduzirAbas(estadoComAba(), { tipo: "definir_copias", id, face: "frente", valor: 3 });
    expect(copiasDe(estado)).toEqual({ frente: 3, verso: 3 });
  });

  it("editar o verso nao muda a frente", () => {
    const estado = reduzirAbas(estadoComAba(), { tipo: "definir_copias", id: 1, face: "verso", valor: 5 });
    expect(copiasDe(estado)).toEqual({ frente: 1, verso: 5 });
  });

  it("editar a frente depois realinha o verso ao novo numero", () => {
    let estado = reduzirAbas(estadoComAba(), { tipo: "definir_copias", id: 1, face: "verso", valor: 5 });
    estado = reduzirAbas(estado, { tipo: "definir_copias", id: 1, face: "frente", valor: 2 });
    expect(copiasDe(estado)).toEqual({ frente: 2, verso: 2 });
  });

  it("trocar de face nao muda nenhum dos dois numeros", () => {
    let estado = reduzirAbas(estadoComAba(), { tipo: "definir_copias", id: 1, face: "frente", valor: 4 });
    estado = reduzirAbas(estado, { tipo: "definir_copias", id: 1, face: "verso", valor: 7 });
    const antes = copiasDe(estado);
    estado = reduzirAbas(estado, { tipo: "definir_face", id: 1, face: "verso" });
    expect(copiasDe(estado)).toEqual(antes);
    estado = reduzirAbas(estado, { tipo: "definir_face", id: 1, face: "frente" });
    expect(copiasDe(estado)).toEqual(antes);
  });

  it("valores fora de 1 a 999 e fracionarios nao mudam nada", () => {
    const com4 = reduzirAbas(estadoComAba(), { tipo: "definir_copias", id: 1, face: "frente", valor: 4 });
    for (const valor of [0, 1000, -1, 1.5, Number.NaN, Number.POSITIVE_INFINITY]) {
      const estado = reduzirAbas(com4, { tipo: "definir_copias", id: 1, face: "frente", valor });
      expect(copiasDe(estado)).toEqual({ frente: 4, verso: 4 });
    }
  });

  it("os limites 1 e 999 valem, e 1000 nao", () => {
    expect([ehQuantidadeDeCopias(0), ehQuantidadeDeCopias(1), ehQuantidadeDeCopias(999), ehQuantidadeDeCopias(1000)]).toEqual([
      false,
      true,
      true,
      false,
    ]);
    const com999 = reduzirAbas(estadoComAba(), { tipo: "definir_copias", id: 1, face: "frente", valor: 999 });
    expect(copiasDe(com999)).toEqual({ frente: 999, verso: 999 });
  });

  it("aba sem o campo, de antes, vale uma copia", () => {
    expect(copiasDaFace({} as Pick<Aba, "copias">, "verso")).toBe(1);
  });
});

const IMPRESSORA: Impressora = { id: "fila:Sala", nome: "Sala", tipo: "fila", nivel: "pronta", condicoes: [], modo: "laser" };
const RESUMO = {
  paginas: 3,
  lotes: 1,
  folhas: 6,
  maiorMaco: 6,
  papeis: [{ formato: "a4", larguraMm: 210, alturaMm: 297 }],
  paresMistos: [],
  assinado: false,
  faixas: [],
  gestos: [],
} as unknown as ResumoDoEnvio;

describe("quantidade de copias: confirmacao do verso", () => {
  it("com numeros diferentes, diz os dois e quantas copias o maco tem", () => {
    const conteudo = conteudoDoVerso([IMPRESSORA], [], RESUMO, null, null, false, "pt-BR", { frente: 3, verso: 2 });
    expect(conteudo.aviso).toBe("A Frente está com 3 cópias e o Verso com 2: coloque um maço com 2 cópias.");
    expect(conteudo.linhas).toContain("Cópias: 2");
  });

  it("com um numero, usa o singular nos dois idiomas", () => {
    const pt = conteudoDoVerso([IMPRESSORA], [], RESUMO, null, null, false, "pt-BR", { frente: 2, verso: 1 });
    expect(pt.aviso).toBe("A Frente está com 2 cópias e o Verso com 1: coloque um maço com 1 cópia.");
    const en = conteudoDoVerso([IMPRESSORA], [], RESUMO, null, null, false, "en", { frente: 1, verso: 3 });
    expect(en.aviso).toBe("Front is set to 1 copy and Back to 3: load a stack with 3 copies.");
  });

  it("com numeros iguais, so diz a quantidade", () => {
    const conteudo = conteudoDoVerso([IMPRESSORA], [], RESUMO, null, null, false, "pt-BR", { frente: 3, verso: 3 });
    expect(conteudo.aviso).toBeUndefined();
    expect(conteudo.linhas).toContain("Cópias: 3");
  });
});

describe("quantidade de copias: campo", () => {
  it("mostra o rotulo e o numero da face, em pt-BR e en", () => {
    const pt = renderToStaticMarkup(
      <CampoCopias face="verso" valor={4} aoDefinir={() => undefined} aoMudarValidade={() => undefined} />,
    );
    expect(pt).toContain("Quantidade");
    expect(pt).toContain('value="4"');
    expect(pt).toContain('data-face="verso"');
    expect(pt).not.toContain('role="alert"');
  });
});

describe("documento mudou depois das frentes", () => {
  it("diz o que fazer, com o texto exato, nos dois idiomas", async () => {
    const { textoDoMotivo } = await import("./cartoes");
    expect(textoDoMotivo({ codigo: "documento_mudou", texto: "" }, "pt-BR")).toBe(
      "O arquivo mudou desde o pedido das frentes, e as folhas já impressas podem não corresponder a ele. Imprima as frentes de novo. Para imprimir só o verso mesmo assim, abra o arquivo em uma aba nova: o PlotS não confere o maço.",
    );
    expect(textoDoMotivo({ codigo: "documento_mudou", texto: "" }, "en")).toBe(
      "The file changed since the fronts were requested, and sheets already printed may not match it. Print the fronts again. To print only the back anyway, open the file in a new tab: PlotS does not check the stack.",
    );
  });
});

describe("campo Quantidade", () => {
  const html = renderToStaticMarkup(
    <CampoCopias face="frente" valor={1} aoDefinir={() => undefined} aoMudarValidade={() => undefined} />,
  );

  it("nao limita os caracteres: digitar 12abc nao pode virar 12ab", () => {
    expect(html).not.toMatch(/maxlength/i);
  });

  it("tem rotulo acessivel ligado ao campo", () => {
    const id = /<input\s(?:[^>]*\s)?id="([^"]+)"/.exec(html)?.[1];
    expect(id).toBeDefined();
    expect(html).toContain(`for="${id}"`);
    expect(html).toMatch(/<label[^>]*>(Quantidade|Quantity)<\/label>/);
  });
});
