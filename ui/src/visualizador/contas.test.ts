import { describe, expect, it, vi } from "vitest";
import {
  acaoDeZoom,
  chaveDoRotuloDaMiniatura,
  chaveDoRotuloDaPagina,
  MARGEM_DA_ROLAGEM,
  maiorLargura,
  aguardarTextoDeCarga,
  ATRASO_DO_TEXTO_DE_CARGA_MS,
  proximoBotaoDaBarra,
  proximoZoom,
  alturasNoZoom,
  ancoraDaRolagem,
  aumentarZoom,
  diminuirZoom,
  empilhar,
  escalaDoCanvas,
  esquerdaDaPagina,
  faixaVisivel,
  foiCancelamento,
  janelaDePaginas,
  LIMITE_DE_PAGINAS,
  limitarZoom,
  miniaturaPorTecla,
  miniaturasAbertas,
  miniaturasAMontar,
  paginaAtual,
  pdfPediuSenha,
  percentualDoZoom,
  pilhaDeMiniaturas,
  PIXELS_POR_PONTO,
  PISO_DO_AJUSTE,
  podeAumentar,
  podeDiminuir,
  rolagemDaAncora,
  rolagemDaPagina,
  rolagemParaMostrar,
  tamanhoDaMiniatura,
  tamanhoSeguro,
  totalSeguro,
  ZOOM_MAXIMO,
  ZOOM_MINIMO,
  zoomParaLargura,
} from "./contas";

/** Paginas de 100 px, sem margem: 0-100, 110-210, 220-320... */
const dez = empilhar(new Array<number>(10).fill(100), 10, 0);

describe("empilhar", () => {
  it("poe a margem nas pontas e o espaco so entre os itens", () => {
    const pilha = empilhar([100, 50], 8, 12);
    expect(pilha.topos).toEqual([12, 120]);
    expect(pilha.alturaTotal).toBe(12 + 100 + 8 + 50 + 12);
  });

  it("documento vazio tem so as margens", () => {
    expect(empilhar([], 8, 12).alturaTotal).toBe(24);
  });
});

describe("pagina visivel a partir da rolagem", () => {
  it("no topo e a primeira", () => {
    expect(paginaAtual(dez, 0, 100)).toBe(1);
  });

  it("escolhe a que ocupa mais altura na janela", () => {
    expect(paginaAtual(dez, 40, 100)).toBe(1);
    expect(paginaAtual(dez, 70, 100)).toBe(2);
  });

  it("no empate fica a de cima", () => {
    expect(paginaAtual(dez, 50, 110)).toBe(1);
  });

  it("no fim da rolagem e a ultima, mesmo com a anterior mais visivel", () => {
    const curta = empilhar([1000, 1000, 10], 10, 0);
    const maxima = curta.alturaTotal - 500;
    expect(paginaAtual(curta, maxima, 500)).toBe(3);
    expect(paginaAtual(curta, maxima - 100, 500)).toBe(2);
  });

  it("no topo da rolagem e a primeira, mesmo com a seguinte mais visivel", () => {
    const capaPequena = empilhar([100, 1000], 10, 12);
    expect(paginaAtual(capaPequena, 0, 800)).toBe(1);
    expect(paginaAtual(capaPequena, 1, 800)).toBe(2);
  });

  it("documento que cabe inteiro na janela fica na primeira", () => {
    expect(paginaAtual(empilhar([100, 100], 10, 0), 0, 800)).toBe(1);
  });

  it("documento vazio nao quebra a conta", () => {
    const vazio = empilhar([], 8, 12);
    expect(paginaAtual(vazio, 0, 100)).toBe(1);
    expect(faixaVisivel(vazio, 0, 100).fim).toBe(0);
  });

  it("a faixa inclui toda pagina que toca a janela", () => {
    expect(faixaVisivel(dez, 0, 100)).toEqual({ inicio: 1, fim: 1 });
    expect(faixaVisivel(dez, 105, 120)).toEqual({ inicio: 2, fim: 3 });
    expect(faixaVisivel(dez, 5000, 100)).toEqual({ inicio: 10, fim: 10 });
  });
});

describe("janela de paginas a manter", () => {
  it("soma a margem e respeita as pontas do documento", () => {
    expect(janelaDePaginas({ inicio: 5, fim: 6 }, 10, 2, 40)).toEqual({ inicio: 3, fim: 8 });
    expect(janelaDePaginas({ inicio: 1, fim: 1 }, 10, 2, 40)).toEqual({ inicio: 1, fim: 3 });
    expect(janelaDePaginas({ inicio: 10, fim: 10 }, 10, 2, 40)).toEqual({ inicio: 8, fim: 10 });
  });

  it("nunca passa do teto, mesmo com centenas de paginas minusculas na tela", () => {
    const janela = janelaDePaginas({ inicio: 100, fim: 300 }, 1000, 2, 40);
    expect(janela.fim - janela.inicio + 1).toBe(40);
    expect(janela.inicio).toBeGreaterThanOrEqual(98);
    expect(janela.fim).toBeLessThanOrEqual(302);
  });

  it("faixa vazia ou documento sem paginas nao mantem nada", () => {
    expect(janelaDePaginas({ inicio: 1, fim: 0 }, 10, 2, 40).fim).toBe(0);
    expect(janelaDePaginas({ inicio: 1, fim: 1 }, 0, 2, 40).fim).toBe(0);
  });
});

describe("zoom com limites", () => {
  it("anda pelos degraus e para nas pontas", () => {
    expect(aumentarZoom(1)).toBe(1.1);
    expect(diminuirZoom(1)).toBe(0.9);
    expect(aumentarZoom(ZOOM_MAXIMO)).toBe(ZOOM_MAXIMO);
    expect(diminuirZoom(ZOOM_MINIMO)).toBe(ZOOM_MINIMO);
  });

  it("de um valor entre degraus vai ao degrau seguinte", () => {
    expect(aumentarZoom(1.17)).toBe(1.25);
    expect(diminuirZoom(1.17)).toBe(1.1);
  });

  it("depois de ajustar a largura, um degrau quase igual ao zoom atual e pulado", () => {
    expect(aumentarZoom(1.0983)).toBe(1.25);
    expect(diminuirZoom(1.1018)).toBe(1);
  });

  it("recusa valores sem sentido", () => {
    expect(limitarZoom(Number.NaN)).toBe(1);
    expect(limitarZoom(Number.POSITIVE_INFINITY)).toBe(1);
    expect(limitarZoom(-3)).toBe(ZOOM_MINIMO);
    expect(limitarZoom(99)).toBe(ZOOM_MAXIMO);
  });

  it("um documento enorme baixa o teto para continuar rolavel", () => {
    const alturaBase = 60_000_000;
    expect(limitarZoom(ZOOM_MAXIMO, alturaBase)).toBe(0.5);
    expect(aumentarZoom(0.5, alturaBase)).toBe(0.5);
    expect(podeAumentar(0.5, alturaBase)).toBe(false);
    expect(podeDiminuir(0.5, alturaBase)).toBe(true);
    expect(limitarZoom(0.01, 300_000_000)).toBe(0.1);
  });

  it("diz quando os botoes ainda tem efeito", () => {
    expect(podeAumentar(ZOOM_MAXIMO)).toBe(false);
    expect(podeAumentar(3)).toBe(true);
    expect(podeDiminuir(ZOOM_MINIMO)).toBe(false);
    expect(podeDiminuir(0.33)).toBe(true);
  });

  it("ajustar a largura desconta as margens e respeita os limites", () => {
    expect(zoomParaLargura(595, 595 * PIXELS_POR_PONTO + 24)).toBeCloseTo(1);
    expect(zoomParaLargura(100_000, 400)).toBe(PISO_DO_AJUSTE);
    expect(zoomParaLargura(1, 4000)).toBe(ZOOM_MAXIMO);
    expect(zoomParaLargura(Number.NaN, 400)).toBe(1);
    expect(zoomParaLargura(595, 10)).toBe(1);
  });

  it("a pagina fica centrada na janela e a que nao cabe encosta na margem", () => {
    expect(esquerdaDaPagina(400, 1000)).toBe(300);
    expect(esquerdaDaPagina(1400, 1000)).toBe(12);
  });

  it("o percentual e inteiro", () => {
    expect(percentualDoZoom(1.2345)).toBe(123);
  });
});

describe("lugar na leitura ao mudar o zoom", () => {
  const duas = [
    { largura: 100, altura: 300 },
    { largura: 100, altura: 300 },
  ];

  it("a mesma parte da pagina continua no topo", () => {
    const antes = empilhar(alturasNoZoom(duas, 1), 8, 12);
    const depois = empilhar(alturasNoZoom(duas, 2), 8, 12);
    const topoSegunda = antes.topos[1] ?? 0;
    const ancora = ancoraDaRolagem(antes, topoSegunda + 100);
    expect(ancora.pagina).toBe(2);
    expect(ancora.fracao).toBeCloseTo(100 / (300 * PIXELS_POR_PONTO));
    expect(rolagemDaAncora(depois, ancora)).toBeCloseTo((depois.topos[1] ?? 0) + 200);
  });

  it("no topo do documento ou entre paginas nao ha salto ao remedir", () => {
    const pilha = empilhar([100, 100], 8, 12);
    expect(rolagemDaAncora(pilha, ancoraDaRolagem(pilha, 0))).toBe(0);
    expect(rolagemDaAncora(pilha, ancoraDaRolagem(pilha, 115))).toBe(115);
    const menor = empilhar([50, 50], 8, 12);
    expect(rolagemDaAncora(menor, ancoraDaRolagem(pilha, 0))).toBe(0);
  });

  it("ir a uma pagina deixa a margem acima dela e recusa numero fora do documento", () => {
    const pilha = empilhar([100, 100], 8, 12);
    expect(rolagemDaPagina(pilha, 1)).toBe(0);
    expect(rolagemDaPagina(pilha, 2)).toBe(108);
    expect(rolagemDaPagina(pilha, 99)).toBe(108);
    expect(rolagemDaPagina(pilha, -5)).toBe(0);
    expect(rolagemDaPagina(empilhar([], 8, 12), 1)).toBe(0);
  });
});

describe("numeros vindos do PDF", () => {
  it("tamanho sem sentido cai no padrao", () => {
    for (const [largura, altura] of [
      [0, 842],
      [-1, 842],
      [Number.NaN, 842],
      [595, Number.POSITIVE_INFINITY],
    ] as const) {
      expect(tamanhoSeguro(largura, altura)).toEqual({ largura: 595, altura: 842 });
    }
  });

  it("tamanho extremo fica dentro dos limites", () => {
    expect(tamanhoSeguro(1e12, 0.001)).toEqual({ largura: 144_000, altura: 1 });
  });

  it("contagem de paginas absurda e recusada", () => {
    expect(totalSeguro(24)).toBe(24);
    expect(totalSeguro(LIMITE_DE_PAGINAS)).toBe(LIMITE_DE_PAGINAS);
    expect(totalSeguro(LIMITE_DE_PAGINAS + 1)).toBeNull();
    expect(totalSeguro(0)).toBeNull();
    expect(totalSeguro(-1)).toBeNull();
    expect(totalSeguro(1.5)).toBeNull();
    expect(totalSeguro(Number.NaN)).toBeNull();
  });

  it("o canvas nunca passa do limite de pixels nem de lado", () => {
    expect(escalaDoCanvas(800, 1000, 1.5)).toBe(1.5);
    const grande = escalaDoCanvas(4000, 6000, 2);
    expect(4000 * 6000 * grande * grande).toBeLessThanOrEqual(16_000_001);
    const comprida = escalaDoCanvas(100, 40_000, 1);
    expect(40_000 * comprida).toBeLessThanOrEqual(8192);
    expect(escalaDoCanvas(100, 100, Number.NaN)).toBe(1);
  });

  it("o pedido de senha vira recusa, e cancelar nao e erro", () => {
    expect(pdfPediuSenha({ name: "PasswordException" })).toBe(true);
    expect(pdfPediuSenha(new Error("x"))).toBe(false);
    expect(pdfPediuSenha(null)).toBe(false);
    expect(foiCancelamento({ name: "RenderingCancelledException" })).toBe(true);
    expect(foiCancelamento(new Error("x"))).toBe(false);
    expect(foiCancelamento("texto")).toBe(false);
  });
});

describe("atalhos de zoom", () => {
  const tecla = (key: string, ctrlKey = true, altKey = false) => ({ key, ctrlKey, altKey });

  it("Ctrl com +, =, - e 0", () => {
    expect(acaoDeZoom(tecla("+"))).toBe("aumentar");
    expect(acaoDeZoom(tecla("="))).toBe("aumentar");
    expect(acaoDeZoom(tecla("-"))).toBe("diminuir");
    expect(acaoDeZoom(tecla("0"))).toBe("ajustar");
  });

  it("sem Ctrl, com AltGr ou outra tecla nao faz nada", () => {
    expect(acaoDeZoom(tecla("+", false))).toBeNull();
    expect(acaoDeZoom(tecla("0", true, true))).toBeNull();
    expect(acaoDeZoom(tecla("o"))).toBeNull();
    expect(acaoDeZoom(tecla("b"))).toBeNull();
    expect(acaoDeZoom(tecla("w"))).toBeNull();
  });
});

describe("estado das miniaturas", () => {
  it("a preferencia vale mais que a largura; sem ela, janela estreita comeca recolhida", () => {
    expect(miniaturasAbertas(null, 1000)).toBe(true);
    expect(miniaturasAbertas(null, 400)).toBe(false);
    expect(miniaturasAbertas(true, 400)).toBe(true);
    expect(miniaturasAbertas(false, 1000)).toBe(false);
  });

  it("a miniatura mantem a proporcao dentro da caixa, com rotacao e pagina extrema", () => {
    expect(tamanhoDaMiniatura({ largura: 595, altura: 842 })).toEqual({ largura: 106, altura: 150 });
    expect(tamanhoDaMiniatura({ largura: 842, altura: 595 })).toEqual({ largura: 112, altura: 79 });
    const fita = tamanhoDaMiniatura({ largura: 144_000, altura: 1 });
    expect(fita.altura).toBeGreaterThanOrEqual(1);
    expect(fita.largura).toBe(112);
  });

  it("monta so o entorno da janela, sob demanda", () => {
    expect(miniaturasAMontar({ inicio: 5, fim: 7 }, 1000, 2, null)).toEqual([3, 4, 5, 6, 7, 8, 9]);
  });

  it("sem faixa nao monta nada", () => {
    expect(miniaturasAMontar({ inicio: 1, fim: 0 }, 1000, 2, null)).toEqual([]);
  });

  it("a miniatura com foco continua montada fora da janela", () => {
    expect(miniaturasAMontar({ inicio: 50, fim: 50 }, 100, 0, 3)).toEqual([3, 50]);
    expect(miniaturasAMontar({ inicio: 50, fim: 50 }, 100, 0, 90)).toEqual([50, 90]);
    expect(miniaturasAMontar({ inicio: 50, fim: 50 }, 100, 0, 50)).toEqual([50]);
    expect(miniaturasAMontar({ inicio: 50, fim: 50 }, 100, 0, 999)).toEqual([50]);
  });

  it("as setas andam pela lista e param nas pontas; outras teclas nao navegam", () => {
    expect(miniaturaPorTecla(1, "ArrowUp", 5)).toBe(1);
    expect(miniaturaPorTecla(1, "ArrowDown", 5)).toBe(2);
    expect(miniaturaPorTecla(5, "ArrowDown", 5)).toBe(5);
    expect(miniaturaPorTecla(3, "Home", 5)).toBe(1);
    expect(miniaturaPorTecla(3, "End", 5)).toBe(5);
    expect(miniaturaPorTecla(3, "Enter", 5)).toBeNull();
    expect(miniaturaPorTecla(3, "ArrowDown", 0)).toBeNull();
  });

  it("a lista acompanha a pagina mexendo o minimo", () => {
    const pilha = pilhaDeMiniaturas(new Array(10).fill({ largura: 595, altura: 842 }));
    expect(rolagemParaMostrar(pilha, 1, 0, 400)).toBe(0);
    const topoQuinta = pilha.topos[4] ?? 0;
    expect(rolagemParaMostrar(pilha, 5, 0, 400)).toBe(topoQuinta + 176 + 8 - 400);
    expect(rolagemParaMostrar(pilha, 1, 500, 400)).toBe(0);
    expect(rolagemParaMostrar(pilha, 99, 123, 400)).toBe(123);
  });
});

describe("ajuste a largura, teclas e barra", () => {
  it("o ajuste usa a maior pagina e nunca passa da largura disponivel", () => {
    const tamanhos = [
      { largura: 595, altura: 842 },
      { largura: 1190, altura: 842 },
      { largura: 300, altura: 400 },
    ];
    expect(maiorLargura(tamanhos)).toBe(1190);
    const disponivel = 900;
    const zoom = zoomParaLargura(maiorLargura(tamanhos), disponivel);
    const larguraNaTela = 1190 * zoom * PIXELS_POR_PONTO + MARGEM_DA_ROLAGEM * 2;
    expect(larguraNaTela).toBeLessThanOrEqual(disponivel);
    expect(zoom * 100).toBeCloseTo(Math.round(zoom * 100), 6);
  });

  it("o zoom do ajuste fica arredondado para baixo, em pontos percentuais", () => {
    const zoom = zoomParaLargura(595, 700);
    expect(zoom * 100).toBeCloseTo(Math.round(zoom * 100), 6);
    expect(zoom).toBeLessThanOrEqual((700 - MARGEM_DA_ROLAGEM * 2) / (595 * PIXELS_POR_PONTO));
  });

  it("um comando que nao muda o zoom devolve nulo, para o ajuste nao ser desligado", () => {
    expect(proximoZoom("aumentar", ZOOM_MAXIMO)).toBeNull();
    expect(proximoZoom("diminuir", ZOOM_MINIMO)).toBeNull();
    expect(proximoZoom("ajustar", 1)).toBeNull();
    expect(proximoZoom("aumentar", 1)).toBe(1.1);
    expect(proximoZoom("diminuir", 1)).toBe(0.9);
  });

  it("cada miniatura ocupa 8 px a mais que antes na pilha", () => {
    const pilha = pilhaDeMiniaturas([{ largura: 595, altura: 842 }, { largura: 595, altura: 842 }]);
    const altura = tamanhoDaMiniatura({ largura: 595, altura: 842 }).altura;
    expect(pilha.alturas[0]).toBe(altura + 18 + 8);
    expect(pilha.topos[1]).toBe(8 + altura + 18 + 8 + 8);
  });

  it("a barra move o foco entre os botoes com as setas, com volta no fim e no inicio", () => {
    expect(proximoBotaoDaBarra(0, "ArrowRight", 5)).toBe(1);
    expect(proximoBotaoDaBarra(4, "ArrowRight", 5)).toBe(0);
    expect(proximoBotaoDaBarra(0, "ArrowLeft", 5)).toBe(4);
    expect(proximoBotaoDaBarra(2, "End", 5)).toBe(4);
    expect(proximoBotaoDaBarra(2, "Home", 5)).toBe(0);
    expect(proximoBotaoDaBarra(2, "a", 5)).toBeNull();
    expect(proximoBotaoDaBarra(0, "ArrowRight", 0)).toBeNull();
  });

  it("o texto de abertura so aparece depois de 200 ms, e some se a abertura termina antes", () => {
    vi.useFakeTimers();
    try {
      const mostrar = vi.fn();
      aguardarTextoDeCarga(mostrar);
      vi.advanceTimersByTime(ATRASO_DO_TEXTO_DE_CARGA_MS - 1);
      expect(mostrar).not.toHaveBeenCalled();
      vi.advanceTimersByTime(1);
      expect(mostrar).toHaveBeenCalledTimes(1);

      const cancelado = vi.fn();
      const cancelar = aguardarTextoDeCarga(cancelado);
      cancelar();
      vi.advanceTimersByTime(ATRASO_DO_TEXTO_DE_CARGA_MS * 2);
      expect(cancelado).not.toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  it("cada estado de pagina tem o rotulo proprio, na area e na miniatura", () => {
    expect(chaveDoRotuloDaPagina("pronto")).toBe("visualizador.pagina.rotulo");
    expect(chaveDoRotuloDaPagina("carregando")).toBe("visualizador.pagina.carregando");
    expect(chaveDoRotuloDaPagina("erro")).toBe("visualizador.pagina.rotuloErro");
    expect(chaveDoRotuloDaMiniatura("pronto")).toBe("visualizador.miniaturas.ir");
    expect(chaveDoRotuloDaMiniatura("erro")).toBe("visualizador.miniaturas.irErro");
  });
});

describe("ajuste em janela estreita", () => {
  it("uma pagina muito maior ainda cabe na largura, mesmo abaixo do zoom minimo dos botoes", () => {
    const zoom = zoomParaLargura(2000, 400);
    expect(zoom).toBeLessThan(ZOOM_MINIMO);
    expect(2000 * zoom * PIXELS_POR_PONTO + MARGEM_DA_ROLAGEM * 2).toBeLessThanOrEqual(400);
  });

  it("diminuir abaixo do minimo dos botoes nao sobe o zoom de volta", () => {
    expect(diminuirZoom(0.14)).toBe(0.14);
    expect(proximoZoom("diminuir", 0.14)).toBeNull();
  });
});

