import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { reduzirAvisos, ESTADO_INICIAL_DOS_AVISOS, type AvisoDoTrabalho } from "./avisos";
import { AvisosDoTopo } from "./components/AvisosDoTopo";
import type { Cartao, EstadoCartao } from "./cartoes";

function cartao(estado: EstadoCartao, lote: number): Cartao {
  return {
    aba: 1,
    impressora: `fila:${lote}`,
    nome: `Impressora ${lote}`,
    simulada: false,
    lote,
    total: 2,
    papel: null,
    estado,
    recolocar: null,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
  };
}

function itensCom(cartoes: Cartao[]): AvisoDoTrabalho[] {
  return cartoes.reduce(
    (estado, cartaoNovo) => reduzirAvisos(estado, { tipo: "publicar", cartao: cartaoNovo, agora: Date.now() }),
    ESTADO_INICIAL_DOS_AVISOS,
  ).itens as AvisoDoTrabalho[];
}

describe("faixa de avisos no topo", () => {
  it("nao mostra nada sem avisos, e nao ocupa altura no fluxo", () => {
    const markup = renderToStaticMarkup(<AvisosDoTopo itens={[]} aoFechar={() => undefined} />);
    expect(markup).not.toContain("data-tom");
  });

  it("so o erro usa o papel de alerta; a faixa e uma regiao viva educada, sem ser so cor", () => {
    const markup = renderToStaticMarkup(
      <AvisosDoTopo
        itens={itensCom([
          cartao({ tipo: "erro", motivo: { codigo: "papel", texto: "" } }, 1),
          cartao({ tipo: "concluido" }, 2),
        ])}
        aoFechar={() => undefined}
      />,
    );
    expect(markup).toMatch(/role="alert"[^>]*data-tom="problema"/);
    expect(markup).toContain('aria-live="polite"');
    expect(markup).toContain('aria-relevant="additions"');
    expect(markup).not.toContain('role="status"');
    expect(markup.match(/role="alert"/g) ?? []).toHaveLength(1);
  });

  it("a regiao viva existe mesmo sem aviso, para anunciar o que chegar depois", () => {
    const markup = renderToStaticMarkup(<AvisosDoTopo itens={[]} aoFechar={() => undefined} />);
    expect(markup).toContain('aria-live="polite"');
    expect(markup).toContain('aria-relevant="additions"');
  });

  it("documento e fila em dois spans, cada um com o titulo completo", () => {
    const itens = reduzirAvisos(ESTADO_INICIAL_DOS_AVISOS, {
      tipo: "publicar",
      cartao: cartao({ tipo: "concluido" }, 1),
      agora: Date.now(),
      documento: "Relatorio-muito-longo-de-2026.pdf",
    }).itens as AvisoDoTrabalho[];
    const markup = renderToStaticMarkup(<AvisosDoTopo itens={itens} aoFechar={() => undefined} />);
    expect(markup).toMatch(/<span class="[^"]*" title="Relatorio-muito-longo-de-2026.pdf">/);
    expect(markup).toMatch(/<span class="[^"]*" title="Impressora 1">/);
  });

  it("preparando, enviando e cancelando usam o circulo que anda, e o concluido nao", () => {
    const girando = (estado: EstadoCartao) =>
      renderToStaticMarkup(
        <AvisosDoTopo itens={itensCom([cartao(estado, 1)])} aoFechar={() => undefined} />,
      ).includes("data-gira");
    expect(girando({ tipo: "preparando" })).toBe(true);
    expect(girando({ tipo: "imprimindo", passada: "frente" })).toBe(true);
    expect(girando({ tipo: "cancelando" })).toBe(true);
    expect(girando({ tipo: "concluido" })).toBe(false);
  });

  it("com um dialogo aberto, a faixa sai da frente sem perder os avisos", () => {
    const itens = itensCom([cartao({ tipo: "erro", motivo: { codigo: "papel", texto: "" } }, 1)]);
    const markup = renderToStaticMarkup(<AvisosDoTopo itens={itens} oculto aoFechar={() => undefined} />);
    expect(markup).not.toContain("data-tom");
    expect(itens).toHaveLength(1);
  });

  it("mostra no maximo tres avisos no topo, mesmo com mais na sessao", () => {
    const markup = renderToStaticMarkup(
      <AvisosDoTopo
        itens={itensCom([
          cartao({ tipo: "concluido" }, 1),
          cartao({ tipo: "concluido" }, 2),
          cartao({ tipo: "concluido" }, 3),
          cartao({ tipo: "concluido" }, 4),
        ])}
        aoFechar={() => undefined}
      />,
    );
    expect(markup.match(/data-tom=/g) ?? []).toHaveLength(3);
  });
});
