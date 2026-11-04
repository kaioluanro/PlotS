import { describe, expect, it } from "vitest";
import { t, traduzir } from "./i18n";
import {
  acaoLimpar,
  acaoRecolocar,
  acaoVirar,
  camadasVisiveis,
  comoErroTela,
  ESCOLHA_VAZIA,
  escolherEntrada,
  estiloDaCamada,
  facesDaEntrada,
  folhaEscolhida,
  formatarTamanho,
  gestoCompleto,
  GuardaDeDesenhos,
  nomeDaFila,
  pontoDoEstado,
  proporcaoDaFolha,
  resumoDaBandeja,
  textoDasOperacoes,
  temBandeja,
  textoComNumero,
  textoDasPaginas,
  textoDoAviso,
  textoDoErro,
  transformacaoDaCamada,
  type BandejaTela,
  type CamadaTela,
  type EstadoFila,
  type FolhaTela,
} from "./tela";

const camada = (mudancas: Partial<CamadaTela> = {}): CamadaTela => ({
  pagina: 1,
  numero: 1,
  branca: false,
  graus: 0,
  atras: false,
  larguraMm: 210,
  alturaMm: 297,
  ...mudancas,
});

const folha = (numero: number, mudancas: Partial<FolhaTela> = {}): FolhaTela => ({
  numero,
  situacao: "certa",
  motivos: [],
  frente: [camada()],
  verso: [camada({ atras: true })],
  ...mudancas,
});

const bandeja = (mudancas: Partial<BandejaTela> = {}): BandejaTela => ({
  etapa: "vazia",
  recebendo: false,
  esperando: 0,
  folhas: [],
  corretas: 0,
  semMarca: 0,
  conferidas: 0,
  gerais: [],
  leitura: null,
  semInstrucao: false,
  operacoes: [],
  paginasSemFolha: 0,
  folhasSemVerso: 0,
  aviso: null,
  ...mudancas,
});

describe("resumo do rodapé", () => {
  it("sem bandeja ou sem folhas, diz que não há nada", () => {
    expect(resumoDaBandeja(null)).toBe(t("rodape.vazia"));
    expect(resumoDaBandeja(bandeja())).toBe(t("rodape.vazia"));
  });

  it("conta as folhas corretas", () => {
    const pronta = bandeja({
      etapa: "conferida",
      folhas: [folha(1), folha(2), folha(3)],
      corretas: 3,
      conferidas: 3,
      leitura: "como_saiu",
    });
    expect(resumoDaBandeja(pronta)).toBe(t("rodape.resumo", { certas: "3", total: "3" }));
  });

  it("não chama de errada a folha sem marca: conta à parte", () => {
    const pronta = bandeja({
      etapa: "conferida",
      folhas: [folha(1), folha(2)],
      corretas: 1,
      semMarca: 1,
      conferidas: 2,
      leitura: "como_saiu",
    });
    expect(resumoDaBandeja(pronta)).toBe(
      t("rodape.resumo.semMarca", { certas: "1", total: "2", semMarca: "1" }),
    );
  });

  it("antes da segunda passada não há veredito", () => {
    const meia = bandeja({ etapa: "aguardando_gesto", folhas: [folha(1), folha(2)] });
    expect(resumoDaBandeja(meia)).toBe(t("rodape.andamento", { n: "2" }));
  });

  it("folha que falta entra no total", () => {
    const pronta = bandeja({
      etapa: "conferida",
      folhas: [folha(1)],
      corretas: 1,
      conferidas: 2,
      leitura: "como_saiu",
    });
    expect(resumoDaBandeja(pronta)).toBe(t("rodape.resumo", { certas: "1", total: "2" }));
  });
});

describe("botões do rodapé", () => {
  it("Recolocar só liga com a primeira passada na saída, e sempre diz por quê", () => {
    expect(acaoRecolocar(bandeja({ etapa: "aguardando_gesto" })).ativa).toBe(true);
    expect(acaoRecolocar(null)).toEqual({ ativa: false, motivo: "rodape.motivo.semPrimeira" });
    expect(acaoRecolocar(bandeja({ etapa: "na_entrada" })).motivo).toBe("rodape.motivo.jaNaEntrada");
    expect(acaoRecolocar(bandeja({ etapa: "conferida" })).motivo).toBe("rodape.motivo.loteCompleto");
  });

  it("Virar folha pede folha escolhida e que ainda exista", () => {
    const comFolhas = bandeja({ folhas: [folha(1)] });
    expect(acaoVirar(null, 1).ativa).toBe(false);
    expect(acaoVirar(comFolhas, null).motivo).toBe("rodape.motivo.semSelecao");
    expect(acaoVirar(comFolhas, 9).ativa).toBe(false);
    expect(acaoVirar(comFolhas, 1).ativa).toBe(true);
  });

  it("Limpar liga com folhas, com trabalho à espera ou com aviso", () => {
    expect(acaoLimpar(null).ativa).toBe(false);
    expect(acaoLimpar(bandeja()).ativa).toBe(false);
    expect(acaoLimpar(bandeja({ folhas: [folha(1)] })).ativa).toBe(true);
    expect(acaoLimpar(bandeja({ esperando: 1 })).ativa).toBe(true);
    expect(acaoLimpar(bandeja({ aviso: { codigo: "pdf_ilegivel", detalhe: null } })).ativa).toBe(true);
  });
});

describe("folha escolhida", () => {
  it("fica com a atual enquanto ela existir e cai na primeira quando some", () => {
    const folhas = [folha(1), folha(2)];
    expect(folhaEscolhida(folhas, 2)).toBe(2);
    expect(folhaEscolhida(folhas, 7)).toBe(1);
    expect(folhaEscolhida(folhas, null)).toBe(1);
    expect(folhaEscolhida([], 2)).toBeNull();
  });

  it("virada, mostra o outro lado", () => {
    const item = folha(1);
    expect(camadasVisiveis(item, false)).toBe(item.frente);
    expect(camadasVisiveis(item, true)).toBe(item.verso);
  });
});

describe("desenho da folha", () => {
  it("atrás do papel, o espelho vem antes do giro", () => {
    expect(transformacaoDaCamada({ graus: 0, atras: false })).toBe("rotate(0deg)");
    expect(transformacaoDaCamada({ graus: 180, atras: true })).toBe("rotate(180deg) scaleX(-1)");
  });

  it("deitada, a página troca largura e altura e fica no centro da folha", () => {
    const proporcao = 2;
    const estilo = estiloDaCamada({ graus: 90, atras: false }, proporcao);
    expect(estilo).toEqual({
      transform: "rotate(90deg)",
      width: "200%",
      height: "50%",
      left: "-50%",
      top: "25%",
    });
    expect(estiloDaCamada({ graus: 180, atras: false }, proporcao)).toEqual({
      transform: "rotate(180deg)",
    });
  });

  it("a proporção vem da página da frente e ignora medida hostil", () => {
    expect(proporcaoDaFolha([camada({ atras: true, alturaMm: 100 }), camada()])).toBeCloseTo(297 / 210);
    expect(proporcaoDaFolha([])).toBeCloseTo(297 / 210);
    expect(proporcaoDaFolha([camada({ larguraMm: 0 })])).toBeCloseTo(297 / 210);
    expect(proporcaoDaFolha([camada({ alturaMm: Number.NaN })])).toBeCloseTo(297 / 210);
    expect(proporcaoDaFolha([camada({ alturaMm: 99999 })])).toBe(2);
    expect(proporcaoDaFolha([camada({ alturaMm: 1 })])).toBe(0.5);
  });

  it("o texto diz o número, ou que não há marca, sem inventar", () => {
    const frente = camada({ numero: 1 });
    const atras = camada({ numero: null, atras: true });
    expect(textoDasPaginas([atras, frente])).toBe(
      t("folha.paginas", { frente: t("folha.paginaLida", { n: "1" }), verso: t("folha.paginaSemMarca") }),
    );
    expect(textoDasPaginas([frente])).toBe(
      t("folha.paginas", { frente: t("folha.paginaLida", { n: "1" }), verso: t("folha.paginaEmBranco") }),
    );
  });

  it("a página em branco diz que está em branco, e não que não tem marca", () => {
    const frente = camada({ numero: 5 });
    const atras = camada({ numero: null, branca: true, atras: true });
    expect(textoDasPaginas([atras, frente])).toBe(
      t("folha.paginas", { frente: t("folha.paginaLida", { n: "5" }), verso: t("folha.paginaEmBranco") }),
    );
    expect(t("folha.paginaEmBranco")).not.toBe(t("folha.paginaSemMarca"));
  });
});

describe("estado da impressora", () => {
  it("cada grupo de estado tem forma própria", () => {
    const estados: EstadoFila[] = ["ociosa", "recebendo", "aguardando_verso", "com_aviso"];
    const formas = new Set(estados.map((estado) => pontoDoEstado(estado).forma));
    expect(formas.size).toBe(estados.length);
  });
});

describe("erros e avisos", () => {
  it("falta de permissão diz o que fazer, sem detalhe técnico", () => {
    const { texto, detalhe } = textoDoErro({ codigo: "sem_permissao", detalhe: null });
    expect(texto).toBe(t("erro.sem_permissao"));
    expect(detalhe).toBeNull();
  });

  it("o texto do Windows vem à parte", () => {
    const { texto, detalhe } = textoDoErro({ codigo: "windows_recusou", detalhe: "spooler parado" });
    expect(texto).toBe(t("erro.windows_recusou"));
    expect(detalhe).toBe("spooler parado");
  });

  it("o detalhe não se repete quando a própria mensagem já o usa", () => {
    const { texto, detalhe } = textoDoErro({ codigo: "nome_caractere", detalhe: "/" });
    expect(texto).toContain("/");
    expect(detalhe).toBeNull();
  });

  it("código desconhecido e erro de forma estranha não quebram a tela", () => {
    expect(textoDoErro({ codigo: "nao-existe", detalhe: null }).texto).toBe(t("erro.interno"));
    expect(comoErroTela(undefined)).toEqual({ codigo: "interno", detalhe: null });
    expect(comoErroTela("texto")).toEqual({ codigo: "interno", detalhe: "texto" });
    expect(comoErroTela({ codigo: 3 })).toEqual({ codigo: "interno", detalhe: null });
    expect(textoDoAviso({ codigo: "nao-existe", detalhe: "x" })).toEqual({
      texto: t("aviso.padrao"),
      detalhe: "x",
    });
  });

  it("PDF ilegível tem texto próprio", () => {
    expect(textoDoAviso({ codigo: "pdf_ilegivel", detalhe: null }).texto).toBe(t("aviso.pdf_ilegivel"));
  });
});

describe("fila sem bandeja", () => {
  it("só fila pronta tem bandeja para consultar", () => {
    const fila = (estado: EstadoFila) => ({ nome: "A", modo: "jato" as const, estado, historico: "h", porta: "p" });
    expect(temBandeja(null)).toBe(false);
    for (const estado of ["sem_modo", "fora_do_windows", "diferente"] as const) {
      expect(temBandeja(fila(estado))).toBe(false);
    }
    for (const estado of ["ociosa", "recebendo", "aguardando_verso", "com_aviso"] as const) {
      expect(temBandeja(fila(estado))).toBe(true);
    }
  });
});

describe("detalhe técnico e idioma", () => {
  it("texto do núcleo, escrito só em português, não aparece na interface em inglês", () => {
    const aviso = { codigo: "pdf_ilegivel", detalhe: "o PDF recebido é inválido" };
    expect(textoDoAviso(aviso, "en").detalhe).toBeNull();
    expect(textoDoAviso(aviso, "pt-BR").detalhe).toBe(aviso.detalhe);
    const erro = { codigo: "erro_de_fila", detalhe: "não deu para ler" };
    expect(textoDoErro(erro, "en").detalhe).toBeNull();
    expect(textoDoErro(erro, "pt-BR").detalhe).toBe(erro.detalhe);
  });

  it("texto do próprio Windows e nome do documento aparecem nos dois idiomas", () => {
    expect(textoDoErro({ codigo: "windows_recusou", detalhe: "x" }, "en").detalhe).toBe("x");
    expect(textoDoAviso({ codigo: "trabalho_falhou", detalhe: "doc" }, "en").detalhe).toBe("doc");
  });
});

describe("singular", () => {
  it("uma folha não vira 'folhas'", () => {
    const meia = bandeja({ etapa: "aguardando_gesto", folhas: [folha(1)] });
    expect(resumoDaBandeja(meia)).toBe(t("rodape.andamento.um"));
    const pronta = bandeja({
      etapa: "conferida",
      folhas: [folha(1)],
      corretas: 1,
      conferidas: 1,
      leitura: "como_saiu",
    });
    expect(resumoDaBandeja(pronta)).toBe(t("rodape.resumo.um", { certas: "1" }));
  });

  it("escolhe a forma pelo número", () => {
    expect(textoComNumero(1, "centro.folhasSemVerso.um", "centro.folhasSemVerso")).toBe(
      t("centro.folhasSemVerso.um"),
    );
    expect(textoComNumero(2, "centro.folhasSemVerso.um", "centro.folhasSemVerso")).toBe(
      t("centro.folhasSemVerso", { n: "2" }),
    );
  });
});

describe("gesto escolhido à mão", () => {
  it("só vira pedido com as quatro escolhas feitas", () => {
    expect(gestoCompleto(ESCOLHA_VAZIA)).toBeNull();
    const parcial = { ...ESCOLHA_VAZIA, entrada: "traseira" as const, faceImpressa: "para_tras" as const };
    expect(gestoCompleto(parcial)).toBeNull();
    expect(gestoCompleto({ ...parcial, bordaQueEntra: "base", inverterOrdem: false })).toEqual({
      entrada: "traseira",
      faceImpressa: "para_tras",
      bordaQueEntra: "base",
      inverterOrdem: false,
    });
  });

  it("a face depende da entrada, como no modelo físico", () => {
    expect(facesDaEntrada(null)).toEqual([]);
    expect(facesDaEntrada("traseira")).toEqual(["para_frente", "para_tras"]);
    expect(facesDaEntrada("gaveta")).toEqual(["para_cima", "para_baixo"]);
    expect(facesDaEntrada("manual")).toEqual(["para_cima", "para_baixo"]);
  });

  it("trocar a entrada apaga a face que deixou de existir e mantém a que continua", () => {
    const traseira = escolherEntrada(ESCOLHA_VAZIA, "traseira");
    const comFace = { ...traseira, faceImpressa: "para_tras" as const };
    expect(escolherEntrada(comFace, "gaveta").faceImpressa).toBeNull();
    const gaveta = { ...escolherEntrada(ESCOLHA_VAZIA, "gaveta"), faceImpressa: "para_baixo" as const };
    expect(escolherEntrada(gaveta, "manual").faceImpressa).toBe("para_baixo");
  });

  it("combinação impossível não vira pedido", () => {
    expect(
      gestoCompleto({
        entrada: "traseira",
        faceImpressa: "para_cima",
        bordaQueEntra: "topo",
        inverterOrdem: true,
      }),
    ).toBeNull();
  });
});

describe("textos auxiliares", () => {
  it("o nome da fila leva o prefixo do simulador", () => {
    expect(nomeDaFila("Jato")).toBe("PlotSim — Jato");
  });

  it("diz o que o gesto fez, ou que não precisou fazer nada", () => {
    expect(textoDasOperacoes([])).toBe(t("centro.operacoes.nenhuma"));
    expect(textoDasOperacoes(["girar", "inverter_ordem"])).toBe(
      t("centro.operacoes", { lista: `${t("operacao.girar")}, ${t("operacao.inverter_ordem")}` }),
    );
  });

  it("tamanho não aceita número estranho", () => {
    expect(formatarTamanho(0)).toBe("0 KB");
    expect(formatarTamanho(Number.NaN)).toBe("0 KB");
    expect(formatarTamanho(-5)).toBe("0 KB");
    expect(formatarTamanho(1500)).toBe("2 KB");
    expect(formatarTamanho(3 * 1024 * 1024)).toMatch(/^3[.,]0 MB$/);
  });
});

describe("guarda de desenhos", () => {
  it("não passa do limite e joga fora o usado há mais tempo", () => {
    const guarda = new GuardaDeDesenhos(2);
    guarda.guardar("a", "1");
    guarda.guardar("b", "2");
    expect(guarda.ler("a")).toBe("1");
    guarda.guardar("c", "3");
    expect(guarda.tamanho).toBe(2);
    expect(guarda.ler("b")).toBeUndefined();
    expect(guarda.ler("a")).toBe("1");
    guarda.esquecerTudo();
    expect(guarda.tamanho).toBe(0);
  });
});

describe("contagem e erros da janela", () => {
  it("uma folha só, sem marca, usa o singular no resumo", () => {
    const umaSemMarca = bandeja({
      folhas: [{ numero: 1, situacao: "sem_marca", motivos: [], frente: [], verso: [] }],
      conferidas: 1,
      corretas: 0,
      semMarca: 1,
    });
    expect(resumoDaBandeja(umaSemMarca)).toBe(
      t("rodape.resumo.um.semMarca", { certas: "0", semMarca: "1" }),
    );
  });

  it("fila ocupada e porta presa têm os textos definitivos em pt-BR e en", () => {
    expect(traduzir("pt-BR", "erro.fila_ocupada")).toBe("A fila ainda está ocupada. Tente de novo.");
    expect(traduzir("en", "erro.fila_ocupada")).toBe("The queue is still busy. Try again.");
    expect(traduzir("pt-BR", "erro.porta_presa")).toBe(
      "Não deu para apagar o último trabalho, e os PDFs recebidos continuam aqui. Se ele ainda estiver imprimindo, espere terminar e tente de novo.",
    );
    expect(traduzir("en", "erro.porta_presa")).toBe(
      "The last job could not be deleted, and the received PDFs are still here. If it is still printing, wait for it to finish and try again.",
    );
    expect(textoDoErro({ codigo: "fila_ocupada", detalhe: null }).texto).toBe(t("erro.fila_ocupada"));
    expect(textoDoErro({ codigo: "porta_presa", detalhe: null }).texto).toBe(t("erro.porta_presa"));
  });

  it("o detalhe da porta presa aparece em pt-BR e some em en, como o do histórico", () => {
    const erro = { codigo: "porta_presa", detalhe: "o arquivo está em uso" };
    expect(textoDoErro(erro, "pt-BR").detalhe).toBe("o arquivo está em uso");
    expect(textoDoErro(erro, "en").detalhe).toBeNull();
  });
});
