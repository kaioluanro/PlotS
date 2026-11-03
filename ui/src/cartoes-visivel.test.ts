import { describe, expect, it } from "vitest";
import barraFonte from "./components/TitleBar.tsx?raw";
import sidebarFonte from "./components/Sidebar.tsx?raw";
import cartaoFonte from "./components/CartaoImpressao.tsx?raw";

// Os CSS de modulo nao voltam como texto com ?raw nem ?inline sob o teste: a folha e lida do disco.
interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;

function css(nome: string): string {
  return fs.readFileSync(new URL(`./components/${nome}`, import.meta.url), "utf8");
}

const barraCss = css("TitleBar.module.css");
const sidebarCss = css("Sidebar.module.css");
import {
  alinhamentoDoCartao,
  cartaoRecusado,
  impressorasOcupadas,
  marcadasTodasOcupadas,
  type Cartao,
  type EstadoCartao,
} from "./cartoes";
import type { Impressora } from "./impressoras";
import { estadoInicial, reduzirAbas, type EstadoAbas } from "./tabs";

function fila(id: string): Impressora {
  return {
    id: `fila:${id}`,
    nome: id,
    tipo: "fila",
    nivel: "pronta",
    condicoes: [],
    modo: "jato",
  };
}

function cartaoNaAba(aba: number, estado: EstadoCartao, impressora: Impressora): Cartao {
  return {
    aba,
    impressora: impressora.id,
    nome: impressora.nome,
    simulada: false,
    lote: 1,
    total: 2,
    papel: { formato: "a4", larguraMm: 210, alturaMm: 297 },
    estado,
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
  };
}

function abaAberta(): { estado: EstadoAbas; id: number } {
  const estado = reduzirAbas(estadoInicial, {
    tipo: "abrir",
    documento: { caminho: "C:\\a.pdf", nome: "a.pdf" },
  });
  return { estado, id: estado.ativa ?? -1 };
}

function cartoesDaAba(estado: EstadoAbas, id: number): readonly Cartao[] {
  return estado.abas.find((aba) => aba.id === id)?.cartoes ?? [];
}

describe("recusa de inicio nao substitui o cartao vivo", () => {
  it("uma recusa ocupada nao troca o cartao em andamento da mesma impressora na aba", () => {
    const { estado: inicial, id } = abaAberta();
    const vivo = cartaoNaAba(id, { tipo: "preparando" }, fila("A"));
    const comVivo = reduzirAbas(inicial, { tipo: "cartao", cartao: vivo });
    const depois = reduzirAbas(comVivo, { tipo: "cartao", cartao: cartaoRecusado(id, fila("A")) });
    expect(cartoesDaAba(depois, id)).toEqual([vivo]);
    expect([...impressorasOcupadas(depois.abas)]).toEqual(["fila:A"]);
  });

  it("sem cartao vivo, a recusa aparece na aba como antes", () => {
    const { estado: inicial, id } = abaAberta();
    const depois = reduzirAbas(inicial, { tipo: "cartao", cartao: cartaoRecusado(id, fila("A")) });
    expect(cartoesDaAba(depois, id)).toEqual([cartaoRecusado(id, fila("A"))]);
  });

  it("cartao vivo de outra impressora nao impede a recusa de aparecer", () => {
    const { estado: inicial, id } = abaAberta();
    const vivo = cartaoNaAba(id, { tipo: "aguardando" }, fila("A"));
    const comVivo = reduzirAbas(inicial, { tipo: "cartao", cartao: vivo });
    const depois = reduzirAbas(comVivo, { tipo: "cartao", cartao: cartaoRecusado(id, fila("B")) });
    expect(cartoesDaAba(depois, id)).toEqual([vivo, cartaoRecusado(id, fila("B"))]);
  });
});

describe("gesto visivel em altura curta", () => {
  it("cartao mais alto que a lista comeca pelo topo dele", () => {
    expect(alinhamentoDoCartao(300, 200)).toBe("start");
  });

  it("cartao que cabe na lista so desce o necessario", () => {
    expect(alinhamentoDoCartao(120, 200)).toBe("nearest");
    expect(alinhamentoDoCartao(200, 200)).toBe("nearest");
  });

  it("rodape some so quando todas as marcadas da lista estao ocupadas", () => {
    const a = fila("A");
    const b = fila("B");
    expect(marcadasTodasOcupadas([a, b], [a.id, b.id], new Set([a.id, b.id]))).toBe(true);
    expect(marcadasTodasOcupadas([a, b], [a.id, b.id], new Set([a.id]))).toBe(false);
    expect(marcadasTodasOcupadas([a, b], [], new Set([a.id]))).toBe(false);
    expect(marcadasTodasOcupadas([a], ["fila:sumiu"], new Set())).toBe(false);
    expect(marcadasTodasOcupadas([a], [a.id, "fila:sumiu"], new Set([a.id]))).toBe(true);
  });

  it("a lateral marca o rodape que o CSS oculta em altura curta, e o cartao usa a rolagem", () => {
    expect(sidebarFonte).toContain("data-sem-destino");
    const curto = sidebarCss.slice(sidebarCss.lastIndexOf("@media (max-height: 560px)"));
    expect(curto).toMatch(/\.lateral\[data-sem-destino\] \.rodape\s*\{\s*display: none;\s*\}/);
    expect(cartaoFonte).toContain("alinhamentoDoCartao");
  });
});

describe("pontos de estado com cores forcadas", () => {
  it("cheio pede acao e vazado segue em curso, nas abas inativa e ativa", () => {
    const bloco = barraCss.slice(barraCss.lastIndexOf("@media (forced-colors: active)"));
    expect(bloco).toMatch(/\.pontoAba \{\s*forced-color-adjust: none;\s*\}/);
    expect(bloco).toMatch(/\.pontoPedeAcao \{\s*background: CanvasText;\s*\}/);
    expect(bloco).toMatch(/\.pontoEmCurso \{\s*border-color: CanvasText;\s*\}/);
    expect(bloco).toMatch(/\.abaAtiva \.pontoPedeAcao \{\s*background: HighlightText;\s*\}/);
    expect(bloco).toMatch(/\.abaAtiva \.pontoEmCurso \{\s*border-color: HighlightText;\s*\}/);
  });

  it("as classes do bloco existem no CSS e o componente as usa", () => {
    for (const nome of ["pontoAba", "pontoPedeAcao", "pontoEmCurso", "abaAtiva"]) {
      expect(barraCss).toMatch(new RegExp(`\\.${nome} \\{`));
      expect(barraFonte).toContain(`styles.${nome}`);
    }
  });
});
