import { describe, expect, it } from "vitest";
import type { Cartao, EstadoCartao } from "./cartoes";
import {
  acoesDoPainel,
  avisosNoTopo,
  cartoesComMudanca,
  entradasDoPainel,
  nomeDoAvisoNoTopo,
  ESPERA_DO_AVISO_MS,
  esperaAteSair,
  ESTADO_INICIAL_DOS_AVISOS,
  LIMITE_DE_AVISOS,
  MAXIMO_NO_TOPO,
  contadorDoSino,
  naoLidos,
  reduzirAvisos,
  tempoDecorrido,
  type EstadoDosAvisos,
} from "./avisos";

function cartao(estado: EstadoCartao, mudancas: Partial<Cartao> = {}): Cartao {
  return {
    aba: 1,
    impressora: "fila:A",
    nome: "A",
    simulada: false,
    lote: 1,
    total: 2,
    papel: { formato: "a4", larguraMm: 210, alturaMm: 297 },
    estado,
    recolocar: { lado: "baixo", virada: "nao_virar", textoDoPerfil: null },
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    ...mudancas,
  };
}

function publicar(estado: EstadoDosAvisos, cartaoNovo: Cartao, agora: number): EstadoDosAvisos {
  return reduzirAvisos(estado, { tipo: "publicar", cartao: cartaoNovo, agora });
}

const CONCLUIDO = cartao({ tipo: "concluido" });
const IMPRIMINDO = cartao({ tipo: "imprimindo", passada: "frente" });
const ERRO = cartao({ tipo: "erro", motivo: { codigo: "papel", texto: "" } });
const AGUARDANDO = cartao({ tipo: "aguardando" });

describe("reduzirAvisos", () => {
  it("põe o aviso mais novo primeiro e guarda o cartao como estava no instante", () => {
    let estado = publicar(ESTADO_INICIAL_DOS_AVISOS, IMPRIMINDO, 1000);
    estado = publicar(estado, CONCLUIDO, 2000);
    expect(estado.itens.map((item) => item.cartao.estado.tipo)).toEqual(["concluido", "imprimindo"]);
    expect(estado.itens[1]?.quando).toBe(1000);
  });

  it("guarda no maximo o limite, e tira os mais antigos", () => {
    let estado = ESTADO_INICIAL_DOS_AVISOS;
    for (let indice = 0; indice < LIMITE_DE_AVISOS + 1; indice += 1) {
      estado = publicar(estado, cartao({ tipo: "imprimindo", passada: "frente" }, { lote: indice }), indice);
    }
    expect(estado.itens).toHaveLength(LIMITE_DE_AVISOS);
    expect(estado.itens[0]?.cartao.lote).toBe(LIMITE_DE_AVISOS);
    expect(estado.itens[LIMITE_DE_AVISOS - 1]?.cartao.lote).toBe(1);
  });

  it("fechar tira o aviso do topo, mas ele continua no painel", () => {
    let estado = publicar(ESTADO_INICIAL_DOS_AVISOS, ERRO, 0);
    const id = estado.itens[0]?.id ?? -1;
    estado = reduzirAvisos(estado, { tipo: "fechar", id });
    expect(avisosNoTopo(estado.itens, 0)).toHaveLength(0);
    expect(estado.itens).toHaveLength(1);
  });

  it("marcar como lidos zera os nao lidos, e um aviso novo conta de novo", () => {
    let estado = publicar(ESTADO_INICIAL_DOS_AVISOS, IMPRIMINDO, 0);
    estado = publicar(estado, CONCLUIDO, 0);
    expect(naoLidos(estado.itens)).toBe(2);
    estado = reduzirAvisos(estado, { tipo: "ler_todos" });
    expect(naoLidos(estado.itens)).toBe(0);
    estado = publicar(estado, ERRO, 0);
    expect(naoLidos(estado.itens)).toBe(1);
  });
});

describe("avisosNoTopo", () => {
  it("um aviso passageiro some ao completar a espera", () => {
    const estado = publicar(ESTADO_INICIAL_DOS_AVISOS, IMPRIMINDO, 1000);
    expect(avisosNoTopo(estado.itens, 1000 + ESPERA_DO_AVISO_MS - 1)).toHaveLength(1);
    expect(avisosNoTopo(estado.itens, 1000 + ESPERA_DO_AVISO_MS)).toHaveLength(0);
  });

  it("erro e pedido de acao ficam ate serem fechados", () => {
    let estado = publicar(ESTADO_INICIAL_DOS_AVISOS, ERRO, 0);
    estado = publicar(estado, cartao({ tipo: "aguardando" }, { impressora: "fila:B" }), 0);
    const muitoDepois = 10 * 60 * 1000;
    expect(avisosNoTopo(estado.itens, muitoDepois)).toHaveLength(2);
    estado = reduzirAvisos(estado, { tipo: "fechar", id: estado.itens[0]?.id ?? -1 });
    expect(avisosNoTopo(estado.itens, muitoDepois)).toHaveLength(1);
  });

  it("mostra no maximo tres, do mais novo para o mais antigo", () => {
    let estado = ESTADO_INICIAL_DOS_AVISOS;
    for (const indice of [1, 2, 3, 4]) {
      estado = publicar(
        estado,
        cartao({ tipo: "imprimindo", passada: "frente" }, { lote: indice, impressora: `fila:${indice}` }),
        0,
      );
    }
    const topo = avisosNoTopo(estado.itens, 0);
    expect(topo).toHaveLength(MAXIMO_NO_TOPO);
    expect(topo.map((item) => item.cartao.lote)).toEqual([4, 3, 2]);
  });

  it("um aviso passageiro que ja saiu nao volta quando um novo chega", () => {
    let estado = publicar(ESTADO_INICIAL_DOS_AVISOS, IMPRIMINDO, 0);
    estado = publicar(estado, CONCLUIDO, 7000);
    expect(avisosNoTopo(estado.itens, 7000).map((item) => item.cartao.estado.tipo)).toEqual(["concluido"]);
  });
});

describe("esperaAteSair", () => {
  it("devolve o tempo que falta para o passageiro mais novo sair", () => {
    let estado = publicar(ESTADO_INICIAL_DOS_AVISOS, IMPRIMINDO, 0);
    estado = publicar(estado, CONCLUIDO, 2000);
    // O mais antigo sai primeiro: o prazo que vale e o do aviso de 0 ms.
    expect(esperaAteSair(estado.itens, 3000)).toBe(ESPERA_DO_AVISO_MS - 3000);
  });

  it("nao pede tempo quando so ha avisos fixos", () => {
    const estado = publicar(ESTADO_INICIAL_DOS_AVISOS, ERRO, 0);
    expect(esperaAteSair(estado.itens, 0)).toBeNull();
  });
});

describe("cartoesComMudanca", () => {
  it("um cartao novo gera aviso, e o mesmo estado nao gera de novo", () => {
    const primeira = cartoesComMudanca(new Map(), [IMPRIMINDO]);
    expect(primeira.novos).toHaveLength(1);
    const segunda = cartoesComMudanca(primeira.proximo, [IMPRIMINDO]);
    expect(segunda.novos).toHaveLength(0);
  });

  it("mudar o tipo ou a passada gera aviso", () => {
    const antes = cartoesComMudanca(new Map(), [IMPRIMINDO]).proximo;
    const verso = cartao({ tipo: "imprimindo", passada: "verso" });
    expect(cartoesComMudanca(antes, [verso]).novos).toHaveLength(1);
    expect(cartoesComMudanca(antes, [AGUARDANDO]).novos).toHaveLength(1);
  });

  it("mensagem do driver que muda nao gera aviso novo", () => {
    const aviso = (mensagem: string) =>
      cartao({
        tipo: "aviso",
        passada: "frente",
        motivo: { codigo: "sem_papel", texto: "" },
        mensagemDoDriver: mensagem,
        respondido: false,
      });
    const antes = cartoesComMudanca(new Map(), [aviso("uma")]).proximo;
    expect(cartoesComMudanca(antes, [aviso("outra")]).novos).toHaveLength(0);
  });

  it("cartao que saiu da janela nao gera aviso e some da leitura", () => {
    const antes = cartoesComMudanca(new Map(), [IMPRIMINDO]).proximo;
    const depois = cartoesComMudanca(antes, []);
    expect(depois.novos).toHaveLength(0);
    expect(depois.proximo.size).toBe(0);
  });
});

describe("acoesDoPainel", () => {
  it("aguardando pode cancelar, mas o verso sai so pelo rodape", () => {
    expect(acoesDoPainel(AGUARDANDO)).toMatchObject({ cancelar: true, reimprimir: false });
    expect(acoesDoPainel(AGUARDANDO)).not.toHaveProperty("verso");
    expect(acoesDoPainel(AGUARDANDO)).not.toHaveProperty("frente");
  });

  it("proximo lote nao oferece a frente no painel: ela sai pelo rodape", () => {
    expect(acoesDoPainel(cartao({ tipo: "proximo_lote" }))).not.toHaveProperty("frente");
  });

  it("erro com trabalho ainda na fila oferece reimprimir e cancelar", () => {
    expect(acoesDoPainel(ERRO)).toMatchObject({ reimprimir: true, cancelar: true });
  });

  it("erro sem trabalho na fila nao oferece cancelar", () => {
    const interrompido = cartao({ tipo: "erro", motivo: { codigo: "interrompido", texto: "" } });
    expect(acoesDoPainel(interrompido).cancelar).toBe(false);
  });

  it("concluido nao oferece nada", () => {
    expect(acoesDoPainel(CONCLUIDO)).toEqual({
      continuar: false,
      reimprimir: false,
      cancelar: false,
    });
  });
});

describe("painel: acoes so na entrada mais recente do trabalho", () => {
  /** Do mais novo para o mais antigo, como o painel recebe. */
  function sessao(...cartoes: Cartao[]) {
    return cartoes.reduce(
      (estado, novo, indice) => publicar(estado, novo, 1000 + indice),
      ESTADO_INICIAL_DOS_AVISOS,
    ).itens;
  }

  it("dezesseis entradas do mesmo trabalho, so a mais nova ganha Cancelar", () => {
    const antigas = Array.from({ length: 15 }, () => cartao({ tipo: "imprimindo", passada: "frente" }));
    const atual = cartao({ tipo: "imprimindo", passada: "verso" });
    const entradas = entradasDoPainel(sessao(...antigas, atual), [atual]);
    expect(entradas).toHaveLength(16);
    expect(entradas.filter((e) => e.acoes !== null)).toHaveLength(1);
    expect(entradas[0]?.acoes?.cancelar).toEqual({ desabilitado: false });
    expect(entradas.slice(1).every((e) => e.acoes === null)).toBe(true);
  });

  it("outro trabalho (outra impressora) tem a propria entrada mais recente", () => {
    const a = cartao({ tipo: "imprimindo", passada: "frente" });
    const b = cartao({ tipo: "imprimindo", passada: "frente" }, { impressora: "fila:B" });
    const entradas = entradasDoPainel(sessao(a, b), [a, b]);
    expect(entradas.map((e) => e.acoes !== null)).toEqual([true, true]);
  });

  it("Cancelar fica desabilitado em Cancelando", () => {
    const cancelando = cartao({ tipo: "cancelando" });
    const [entrada] = entradasDoPainel(sessao(cancelando), [cancelando]);
    expect(entrada?.acoes?.cancelar).toEqual({ desabilitado: true });
  });

  it("Reimprimir so aparece em erro, nao em aviso nem em andamento", () => {
    const aviso = cartao({
      tipo: "aviso",
      passada: "frente",
      motivo: { codigo: "sem_papel", texto: "" },
      respondido: false,
    });
    const emAviso = entradasDoPainel(sessao(aviso), [aviso])[0]?.acoes;
    expect(emAviso?.reimprimir).toBe(false);
    expect(emAviso?.continuar).toBe(true);
    expect(entradasDoPainel(sessao(ERRO), [ERRO])[0]?.acoes?.reimprimir).toBe(true);
    expect(entradasDoPainel(sessao(IMPRIMINDO), [IMPRIMINDO])[0]?.acoes?.reimprimir).toBe(false);
  });

  it("Cancelar nao aparece em erro sem trabalho na fila, concluido nem cancelado", () => {
    for (const estado of [
      { tipo: "concluido" },
      { tipo: "cancelado" },
      { tipo: "erro", motivo: { codigo: "interrompido", texto: "" } },
    ] satisfies EstadoCartao[]) {
      const c = cartao(estado);
      expect(entradasDoPainel(sessao(c), [c])[0]?.acoes?.cancelar, estado.tipo).toBeNull();
    }
  });

  it("a dica dos versos so aparece na entrada de frentes prontas", () => {
    const antes = cartao({ tipo: "imprimindo", passada: "frente" });
    const prontas = cartao({ tipo: "frentes_prontas" });
    const entradas = entradasDoPainel(sessao(antes, prontas), [prontas]);
    expect(entradas[0]?.dicaDe?.estado.tipo).toBe("frentes_prontas");
    expect(entradas[1]?.dicaDe).toBeNull();
    // Entrada de frentes prontas que ja nao e a atual: o trabalho seguiu, a dica nao vale.
    const depois = cartao({ tipo: "imprimindo", passada: "verso" });
    const seguiu = entradasDoPainel(sessao(prontas, depois), [depois]);
    expect(seguiu.map((e) => e.dicaDe)).toEqual([null, null]);
  });

  it("sem cartao vivo, a entrada mais recente diz que nao ha trabalho e as outras ficam quietas", () => {
    const entradas = entradasDoPainel(sessao(IMPRIMINDO, CONCLUIDO), []);
    expect(entradas.map((e) => e.semTrabalho)).toEqual([true, false]);
  });
});

describe("topo: nome do documento e validade", () => {
  it("o aviso diz o documento e a fila", () => {
    const a = reduzirAvisos(ESTADO_INICIAL_DOS_AVISOS, {
      tipo: "publicar",
      cartao: IMPRIMINDO,
      agora: 1,
      documento: "contrato.pdf",
    }).itens[0];
    expect(a && nomeDoAvisoNoTopo(a)).toBe("contrato.pdf · A");
    const sem = publicar(ESTADO_INICIAL_DOS_AVISOS, IMPRIMINDO, 1).itens[0];
    expect(sem && nomeDoAvisoNoTopo(sem)).toBe("A");
  });

  it("aviso fixo que o trabalho ja ultrapassou sai do topo e continua na lista", () => {
    const troca = cartao({ tipo: "proximo_lote" }, { trocaDePapel: { formato: "a5", larguraMm: 148, alturaMm: 210 } });
    let estado = publicar(ESTADO_INICIAL_DOS_AVISOS, troca, 1000);
    expect(avisosNoTopo(estado.itens, 1100)).toHaveLength(1);
    estado = publicar(estado, cartao({ tipo: "imprimindo", passada: "frente" }, { lote: 2 }), 2000);
    const noTopo = avisosNoTopo(estado.itens, 2100);
    expect(noTopo.map((item) => item.cartao.estado.tipo)).toEqual(["imprimindo"]);
    expect(estado.itens).toHaveLength(2);
  });

  it("aviso fixo de outro trabalho continua no topo", () => {
    let estado = publicar(ESTADO_INICIAL_DOS_AVISOS, ERRO, 1000);
    estado = publicar(estado, cartao({ tipo: "imprimindo", passada: "frente" }, { impressora: "fila:B" }), 2000);
    expect(avisosNoTopo(estado.itens, 2100)).toHaveLength(2);
  });
});

describe("topo: um aviso por trabalho, fixos primeiro", () => {
  it("so o aviso mais novo de cada trabalho fica no topo", () => {
    let estado = publicar(ESTADO_INICIAL_DOS_AVISOS, cartao({ tipo: "preparando" }), 1000);
    estado = publicar(estado, IMPRIMINDO, 2000);
    estado = publicar(estado, CONCLUIDO, 3000);
    const topo = avisosNoTopo(estado.itens, 3100);
    expect(topo.map((item) => item.cartao.estado.tipo)).toEqual(["concluido"]);
    expect(estado.itens).toHaveLength(3);
  });

  it("o mais novo fechado nao deixa o anterior voltar", () => {
    let estado = publicar(ESTADO_INICIAL_DOS_AVISOS, IMPRIMINDO, 1000);
    estado = publicar(estado, CONCLUIDO, 2000);
    estado = reduzirAvisos(estado, { tipo: "fechar", id: estado.itens[0]?.id ?? -1 });
    expect(avisosNoTopo(estado.itens, 2100)).toHaveLength(0);
  });

  it("os fixos vem primeiro, mesmo que um passageiro seja mais novo", () => {
    let estado = publicar(ESTADO_INICIAL_DOS_AVISOS, ERRO, 1000);
    estado = publicar(estado, cartao({ tipo: "imprimindo", passada: "frente" }, { impressora: "fila:B" }), 2000);
    estado = publicar(estado, AGUARDANDO_C, 1500);
    const topo = avisosNoTopo(estado.itens, 2100);
    expect(topo.map((item) => item.cartao.impressora)).toEqual(["fila:C", "fila:A", "fila:B"]);
  });
});

const AGUARDANDO_C = cartao({ tipo: "aguardando" }, { impressora: "fila:C" });

describe("topo: prazo pausado", () => {
  it("o tempo parado nao conta para o prazo do aviso passageiro", () => {
    const pausas = [{ de: 2000, ate: 5000 }];
    expect(tempoDecorrido(1000, 8000, pausas)).toBe(4000);
    expect(tempoDecorrido(1000, 8000)).toBe(7000);
    // Pausa em curso: vale ate agora.
    expect(tempoDecorrido(1000, 4000, [{ de: 2000, ate: null }])).toBe(1000);
    // Pausa anterior ao aviso nao o atinge.
    expect(tempoDecorrido(6000, 8000, pausas)).toBe(2000);
  });

  it("com o ponteiro sobre a faixa, o aviso passageiro fica; ao sair, cumpre o que faltava", () => {
    const estado = publicar(ESTADO_INICIAL_DOS_AVISOS, IMPRIMINDO, 0);
    const pausas = [{ de: 1000, ate: null as number | null }];
    expect(avisosNoTopo(estado.itens, 60000, pausas)).toHaveLength(1);
    expect(esperaAteSair(estado.itens, 60000, pausas)).toBe(ESPERA_DO_AVISO_MS - 1000);
    const depois = [{ de: 1000, ate: 60000 as number | null }];
    expect(avisosNoTopo(estado.itens, 60000 + ESPERA_DO_AVISO_MS - 1001, depois)).toHaveLength(1);
    expect(avisosNoTopo(estado.itens, 60000 + ESPERA_DO_AVISO_MS - 1000, depois)).toHaveLength(0);
  });
});

describe("contador do sino", () => {
  it("passa de 99 vira 99+", () => {
    expect(contadorDoSino(1)).toBe("1");
    expect(contadorDoSino(99)).toBe("99");
    expect(contadorDoSino(100)).toBe("99+");
  });
});
