import { describe, expect, it } from "vitest";
import { traducoes, traduzir } from "./i18n";
import {
  chaveDoDesenhoFalho,
  iconeDoResumo,
  pendenteDoPapel,
  rotuloDeVirar,
  type BandejaTela,
  type CamadaTela,
} from "./tela";

// Os CSS de módulo e os componentes não voltam como valor sob o teste: são lidos do disco.
interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;
const css = fs.readFileSync(new URL("./components/Folha.module.css", import.meta.url), "utf8");
const folhaTsx = fs.readFileSync(new URL("./components/Folha.tsx", import.meta.url), "utf8");
const appTsx = fs.readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const estilosDoApp = fs.readFileSync(new URL("./App.module.css", import.meta.url), "utf8");

function camada(pagina: number, atras: boolean, branca = false): CamadaTela {
  return { pagina, numero: pagina, branca, graus: 0, atras, larguraMm: 210, alturaMm: 297 };
}

function bandeja(parcial: Partial<BandejaTela>): BandejaTela {
  return {
    etapa: "conferida",
    recebendo: false,
    esperando: 0,
    folhas: [],
    corretas: 0,
    semMarca: 0,
    conferidas: 0,
    gerais: [],
  conferenciaPorFolha: false,
    leitura: null,
    semInstrucao: false,
    operacoes: [],
    paginasSemFolha: 0,
    folhasSemVerso: 0,
    aviso: null,
    ...parcial,
  };
}

describe("desenho a caminho", () => {
  it("uma camada pendente basta para a folha mostrar que está desenhando", () => {
    const prontas = new Map<number, "pronta" | "falhou">([[1, "pronta"]]);
    expect(pendenteDoPapel([camada(1, false), camada(2, true)], prontas)).toBe(true);
  });

  it("com a frente pronta e só o verso faltando, ainda está desenhando", () => {
    const prontas = new Map<number, "pronta" | "falhou">([[1, "pronta"]]);
    expect(pendenteDoPapel([camada(1, false), camada(2, true)], prontas)).toBe(true);
  });

  it("quando tudo ficou pronto ou falhou, não há o que desenhar", () => {
    const prontas = new Map<number, "pronta" | "falhou">([
      [1, "pronta"],
      [2, "falhou"],
    ]);
    expect(pendenteDoPapel([camada(1, false), camada(2, true)], prontas)).toBe(false);
  });

  it("página em branco nunca conta como pendente", () => {
    expect(pendenteDoPapel([camada(3, false, true)], new Map())).toBe(false);
  });

  it("o indicador e o botão da folha usam os textos pedidos", () => {
    expect(traduzir("pt-BR", "folha.desenhando")).toBe("Desenhando…");
    expect(traduzir("en", "folha.desenhando")).toBe("Drawing…");
  });

  it("o indicador é lido pelo botão da folha, sem região viva própria", () => {
    expect(folhaTsx).toMatch(/aria-busy=\{/);
    expect(folhaTsx).not.toMatch(/role="status"[^>]*desenhando/);
  });

  it("o indicador usa o estilo do papel: fundo, texto e medidas pedidos", () => {
    const bloco = css.match(/\.desenhando\s*\{[^}]*\}/)?.[0] ?? "";
    expect(bloco).toMatch(/background:\s*var\(--papel\)/);
    expect(bloco).toMatch(/color:\s*var\(--papel-texto\)/);
    expect(bloco).toMatch(/padding:\s*2px 6px/);
    expect(bloco).toMatch(/border-radius:\s*2px/);
    expect(bloco).toMatch(/font-size:\s*11px/);
  });
});

describe("falha do desenho", () => {
  it("falha da frente diz sem desenho, e falha do verso diz verso sem desenho", () => {
    expect(chaveDoDesenhoFalho(camada(1, false), false)).toBe("folha.semDesenho");
    expect(chaveDoDesenhoFalho(camada(2, true), false)).toBe("folha.semDesenhoVerso");
    expect(traduzir("pt-BR", "folha.semDesenhoVerso")).toBe("Verso sem desenho");
    expect(traduzir("en", "folha.semDesenhoVerso")).toBe("Back not drawn");
  });

  it("com a folha virada, aviso segue o lado que está à vista", () => {
    // Virada, a página que era da frente é a que aparece no verso da folha.
    expect(chaveDoDesenhoFalho(camada(1, false), true)).toBe("folha.semDesenhoVerso");
    expect(chaveDoDesenhoFalho(camada(2, true), true)).toBe("folha.semDesenho");
  });

  it("o texto da falha fica num bloco com o fundo do papel, como o indicador", () => {
    const bloco = css.match(/\.textoSemDesenho\s*\{[^}]*\}/)?.[0] ?? "";
    expect(bloco).toMatch(/background:\s*var\(--papel\)/);
    expect(bloco).toMatch(/padding:\s*2px 6px/);
    expect(bloco).toMatch(/border-radius:\s*2px/);
    expect(folhaTsx).toMatch(/<span className=\{styles\.textoSemDesenho\}>\s*\{t\(chaveDoDesenhoFalho\(camada, virada\)\)\}/);
  });
});

describe("seleção e erro na folha", () => {
  it("a seleção aparece por contorno interno, sem o ✓", () => {
    expect(css).toMatch(/\.selecionada\s*\{[^}]*box-shadow:\s*inset 0 0 0 1px var\(--accent\)/);
    expect(folhaTsx).not.toMatch(/marcaSelecao/);
    expect(folhaTsx).not.toMatch(/\bCheck\b/);
    expect(folhaTsx).toMatch(/aria-pressed=\{selecionada\}/);
  });

  it("folha errada tem borda de 1 px na cor de erro", () => {
    const bloco = css.match(/\.errada\s*\{[^}]*\}/)?.[0] ?? "";
    expect(bloco).toMatch(/border:\s*1px solid var\(--status-offline\)/);
  });

  it("ao passar o ponteiro, a folha errada e a selecionada mantêm a cor do próprio contorno", () => {
    expect(css).toMatch(/\.errada:hover\s*\{[^}]*border-color:\s*var\(--status-offline\)/);
    expect(css).toMatch(/\.selecionada:hover\s*\{[^}]*border-color:\s*var\(--accent\)/);
  });

  it("o botão de virar diz a folha escolhida, e sem seleção diz só o verbo", () => {
    expect(rotuloDeVirar(bandeja({ folhas: [] }), 2)).toEqual({
      chave: "rodape.virar",
      parametros: {},
    });
    expect(traduzir("pt-BR", "rodape.virar.folha", { n: "2" })).toBe(
      "Ver o outro lado da folha 2",
    );
    expect(traduzir("en", "rodape.virar.folha", { n: "2" })).toBe(
      "See the other side of sheet 2",
    );
  });

  it("o botão de virar avisa que só muda a vista, e o gesto fica em Recolocar", () => {
    expect(traduzir("pt-BR", "rodape.virar")).toBe("Ver o outro lado");
    expect(traduzir("en", "rodape.virar")).toBe("See the other side");
    expect(traduzir("pt-BR", "rodape.virar.dica")).toBe(
      "Só muda o que aparece na tela. O gesto é aplicado em Recolocar.",
    );
    expect(traduzir("en", "rodape.virar.dica")).toBe(
      "Only changes what the screen shows. The gesture is applied in Reload.",
    );
    expect(appTsx).toMatch(/aria-description=\{titulo\}/);
    expect(appTsx).toMatch(/title=\{titulo\}/);
  });

  it("o botão de virar com folha escolhida leva o número", () => {
    const tela = bandeja({ folhas: [{ numero: 2, situacao: "certa", motivos: [], frente: [], verso: [] }] });
    expect(rotuloDeVirar(tela, 2)).toEqual({ chave: "rodape.virar.folha", parametros: { n: "2" } });
  });

  it("virada a folha, o botão oferece a frente, nos dois idiomas", () => {
    const tela = bandeja({ folhas: [{ numero: 2, situacao: "certa", motivos: [], frente: [], verso: [] }] });
    expect(rotuloDeVirar(tela, 2, true)).toEqual({
      chave: "rodape.virar.frente.folha",
      parametros: { n: "2" },
    });
    expect(rotuloDeVirar(tela, null, true)).toEqual({ chave: "rodape.virar.frente", parametros: {} });
    expect(traduzir("pt-BR", "rodape.virar.frente.folha", { n: "2" })).toBe(
      "Ver a frente da folha 2",
    );
    expect(traduzir("en", "rodape.virar.frente.folha", { n: "2" })).toBe("See the front of sheet 2");
    expect(traduzir("pt-BR", "rodape.virar.frente")).toBe("Ver a frente");
    expect(traduzir("en", "rodape.virar.frente")).toBe("See the front");
  });

  it("a folha leva a situação do verso recebido para o rótulo acessível", () => {
    expect(folhaTsx).toMatch(/chaveDaSituacao\(folha\.situacao, versoRecebido\)/);
    expect(appTsx).toMatch(/versoRecebido=\{versoRecebido\}/);
  });

  it("o rótulo do botão segue a folha escolhida, não o estado da lista", () => {
    expect(appTsx).toMatch(/rotuloDeVirar\(bandeja, folha, folhaVirada\)/);
  });
});

describe("resumo do rodapé", () => {
  const umaFolha = [{ numero: 1, situacao: "certa" as const, motivos: [], frente: [], verso: [] }];

  it("tudo certo mostra o ícone de certo; alguma errada mostra o de errado", () => {
    expect(iconeDoResumo(bandeja({ folhas: umaFolha, corretas: 3, conferidas: 3 }))).toBe("certo");
    expect(iconeDoResumo(bandeja({ folhas: umaFolha, corretas: 1, conferidas: 3 }))).toBe("com_erro");
  });

  it("sem veredito não há ícone", () => {
    expect(iconeDoResumo(bandeja({ folhas: [] }))).toBe(null);
  });

  it("com a forma de pegar vinda da bandeja, tudo certo não ganha o ícone verde", () => {
    const pronta = bandeja({ folhas: umaFolha, corretas: 3, conferidas: 3, semInstrucao: true });
    expect(iconeDoResumo(pronta)).toBe("neutro");
    const comErro = bandeja({ folhas: umaFolha, corretas: 1, conferidas: 3, semInstrucao: true });
    expect(iconeDoResumo(comErro)).toBe("com_erro");
  });

  it("o ícone neutro é um círculo na cor de texto discreto, e o de certo fica na cor do sistema no contraste forçado", () => {
    expect(appTsx).toMatch(/icone === "neutro"[\s\S]*styles\.iconeNeutro/);
    expect(appTsx).toMatch(/import \{[\s\S]*?\bCircle,[\s\S]*?\} from "lucide-react"/);
    expect(estilosDoApp).toMatch(/\.iconeNeutro\s*\{[^}]*color:\s*var\(--fg-muted\)/);
    const forcado = estilosDoApp.slice(estilosDoApp.indexOf("@media (forced-colors: active)"));
    expect(forcado).toMatch(/\.iconeCerto,\s*\.iconeErrado\s*\{\s*color:\s*CanvasText/);
  });

  it("a nota do modelo usa 11 px com entrelinha de 16 px", () => {
    const bloco = estilosDoApp.match(/\.notaDoModelo\s*\{[^}]*\}/)?.[0] ?? "";
    expect(bloco).toMatch(/font-size:\s*11px/);
    expect(bloco).toMatch(/line-height:\s*16px/);
  });
});

describe("textos definitivos", () => {
  it("a forma de pegar escolhida sem a instrução do PlotS é dita na tela", () => {
    expect(traduzir("pt-BR", "centro.semInstrucao")).toBe("Sem instrução do PlotS: melhor forma de pegar.");
    expect(traduzir("en", "centro.semInstrucao")).toBe("No instruction from PlotS: best way to hold the stack.");
  });

  it("canal fechado diz o que fazer", () => {
    expect(traduzir("pt-BR", "centro.canalFechado")).toBe(
      "O canal com o PlotS não abriu. O gesto do modo de teste não chega aqui; marque-o à mão em Recolocar. Se houver outro PlotSim aberto, feche-o e abra este de novo.",
    );
    expect(traduzir("en", "centro.canalFechado")).toBe(
      "The channel to PlotS did not open. The test-mode gesture will not arrive here; mark it by hand in Reload. If another PlotSim is open, close it and open this one again.",
    );
  });

  it("a pasta dos PDFs diz onde ficam e onde o último trabalho também fica", () => {
    expect(
      traduzir("pt-BR", "centro.pastaDosPdfs", { historico: "H", porta: "P" }),
    ).toBe("PDFs recebidos ficam em H. O último trabalho também fica em P, que outras contas deste computador podem ler.");
    expect(
      traduzir("en", "centro.pastaDosPdfs", { historico: "H", porta: "P" }),
    ).toBe("Received PDFs stay in H. The last job also stays in P, which other accounts on this computer can read.");
  });

  it("não há rede e a conferência é do modelo", () => {
    expect(traduzir("pt-BR", "centro.semEnvio")).toBe("O PlotSim não envia nada pela rede.");
    expect(traduzir("en", "centro.semEnvio")).toBe("PlotSim sends nothing over the network.");
    expect(traduzir("pt-BR", "centro.conferidoPeloModelo")).toBe(
      "Conferido pelo modelo do PlotSim; não substitui o teste em papel.",
    );
    expect(traduzir("en", "centro.conferidoPeloModelo")).toBe(
      "Checked by the PlotSim model; it does not replace the paper test.",
    );
  });

  it("o texto da folha selecionada saiu dos dois idiomas, e a contagem sem marca diz o singular", () => {
    expect(Object.keys(traducoes["pt-BR"])).not.toContain("folha.selecionada");
    expect(Object.keys(traducoes.en)).not.toContain("folha.selecionada");
    expect(traduzir("pt-BR", "rodape.resumo.um.semMarca", { certas: "0", semMarca: "1" })).toBe(
      "0 de 1 folha correta, 1 sem marca",
    );
    expect(traduzir("en", "rodape.resumo.um.semMarca", { certas: "0", semMarca: "1" })).toBe(
      "0 of 1 sheet correct, 1 no mark",
    );
  });

  it("PDF acima do limite, falha do Windows, permissão e criação usam os textos novos", () => {
    expect(traduzir("pt-BR", "aviso.pdf_grande")).toBe(
      "O PDF recebido passa do limite aceito e não entrou no histórico.",
    );
    expect(traduzir("en", "aviso.pdf_grande")).toBe(
      "The received PDF is over the accepted limit and did not go into the history.",
    );
    expect(traduzir("pt-BR", "erro.windows_recusou")).toBe(
      "O Windows recusou o pedido. Atualize a lista para ver como a fila ficou.",
    );
    expect(traduzir("en", "erro.windows_recusou")).toBe(
      "Windows refused the request. Refresh the list to see how the queue stands.",
    );
    expect(traduzir("pt-BR", "erro.sem_permissao")).not.toContain("Nada foi alterado.");
    expect(traduzir("en", "erro.sem_permissao")).not.toContain("Nothing was changed.");
    expect(traduzir("pt-BR", "nova.texto")).toBe(
      "Ela grava PDF e guarda uma cópia de cada trabalho; nenhum papel é usado.",
    );
    expect(traduzir("en", "nova.texto")).toBe(
      "It writes a PDF and keeps a copy of each job; no paper is used.",
    );
  });
});
