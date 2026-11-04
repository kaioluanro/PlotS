import * as Dialog from "@radix-ui/react-dialog";
import { isValidElement, type ReactElement, type ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";

const backend = vi.hoisted(() => ({
  gravarCalibracao: vi.fn(() => Promise.resolve()),
  encerrarCalibracao: vi.fn(() => Promise.resolve()),
}));

vi.mock("./calibracao-backend", async (original) => ({
  ...(await original<typeof import("./calibracao-backend")>()),
  ...backend,
}));

import { estadoInicial, type Acao, type Estado, type PerfilDaRodada } from "./calibracao";
import type { Trava } from "./calibracao-envio";
import { Conteudo, CorpoDoPasso, criarSaidas, LinhaDeEnvio, Respostas, Rodape } from "./components/CalibracaoDialog";
import { t } from "./i18n";
import type { Impressora } from "./impressoras";

/** Uma trava ocupada, como a da tela no meio de uma chamada: nada do que a pessoa fizer deve sair dela. */
const ocupada = (ocupacao: "enviando" | "gravando"): Trava => ({
  atual: () => ocupacao,
  executar: () => false,
});

/** Uma trava livre: roda a tarefa na hora, como a da tela quando nada esta em andamento. */
const LIVRE: Trava = {
  atual: () => null,
  executar: (_ocupacao, tarefa) => {
    void tarefa();
    return true;
  },
};

const FILA: Impressora = {
  id: "fila:Sala",
  nome: "Sala",
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "sem_perfil",
};

const PERFIL: PerfilDaRodada = {
  frenteInversa: true,
  versoInverso: true,
  versoGirado: false,
  virada: "lateral",
  borda: "curta",
  driverInverte: false,
};

const CERTO: Estado = { ...estadoInicial(FILA), passo: "certo", perfil: PERFIL, saiuParaCima: true };

/** Os botoes que o rodape devolveu, para apertar pelo proprio `onClick`. */
function botoes(no: ReactNode): ReactElement<{ onClick: () => void; children: string }>[] {
  if (Array.isArray(no)) return no.flatMap(botoes);
  if (!isValidElement<{ children?: ReactNode }>(no)) return [];
  if (no.type === "button") return [no as ReactElement<{ onClick: () => void; children: string }>];
  return botoes(no.props.children);
}

/** O rodape de verdade, ligado as saidas de verdade, com o backend simulado. */
interface OpcoesDaTela {
  simulada?: boolean;
  comErro?: boolean;
  naoServe?: boolean;
  trava?: Trava;
}

function montar(estado: Estado, opcoes: OpcoesDaTela = {}) {
  const { simulada = false, comErro = false, naoServe = false, trava = LIVRE } = opcoes;
  let confirmando = false;
  const aoFechar = vi.fn();
  const despachar = vi.fn<(acao: Acao) => void>();
  const saidas = criarSaidas({
    estado: () => estado,
    id: FILA.id,
    ehSimulada: simulada,
    despachar,
    aoFechar,
    confirmando: () => confirmando,
    confirmar: (aberta) => {
      confirmando = aberta;
    },
    trava,
  });
  const nada = () => undefined;
  const apertar = (rotulo: string) => {
    const lista = botoes(
      Rodape({
        estado,
        ehSimulada: simulada,
        comErro,
        naoServe,
        confirmando,
        ocupado: trava.atual() !== null,
        respondida: false,
        aoDesistir: saidas.pedirDesistir,
        aoDesistirDeVez: saidas.desistirDeVez,
        aoFecharConfirmacao: () => {
          confirmando = false;
        },
        aoFechar: nada,
        aoVoltar: nada,
        aoContinuar: nada,
        aoSeguir: nada,
        aoImprimirFrente: nada,
        aoImprimirVerso: nada,
        aoImprimirDeNovo: nada,
        aoRefazer: nada,
        aoGravar: saidas.gravar,
      }),
    );
    const botao = lista.find((item) => item.props.children === rotulo);
    if (botao === undefined) throw new Error(`sem o botao ${rotulo}: ${lista.map((b) => b.props.children).join(", ")}`);
    botao.props.onClick();
  };
  return { apertar, aoFechar, despachar, saidas, estaConfirmando: () => confirmando };
}

const rotulos = (props: Partial<Parameters<typeof Rodape>[0]> & { estado: Estado }) => {
  const nada = () => undefined;
  return botoes(
    Rodape({
      ehSimulada: false,
      comErro: false,
      naoServe: false,
      confirmando: false,
      respondida: false,
      aoDesistir: nada,
      aoDesistirDeVez: nada,
      aoFecharConfirmacao: nada,
      aoFechar: nada,
      aoVoltar: nada,
      aoContinuar: nada,
      aoSeguir: nada,
      aoImprimirFrente: nada,
      aoImprimirVerso: nada,
      aoImprimirDeNovo: nada,
      aoRefazer: nada,
      aoGravar: nada,
      ...props,
    }),
  ).map((botao) => botao.props.children);
};

const telaInicial = (impressora: Impressora) =>
  renderToStaticMarkup(
    <Dialog.Root>
      <Conteudo impressora={impressora} aoFechar={() => undefined} />
    </Dialog.Root>,
  );

beforeEach(() => {
  backend.gravarCalibracao.mockClear();
  backend.encerrarCalibracao.mockClear();
});

describe("desistir nao grava", () => {
  it("na tela certa, o botao pede confirmacao e so depois fecha, sem gravar", () => {
    const tela = montar(CERTO);
    tela.apertar(t("calibracao.desistir"));
    expect(tela.estaConfirmando()).toBe(true);
    expect(tela.aoFechar).not.toHaveBeenCalled();
    expect(backend.encerrarCalibracao).not.toHaveBeenCalled();

    tela.apertar(t("calibracao.desistir.confirmar"));
    expect(tela.aoFechar).toHaveBeenCalledWith(false);
    expect(backend.encerrarCalibracao).toHaveBeenCalledWith(FILA.id, false);
    expect(backend.gravarCalibracao).not.toHaveBeenCalled();
  });

  it("voltar da confirmacao nao fecha, e gravar ainda funciona", () => {
    const tela = montar(CERTO);
    tela.apertar(t("calibracao.desistir"));
    tela.apertar(t("calibracao.voltar"));
    expect(tela.estaConfirmando()).toBe(false);
    expect(tela.aoFechar).not.toHaveBeenCalled();
    tela.apertar(t("calibracao.gravar"));
    expect(backend.gravarCalibracao).toHaveBeenCalledTimes(1);
    expect(backend.gravarCalibracao).toHaveBeenCalledWith(FILA.id, PERFIL, true, "sem_virar");
  });

  it("o Esc faz o mesmo caminho: pergunta, e o segundo Esc so volta", () => {
    const tela = montar(CERTO);
    tela.saidas.pedirDesistir();
    expect(tela.estaConfirmando()).toBe(true);
    tela.saidas.pedirDesistir();
    expect(tela.estaConfirmando()).toBe(false);
    expect(tela.aoFechar).not.toHaveBeenCalled();
    expect(backend.gravarCalibracao).not.toHaveBeenCalled();
  });

  it("antes da tela certa, desistir fecha de uma vez e nao grava", () => {
    const tela = montar({ ...CERTO, passo: "ajustar" });
    tela.apertar(t("calibracao.desistir"));
    expect(tela.aoFechar).toHaveBeenCalledWith(false);
    expect(backend.gravarCalibracao).not.toHaveBeenCalled();
  });

  it("com uma passada na fila, desistir pede para tira-la", () => {
    const tela = montar({ ...CERTO, passo: "verso" });
    tela.apertar(t("calibracao.desistir"));
    expect(backend.encerrarCalibracao).toHaveBeenCalledWith(FILA.id, true);
    expect(backend.gravarCalibracao).not.toHaveBeenCalled();
  });

  it("a simulada nao tem folha gasta nem o que gravar: nao pergunta", () => {
    const tela = montar(CERTO, { simulada: true });
    tela.apertar(t("calibracao.desistir"));
    expect(tela.estaConfirmando()).toBe(false);
    expect(tela.aoFechar).toHaveBeenCalledWith(false);
    tela.apertar(t("calibracao.concluir"));
    expect(backend.gravarCalibracao).not.toHaveBeenCalled();
  });
});

describe("recusa da fila que pede arquivo", () => {
  it("so oferece Fechar, e fechar encerra sem pedir para cancelar", () => {
    const estado = { ...estadoInicial(FILA), passo: "frente" as const };
    expect(rotulos({ estado, comErro: true, naoServe: true })).toEqual([t("calibracao.fechar")]);
    const tela = montar(estado, { comErro: true, naoServe: true });
    tela.apertar(t("calibracao.fechar"));
    expect(tela.aoFechar).toHaveBeenCalledWith(false);
    expect(backend.gravarCalibracao).not.toHaveBeenCalled();
  });

  it("os outros erros de fila continuam oferecendo tentar de novo", () => {
    const estado = { ...estadoInicial(FILA), passo: "frente" as const };
    expect(rotulos({ estado, comErro: true })).toEqual([t("calibracao.desistir"), t("calibracao.tentarDeNovo")]);
  });
});

describe("leitor de tela", () => {
  it("o corpo tem rotulo e o passo e anunciado por uma regiao viva", () => {
    const html = telaInicial(FILA);
    expect(html).toContain(`aria-label="${t("calibracao.corpo")}"`);
    expect(html).toContain('role="group"');
    expect(html).toMatch(/aria-live="polite"[^>]*>[^<]*1[^<]*5/);
  });
});

describe("papel gasto", () => {
  it("a fila real diz antes de imprimir quantas folhas pode gastar", () => {
    const html = telaInicial(FILA);
    expect(html).toContain(
      t("calibracao.papelGasto", { folhasPorRodada: "2", rodadas: "5", folhas: "10" }),
    );
  });

  it("a simulada e a automatica nao falam em gastar as rodadas", () => {
    const frase = t("calibracao.papelGasto", { folhasPorRodada: "2", rodadas: "5", folhas: "10" });
    const simulada = { ...FILA, id: "simulada:traseira", tipo: "simulada" as const };
    const automatica = { ...FILA, modo: "automatico" as const };
    for (const impressora of [simulada, automatica]) {
      const html = telaInicial(impressora);
      expect(html).not.toContain(frase);
    }
  });
});

describe("limite de rodadas", () => {
  it("e o mesmo no nucleo e na interface", async () => {
    const modulofs = "node:fs";
    const fs = (await import(/* @vite-ignore */ modulofs)) as {
      readFileSync(caminho: URL, codificacao: "utf8"): string;
    };
    const nucleo = fs.readFileSync(new URL("../../plots-core/src/calibracao.rs", import.meta.url), "utf8");
    const interfaceTs = fs.readFileSync(new URL("./calibracao.ts", import.meta.url), "utf8");
    const noNucleo = nucleo.match(/RODADAS_MAXIMAS: u8 = (\d+);/)?.[1];
    const naInterface = interfaceTs.match(/RODADAS_MAXIMAS = (\d+);/)?.[1];
    expect(noNucleo).toBe("5");
    expect(naInterface).toBe(noNucleo);
  });
});

const propsDoRodape = (props: Partial<Parameters<typeof Rodape>[0]> & { estado: Estado }) => {
  const nada = () => undefined;
  return {
    ehSimulada: false,
    comErro: false,
    naoServe: false,
    confirmando: false,
    ocupado: false,
    respondida: true,
    aoDesistir: nada,
    aoDesistirDeVez: nada,
    aoFecharConfirmacao: nada,
    aoFechar: nada,
    aoVoltar: nada,
    aoContinuar: nada,
    aoSeguir: nada,
    aoImprimirFrente: nada,
    aoImprimirVerso: nada,
    aoImprimirDeNovo: nada,
    aoRefazer: nada,
    aoGravar: nada,
    aoCorrigir: nada,
    ...props,
  };
};

describe("chamada em andamento trava a saida", () => {
  it("gravando: Desistir e Esc nao fecham a tela, nao abrem a confirmacao e nao encerram", () => {
    const tela = montar(CERTO, { trava: ocupada("gravando") });
    tela.saidas.desistirDeVez();
    tela.saidas.pedirDesistir();
    expect(tela.aoFechar).not.toHaveBeenCalled();
    expect(tela.estaConfirmando()).toBe(false);
    expect(backend.encerrarCalibracao).not.toHaveBeenCalled();
  });

  it("gravando: um segundo Gravar nao manda outra gravacao", () => {
    const tela = montar(CERTO, { trava: ocupada("gravando") });
    tela.saidas.gravar();
    expect(backend.gravarCalibracao).not.toHaveBeenCalled();
  });

  it("enviando o teste: Desistir nao cancela a passada que ainda vai entrar na fila", () => {
    const tela = montar({ ...CERTO, passo: "confirmar" }, { trava: ocupada("enviando") });
    tela.saidas.desistirDeVez();
    tela.saidas.pedirDesistir();
    expect(tela.aoFechar).not.toHaveBeenCalled();
    expect(backend.encerrarCalibracao).not.toHaveBeenCalled();
  });

  it("enviando o teste: todos os botoes do rodape ficam desabilitados", () => {
    const html = renderToStaticMarkup(<Rodape {...propsDoRodape({ estado: CERTO, ocupado: true })} />);
    const todos = html.match(/<button[^>]*>/g) ?? [];
    expect(todos.length).toBeGreaterThan(1);
    for (const botao of todos) expect(botao).toContain("disabled");
  });

  it("gravada: Desistir devolve que a gravacao foi feita, para a lista ser lida de novo", () => {
    const tela = montar({ ...CERTO, passo: "gravada" });
    tela.saidas.desistirDeVez();
    expect(tela.aoFechar).toHaveBeenCalledWith(true);
  });
});

describe("tela certa", () => {
  it("o rodape mostra Corrigir uma resposta entre Desistir e o botao principal", () => {
    expect(rotulos({ estado: CERTO })).toEqual([
      t("calibracao.desistir"),
      t("calibracao.certo.corrigir"),
      t("calibracao.gravar"),
    ]);
  });

  it("a tela certa diz a frase de gravar, o grupo com o gesto e o resumo das respostas", () => {
    const recolocar = { lado: "baixo", virada: "lateral", textoDoPerfil: null } as const;
    const estado = { ...CERTO, respostas: { lado: "em_branco", leitura: "sem_virar", par: "sim", setas: "nao" } as const };
    const html = renderToStaticMarkup(<CorpoDoPasso estado={estado} nome="Sala" ehSimulada={false} recolocar={recolocar} aoEscolherModelo={() => undefined} aoEscolherPapel={() => undefined} aoResponder={() => undefined} aoConfirmarResposta={() => undefined} />);
    const posicoes = [
      t("calibracao.certo.gravar"),
      t("calibracao.certo.depoisDeGravar"),
      "Vire o maço pela lateral, como a página de um livro.",
      t("calibracao.certo.lerSemVirar"),
      t("calibracao.certo.respostas"),
      t("calibracao.resposta.parCerto"),
    ].map((frase) => html.indexOf(frase));
    expect(posicoes.every((posicao) => posicao >= 0)).toBe(true);
    expect([...posicoes].sort((a, b) => a - b)).toEqual(posicoes);
  });

  it("na simulada, o grupo fala de este perfil, e nao de gravar", () => {
    const html = renderToStaticMarkup(<CorpoDoPasso estado={CERTO} nome="Sala" ehSimulada recolocar={null} aoEscolherModelo={() => undefined} aoEscolherPapel={() => undefined} aoResponder={() => undefined} aoConfirmarResposta={() => undefined} />);
    expect(html).toContain(t("calibracao.certo.comEstePerfil"));
    expect(html).not.toContain(t("calibracao.certo.depoisDeGravar"));
  });
});

describe("confirmar nao repete o total de folhas", () => {
  it("o passo de confirmar nao traz a frase de folhas gastas por rodada", () => {
    const estado = { ...estadoInicial(FILA), passo: "confirmar" as const };
    const html = renderToStaticMarkup(<CorpoDoPasso estado={estado} nome="Sala" ehSimulada={false} recolocar={null} aoEscolherModelo={() => undefined} aoEscolherPapel={() => undefined} aoResponder={() => undefined} aoConfirmarResposta={() => undefined} />);
    expect(html).not.toContain("Cada rodada gasta");
  });
});

describe("perguntas com instrucao e pergunta", () => {
  it("a instrucao vem em texto normal e a pergunta em negrito, em elementos separados", () => {
    const html = renderToStaticMarkup(<Respostas pergunta="leitura" valor={undefined} aoResponder={() => undefined} aoConfirmar={() => undefined} />);
    expect(html).toContain(t("calibracao.instrucao.leitura"));
    expect(html).toContain(t("calibracao.pergunta.leitura"));
    expect(html.indexOf(t("calibracao.instrucao.leitura"))).toBeLessThan(html.indexOf(t("calibracao.pergunta.leitura")));
  });
});

describe("andamento no lugar do alerta", () => {
  it("enviando mostra a linha de andamento com status, sem alerta", () => {
    const html = renderToStaticMarkup(<LinhaDeEnvio ocupacao="enviando" />);
    expect(html).toContain('role="status"');
    expect(html).toContain(t("calibracao.ocupacao.enviando"));
    expect(html).not.toContain("role=\"alert\"");
  });

  it("sem chamada, a linha nao aparece", () => {
    expect(renderToStaticMarkup(<LinhaDeEnvio ocupacao={null} />)).toBe("");
  });
});
