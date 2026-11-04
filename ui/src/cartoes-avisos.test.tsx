import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  acoesDoCartao,
  frasesDoCartao,
  linhasDoCartao,
  textoDoEstadoDoCartao,
  type Cartao,
  type EstadoCartao,
  type Passada,
} from "./cartoes";
import { CartaoImpressao } from "./components/CartaoImpressao";
import { t } from "./i18n";

// Os CSS de modulo nao voltam como texto sob o teste: a folha e lida do disco.
interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}
const modulofs = "node:fs";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;

function cartao(estado: EstadoCartao, mudancas: Partial<Cartao> = {}): Cartao {
  return {
    aba: 1,
    impressora: "fila:A",
    nome: "A",
    simulada: false,
    lote: 1,
    total: 2,
    papel: null,
    estado,
    recolocar: { lado: "baixo", virada: "borda_de_cima", textoDoPerfil: null },
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    ...mudancas,
  };
}

function aviso(codigo: string, passada: Passada, texto = "frase qualquer da fila"): EstadoCartao {
  return { tipo: "aviso", passada, motivo: { codigo, texto }, respondido: false };
}

const TABELA: readonly (readonly [string, Passada, { pt: string[]; en: string[] }])[] = [
  [
    "sem_papel",
    "frente",
    {
      pt: ["A impressora está sem papel.", "Coloque folhas em branco na entrada."],
      en: ["The printer is out of paper.", "Add blank sheets to the feed."],
    },
  ],
  [
    "sem_papel",
    "verso",
    {
      pt: [
        "A impressora acusa falta de papel.",
        "Não coloque folhas em branco. Se ainda há parte do maço, coloque o resto agora, sem mudar a ordem, e retome na impressora.",
        "Se o maço acabou antes do fim, o trecho desalinhou: use Reimprimir.",
      ],
      en: [
        "The printer reports it is out of paper.",
        "Do not add blank sheets. If part of the stack is still out, load the rest now, without changing the order, and resume at the printer.",
        "If the stack ran out early, the section is misaligned: use Reprint.",
      ],
    },
  ],
  [
    "atolado",
    "frente",
    {
      pt: ["O papel atolou na impressora.", "Retire a folha presa como indica o manual da impressora."],
      en: ["The paper is jammed in the printer.", "Remove the stuck sheet as the printer manual describes."],
    },
  ],
  [
    "atolado",
    "verso",
    {
      pt: [
        "O papel atolou na impressora.",
        "Retire a folha presa como indica o manual da impressora. O trecho pode ter desalinhado: confira antes de seguir.",
      ],
      en: [
        "The paper is jammed in the printer.",
        "Remove the stuck sheet as the printer manual describes. The section may be misaligned: check it before going on.",
      ],
    },
  ],
  [
    "sem_resposta",
    "frente",
    {
      pt: ["A impressora não responde.", "Confira se ela está ligada e conectada."],
      en: ["The printer is not responding.", "Check that it is on and connected."],
    },
  ],
  [
    "sem_resposta",
    "verso",
    {
      pt: ["A impressora não responde.", "Confira se ela está ligada e conectada."],
      en: ["The printer is not responding.", "Check that it is on and connected."],
    },
  ],
];

describe("aviso de cada condicao, por passada", () => {
  it.each(TABELA)("%s na passada de %s", (codigo, passada, esperado) => {
    const pedido = cartao(aviso(codigo, passada));
    expect(linhasDoCartao(pedido, "pt-BR").map((linha) => linha.texto)).toEqual(esperado.pt);
    expect(linhasDoCartao(pedido, "en").map((linha) => linha.texto)).toEqual(esperado.en);
  });

  it("a orientacao vem do codigo: a mesma frase sem o codigo nao muda nada", () => {
    const pedido = cartao(aviso("fila", "verso", "a impressora está sem papel"));
    expect(linhasDoCartao(pedido, "pt-BR").map((linha) => linha.texto)).toEqual([
      "A impressora está sem papel.",
    ]);
    expect(textoDoEstadoDoCartao(pedido, "pt-BR")).toBe("Aguardando a impressora");
  });

  it("forte, instrucao e ressalva tem o papel certo", () => {
    const linhas = linhasDoCartao(cartao(aviso("sem_papel", "verso")), "pt-BR");
    expect(linhas.map((linha) => linha.papel)).toEqual(["contexto", "acao", "ressalva"]);
  });

  it("respondido, a orientacao continua e diz que segue esperando", () => {
    const estado: EstadoCartao = {
      tipo: "aviso",
      passada: "frente",
      motivo: { codigo: "atolado", texto: "x" },
      respondido: true,
    };
    const linhas = linhasDoCartao(cartao(estado), "pt-BR");
    expect(linhas.map((linha) => linha.texto)).toEqual([
      "O papel atolou na impressora.",
      "Retire a folha presa como indica o manual da impressora.",
      "Continuando a esperar o trabalho na fila.",
    ]);
    expect(linhas.at(-1)?.papel).toBe("ressalva");
  });

  it.each(["sem_papel", "atolado", "sem_resposta"])("%s muda o estado para verificar", (codigo) => {
    for (const passada of ["frente", "verso"] as const) {
      const pedido = cartao(aviso(codigo, passada));
      expect(textoDoEstadoDoCartao(pedido, "pt-BR")).toBe("Verifique a impressora");
      expect(textoDoEstadoDoCartao(pedido, "en")).toBe("Check the printer");
    }
  });
});

function html(pedido: Cartao): string {
  return renderToStaticMarkup(
    <CartaoImpressao
      cartao={pedido}
      aoComandar={() => {}}
      aoPedirVerso={() => undefined}
      aoPedirCancelar={() => {}}
      aoDispensar={() => {}}
    />,
  );
}

describe("botoes no aviso", () => {
  it.each(["frente", "verso"] as const)("na passada de %s, reimprimir aparece", (passada) => {
    const pedido = cartao(aviso("sem_papel", passada));
    expect(acoesDoCartao(pedido).reimprimir).toBe(true);
    expect(html(pedido)).toMatch(/<button[^>]*>(Reimprimir|Reprint)<\/button>/);
  });

  it.each(["frente", "verso"] as const)(
    "na passada de %s, a ordem e verso, continuar, reimprimir e cancelar",
    (passada) => {
      const rotulos = [...html(cartao(aviso("atolado", passada))).matchAll(/<button[^>]*>([^<]*)<\/button>/g)].map(
        (achado) => achado[1],
      );
      const emPortugues = rotulos.includes("Reimprimir");
      expect(rotulos).toEqual(
        emPortugues
          ? ["Imprimir versos", "Continuar esperando", "Reimprimir", "Cancelar trabalho"]
          : ["Print backs", "Keep waiting", "Reprint", "Cancel job"],
      );
    },
  );

  it("sem lote ainda, nao ha o que reimprimir", () => {
    expect(acoesDoCartao(cartao(aviso("sem_papel", "frente"), { total: 0 })).reimprimir).toBe(false);
  });

  it.each(["frente", "verso"] as const)(
    "na passada de %s, o verso fica visivel e desabilitado",
    (passada) => {
      const pedido = cartao(aviso("atolado", passada));
      expect(acoesDoCartao(pedido).verso).toEqual({ visivel: true, liberado: false });
      expect(html(pedido)).toMatch(/<button[^>]*disabled=""[^>]*>[^<]*<\/button>/);
    },
  );
});

describe("mensagem da impressora no aviso", () => {
  const comMensagem = (codigo: string, mensagemDoDriver: string | null, respondido = false): EstadoCartao => ({
    tipo: "aviso",
    passada: "verso",
    motivo: { codigo, texto: "a impressora está sem papel" },
    mensagemDoDriver,
    respondido,
  });

  it("vem como ultima linha, com o papel de ressalva, nos dois idiomas", () => {
    const pedido = cartao(comMensagem("sem_papel", "Bandeja 2 vazia", true));
    const pt = linhasDoCartao(pedido, "pt-BR");
    expect(pt.at(-1)).toMatchObject({ texto: "Mensagem da impressora: Bandeja 2 vazia", papel: "ressalva" });
    const en = linhasDoCartao(pedido, "en");
    expect(en.at(-1)).toMatchObject({ texto: "Printer message: Bandeja 2 vazia", papel: "ressalva" });
  });

  it("a frase principal fica so com a condicao mais urgente, sem o texto do driver", () => {
    const linhas = linhasDoCartao(cartao(comMensagem("atolado", "Bandeja 2")), "pt-BR");
    expect(linhas.map((linha) => linha.texto)).toEqual([
      "O papel atolou na impressora.",
      "Retire a folha presa como indica o manual da impressora. O trecho pode ter desalinhado: confira antes de seguir.",
      "Mensagem da impressora: Bandeja 2",
    ]);
  });

  it("parenteses no motivo nao viram mensagem: ela so vem do campo proprio", () => {
    const pedido = cartao(aviso("fila", "frente", "a impressora pede atenção (Bandeja 2)"));
    expect(frasesDoCartao(pedido, "pt-BR").join(" ")).not.toContain("Mensagem da impressora");
  });

  it.each([null, "", undefined])("sem texto (%s), a linha nao existe", (mensagem) => {
    const pedido = cartao({ ...aviso("atolado", "frente"), mensagemDoDriver: mensagem } as EstadoCartao);
    expect(frasesDoCartao(pedido, "pt-BR").join(" ")).not.toContain("Mensagem da impressora");
    expect(html(pedido)).not.toContain('title="Mensagem');
  });

  it("no pedido de atencao, sem orientacao propria, tambem aparece", () => {
    const linhas = linhasDoCartao(cartao(comMensagem("fila", "Tampa aberta")), "pt-BR");
    expect(linhas.at(-1)?.texto).toBe("Mensagem da impressora: Tampa aberta");
  });

  it("o cartao limita a duas linhas e guarda o texto inteiro no titulo", () => {
    const longa = "x".repeat(200);
    const pagina = html(cartao(comMensagem("sem_papel", longa)));
    expect(pagina).toContain(`title="${t("spooler.mensagemDaImpressora", { texto: longa })}"`);
    const folha = fs.readFileSync(
      new URL("./components/CartaoImpressao.module.css", import.meta.url),
      "utf8",
    );
    const inicio = folha.indexOf(".fraseLimitada {");
    expect(inicio).toBeGreaterThan(-1);
    const regra = folha.slice(inicio, folha.indexOf("}", inicio));
    expect(regra).toMatch(/line-clamp:\s*2/);
    expect(regra).toMatch(/overflow:\s*hidden/);
    expect(regra).toMatch(/overflow-wrap:\s*anywhere/);
  });

  it("marcacao que escapar chega como texto, nunca como elemento", () => {
    const pagina = html(cartao(comMensagem("sem_papel", "<b>oi</b>")));
    expect(pagina).not.toContain("<b>");
  });
});

describe("foco vindo da linha da rolagem adiada", () => {
  it("o cartao aceita foco por programa, sem entrar na ordem do Tab", () => {
    expect(html(cartao({ tipo: "aguardando" }))).toMatch(/<article[^>]*tabindex="-1"/);
  });
});

describe("ordem do cartao que espera a recolocacao", () => {
  it("dados e sequencia; ressalva e espera; figura; gesto; clique; botoes", () => {
    const pagina = html(
      cartao(
        { tipo: "aguardando" },
        {
          sequencia: {
            frente: [{ numero: 2, girada: false }],
            verso: [{ numero: 1, girada: true }],
          },
        },
      ),
    );
    const figura = pagina.indexOf("<svg");
    const botoes = pagina.indexOf("<button");
    expect(figura).toBeGreaterThan(-1);
    expect(botoes).toBeGreaterThan(figura);
    const frases = [...pagina.matchAll(/<p [^>]*>([^<]*)<\/p>/g)];
    const antes = frases.filter((frase) => frase.index < figura).map((frase) => frase[1]);
    const depois = frases.filter((frase) => frase.index > figura).map((frase) => frase[1]);
    // Depois da figura, so a virada, a colocacao e o clique; a sequencia e a espera vem antes.
    expect(depois).toHaveLength(3);
    expect(antes.length).toBeGreaterThanOrEqual(4);
    const emClaro = pagina.replace(/<[^>]+>/g, "|");
    expect(emClaro.indexOf(depois[0] ?? "?")).toBeGreaterThan(emClaro.indexOf(antes.at(-1) ?? "?"));
  });
});

describe("tipografia das frases e margem de rolagem do cartao", () => {
  const folha = fs.readFileSync(
    new URL("./components/CartaoImpressao.module.css", import.meta.url),
    "utf8",
  );
  const css = `\n${folha.replaceAll("\r\n", "\n")}`;
  const regra = (seletor: string) => {
    const inicio = css.lastIndexOf(`\n${seletor} {`);
    expect(inicio, seletor).toBeGreaterThan(-1);
    return css.slice(inicio, css.indexOf("}", inicio));
  };

  it("todas as frases em 12/16", () => {
    const comum = regra(".frase,\n.fraseForte,\n.fraseRessalva,\n.fraseGesto,\n.fraseAcao");
    expect(comum).toMatch(/font-size:\s*12px/);
    expect(comum).toMatch(/line-height:\s*16px/);
  });

  it("forte e gesto em 600 com a cor do texto; clique em 400", () => {
    expect(regra(".fraseForte")).toMatch(/font-weight:\s*600/);
    expect(regra(".fraseGesto")).toMatch(/font-weight:\s*600/);
    expect(regra(".fraseGesto")).toMatch(/color:\s*var\(--fg\)/);
    expect(regra(".fraseAcao")).toMatch(/font-weight:\s*400/);
  });

  it("a rolagem deixa 8 px em volta do cartao", () => {
    expect(regra(".cartao")).toMatch(/scroll-margin-top:\s*8px/);
    expect(regra(".cartao")).toMatch(/scroll-margin-bottom:\s*8px/);
  });
});
