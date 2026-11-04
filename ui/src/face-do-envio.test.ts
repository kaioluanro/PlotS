import { describe, expect, it } from "vitest";
import {
  acoesDoCartao,
  cartaoEmAndamento,
  estadoDoEnvio,
  frasesDoCartao,
  impressorasOcupadas,
  linhaDoLote,
  rotuloDoVerso,
  sequenciaDoCartao,
  tomDoCartao,
  type Cartao,
  type EstadoCartao,
  type ResumoDoEnvio,
} from "./cartoes";
import { desenhoDoCartao } from "./cartoes-gesto";
import { conteudoDaConfirmacao } from "./confirmacao";
import {
  conteudoDoVerso,
  envioComFace,
  faceDisponivel,
  faceDoEnvio,
  mudouDesdeAFrente,
  notaDoMaco,
  notasDaFrente,
  quemJaSaiu,
} from "./face-do-envio";
import { traduzir } from "./i18n";
import type { Impressora } from "./impressoras";
import { estadoInicial, reduzirAbas, type Aba, type EstadoAbas } from "./tabs";

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
const DUAS = impressora("Duas", { modo: "automatico", duasFaces: true });
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

const RESUMO: ResumoDoEnvio = {
  paginas: 24,
  lotes: 1,
  folhas: 12,
  maiorMaco: 12,
  papeis: [{ formato: "a4", larguraMm: 210, alturaMm: 297 }],
  paresMistos: [],
  assinado: false,
  fluxos: [{ automatico: false, inicio: 1, fim: 24 }],
  faixas: [{ lote: 1, inicio: 1, fim: 24 }],
  gestos: [{ impressora: SALA.id, recolocar: GESTO }],
};

describe("a escolha entre frente e verso", () => {
  it("comeca em frente e so muda quando a pessoa muda", () => {
    let estado = comAba([SALA]);
    expect(abaDe(estado).face).toBe("frente");
    estado = reduzirAbas(estado, {
      tipo: "frente_enviada",
      id: 1,
      frente: { trecho: null, papel: null },
    });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao({ tipo: "imprimindo", passada: "frente" }) });
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao({ tipo: "frentes_prontas" }) });
    expect(abaDe(estado).face).toBe("frente");
    estado = reduzirAbas(estado, { tipo: "definir_face", id: 1, face: "verso" });
    expect(abaDe(estado).face).toBe("verso");
    estado = reduzirAbas(estado, {
      tipo: "cartao",
      cartao: cartao({ tipo: "concluido" }, { alcance: "so_versos" }),
    });
    expect(abaDe(estado).face).toBe("verso");
  });

  it("cada aba tem a sua, e aba desconhecida nao muda nada", () => {
    let estado = comAba([SALA]);
    estado = reduzirAbas(estado, { tipo: "abrir", documento: { caminho: "C:\\b.pdf", nome: "b.pdf" } });
    const mudado = reduzirAbas(estado, { tipo: "definir_face", id: 1, face: "verso" });
    expect(mudado.abas.map((aba) => aba.face)).toEqual(["verso", "frente"]);
    expect(reduzirAbas(estado, { tipo: "definir_face", id: 99, face: "verso" })).toBe(estado);
  });

  it("some so com automaticas, e o verso escolhido antes deixa de valer", () => {
    expect(faceDisponivel([])).toBe(false);
    expect(faceDisponivel([DUAS])).toBe(false);
    expect(faceDisponivel([DUAS, SALA])).toBe(true);
    const aba = { ...abaDe(comAba([DUAS])), face: "verso" as const };
    expect(faceDoEnvio(aba, [DUAS])).toBe("frente");
    expect(faceDoEnvio(aba, [DUAS, SALA])).toBe("verso");
    expect(faceDoEnvio(null, [SALA])).toBe("frente");
  });
});

describe("o botao do rodape", () => {
  const botao = (aba: Aba, lista: readonly Impressora[], papel: { temPapelManual: boolean } | null = null, idioma: "pt-BR" | "en" = "pt-BR") =>
    envioComFace(estadoDoEnvio(aba, lista, impressorasOcupadas([aba]), idioma), aba, lista, papel, idioma);

  it("diz o que fara e conta quem recebe, nos dois idiomas", () => {
    const frente = abaDe(comAba([SALA, COPA]));
    expect(botao(frente, [SALA, COPA]).rotulo).toBe("Imprimir frentes em 2");
    const verso = { ...frente, face: "verso" as const };
    expect(botao(verso, [SALA, COPA]).rotulo).toBe("Imprimir versos em 2");
    expect(botao(verso, [SALA, COPA], null, "en").rotulo).toBe("Print backs on 2");
    const simulada = { ...abaDe(comAba([SIMULADA])), face: "verso" as const };
    expect(botao(simulada, [SIMULADA]).rotulo).toBe("Simular versos em 1");
  });

  it("no verso, recusa com o motivo enquanto a fila nao confirmou a frente desta aba", () => {
    const base = abaDe(comAba([SALA, COPA]));
    const aba: Aba = {
      ...base,
      face: "verso",
      cartoes: [cartao({ tipo: "imprimindo", passada: "frente" })],
    };
    const envio = botao(aba, [SALA, COPA]);
    expect(envio.desabilitado).toBe(true);
    expect(envio.destinos).toEqual([]);
    expect(envio.motivo).toBe("A fila ainda não confirmou as frentes em Sala. Aguarde para enviar os versos.");
    expect(botao({ ...aba, face: "frente" }, [SALA, COPA]).motivo).not.toContain("confirmou");
  });

  it("com as frentes prontas, o verso sai, sem nada desta sessao tambem", () => {
    const base = abaDe(comAba([SALA]));
    const pronta: Aba = { ...base, face: "verso", cartoes: [cartao({ tipo: "frentes_prontas" })] };
    expect(botao(pronta, [SALA]).desabilitado).toBe(false);
    expect(botao({ ...base, face: "verso" }, [SALA]).desabilitado).toBe(false);
  });

  it("no verso, quem imprime as duas faces fica de fora; se o documento tem papel manual, entra", () => {
    const aba = { ...abaDe(comAba([SALA, DUAS])), face: "verso" as const };
    expect(botao(aba, [SALA, DUAS], { temPapelManual: false }).destinos).toEqual([SALA]);
    expect(botao(aba, [SALA, DUAS], { temPapelManual: true }).destinos).toEqual([SALA, DUAS]);
  });
});

describe("cartao das frentes prontas", () => {
  const prontas = cartao({ tipo: "frentes_prontas" });

  it("libera a impressora, mas fechar ainda pergunta", () => {
    expect(cartaoEmAndamento(prontas)).toBe(false);
    let estado = comAba([SALA]);
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: prontas });
    expect(impressorasOcupadas(estado.abas).size).toBe(0);
    expect(abaDe(estado).impressaoEmAndamento).toBe(true);
    expect(abaDe(estado).frentesProntas).toEqual([SALA.id]);
    estado = reduzirAbas(estado, { tipo: "dispensar_cartao", id: 1, impressora: SALA.id });
    expect(abaDe(estado).impressaoEmAndamento).toBe(false);
    expect(abaDe(estado).frentesProntas).toEqual([SALA.id]);
  });

  it("mostra o gesto e pede os versos, com a figura e sem numerar um trecho so", () => {
    expect(acoesDoCartao(prontas).pedirVerso).toBe(true);
    expect(acoesDoCartao(prontas).verso.visivel).toBe(false);
    expect(acoesDoCartao(prontas).dispensar).toBe(true);
    expect(rotuloDoVerso(prontas, "pt-BR")).toBe("Imprimir versos");
    expect(rotuloDoVerso(prontas, "en")).toBe("Print backs");
    expect(desenhoDoCartao(prontas)).not.toBeNull();
    expect(tomDoCartao(prontas)).toBe("espera");
    expect(linhaDoLote(prontas, "pt-BR")).toBeNull();
    expect(linhaDoLote({ ...prontas, lote: 2, total: 3 }, "pt-BR")).toBe("Trecho 2 de 3");
    expect(linhaDoLote({ ...prontas, lote: 2, total: 3 }, "en")).toBe("Section 2 of 3");
    expect(frasesDoCartao(prontas, "pt-BR")).toEqual([
      "O trabalho das frentes terminou ou saiu da fila. Isso não garante que o papel já saiu.",
      "Espere a última folha sair antes de recolocar o maço.",
      "Vire o maço pela borda de cima, como um bloco de notas.",
      "Depois, sem mudar a ordem das folhas, coloque na entrada com a face impressa para baixo.",
      "Pode ser depois. Se fechar o PlotS, abra o mesmo arquivo e escolha Verso. Nesse caso, o PlotS não confere se ele mudou.",
    ]);
  });

  it("acima de 30 folhas ensina a colocar o maco em partes, nos dois idiomas", () => {
    const grande = cartao({ tipo: "frentes_prontas" }, { folhasDoLote: 31 });
    expect(frasesDoCartao(grande, "pt-BR")).toContain(
      "Se o maço não couber na entrada, coloque a parte de cima e complete com o resto quando a entrada esvaziar, sem mudar a ordem.",
    );
    expect(frasesDoCartao(grande, "en")).toContain(
      "If the stack does not fit in the feed, load the top part and add the rest when the feed runs empty, without changing the order.",
    );
    const justo = cartao({ tipo: "frentes_prontas" }, { folhasDoLote: 30 });
    expect(frasesDoCartao(justo, "pt-BR").join(" ")).not.toContain("não couber");
  });

  it("nao repete a sequencia do ultimo trecho", () => {
    const comSequencia = {
      ...prontas,
      sequencia: { frente: [{ numero: 2, girada: false }], verso: [{ numero: 1, girada: true }] },
    };
    expect(sequenciaDoCartao(comSequencia, "pt-BR")).toEqual([]);
  });
});

describe("cartao na troca de maco dos versos", () => {
  it("diz o papel e o maco, antes do gesto, e o verso segue no cartao", () => {
    const espera = cartao(
      { tipo: "aguardando" },
      {
        alcance: "so_versos",
        lote: 2,
        total: 2,
        trocaDePapel: { formato: "a5", larguraMm: 148, alturaMm: 210 },
        faixaDoLote: { lote: 2, inicio: 5, fim: 8 },
      },
    );
    const frases = frasesDoCartao(espera, "pt-BR");
    expect(frases).toContain("Ajuste a entrada para A5.");
    expect(frases).toContain("Pegue o maço das págs. 5–8.");
    expect(frases.indexOf("Pegue o maço das págs. 5–8.")).toBeLessThan(
      frases.indexOf("Vire o maço pela borda de cima, como um bloco de notas."),
    );
    expect(frasesDoCartao(espera, "en")).toContain("Take the stack of pp. 5–8.");
    expect(acoesDoCartao(espera).verso).toEqual({ visivel: true, liberado: true });
  });

  it("depois de todos os versos, diz como empilhar os macos", () => {
    const fim = cartao({ tipo: "concluido" }, { alcance: "so_versos", lote: 2, total: 2 });
    expect(frasesDoCartao(fim, "pt-BR")).toContain(
      "Empilhe os maços na ordem em que saíram, com o primeiro por cima.",
    );
    expect(frasesDoCartao({ ...fim, total: 1 }, "pt-BR").join(" ")).not.toContain("Empilhe");
  });
});

describe("confirmacao da frente", () => {
  it("avisa quem ja entregou as frentes, sem mudar a escolha", () => {
    let estado = comAba([SALA, COPA]);
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao({ tipo: "frentes_prontas" }) });
    const aba = abaDe(estado);
    expect(aba.face).toBe("frente");
    expect(quemJaSaiu(aba, [SALA, COPA])).toEqual([SALA]);
    expect(quemJaSaiu(aba, [COPA])).toEqual([]);
    expect(notasDaFrente(aba, [SALA, COPA], RESUMO, "pt-BR").antes).toEqual([
      "O trabalho das frentes deste documento já terminou ou saiu da fila em Sala. Enviar de novo repete as frentes e, em impressora real, gasta papel. Para os versos, escolha Verso.",
    ]);
    expect(notasDaFrente(aba, [COPA], RESUMO, "en").antes).toEqual([]);
  });

  it("diz que a automatica imprime as duas faces, se ha papel que ela aceita", () => {
    const aba = abaDe(comAba([SALA, DUAS]));
    const misto: ResumoDoEnvio = {
      ...RESUMO,
      fluxos: [
        { automatico: true, inicio: 1, fim: 12 },
        { automatico: false, inicio: 13, fim: 24 },
      ],
    };
    expect(notasDaFrente(aba, [SALA, DUAS], misto, "pt-BR").depois).toEqual([
      "Duas está configurada para imprimir as duas faces de uma vez nos papéis que aceita.",
    ]);
    expect(notasDaFrente(aba, [SALA], misto, "pt-BR").depois).toEqual([]);
  });

  it("avisa do maco grande uma vez, so acima de 30 folhas e so para quem manuseia", () => {
    const grande = { ...RESUMO, maiorMaco: 31 };
    expect(notaDoMaco(grande, [SALA], "pt-BR")).toBe(
      "Se o maço não couber na entrada, coloque em partes, sem mudar a ordem.",
    );
    expect(notaDoMaco({ ...RESUMO, maiorMaco: 30 }, [SALA], "pt-BR")).toBeNull();
    const soAutomatico = { ...grande, fluxos: [{ automatico: true, inicio: 1, fim: 24 }] };
    expect(notaDoMaco(soAutomatico, [DUAS], "pt-BR")).toBeNull();
    expect(notaDoMaco(soAutomatico, [DUAS, SALA], "pt-BR")).not.toBeNull();
  });
});

describe("confirmacao do verso", () => {
  const conteudo = (mudancas: Partial<ResumoDoEnvio> = {}, mudou = false, destinos = [SALA]) =>
    conteudoDoVerso(destinos, [], { ...RESUMO, ...mudancas }, null, null, mudou, "pt-BR");

  it("pergunta, com a frase e a figura do gesto de cada impressora", () => {
    const dialogo = conteudo();
    expect(traduzir("pt-BR", dialogo.texto)).toBe(
      "As frentes já saíram? Coloque o maço na entrada, como manda o cartão.",
    );
    expect(traduzir("en", dialogo.texto)).toBe(
      "Are the fronts out? Load the stack in the feed, as the card says.",
    );
    expect(traduzir("pt-BR", dialogo.titulo)).toBe("Imprimir os versos?");
    expect(traduzir("pt-BR", dialogo.fechar)).toBe("Imprimir versos");
    expect(dialogo.confirmaAvancando).toBe(true);
    expect(dialogo.gestos?.[0]?.nome).toBe("Sala");
    expect(dialogo.gestos?.[0]?.frases).toEqual([
      "Vire o maço pela borda de cima, como um bloco de notas.",
      "Depois, sem mudar a ordem das folhas, coloque na entrada com a face impressa para baixo.",
    ]);
    expect(dialogo.gestos?.[0]?.desenho).not.toBeNull();
    expect(dialogo.linhas).toContain("24 pág · 12 folhas");
  });

  it("sem gesto da impressora, nao afirma lado nem desenha", () => {
    const dialogo = conteudo({}, false, [COPA]);
    expect(dialogo.gestos?.[0]?.desenho).toBeNull();
    expect(dialogo.gestos?.[0]?.frases[0]).toContain("calibração");
  });

  it("avisa se o trecho ou o papel mudou desde a frente", () => {
    const frente = { trecho: { inicio: 1, fim: 8 }, papel: null };
    expect(mudouDesdeAFrente(null, { inicio: 3, fim: 4 }, "a4")).toBe(false);
    expect(mudouDesdeAFrente(frente, { inicio: 1, fim: 8 }, null)).toBe(false);
    expect(mudouDesdeAFrente(frente, { inicio: 1, fim: 9 }, null)).toBe(true);
    expect(mudouDesdeAFrente(frente, null, null)).toBe(true);
    expect(mudouDesdeAFrente(frente, { inicio: 1, fim: 8 }, "a4")).toBe(true);
    expect(conteudo({}, true).notas?.[0]).toContain("mudou desde a frente");
    expect(conteudo({}, false).notas).toEqual([]);
  });

  it("com mais de um papel diz por qual maco comecar; com muitas folhas, como colocar", () => {
    const misto = conteudo({
      faixas: [
        { lote: 1, inicio: 1, fim: 8 },
        { lote: 2, inicio: 9, fim: 12 },
      ],
      maiorMaco: 31,
    });
    expect(misto.notas).toEqual([
      "Comece pelo maço das págs. 1–8. Os outros papéis vêm depois, um de cada vez.",
      "Se o maço não couber na entrada, coloque a parte de cima e complete com o resto quando a entrada esvaziar, sem mudar a ordem.",
    ]);
  });

  it("diz quem fica de fora dos versos", () => {
    const dialogo = conteudoDoVerso([SALA], [DUAS], RESUMO, null, null, false, "en");
    expect(dialogo.notas).toEqual(["Duas is set to print both sides at once and stays out of the backs."]);
  });
});

describe("fechar com versos por imprimir", () => {
  const nota = "Os versos ainda não foram pedidos. Depois, abra o mesmo arquivo, marque as impressoras e escolha Verso. O PlotS não confere se ele mudou.";

  it("a aba e a janela dizem isso, com as frentes prontas ou ainda saindo", () => {
    let estado = comAba([SALA]);
    estado = reduzirAbas(estado, { tipo: "cartao", cartao: cartao({ tipo: "frentes_prontas" }) });
    const aba = reduzirAbas(estado, { tipo: "pedir_fechamento", id: 1 });
    expect(conteudoDaConfirmacao(aba, "pt-BR")?.notas).toEqual([nota]);
    const janela = reduzirAbas(estado, { tipo: "pedir_fechamento_janela" });
    expect(conteudoDaConfirmacao(janela, "pt-BR")?.notas).toEqual([nota]);
    const saindo = reduzirAbas(comAba([SALA]), {
      tipo: "cartao",
      cartao: cartao({ tipo: "imprimindo", passada: "frente" }),
    });
    expect(conteudoDaConfirmacao(reduzirAbas(saindo, { tipo: "pedir_fechamento", id: 1 }), "pt-BR")?.notas).toEqual([nota]);
  });

  it("com os versos saindo, ou na automatica, nao fala disso", () => {
    const versos = reduzirAbas(comAba([SALA]), {
      tipo: "cartao",
      cartao: cartao({ tipo: "imprimindo", passada: "verso" }, { alcance: "so_versos" }),
    });
    expect(conteudoDaConfirmacao(reduzirAbas(versos, { tipo: "pedir_fechamento", id: 1 }), "pt-BR")?.notas).toBeUndefined();
    const automatico = reduzirAbas(comAba([DUAS]), {
      tipo: "cartao",
      cartao: cartao({ tipo: "imprimindo", passada: "duas_faces" }, { loteAutomatico: true }),
    });
    expect(conteudoDaConfirmacao(reduzirAbas(automatico, { tipo: "pedir_fechamento", id: 1 }), "pt-BR")?.notas).toBeUndefined();
  });
});
