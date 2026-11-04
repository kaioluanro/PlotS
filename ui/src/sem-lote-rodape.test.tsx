import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  acoesDoCartao,
  estadoDoEnvio,
  linhasDoCartao,
  rotuloDoVerso,
  textoDoTamanhoDoEnvio,
  tituloDaSecao,
  type Cartao,
  type EstadoCartao,
  type ResumoDoEnvio,
} from "./cartoes";
import { CartaoImpressao } from "./components/CartaoImpressao";
import { BlocoFace } from "./components/BlocoFace";
import { conteudoDoVerso, dicaDaFace, envioComFace } from "./face-do-envio";
import { t, traduzir } from "./i18n";
import type { Impressora } from "./impressoras";
import { resumoDoDocumento } from "./impressoras";
import { estadoInicial, lerQuantidadeDeCopias, reduzirAbas, type Aba, type EstadoAbas } from "./tabs";

const impressora = (nome: string, mudancas: Partial<Impressora> = {}): Impressora => ({
  id: `fila:${nome}`,
  nome,
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "laser",
  ...mudancas,
});
const SALA = impressora("Sala");
const COPA = impressora("Copa", { modo: "jato" });
const SIMULADA = impressora("Sim", { id: "simulada:gaveta", tipo: "simulada" });
const GESTO = { lado: "baixo", virada: "borda_de_cima", textoDoPerfil: null };

function cartao(estado: EstadoCartao, mudancas: Partial<Cartao> = {}): Cartao {
  return {
    aba: 1,
    impressora: SALA.id,
    nome: SALA.nome,
    simulada: false,
    lote: 1,
    total: 1,
    papel: { formato: "a4", larguraMm: 210, alturaMm: 297 },
    estado,
    recolocar: GESTO,
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    alcance: "so_frentes",
    folhasDoLote: 12,
    faixaDoLote: { lote: 1, inicio: 1, fim: 24 },
    ...mudancas,
  };
}

function comAba(marcadas: readonly Impressora[]): EstadoAbas {
  let estado = reduzirAbas(estadoInicial, {
    tipo: "abrir",
    documento: { caminho: "C:\\a.pdf", nome: "a.pdf" },
  });
  estado = reduzirAbas(estado, { tipo: "paginas_contadas", id: 1, total: 24 });
  for (const item of marcadas) {
    estado = reduzirAbas(estado, { tipo: "alternar_impressora", id: 1, impressora: item.id });
  }
  return estado;
}
const abaDe = (estado: EstadoAbas) => estado.abas[0] as Aba;

const RESUMO_COM_COPIAS = {
  paginas: 24,
  lotes: 1,
  folhas: 36,
  maiorMaco: 36,
  papeis: [{ formato: "a4", larguraMm: 210, alturaMm: 297 }],
  paresMistos: [],
  assinado: false,
  fluxos: [{ automatico: false, inicio: 1, fim: 24 }],
  faixas: [],
  gestos: [],
} as unknown as ResumoDoEnvio;

describe("rodape: nome por extenso do botao de envio", () => {
  it("diz quantas impressoras, em pt-BR, no plural e no singular", () => {
    const aba = abaDe(comAba([SALA, COPA]));
    expect(estadoDoEnvio(aba, [SALA, COPA], new Set(), "pt-BR").rotuloAcessivel).toBe(
      "Imprimir frentes em 2 impressoras",
    );
    const uma = abaDe(comAba([SALA]));
    expect(estadoDoEnvio(uma, [SALA], new Set(), "pt-BR").rotuloAcessivel).toBe("Imprimir frentes em 1 impressora");
  });

  it("diz quantas impressoras, em inglês, e vale para o verso e para a simulacao", () => {
    const aba = abaDe(comAba([SALA, COPA]));
    expect(estadoDoEnvio(aba, [SALA, COPA], new Set(), "en").rotuloAcessivel).toBe("Print fronts on 2 printers");
    const versos = envioComFace(
      estadoDoEnvio({ ...aba, face: "verso" }, [SALA, COPA], new Set(), "pt-BR"),
      { ...aba, face: "verso" },
      [SALA, COPA],
      null,
      "pt-BR",
    );
    expect(versos.rotuloAcessivel).toBe("Imprimir versos em 2 impressoras");
    const simuladas = abaDe(comAba([SIMULADA]));
    expect(estadoDoEnvio(simuladas, [SIMULADA], new Set(), "en").rotuloAcessivel).toBe("Simulate fronts on 1 printer");
  });

  it("sem contagem no rotulo, nao ha nome por extenso a acrescentar", () => {
    const aba = abaDe(comAba([SALA]));
    expect(estadoDoEnvio(aba, [], new Set(), "pt-BR").rotuloAcessivel).toBeUndefined();
  });
});

describe("rodape: dica que diz o estado da face, sem trocar a escolha", () => {
  it("frente escolhida, com frentes prontas nas marcadas: manda escolher Verso", () => {
    const base = abaDe(comAba([SALA]));
    const aba = { ...base, frentesProntas: [SALA.id] };
    expect(dicaDaFace(aba, [SALA], "pt-BR")).toBe(
      "O trabalho das frentes terminou ou saiu da fila. Para os versos, escolha Verso.",
    );
    expect(dicaDaFace(aba, [SALA], "en")).toBe(
      "The fronts job finished or left the queue. For the backs, choose Back.",
    );
  });

  it("verso escolhido, com versos concluidos nas marcadas: repete so se a pessoa clicar de novo", () => {
    const base = abaDe(comAba([SALA]));
    const aba: Aba = {
      ...base,
      face: "verso",
      cartoes: [cartao({ tipo: "concluido" }, { alcance: "so_versos" })],
    };
    expect(dicaDaFace(aba, [SALA], "pt-BR")).toBe(
      "O trabalho dos versos terminou ou saiu da fila. Clique de novo só para repetir.",
    );
    expect(dicaDaFace(aba, [SALA], "en")).toBe(
      "The backs job finished or left the queue. Click again only to repeat it.",
    );
  });

  it("sem estado a dizer, nao ha dica nova; a frente nao vira verso sozinha", () => {
    const aba = abaDe(comAba([SALA]));
    expect(dicaDaFace(aba, [SALA], "pt-BR")).toBeNull();
    expect(aba.face).toBe("frente");
  });

  it("a dica de estado fica fora da regra de altura curta: nao leva o marcador de dica do modo", () => {
    const markup = renderToStaticMarkup(
      <BlocoFace face="frente" aoDefinir={() => undefined} dicaDoEstado="Estado de teste" />,
    );
    expect(markup).toContain("Estado de teste");
    expect(markup).not.toContain("data-dica-modo");
  });
});

describe("rodape: resumo com copias", () => {
  it("com duas ou mais copias, o resumo mostra o numero e o total de folhas", () => {
    expect(resumoDoDocumento(24, "pt-BR", 3)).toBe("24 pág × 3 · 36 folhas");
    expect(resumoDoDocumento(24, "en", 3)).toBe("24 pages × 3 · 36 sheets");
  });

  it("com uma copia, o resumo fica como era", () => {
    expect(resumoDoDocumento(24, "pt-BR")).toBe("24 pág · 12 folhas");
    expect(resumoDoDocumento(24, "pt-BR", 1)).toBe("24 pág · 12 folhas");
  });

  it("a primeira linha da confirmacao segue o mesmo formato", () => {
    expect(textoDoTamanhoDoEnvio(RESUMO_COM_COPIAS, "pt-BR", 3)).toBe("24 pág × 3 · 36 folhas");
    expect(textoDoTamanhoDoEnvio(RESUMO_COM_COPIAS, "en", 3)).toBe("24 pages × 3 · 36 sheets");
    expect(textoDoTamanhoDoEnvio({ ...RESUMO_COM_COPIAS, folhas: 12 }, "pt-BR", 1)).toBe("24 pág · 12 folhas");
  });
});

describe("quantidade: leitura do texto digitado", () => {
  it("aceita de 1 a 999, com espacos em volta, e recusa o resto", () => {
    expect(lerQuantidadeDeCopias(" 12 ")).toBe(12);
    expect(lerQuantidadeDeCopias("1")).toBe(1);
    expect(lerQuantidadeDeCopias("999")).toBe(999);
    for (const texto of ["", "0", "1000", "1.5", "-1", "abc", "1e2"]) {
      expect(lerQuantidadeDeCopias(texto), texto).toBeNull();
    }
  });
});

describe("cartao das frentes prontas", () => {
  it("a secao diz Em andamento enquanto o cartao espera os versos", () => {
    expect(tituloDaSecao([cartao({ tipo: "frentes_prontas" })])).toBe("cartao.secao");
    expect(tituloDaSecao([cartao({ tipo: "concluido" })])).toBe("cartao.secao.encerrados");
  });

  it("o botao de versos vem antes da frase de espera, e sem a linha de clique", () => {
    const pronto = cartao({ tipo: "frentes_prontas" }, { simulada: true, impressora: SIMULADA.id, nome: SIMULADA.nome });
    const markup = renderToStaticMarkup(
      <CartaoImpressao cartao={pronto} aoComandar={() => undefined} aoPedirVerso={() => undefined} aoPedirCancelar={() => undefined} aoDispensar={() => undefined} />,
    );
    expect(markup).not.toContain(t("cartao.simulada.clique"));
    const botao = markup.indexOf(rotuloDoVerso(pronto));
    const espera = markup.indexOf(t("face.espera"));
    expect(botao).toBeGreaterThan(-1);
    expect(espera).toBeGreaterThan(botao);
    expect(linhasDoCartao(pronto).map((linha) => linha.texto)).not.toContain(t("cartao.simulada.clique"));
    expect(acoesDoCartao(pronto).pedirVerso).toBe(true);
  });
});

describe("confirmacao do verso: aviso de copias diferentes", () => {
  const IMPRESSORA = impressora("Sala");
  it("sai do fim, num campo proprio, e nao entra nas notas", () => {
    const conteudo = conteudoDoVerso([IMPRESSORA], [], RESUMO_COM_COPIAS, null, null, false, "pt-BR", {
      frente: 3,
      verso: 2,
    });
    expect(conteudo.aviso).toBe("A Frente está com 3 cópias e o Verso com 2: coloque um maço com 2 cópias.");
    expect(conteudo.notas ?? []).not.toContain(conteudo.aviso);
  });

  it("com numeros iguais, nao ha aviso", () => {
    const conteudo = conteudoDoVerso([IMPRESSORA], [], RESUMO_COM_COPIAS, null, null, false, "pt-BR", {
      frente: 2,
      verso: 2,
    });
    expect(conteudo.aviso).toBeUndefined();
  });
});

describe("frases definitivas, guarda", () => {
  it("mantem as frases definitivas nos dois idiomas", () => {
    expect(traduzir("pt-BR", "face.dica.frente")).toBe("Só as frentes, sem verso no meio.");
    expect(traduzir("pt-BR", "face.espera")).toBe(
      "Pode ser depois. Se fechar o PlotS, abra o mesmo arquivo e escolha Verso. Nesse caso, o PlotS não confere se ele mudou.",
    );
    expect(traduzir("pt-BR", "cartao.frente.espera")).toBe("As frentes estão indo para a fila.");
    expect(traduzir("en", "cartao.frente.espera")).toBe("The fronts are going to the queue.");
    expect(traduzir("en", "face.estado.frentesProntas")).toBe("Fronts done in the queue");
  });
});

describe("janela minima: rodape em altura curta", () => {
  it("os grupos saem em altura curta com ou sem cartao, para a lista de impressoras caber", async () => {
    const modulofs = "node:fs";
    const fs = (await import(/* @vite-ignore */ modulofs)) as {
      readFileSync(caminho: URL, codificacao: "utf8"): string;
    };
    const folha = fs.readFileSync(new URL("./components/Sidebar.module.css", import.meta.url), "utf8");
    expect(folha).toMatch(/\.lateral \[data-bloco-grupos\],\s*\n\s*\.lateral\[data-com-cartao\] \[data-opcoes-do-envio\]/);
  });
});

describe("cartao das frentes prontas: acao a vista, antes do gesto", () => {
  it("a fileira de acoes vem antes da frase do gesto, e nao depende de rolar", () => {
    const pronto = cartao({ tipo: "frentes_prontas" }, { simulada: true, impressora: SIMULADA.id, nome: SIMULADA.nome });
    const markup = renderToStaticMarkup(
      <CartaoImpressao cartao={pronto} aoComandar={() => undefined} aoPedirVerso={() => undefined} aoPedirCancelar={() => undefined} aoDispensar={() => undefined} />,
    );
    const gesto = linhasDoCartao(pronto).find((linha) => linha.papel === "gesto");
    expect(gesto).toBeDefined();
    const botao = markup.indexOf(rotuloDoVerso(pronto));
    expect(botao).toBeGreaterThan(-1);
    expect(botao).toBeLessThan(markup.indexOf(gesto?.texto ?? ""));
  });
});

describe("cartao com quantidade de versos invalida", () => {
  it("o botao de versos fica desabilitado, com o motivo no titulo", () => {
    const pronto = cartao({ tipo: "frentes_prontas" }, { simulada: true, impressora: SIMULADA.id, nome: SIMULADA.nome });
    const motivo = traduzir("pt-BR", "copias.invalida");
    const markup = renderToStaticMarkup(
      <CartaoImpressao cartao={pronto} aoComandar={() => undefined} aoPedirVerso={() => undefined} aoPedirCancelar={() => undefined} aoDispensar={() => undefined} motivoDoVerso={motivo} />,
    );
    // Cada botao vai do seu <button> ao </button>: o rotulo fica depois da marcacao de abertura.
    const botoes = markup.match(/<button[^>]*>[^<]*<\/button>/g) ?? [];
    const botao = botoes.find((item) => item.includes(rotuloDoVerso(pronto))) ?? "";
    expect(botao).toContain("disabled");
    expect(botao).toContain(`title="${motivo}"`);
  });

  it("sem motivo, o botao de versos segue habilitado", () => {
    const pronto = cartao({ tipo: "frentes_prontas" }, { simulada: true, impressora: SIMULADA.id, nome: SIMULADA.nome });
    const markup = renderToStaticMarkup(
      <CartaoImpressao cartao={pronto} aoComandar={() => undefined} aoPedirVerso={() => undefined} aoPedirCancelar={() => undefined} aoDispensar={() => undefined} />,
    );
    expect(markup).not.toMatch(/<button[^>]*disabled=""[^>]*>Simular versos</);
  });
});
