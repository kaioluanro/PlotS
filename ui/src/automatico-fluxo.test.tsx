import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  acoesDoCartao,
  cartaoEmAndamento,
  linhasDoCartao,
  linhasDoEnvio,
  sequenciaDoCartao,
  type Cartao,
  type EstadoCartao,
} from "./cartoes";
import { CartaoImpressao } from "./components/CartaoImpressao";
import { AjusteSemLinha, LinhaImpressora } from "./components/Sidebar";
import { conteudoDoEnvio } from "./confirmacao";
import { comModeloGravado, estadoDoSeletor, type Impressora } from "./impressoras";
import { traduzir } from "./i18n";
import { estadoInicial, reduzirAbas, type Aba } from "./tabs";

const AUTOMATICA: Impressora = {
  id: "simulada:automatica",
  nome: "Simulada — Automática",
  tipo: "simulada",
  nivel: "pronta",
  condicoes: [],
  modo: "automatico",
  duasFaces: true,
};
const TRASEIRA: Impressora = {
  id: "simulada:traseira",
  nome: "Simulada — Traseira",
  tipo: "simulada",
  nivel: "pronta",
  condicoes: [],
  modo: "jato",
};
const fila = (mudancas: Partial<Impressora>): Impressora => ({
  id: "fila:Sala",
  nome: "Sala",
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "sem_perfil",
  ...mudancas,
});

function cartao(estado: EstadoCartao, mudancas: Partial<Cartao> = {}): Cartao {
  return {
    aba: 1,
    impressora: AUTOMATICA.id,
    nome: AUTOMATICA.nome,
    simulada: true,
    lote: 1,
    total: 1,
    papel: null,
    estado,
    recolocar: { lado: "baixo", virada: "borda_de_cima", textoDoPerfil: null },
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    loteAutomatico: true,
    temAutomatico: true,
    ...mudancas,
  };
}

const textos = (item: Cartao, idioma: "pt-BR" | "en" = "pt-BR") =>
  linhasDoCartao(item, idioma).map((linha) => linha.texto);

const RESUMO = {
  paginas: 8,
  lotes: 4,
  papeis: [],
  paresMistos: [],
  assinado: false,
  lotesAutomaticos: 1,
  fluxos: [{ automatico: true, inicio: 1, fim: 8 }],
};

describe("frase final da impressao automatica", () => {
  const FINAL = "Se a primeira página não estiver em cima, vire o maço.";

  it("documento misto que acaba em papel manual ainda avisa do maco automatico", () => {
    const fim = cartao(
      { tipo: "concluido" },
      { loteAutomatico: false, anteriorAutomatico: false, viraParaLer: true },
    );
    expect(textos(fim)).toContain(FINAL);
    expect(textos(fim)).not.toContain(traduzir("pt-BR", "cartao.virarParaLer"));
  });

  it("documento que so saiu pelo fluxo manual mantem a frase do modelo", () => {
    const fim = cartao(
      { tipo: "concluido" },
      { loteAutomatico: false, temAutomatico: false, viraParaLer: true },
    );
    expect(textos(fim)).toContain(traduzir("pt-BR", "cartao.virarParaLer"));
    expect(textos(fim)).not.toContain(FINAL);
  });

  it("existe nos dois idiomas com o texto combinado", () => {
    const fim = cartao({ tipo: "concluido" }, { anteriorAutomatico: true });
    expect(textos(fim, "en")).toContain("If the first page is not on top, flip the stack.");
  });
});

describe("cartao de lote automatico", () => {
  it("na simulacao, a ordem aparece em folhas, sem falar em frente e verso", () => {
    const face = (numero: number) => ({ numero, girada: false });
    const item = cartao(
      { tipo: "concluido" },
      { sequencia: { frente: [1, 2, 3, 4, 5].map(face), verso: [] } },
    );
    expect(sequenciaDoCartao(item, "pt-BR")).toEqual(["Folhas: 1|2, 3|4, 5"]);
    expect(sequenciaDoCartao(item, "en")).toEqual(["Sheets: 1|2, 3|4, 5"]);
    const manual = cartao(
      { tipo: "aguardando" },
      { loteAutomatico: false, sequencia: { frente: [face(2)], verso: [face(1)] } },
    );
    expect(sequenciaDoCartao(manual, "pt-BR")).toEqual(["Frente: 2", "Verso: 1"]);
  });

  it("nao tem figura nem verso em nenhum estado do envio", () => {
    const estados: EstadoCartao[] = [
      { tipo: "imprimindo", passada: "duas_faces" },
      {
        tipo: "aviso",
        passada: "duas_faces",
        motivo: { codigo: "sem_papel", texto: "x" },
        respondido: false,
      },
    ];
    for (const estado of estados) {
      const item = cartao(estado);
      const html = renderToStaticMarkup(
        <CartaoImpressao
          cartao={item}
          aoComandar={() => undefined}
          aoPedirVerso={() => undefined}
          aoPedirCancelar={() => undefined}
          aoDispensar={() => undefined}
        />,
      );
      expect(html).not.toContain("<svg role");
      expect(html).not.toContain(traduzir("pt-BR", "cartao.simular.verso"));
      expect(acoesDoCartao(item).verso.visivel).toBe(false);
    }
  });

  it("a recusa do driver manda escolher um modelo manual, em vez da frase tecnica", () => {
    const motivo = { codigo: "duas_faces_recusadas", texto: "o driver não aceitou imprimir nas duas faces; nada foi enviado" };
    const item = cartao({ tipo: "erro", motivo });
    expect(textos(item)).toEqual(["O driver recusou duas faces. Nada foi enviado. Escolha um modelo manual."]);
    expect(textos(item, "en")).toEqual([
      "The driver rejected two-sided printing. Nothing was sent. Choose a manual model.",
    ]);
  });

  it("no aviso de papel pede folhas em branco e deixa reimprimir o lote", () => {
    const item = cartao({
      tipo: "aviso",
      passada: "duas_faces",
      motivo: { codigo: "sem_papel", texto: "x" },
      respondido: false,
    });
    expect(textos(item)).toContain("Coloque folhas em branco na entrada.");
    expect(acoesDoCartao(item).reimprimir).toBe(true);
    expect(cartaoEmAndamento(item)).toBe(true);
  });
});

describe("confirmacao com impressora automatica", () => {
  it("diz as paginas e as folhas, com ou sem impressora com gesto", () => {
    const comFolhas = { ...RESUMO, folhas: 4 };
    const { linhas } = linhasDoEnvio([AUTOMATICA], comFolhas, "pt-BR", new Set());
    expect(linhas).toContain("8 pág · 4 folhas");
    const junto = linhasDoEnvio([AUTOMATICA, TRASEIRA], comFolhas, "pt-BR", new Set());
    expect(junto.linhas).toContain("8 pág · 4 folhas");
  });

  it("so com a automatica, diz o fluxo de cada trecho sem condicional", () => {
    const { notas } = linhasDoEnvio(
      [AUTOMATICA],
      {
        ...RESUMO,
        fluxos: [
          { automatico: true, inicio: 1, fim: 4 },
          { automatico: false, inicio: 5, fim: 5 },
        ],
      },
      "pt-BR",
      new Set(),
    );
    expect(notas).toContain("Frente e verso automático, sem gesto: págs. 1–4.");
    expect(notas).toContain("Frente e verso manual, com gesto: pág. 5.");
    expect(notas).toContain("Use papel normal nos trechos automáticos.");
  });

  it("junto de outra impressora, diz de qual ela fala", () => {
    const { notas } = linhasDoEnvio([AUTOMATICA, TRASEIRA], RESUMO, "en", new Set());
    expect(notas).toContain("On Simulated — Automatic, pp. 1–8: automatic duplex, no handling.");
  });

  it("sem nenhum trecho automatico, nao pede papel normal", () => {
    const { notas } = linhasDoEnvio(
      [AUTOMATICA],
      { ...RESUMO, fluxos: [{ automatico: false, inicio: 1, fim: 8 }] },
      "pt-BR",
      new Set(),
    );
    expect(notas).not.toContain("Use papel normal nos trechos automáticos.");
  });
});

describe("confirmacao so com simuladas", () => {
  it("nomeia quem recebe e diz que nada vai a impressora", () => {
    const { linhas, notas } = linhasDoEnvio([AUTOMATICA, TRASEIRA], RESUMO, "pt-BR", new Set());
    expect(linhas).toContain("Simulada — Automática");
    expect(linhas).toContain("Simulada — Traseira");
    expect(notas).toContain("Simulação: nada vai à impressora.");
    expect(notas).not.toContain(traduzir("pt-BR", "cartao.envio.simuladas"));
  });

  it("chama de simulacao o titulo e o botao", () => {
    const conteudo = conteudoDoEnvio([], [], true, true);
    expect(conteudo.titulo).toBe("modo.automatico.simulacaoTitulo");
    expect(conteudo.fechar).toBe("modo.automatico.simulacaoConfirmar");
  });

  it("com uma fila junto, a simulada segue como nota e a fila e quem aparece", () => {
    const real = fila({ modo: "automatico", duasFaces: true });
    const { linhas, notas } = linhasDoEnvio([real, TRASEIRA], RESUMO, "pt-BR", new Set());
    expect(linhas[0]).toBe("Sala");
    expect(linhas).not.toContain("Simulada — Traseira");
    expect(notas).toContain(traduzir("pt-BR", "cartao.envio.simuladas"));
  });
});

describe("escolha do modelo automatico na lista", () => {
  const lista = (impressora: Impressora) => ({ impressoras: [impressora], avisos: [] });

  it("fila laser que passa a automatica mostra o gesto que foi gravado", () => {
    const nova = comModeloGravado(lista(fila({ modo: "laser", duasFaces: true })), "Sala", "automatico");
    expect(nova.impressoras[0]?.modo).toBe("automatico");
    expect(nova.impressoras[0]?.gestoManual).toBe("laser");
  });

  it("fila sem perfil fica com o gesto de jato, o padrao", () => {
    const nova = comModeloGravado(lista(fila({ duasFaces: true })), "Sala", "automatico");
    expect(nova.impressoras[0]?.gestoManual).toBe("jato");
  });

  it("voltar a um modelo manual tira o gesto a parte", () => {
    const nova = comModeloGravado(
      lista(fila({ modo: "automatico", gestoManual: "laser", duasFaces: true })),
      "Sala",
      "laser",
    );
    expect(nova.impressoras[0]?.modo).toBe("laser");
    expect(nova.impressoras[0]?.gestoManual).toBeUndefined();
  });

  it("fila sem duas faces nao oferece o modelo", () => {
    const opcoes = estadoDoSeletor(fila({ modo: "jato" }), undefined)?.opcoes ?? [];
    expect(opcoes.map((opcao) => opcao.valor)).not.toContain("automatico");
  });
});

describe("linha da impressora automatica", () => {
  const sem = () => undefined;
  const linha = (impressora: Impressora) =>
    renderToStaticMarkup(
      <LinhaImpressora
        impressora={impressora}
        marcada={false}
        desabilitada={false}
        idExplicacao={undefined}
        idNotaDoModelo="nota"
        gravando={undefined}
        falha={null}
        aoAlternar={sem}
        aoEscolherModelo={sem}
        aoPedirSubstituicao={sem}
        aoCalibrar={sem}
        comAjuste={false}
        duasFaces={false}
        janelaAberta={false}
        avisoDoAjuste={null}
        aoAbrirPreferencias={sem}
        aoAbrirOpcoes={sem}
        aoPedirVolta={sem}
        aoDispensarAviso={sem}
      />,
    );

  it("o gesto dos outros papeis tem nome visivel, e nao so para leitor de tela", () => {
    const html = linha(fila({ modo: "automatico", gestoManual: "laser", duasFaces: true }));
    expect(html).toMatch(/<label[^>]*><span[^>]*>Gesto para outros papéis<\/span>/);
    expect(html).not.toContain('aria-label="Gesto para outros papéis"');
  });

  it("o X do ajuste fora da lista fica na fileira do texto, antes de apagar", () => {
    const html = renderToStaticMarkup(
      <AjusteSemLinha fila="Sala" aoApagar={sem} aoDispensar={sem} />,
    );
    expect(html.indexOf("Dispensar aviso")).toBeLessThan(html.indexOf("Apagar ajuste"));
  });

  it("a simulada nao oferece a escolha do gesto", () => {
    expect(linha(AUTOMATICA)).not.toContain("Gesto para outros papéis");
  });
});

describe("redutor", () => {
  it("leva o cartao automatico para a aba sem perder os campos novos", () => {
    const aberta = reduzirAbas(estadoInicial, {
      tipo: "abrir",
      documento: { caminho: "C:\\a.pdf", nome: "a.pdf" },
    });
    const aba = (aberta.abas[0] as Aba).id;
    const item = cartao({ tipo: "imprimindo", passada: "duas_faces" }, { aba });
    const depois = reduzirAbas(aberta, { tipo: "cartao", cartao: item });
    const guardado = depois.abas[0]?.cartoes[0];
    expect(guardado?.loteAutomatico).toBe(true);
    expect(guardado?.temAutomatico).toBe(true);
    expect(depois.abas[0]?.cartoes.some(cartaoEmAndamento)).toBe(true);
  });
});

describe("garantias do automatico que voltam sem a palavra lote", () => {
  // Restaurado do teste de contagem de trabalhos: a contagem de lotes saiu da janela (plano 24).
  it("conta as paginas da automatica, sem a palavra lote", () => {
    const resumo = { ...RESUMO, fluxos: [{ automatico: true, inicio: 1, fim: 8 }] };
    const { linhas } = linhasDoEnvio([AUTOMATICA], resumo, "pt-BR", new Set());
    expect(linhas).toContain("8 pág");
    expect(linhas.join(" ")).not.toMatch(/lote/);
  });
});
