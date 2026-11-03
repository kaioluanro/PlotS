import { describe, expect, it } from "vitest";
import {
  ATRIBUTO_DA_LINHA,
  criarRolagensAdiadas,
  espacoAoSumir,
  type SaidaDoPonteiro,
  posicaoDoCartao,
  rolarAteOCartao,
  type ListaQueRola,
  type PosicaoDoCartao,
} from "./cartoes-gesto";

// Os CSS de modulo nao voltam como texto sob o teste: a folha e lida do disco.
interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;

class ListaFalsa implements ListaQueRola {
  ouvintes = new Map<string, Set<(evento?: SaidaDoPonteiro) => void>>();
  constructor(public ponteiroSobre: boolean) {}
  matches(seletor: string): boolean {
    return seletor === ":hover" && this.ponteiroSobre;
  }
  addEventListener(tipo: "pointerleave" | "scroll", ouvinte: (evento?: SaidaDoPonteiro) => void): void {
    const lista = this.ouvintes.get(tipo) ?? new Set();
    lista.add(ouvinte);
    this.ouvintes.set(tipo, lista);
  }
  removeEventListener(tipo: "pointerleave" | "scroll", ouvinte: (evento?: SaidaDoPonteiro) => void): void {
    this.ouvintes.get(tipo)?.delete(ouvinte);
  }
  disparar(tipo: "pointerleave" | "scroll", evento?: SaidaDoPonteiro): void {
    for (const ouvinte of [...(this.ouvintes.get(tipo) ?? [])]) ouvinte(evento);
  }
  ativos(): number {
    return [...this.ouvintes.values()].reduce((total, lista) => total + lista.size, 0);
  }
}

function cena(linhaTemFoco: () => boolean = () => false) {
  const registro = criarRolagensAdiadas(linhaTemFoco);
  const lista = new ListaFalsa(true);
  const pedir = (posicao: PosicaoDoCartao = "acima") => {
    const marca = { rolou: 0, focou: 0, posicao };
    const desistir = rolarAteOCartao(
      lista,
      () => (marca.rolou += 1),
      { posicao: () => marca.posicao, focar: () => (marca.focou += 1) },
      registro,
    );
    return { marca, desistir };
  };
  return { registro, lista, pedir };
}

describe("linha da rolagem adiada", () => {
  it("sem ponteiro sobre a lista, desce na hora e a linha nao aparece", () => {
    const registro = criarRolagensAdiadas();
    let rolou = 0;
    rolarAteOCartao(
      new ListaFalsa(false),
      () => (rolou += 1),
      { posicao: () => "acima", focar: () => {} },
      registro,
    );
    expect(rolou).toBe(1);
    expect(registro.quantas()).toBe(0);
  });

  it("cartao que ja esta em vista nao adia nada nem mostra a linha", () => {
    const { registro, lista, pedir } = cena();
    const { marca } = pedir("em_vista");
    expect(registro.quantas()).toBe(0);
    expect(lista.ativos()).toBe(0);
    expect(marca.rolou).toBe(0);
  });

  it("conta um e varios cartoes, e avisa quem assina", () => {
    const { registro, pedir } = cena();
    let avisos = 0;
    const sair = registro.assinar(() => (avisos += 1));
    pedir();
    expect(registro.quantas()).toBe(1);
    pedir("abaixo");
    expect(registro.quantas()).toBe(2);
    expect(avisos).toBe(2);
    sair();
    pedir();
    expect(avisos).toBe(2);
  });

  it("o clique desce ate o primeiro cartao e leva o foco; sem outro pendente, a linha some", () => {
    const { registro, pedir } = cena();
    const { marca } = pedir();
    registro.irParaAPrimeira();
    expect(marca).toMatchObject({ rolou: 1, focou: 1 });
    expect(registro.quantas()).toBe(0);
  });

  it("com outro pendente, o clique tira da linha so o cartao de destino", () => {
    const { registro, pedir } = cena();
    const primeiro = pedir();
    const segundo = pedir();
    registro.irParaAPrimeira();
    expect(primeiro.marca).toMatchObject({ rolou: 1, focou: 1 });
    expect(segundo.marca).toMatchObject({ rolou: 0, focou: 0 });
    expect(registro.quantas()).toBe(1);
    registro.irParaAPrimeira();
    expect(segundo.marca).toMatchObject({ rolou: 1, focou: 1 });
    expect(registro.quantas()).toBe(0);
  });

  it("depois do clique, sair da linha ou da lista nao tira o primeiro cartao de vista", () => {
    const { registro, lista, pedir } = cena();
    pedir();
    const segundo = pedir();
    registro.irParaAPrimeira();
    lista.ponteiroSobre = false;
    registro.ponteiroSaiuDaLinha();
    lista.disparar("pointerleave");
    expect(segundo.marca.rolou).toBe(0);
    expect(registro.quantas()).toBe(1);
  });

  it("o pendente que ficou na linha ainda sai dela ao chegar em vista ou deixar de pedir acao", () => {
    const { registro, lista, pedir } = cena();
    pedir();
    const segundo = pedir();
    const terceiro = pedir();
    registro.irParaAPrimeira();
    expect(registro.quantas()).toBe(2);
    segundo.marca.posicao = "em_vista";
    lista.disparar("scroll");
    expect(registro.quantas()).toBe(1);
    terceiro.desistir();
    expect(registro.quantas()).toBe(0);
  });

  it("cartao novo depois do clique espera o ponteiro sair, como qualquer outro", () => {
    const { registro, lista, pedir } = cena();
    pedir();
    registro.irParaAPrimeira();
    const novo = pedir();
    lista.disparar("pointerleave");
    expect(novo.marca.rolou).toBe(1);
  });

  it("no clique duplo com ponteiro, o segundo clique nao aciona nada", () => {
    const { registro, pedir } = cena();
    const primeiro = pedir();
    const segundo = pedir();
    registro.cliqueNaLinha(1);
    registro.cliqueNaLinha(2);
    registro.cliqueNaLinha(3);
    expect(primeiro.marca).toMatchObject({ rolou: 1, focou: 1 });
    expect(segundo.marca).toMatchObject({ rolou: 0, focou: 0 });
    expect(registro.quantas()).toBe(1);
  });

  it("pelo teclado, o clique tambem leva ao cartao e tira ele da linha", () => {
    const { registro, pedir } = cena();
    const { marca } = pedir();
    registro.cliqueNaLinha(0);
    expect(marca).toMatchObject({ rolou: 1, focou: 1 });
    expect(registro.quantas()).toBe(0);
  });

  it("clique sem nada na linha nao faz nada", () => {
    expect(() => criarRolagensAdiadas().cliqueNaLinha(1)).not.toThrow();
  });

  it("quem assina e avisado com a saida ja contada, para ler a linha antes de ela sumir", () => {
    const { registro, pedir } = cena();
    const { desistir } = pedir();
    const vistos: number[] = [];
    registro.assinar(() => vistos.push(registro.quantas()));
    desistir();
    expect(vistos).toEqual([0]);
  });

  it("linha que some com foco leva o foco ao cartao dela", () => {
    const { registro, lista, pedir } = cena(() => true);
    const { marca, desistir } = pedir();
    desistir();
    expect(marca.focou).toBe(1);
    expect(lista.ativos()).toBe(0);
    expect(registro.quantas()).toBe(0);
  });

  it("linha que some com foco ao chegar em vista tambem leva o foco, uma vez so", () => {
    const { lista, pedir } = cena(() => true);
    const { marca, desistir } = pedir("abaixo");
    marca.posicao = "em_vista";
    lista.disparar("scroll");
    expect(marca.focou).toBe(1);
    desistir();
    expect(marca.focou).toBe(1);
  });

  it("enquanto a linha continua, o foco fica nela; sai junto com o ultimo cartao", () => {
    const { pedir } = cena(() => true);
    const primeiro = pedir();
    const segundo = pedir();
    primeiro.desistir();
    expect(primeiro.marca.focou).toBe(0);
    segundo.desistir();
    expect(segundo.marca.focou).toBe(1);
  });

  it("linha que some sem foco nao mexe no foco de ninguem", () => {
    const { pedir } = cena(() => false);
    const { marca, desistir } = pedir();
    desistir();
    expect(marca.focou).toBe(0);
  });

  it("rolar a mao ate o cartao tira ele da linha, e rolar sem chegar nele nao", () => {
    const { registro, lista, pedir } = cena();
    const { marca } = pedir("abaixo");
    lista.disparar("scroll");
    expect(registro.quantas()).toBe(1);
    marca.posicao = "em_vista";
    lista.disparar("scroll");
    expect(registro.quantas()).toBe(0);
  });

  it("depois de sair da linha, um cartao novo a traz de volta contando so ele", () => {
    const { registro, lista, pedir } = cena();
    const { marca } = pedir("abaixo");
    marca.posicao = "em_vista";
    lista.disparar("scroll");
    pedir("abaixo");
    expect(registro.quantas()).toBe(1);
  });

  it("o ponteiro saindo desce, tira da linha e nao deixa ouvinte para tras", () => {
    const { registro, lista, pedir } = cena();
    const { marca } = pedir();
    lista.disparar("pointerleave");
    expect(marca.rolou).toBe(1);
    expect(registro.quantas()).toBe(0);
    expect(lista.ativos()).toBe(0);
  });

  it("ir da lista para a linha nao desce: o botao continua la para o clique", () => {
    const { registro, lista, pedir } = cena();
    const { marca } = pedir();
    const linha = { closest: (seletor: string) => seletor === `[${ATRIBUTO_DA_LINHA}]` };
    lista.disparar("pointerleave", { relatedTarget: linha });
    expect(marca.rolou).toBe(0);
    expect(registro.quantas()).toBe(1);
  });

  it("sair da linha para fora da lista desce; voltar para a lista nao", () => {
    const { registro, lista, pedir } = cena();
    const { marca } = pedir();
    registro.ponteiroSaiuDaLinha();
    expect(marca.rolou).toBe(0);
    lista.ponteiroSobre = false;
    registro.ponteiroSaiuDaLinha();
    expect(marca.rolou).toBe(1);
    expect(registro.quantas()).toBe(0);
  });

  it("cartao que deixa de pedir acao sai da linha", () => {
    const { registro, lista, pedir } = cena();
    const { desistir, marca } = pedir();
    desistir();
    expect(registro.quantas()).toBe(0);
    expect(lista.ativos()).toBe(0);
    lista.disparar("pointerleave");
    expect(marca.rolou).toBe(0);
  });

  it("sem nada adiado, o clique nao faz nada", () => {
    const registro = criarRolagensAdiadas();
    expect(() => registro.irParaAPrimeira()).not.toThrow();
    expect(registro.posicaoDaPrimeira()).toBe("em_vista");
  });

  it("diz para que lado esta o primeiro cartao, e a seta acompanha a rolagem", () => {
    const { registro, lista, pedir } = cena();
    const { marca } = pedir("abaixo");
    pedir("acima");
    expect(registro.posicaoDaPrimeira()).toBe("abaixo");
    let avisos = 0;
    registro.assinar(() => (avisos += 1));
    marca.posicao = "acima";
    lista.disparar("scroll");
    expect(avisos).toBeGreaterThan(0);
    expect(registro.posicaoDaPrimeira()).toBe("acima");
  });
});

describe("espaco que fica quando a linha some", () => {
  it("com o ponteiro sobre a linha, fica com a altura dela, seja qual for a causa", () => {
    expect(espacoAoSumir(0, { sobPonteiro: true, altura: 28 })).toBe(28);
    expect(espacoAoSumir(0, { sobPonteiro: true, altura: 44 })).toBe(44);
  });

  it("sem o ponteiro sobre a linha, some na hora", () => {
    expect(espacoAoSumir(0, { sobPonteiro: false, altura: 28 })).toBeNull();
  });

  it("enquanto a linha existe, nao ha espaco", () => {
    expect(espacoAoSumir(1, { sobPonteiro: true, altura: 28 })).toBeNull();
  });
});

describe("aparencia da linha", () => {
  const folha = fs.readFileSync(
    new URL("./components/LinhaRolagemAdiada.module.css", import.meta.url),
    "utf8",
  );

  it("no hover, usa o fundo suave do destaque e sublinha o texto", () => {
    const inicio = folha.indexOf(".linha:hover {");
    expect(inicio).toBeGreaterThan(-1);
    const regra = folha.slice(inicio, folha.indexOf("}", inicio));
    expect(regra).toMatch(/background:\s*var\(--accent-bg-soft\)/);
    expect(regra).toMatch(/text-decoration:\s*underline/);
  });

  it("o espaco inerte guarda os 28 px, sem texto e sem responder ao ponteiro como botao", () => {
    const inicio = folha.indexOf(".espaco {");
    expect(inicio).toBeGreaterThan(-1);
    const regra = folha.slice(inicio, folha.indexOf("}", inicio));
    expect(regra).toMatch(/min-height:\s*28px/);
    expect(regra).toMatch(/flex-shrink:\s*0/);
    expect(regra).not.toMatch(/cursor:\s*pointer/);
  });
});

describe("posicao do cartao em relacao a lista", () => {
  const lista = { top: 100, bottom: 300 };
  it.each([
    ["inteiro dentro", { top: 120, bottom: 280 }, "em_vista"],
    ["encostado nas bordas", { top: 100, bottom: 300 }, "em_vista"],
    ["cortado em cima", { top: 80, bottom: 200 }, "acima"],
    ["todo acima", { top: 0, bottom: 90 }, "acima"],
    ["cortado embaixo", { top: 250, bottom: 350 }, "abaixo"],
    ["todo abaixo", { top: 310, bottom: 400 }, "abaixo"],
    ["mais alto que a lista, com o topo a vista", { top: 110, bottom: 500 }, "em_vista"],
    ["mais alto que a lista, com o topo fora, acima", { top: 20, bottom: 400 }, "acima"],
    ["mais alto que a lista, com o topo fora, abaixo", { top: 310, bottom: 800 }, "abaixo"],
  ] as const)("%s", (_, cartao, esperado) => {
    expect(posicaoDoCartao(cartao, lista)).toBe(esperado);
  });
});
