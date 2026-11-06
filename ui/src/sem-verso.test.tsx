import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  acoesDoCartao,
  cartaoPedeAcao,
  cartaoSeguraAAba,
  estadoDoEnvio,
  linhasDoCartao,
  textoDoEstadoDoCartao,
  tomDoCartao,
  type Cartao,
  type EstadoCartao,
  type ResumoDoEnvio,
} from "./cartoes";
import { desenhoDoCartao } from "./cartoes-gesto";
import { BlocoFace } from "./components/BlocoFace";
import { CartaoImpressao } from "./components/CartaoImpressao";
import { conteudoDaConfirmacao } from "./confirmacao";
import { dicaDaFace, envioComFace, faceDoEnvio, notasDaFrente } from "./face-do-envio";
import { t } from "./i18n";
import type { Impressora } from "./impressoras";
import { estadoInicial, reduzirAbas, temVersoDaAba, type Aba, type EstadoAbas } from "./tabs";
import type { RascunhoIntervalo } from "./intervalo";

const SALA: Impressora = {
  id: "fila:Sala",
  nome: "Sala",
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "laser",
};

function abaCom(total: number, intervalo?: RascunhoIntervalo): EstadoAbas {
  let estado = reduzirAbas(estadoInicial, {
    tipo: "abrir",
    documento: { caminho: "C:\\a.pdf", nome: "a.pdf" },
  });
  estado = reduzirAbas(estado, { tipo: "paginas_contadas", id: 1, total });
  estado = reduzirAbas(estado, { tipo: "alternar_impressora", id: 1, impressora: SALA.id });
  if (intervalo !== undefined) {
    estado = reduzirAbas(estado, { tipo: "definir_intervalo", id: 1, intervalo });
  }
  return estado;
}

const abaDe = (estado: EstadoAbas): Aba => estado.abas[0] as Aba;

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
    recolocar: { lado: "baixo", virada: "borda_de_cima", textoDoPerfil: null },
    trocaDePapel: null,
    resultado: null,
    sequencia: null,
    viraParaLer: false,
    alcance: "so_frentes",
    folhasDoLote: 1,
    faixaDoLote: { lote: 1, inicio: 1, fim: 1 },
    ...mudancas,
  };
}

function comCartao(estado: EstadoAbas, item: Cartao): EstadoAbas {
  return reduzirAbas(estado, { tipo: "cartao", cartao: item });
}

describe("trabalho sem verso", () => {
  it("uma página força a frente e o seletor não oferece verso", () => {
    let estado = abaCom(1);
    estado = reduzirAbas(estado, { tipo: "definir_face", id: 1, face: "verso" });
    const aba = abaDe(estado);
    expect(aba.face).toBe("frente");
    expect(temVersoDaAba(aba)).toBe(false);
    expect(faceDoEnvio(aba, [SALA])).toBe("frente");
    expect(envioComFace(estadoDoEnvio(aba, [SALA], new Set()), aba, [SALA], null).rotulo).toBe(
      t("imprimir.frenteContagem").replace("{n}", "1"),
    );
    expect(dicaDaFace(aba, [SALA])).toBe(t("face.semVerso"));
  });

  it("um trecho de uma página também força a frente", () => {
    const estado = abaCom(24, { inicio: "3", fim: "3" });
    const aba = abaDe(estado);
    expect(temVersoDaAba(aba)).toBe(false);
    expect(aba.face).toBe("frente");
    expect(faceDoEnvio(aba, [SALA])).toBe("frente");
  });

  it("três páginas continuam com as duas faces", () => {
    let estado = abaCom(3);
    expect(temVersoDaAba(abaDe(estado))).toBe(true);
    estado = reduzirAbas(estado, { tipo: "definir_face", id: 1, face: "verso" });
    expect(abaDe(estado).face).toBe("verso");
    expect(faceDoEnvio(abaDe(estado), [SALA])).toBe("verso");
  });

  it("o seletor desabilita Verso com o motivo visível e fora da tabulação", () => {
    const html = renderToStaticMarkup(
      <BlocoFace face="verso" aoDefinir={() => undefined} semVerso />,
    );
    expect(html).toContain(t("face.semVerso"));
    const campo = (valor: "frente" | "verso") => {
      const indice = html.indexOf(`value="${valor}"`);
      const inicio = html.lastIndexOf("<input", indice);
      const fim = html.indexOf(">", indice);
      return html.slice(inicio, fim);
    };
    expect(campo("verso")).toContain('disabled=""');
    expect(campo("verso")).toContain('tabindex="-1"');
    expect(campo("frente")).toContain('checked=""');
  });

  it("as frentes de uma página são terminais no cartão", () => {
    const item = cartao({ tipo: "frentes_prontas" }, { temVerso: false });
    const linhas = linhasDoCartao(item).map((linha) => linha.texto);
    expect(tomDoCartao(item)).toBe("ativo");
    expect(textoDoEstadoDoCartao(item)).toBe(t("cartao.estado.concluido"));
    expect(linhas).toContain(t("face.semVerso"));
    expect(linhas).not.toContain(t("cartao.esperar"));
    expect(linhas).not.toContain(t("face.espera"));
    expect(acoesDoCartao(item).pedirVerso).toBe(false);
    expect(acoesDoCartao(item).verso.visivel).toBe(false);
    expect(cartaoPedeAcao(item)).toBe(false);
    expect(cartaoSeguraAAba(item)).toBe(false);
    expect(desenhoDoCartao(item)).toBeNull();

    const html = renderToStaticMarkup(<CartaoImpressao cartao={item} />);
    expect(html).toContain("1 folha");
    expect(html).toContain("A4");
    expect(html).toContain(t("cartao.estado.concluido"));
    expect(html).toContain(t("face.semVerso"));
    expect(html).not.toContain(t("cartao.esperar"));
  });

  it("repetir as frentes usa o nome certo e mantém a confirmação", () => {
    let estado = abaCom(1);
    estado = comCartao(estado, cartao({ tipo: "frentes_prontas" }, { temVerso: false }));
    const aba = abaDe(estado);
    const envio = envioComFace(estadoDoEnvio(aba, [SALA], new Set()), aba, [SALA], null);
    expect(envio.rotulo).toBe(
      t("imprimir.frenteRepetirContagem").replace("{n}", "1"),
    );
    expect(aba.frentesProntas).toContain(SALA.id);
  });

  it("repetir as frentes não manda escolher Verso na nota", () => {
    let estado = abaCom(1);
    estado = comCartao(estado, cartao({ tipo: "frentes_prontas" }, { temVerso: false }));
    const resumo: ResumoDoEnvio = {
      paginas: 1,
      lotes: 1,
      folhas: 1,
      maiorMaco: 1,
      papeis: [{ formato: "a4", larguraMm: 210, alturaMm: 297 }],
      paresMistos: [],
      assinado: false,
    };
    const notas = notasDaFrente(abaDe(estado), [SALA], resumo);
    expect(notas.antes.join(" ")).toContain(
      t("face.envio.jaSaiuSemVerso").replace("{lista}", "Sala"),
    );
    expect(notas.antes.join(" ")).not.toContain(t("face.envio.jaSaiu").replace("{lista}", "Sala"));
  });

  it("fechar a aba de um trabalho sem verso não acrescenta a nota dos versos", () => {
    let estado = abaCom(1);
    estado = comCartao(estado, cartao({ tipo: "frentes_prontas" }, { temVerso: false }));
    const pedido = reduzirAbas(estado, { tipo: "pedir_fechamento", id: 1 });
    const conteudo = conteudoDaConfirmacao(pedido);
    expect(conteudo?.notas ?? []).not.toContain(t("face.fechar.versos"));
  });
});