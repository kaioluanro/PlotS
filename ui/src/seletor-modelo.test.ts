import { describe, expect, it } from "vitest";
import { conteudoDaSubstituicao } from "./confirmacao";
import { traduzir } from "./i18n";
import {
  abrirListaPeloTeclado,
  avisoDeFalhaAoGravar,
  avisosDoTopo,
  caminhoDoAviso,
  CALIBRAR,
  comModeloGravado,
  ESPERA_DO_INDICADOR_MS,
  estadoDoSeletor,
  gravarEscolha,
  indicaGravacao,
  interpretarEscolha,
  linhaDaFalha,
  nomeDeExibicao,
  pedeConfirmacao,
  teclaAbreALista,
  textoDoAviso,
  type SeletorComLista,
  type Impressora,
  type Modo,
} from "./impressoras";

function fila(modo: Modo, nome = "Fila"): Impressora {
  return { id: `fila:${nome}`, nome, tipo: "fila", nivel: "pronta", condicoes: [], modo };
}

function simulada(modo: Modo): Impressora {
  return { ...fila(modo), id: "simulada:x", tipo: "simulada" };
}

const visiveis = (impressora: Impressora) =>
  estadoDoSeletor(impressora, undefined, "pt-BR")
    ?.opcoes.filter((opcao) => !opcao.soEstado)
    .map((opcao) => opcao.rotulo);

describe("estado do seletor de modelo", () => {
  it("oferece o modelo automatico apenas quando o driver declara duas faces", () => {
    expect(visiveis(fila("sem_perfil"))).not.toContain("Frente e verso automático");
    expect(visiveis({ ...fila("sem_perfil"), duasFaces: true })).toContain("Frente e verso automático");
    expect(interpretarEscolha("automatico")).toEqual({ acao: "gravar", modelo: "automatico" });
  });

  it("mantem o modelo automatico visivel enquanto grava o gesto manual", () => {
    expect(estadoDoSeletor({ ...fila("automatico"), duasFaces: true }, "automatico:laser")?.valor).toBe("automatico");
    expect(nomeDeExibicao(simulada("automatico"), "en")).toBe("Simulated — Automatic");
  });
  it.each<[Modo, string, string]>([
    ["sem_perfil", "sem_perfil", "Sem perfil"],
    ["personalizado", "personalizado", "Personalizado"],
    ["jato", "jato", "Jato de tinta"],
    ["laser", "laser", "Laser"],
  ])("fila com modo %s mostra %s", (modo, valor, rotulo) => {
    const estado = estadoDoSeletor(fila(modo), undefined, "pt-BR");
    expect(estado?.valor).toBe(valor);
    expect(estado?.opcoes.find((opcao) => opcao.valor === valor)?.rotulo).toBe(rotulo);
  });

  it.each<Modo>(["sem_perfil", "personalizado", "jato", "laser"])(
    "com %s a lista oferece os dois modelos e calibrar, nessa ordem",
    (modo) => {
      expect(visiveis(fila(modo))).toEqual(["Jato de tinta", "Laser", "Calibrar…"]);
    },
  );

  it("sem perfil e personalizado sao estados, nao escolhas", () => {
    for (const modo of ["sem_perfil", "personalizado"] as const) {
      const estado = estadoDoSeletor(fila(modo), undefined, "pt-BR");
      expect(estado?.opcoes.filter((opcao) => opcao.soEstado).map((o) => o.valor)).toEqual([modo]);
    }
  });

  it("escolher modelo sobre um perfil personalizado avisa que substitui os valores", () => {
    expect(estadoDoSeletor(fila("personalizado"), undefined, "pt-BR")?.titulo).toMatch(/substitui/);
    expect(estadoDoSeletor(fila("personalizado"), undefined, "en")?.titulo).toMatch(/replaces/);
  });

  it("simulada e perfil nao lido nao tem seletor", () => {
    expect(estadoDoSeletor(simulada("jato"), undefined)).toBeNull();
    expect(estadoDoSeletor(simulada("laser"), undefined)).toBeNull();
    expect(estadoDoSeletor(fila("desconhecido"), undefined)).toBeNull();
  });

  it("durante a gravacao mostra a escolha; sem ela, volta ao salvo", () => {
    expect(estadoDoSeletor(fila("sem_perfil"), "laser")?.valor).toBe("laser");
    expect(estadoDoSeletor(fila("jato"), "laser")?.valor).toBe("laser");
    expect(estadoDoSeletor(fila("jato"), undefined)?.valor).toBe("jato");
  });

  it("todo valor do seletor tem uma opcao", () => {
    for (const modo of ["sem_perfil", "personalizado", "jato", "laser"] as const) {
      for (const gravando of [undefined, "jato", "laser"] as const) {
        const estado = estadoDoSeletor(fila(modo), gravando);
        expect(estado?.opcoes.map((opcao) => opcao.valor)).toContain(estado?.valor);
      }
    }
  });

  it("a simulada mantem o nome depois da troca de modo para modelo", () => {
    expect(nomeDeExibicao(simulada("jato"), "pt-BR")).toBe("Simulada — Traseira");
    expect(nomeDeExibicao(simulada("laser"), "pt-BR")).toBe("Simulada — Gaveta");
  });
});

describe("escolha no seletor", () => {
  it("modelos gravam, calibrar abre o dialogo e o resto nao faz nada", () => {
    expect(interpretarEscolha("jato")).toEqual({ acao: "gravar", modelo: "jato" });
    expect(interpretarEscolha("laser")).toEqual({ acao: "gravar", modelo: "laser" });
    expect(interpretarEscolha(CALIBRAR)).toEqual({ acao: "calibrar" });
    for (const valor of ["sem_perfil", "personalizado", "", "Jato"]) {
      expect(interpretarEscolha(valor), valor).toEqual({ acao: "nada" });
    }
  });
});

describe("gravar o modelo", () => {
  it("depois de gravar, so a fila escolhida mostra o modelo", () => {
    const lista = {
      impressoras: [fila("sem_perfil", "A"), fila("personalizado", "B"), simulada("jato")],
      avisos: [{ codigo: "filas" }],
    };
    const nova = comModeloGravado(lista, "A", "laser");
    expect(nova.impressoras.map((impressora) => impressora.modo)).toEqual([
      "laser",
      "personalizado",
      "jato",
    ]);
    expect(nova.avisos).toEqual(lista.avisos);
  });

  it("simulada com o mesmo nome da fila nao muda", () => {
    const lista = { impressoras: [simulada("jato")], avisos: [] };
    expect(comModeloGravado(lista, "Fila", "laser").impressoras[0]?.modo).toBe("jato");
  });

  it("gravacao aceita diz que gravou e passa a fila e o modelo", async () => {
    const pedidos: string[] = [];
    const resposta = await gravarEscolha("A", "jato", async (nome, modelo) => {
      pedidos.push(`${nome}=${modelo}`);
    });
    expect(resposta).toEqual({ gravado: true });
    expect(pedidos).toEqual(["A=jato"]);
  });

  it("falha ao gravar vira aviso com a fila e o arquivo, sem mudar a lista", async () => {
    const lista = { impressoras: [fila("personalizado", "A")], avisos: [] };
    const resposta = await gravarEscolha("A", "laser", () =>
      Promise.reject({ codigo: "modelo_nao_gravado", detalhe: "A", caminho: "C:\\p\\perfis.toml" }),
    );
    expect(resposta).toEqual({
      gravado: false,
      aviso: { codigo: "modelo_nao_gravado", detalhe: "A", caminho: "C:\\p\\perfis.toml" },
    });
    // Sem nada sendo gravado, o seletor mostra de novo o valor salvo.
    expect(estadoDoSeletor(lista.impressoras[0] as Impressora, undefined)?.valor).toBe("personalizado");
    if (!resposta.gravado) {
      expect(textoDoAviso(resposta.aviso, "pt-BR")).toContain("Não foi possível gravar o modelo de A.");
      expect(textoDoAviso(resposta.aviso, "en")).toContain("Could not save the model for A.");
      expect(caminhoDoAviso(resposta.aviso)).toBe("C:\\p\\perfis.toml");
    }
  });

  it.each([[undefined], [null], ["texto"], [{ caminho: 3 }], [{ caminho: "" }], [new Error("x")]])(
    "falha em formato inesperado (%j) ainda vira aviso, sem caminho",
    (falha) => {
      const aviso = avisoDeFalhaAoGravar(falha, "A");
      expect(aviso).toEqual({ codigo: "modelo_nao_gravado", detalhe: "A" });
      expect(caminhoDoAviso(aviso)).toBeNull();
    },
  );
});

describe("confirmar antes de substituir um perfil personalizado", () => {
  it("so o personalizado pede confirmacao", () => {
    expect(pedeConfirmacao("personalizado", undefined)).toBe(true);
    for (const modo of ["sem_perfil", "jato", "laser", "desconhecido"] as const) {
      expect(pedeConfirmacao(modo, undefined), modo).toBe(false);
    }
  });

  it("com uma gravacao em andamento nao abre outro pedido", () => {
    expect(pedeConfirmacao("personalizado", "laser")).toBe(false);
  });

  it.each([
    [
      "pt-BR",
      "Substituir o perfil personalizado de Sala?",
      "Os valores salvos para esta impressora serão trocados pelos do modelo Laser. Para recuperá-los, será preciso calibrar de novo.",
      "Cancelar",
      "Substituir perfil",
    ],
    [
      "en",
      "Replace the custom profile of Sala?",
      "The values saved for this printer will be replaced by those of the Laser model. To get them back, you will need to calibrate again.",
      "Cancel",
      "Replace profile",
    ],
  ] as const)("%s diz o que sera perdido e como recuperar", (idioma, titulo, texto, manter, fechar) => {
    const conteudo = conteudoDaSubstituicao({ fila: "Sala", modelo: "laser" }, idioma);
    expect(traduzir(idioma, conteudo.titulo, conteudo.parametros)).toBe(titulo);
    expect(traduzir(idioma, conteudo.texto, conteudo.parametros)).toBe(texto);
    expect(traduzir(idioma, conteudo.manter)).toBe(manter);
    expect(traduzir(idioma, conteudo.fechar)).toBe(fechar);
    expect(conteudo.consequencia).toBeNull();
    expect(conteudo.focoNaOrigemAoConfirmar).toBe(true);
  });

  it("o nome do modelo sai no idioma da interface", () => {
    const { parametros } = conteudoDaSubstituicao({ fila: "Sala", modelo: "jato" }, "en");
    expect(parametros.modelo).toBe("Inkjet");
  });
});

describe("indicador de gravacao lenta", () => {
  it("espera 400 ms", () => {
    expect(ESPERA_DO_INDICADOR_MS).toBe(400);
  });

  it.each([
    [0, false],
    [399, false],
    [400, true],
    [5000, true],
  ])("gravando ha %i ms: %s", (decorrido, esperado) => {
    expect(indicaGravacao("laser", decorrido)).toBe(esperado);
  });

  it("sem gravacao nunca aparece, mesmo com tempo sobrando", () => {
    expect(indicaGravacao(undefined, 5000)).toBe(false);
  });

  it("o texto existe nos dois idiomas", () => {
    expect(traduzir("pt-BR", "modo.seletor.gravando")).toBe("Gravando modelo…");
    expect(traduzir("en", "modo.seletor.gravando")).toBe("Saving model…");
  });
});

describe("rotulos do seletor", () => {
  it("o estado personalizado fica curto e o titulo mantem a palavra perfil", () => {
    const pt = estadoDoSeletor(fila("personalizado"), undefined, "pt-BR");
    const en = estadoDoSeletor(fila("personalizado"), undefined, "en");
    expect(pt?.opcoes.find((o) => o.valor === "personalizado")?.rotulo).toBe("Personalizado");
    expect(en?.opcoes.find((o) => o.valor === "personalizado")?.rotulo).toBe("Custom");
    expect(pt?.titulo).toMatch(/perfil/i);
    expect(en?.titulo).toMatch(/profile/i);
  });

  it.each<Modo>(["sem_perfil", "personalizado", "jato", "laser"])(
    "com %s so calibrar vem depois de um divisor",
    (modo) => {
      const opcoes = estadoDoSeletor(fila(modo), undefined, "pt-BR")?.opcoes ?? [];
      expect(opcoes.filter((o) => o.divisorAntes).map((o) => o.valor)).toEqual([CALIBRAR]);
    },
  );
});

describe("teclado com a lista do sistema", () => {
  function seletor(aberta: boolean | "sem suporte", abrir: () => void = () => undefined): SeletorComLista {
    return {
      matches() {
        if (aberta === "sem suporte") throw new SyntaxError("seletor desconhecido");
        return aberta;
      },
      showPicker: abrir,
    };
  }

  it.each(["ArrowDown", "ArrowUp", "Home", "End", "PageDown", "PageUp", "Enter"])(
    "%s abre a lista em vez de trocar o valor",
    (tecla) => {
      expect(teclaAbreALista(tecla)).toBe(true);
    },
  );

  it.each([" ", "Tab", "Escape", "a"])("%j segue seu caminho", (tecla) => {
    expect(teclaAbreALista(tecla)).toBe(false);
  });

  it("lista fechada abre e a tecla fica com o seletor", () => {
    let aberturas = 0;
    expect(abrirListaPeloTeclado(seletor(false, () => (aberturas += 1)))).toBe(true);
    expect(aberturas).toBe(1);
  });

  it("lista aberta deixa a tecla andar nas opcoes", () => {
    let aberturas = 0;
    expect(abrirListaPeloTeclado(seletor(true, () => (aberturas += 1)))).toBe(false);
    expect(aberturas).toBe(0);
  });

  it("sem saber se esta aberta, a tecla segue seu caminho", () => {
    expect(abrirListaPeloTeclado(seletor("sem suporte"))).toBe(false);
  });

  it("recusa do sistema ao abrir nao derruba nada e o valor nao muda", () => {
    const recusa = () => {
      throw new DOMException("sem gesto", "NotAllowedError");
    };
    expect(abrirListaPeloTeclado(seletor(false, recusa))).toBe(true);
  });
});

describe("falha ao gravar junto da impressora", () => {
  const falha = { codigo: "modelo_nao_gravado", detalhe: "A", caminho: "C:\\p\\perfis.toml" };
  const geral = { codigo: "filas" };

  it("com a linha na lista, a falha fica nela e o topo so tem avisos gerais", () => {
    const impressoras = [fila("personalizado", "A"), fila("jato", "B")];
    expect(linhaDaFalha(falha, impressoras)).toBe("fila:A");
    expect(avisosDoTopo([geral], falha, impressoras)).toEqual([geral]);
  });

  it("sem a linha, a falha nao some: vai para o topo", () => {
    const impressoras = [fila("jato", "B")];
    expect(linhaDaFalha(falha, impressoras)).toBeNull();
    expect(avisosDoTopo([geral], falha, impressoras)).toEqual([falha, geral]);
  });

  it("simulada com o mesmo nome nao recebe a falha", () => {
    const impressoras = [{ ...simulada("jato"), nome: "A" }];
    expect(linhaDaFalha(falha, impressoras)).toBeNull();
  });

  it("sem falha, nada muda", () => {
    expect(linhaDaFalha(null, [fila("jato", "A")])).toBeNull();
    expect(avisosDoTopo([geral], null, [])).toEqual([geral]);
  });
});
