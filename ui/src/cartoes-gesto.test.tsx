import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  desenhoDoCartao,
  desenhoDoGesto,
  rolarAteOCartao,
  type ListaQueRola,
} from "./cartoes-gesto";
import type { Cartao, EstadoCartao, Recolocar } from "./cartoes";
import { frasesDoGesto } from "./cartoes";
import { FiguraGesto } from "./components/FiguraGesto";

function recolocar(lado: string | null, virada: string | null): Recolocar {
  return { lado, virada, textoDoPerfil: null };
}

// Virar troca a face de lado; nao virar e girar no plano mantem. A entrada e sempre o `lado`.
const TABELA = [
  ["baixo", "borda_de_cima", "cima"],
  ["cima", "borda_de_cima", "baixo"],
  ["baixo", "lateral", "cima"],
  ["cima", "lateral", "baixo"],
  ["baixo", "nao_virar", "baixo"],
  ["cima", "nao_virar", "cima"],
  ["baixo", "girar", "baixo"],
  ["cima", "girar", "cima"],
] as const;

function figura(lado: string, virada: string): string {
  const desenho = desenhoDoGesto(recolocar(lado, virada));
  if (desenho === null) throw new Error(`sem desenho para ${lado}, ${virada}`);
  return renderToStaticMarkup(<FiguraGesto desenho={desenho} />);
}

function cartaoAguardando(estado: EstadoCartao, valor: Recolocar | null): Cartao {
  return {
    aba: 1,
    impressora: "fila:a",
    nome: "a",
    simulada: false,
    lote: 1,
    total: 1,
    papel: null,
    estado,
    recolocar: valor,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
  };
}

describe("dados da figura do gesto", () => {
  it.each(TABELA)("entrada %s com %s saiu com a face para %s", (lado, virada, saida) => {
    expect(desenhoDoGesto(recolocar(lado, virada))).toEqual({ saida, gesto: virada, entrada: lado });
  });

  it("os dois modelos seguem a tabela deles", () => {
    // Jato: sai para cima, vira pela borda de cima, entra para baixo.
    expect(desenhoDoGesto(recolocar("baixo", "borda_de_cima"))).toEqual({
      saida: "cima",
      gesto: "borda_de_cima",
      entrada: "baixo",
    });
    // Laser: sai para baixo, nao vira, entra para baixo.
    expect(desenhoDoGesto(recolocar("baixo", "nao_virar"))).toEqual({
      saida: "baixo",
      gesto: "nao_virar",
      entrada: "baixo",
    });
  });

  it.each([
    ["lado nulo", null, "borda_de_cima"],
    ["virada nula", "baixo", null],
    ["os dois nulos", null, null],
    ["lado desconhecido", "lado_novo", "lateral"],
    ["virada desconhecida", "baixo", "dobrar"],
    ["texto vazio", "", ""],
    ["nome herdado do objeto", "baixo", "constructor"],
  ])("sem figura com %s", (_, lado, virada) => {
    expect(desenhoDoGesto(recolocar(lado, virada))).toBeNull();
  });

  it("sem recolocacao nao ha figura", () => {
    expect(desenhoDoGesto(null)).toBeNull();
  });
});

describe("desenho de cada combinacao", () => {
  it("as oito combinacoes tem desenhos diferentes", () => {
    const desenhos = new Set(TABELA.map(([lado, virada]) => figura(lado, virada)));
    expect(desenhos.size).toBe(TABELA.length);
  });

  it.each(TABELA)("%s com %s desenha o gesto e as duas faces certas", (lado, virada, saida) => {
    const svg = figura(lado, virada);
    const inicio = svg.indexOf(`<g data-saida="${saida}">`);
    const entrada = svg.indexOf(`<g data-entrada="${lado}">`);
    expect(inicio).toBeGreaterThan(-1);
    expect(entrada).toBeGreaterThan(inicio);
    // So o gesto pedido aparece.
    const gestos = [...svg.matchAll(/<g data-gesto="([a-z_]+)"/g)].map((achado) => achado[1]);
    expect(gestos).toEqual([virada]);
    // Face para cima mostra as linhas; face para baixo, o canto levantado com a tinta embaixo.
    const marcaEsperada = (face: string) => (face === "cima" ? "linhas" : "canto");
    const marcas = [...svg.matchAll(/data-marca="([a-z]+)"/g)].map((achado) => achado[1]);
    expect(marcas).toEqual([marcaEsperada(saida), marcaEsperada(lado)]);
  });

  it("a cena da entrada usa o maco visto de cima, com a seta saindo da borda curta ate a impressora", () => {
    for (const [lado, virada] of TABELA) {
      const svg = figura(lado, virada);
      expect(svg).toContain('viewBox="0 0 196 84"');
      expect(svg).toContain('d="M118 27V18H149"');
      expect(svg).toContain('x="152" y="11"');
      // A folha de cima da cena da entrada comeca em x=104, y=30.
      const cena = svg.slice(svg.indexOf(`<g data-entrada="${lado}">`));
      expect(cena).toMatch(/M104 30H132/);
    }
  });

  it("as linhas da entrada giram 180 graus so com a face para cima e um gesto que gira a folha", () => {
    const giradas = new Set(["cima girar", "cima borda_de_cima"]);
    for (const [lado, virada] of TABELA) {
      const svg = figura(lado, virada);
      const corte = svg.indexOf(`<g data-entrada="${lado}">`);
      const nome = `${lado} ${virada}`;
      expect(svg.slice(corte).includes('transform="rotate(180 118 48)"'), nome).toBe(giradas.has(nome));
      // A cena de saida nunca gira: o maco ainda esta como saiu.
      expect(svg.slice(0, corte), nome).not.toContain("rotate(");
    }
  });

  it("nao virar usa o mesmo arco de virar, com o X no meio e o maco no mesmo lugar", () => {
    const arco = 'd="M63 66C80 46 76 12 43 12"';
    expect(figura("baixo", "borda_de_cima")).toContain(arco);
    const svg = figura("baixo", "nao_virar");
    expect(svg).toContain(arco);
    expect(svg).toContain('d="M26 40H54');
    // O meio da curva: (71.75, 31.5).
    expect(svg).toContain('d="M65.75 25.5L77.75 37.5"');
    expect(svg).toContain('d="M77.75 25.5L65.75 37.5"');
  });

  it("a figura nao tem texto e fica fora do leitor de tela", () => {
    for (const [lado, virada] of TABELA) {
      const svg = figura(lado, virada);
      expect(svg).toContain('aria-hidden="true"');
      expect(svg).toContain('focusable="false"');
      expect(svg).not.toMatch(/<text|<title|<desc|<animate|role=/);
    }
  });

  it("a figura concorda com a frase do cartao", () => {
    for (const [lado, virada] of TABELA) {
      const frases = frasesDoGesto(recolocar(lado, virada), "pt-BR");
      expect(frases.at(-1)).toContain(`face impressa para ${lado}`);
    }
  });
});

describe("quando o cartao mostra a figura", () => {
  const valor = recolocar("baixo", "nao_virar");

  it("so enquanto espera a recolocacao", () => {
    expect(desenhoDoCartao(cartaoAguardando({ tipo: "aguardando" }, valor))).not.toBeNull();
    const outros: EstadoCartao[] = [
      { tipo: "preparando" },
      { tipo: "imprimindo", passada: "frente" },
      { tipo: "imprimindo", passada: "verso" },
      { tipo: "proximo_lote" },
      {
        tipo: "aviso",
        passada: "frente",
        motivo: { codigo: "fila", texto: "a impressora está sem papel" },
        respondido: false,
      },
      { tipo: "erro", motivo: { codigo: "fila", texto: "" } },
      { tipo: "concluido" },
      { tipo: "cancelado" },
    ];
    for (const estado of outros) {
      expect(desenhoDoCartao(cartaoAguardando(estado, valor))).toBeNull();
    }
  });

  it("fila nao calibrada espera sem figura", () => {
    const cartao = cartaoAguardando({ tipo: "aguardando" }, recolocar(null, null));
    expect(desenhoDoCartao(cartao)).toBeNull();
    expect(frasesDoGesto(recolocar(null, null), "pt-BR").length).toBeGreaterThan(0);
  });
});

class ListaFalsa implements ListaQueRola {
  ouvintes = new Set<() => void>();
  constructor(public ponteiroSobre: boolean) {}
  matches(seletor: string): boolean {
    return seletor === ":hover" && this.ponteiroSobre;
  }
  addEventListener(_: "pointerleave", ouvinte: () => void): void {
    this.ouvintes.add(ouvinte);
  }
  removeEventListener(_: "pointerleave", ouvinte: () => void): void {
    this.ouvintes.delete(ouvinte);
  }
  sair(): void {
    this.ponteiroSobre = false;
    for (const ouvinte of [...this.ouvintes]) ouvinte();
  }
}

describe("descer ate o cartao", () => {
  it("sem o ponteiro sobre a lista, desce na hora", () => {
    let vezes = 0;
    rolarAteOCartao(new ListaFalsa(false), () => (vezes += 1));
    expect(vezes).toBe(1);
  });

  it("sem lista que role, desce na hora", () => {
    let vezes = 0;
    rolarAteOCartao(null, () => (vezes += 1));
    expect(vezes).toBe(1);
  });

  it("com o ponteiro sobre a lista, espera ele sair e desce uma vez so", () => {
    let vezes = 0;
    const lista = new ListaFalsa(true);
    rolarAteOCartao(lista, () => (vezes += 1));
    expect(vezes).toBe(0);
    lista.sair();
    expect(vezes).toBe(1);
    expect(lista.ouvintes.size).toBe(0);
    lista.sair();
    expect(vezes).toBe(1);
  });

  it("se o cartao deixar de pedir acao, a descida adiada e esquecida", () => {
    let vezes = 0;
    const lista = new ListaFalsa(true);
    const desistir = rolarAteOCartao(lista, () => (vezes += 1));
    desistir();
    lista.sair();
    expect(vezes).toBe(0);
    expect(lista.ouvintes.size).toBe(0);
  });
});
