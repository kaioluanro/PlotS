import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  CANAIS_DE_TINTA,
  CHAVE_DO_CANAL,
  configuracaoPadrao,
  type ConfiguracaoDeTinta,
  type PerfilDeTinta,
} from "./analise-tinta";
import { PainelDaEngrenagem } from "./components/PlotInfoEngrenagem";
import { traduzir } from "./i18n";
import {
  errosDoRascunho,
  rascunhoDoPerfil,
  type RascunhoDoPerfil,
} from "./plotinfo-estado";

const PERFIL: PerfilDeTinta = {
  id: "tanque-generico",
  nome: "Tanque de tinta genérico",
  moeda: "BRL",
  volumes_ml: [65, 65, 65, 65],
  rendimentos_paginas: [6500, 6500, 6500, 4500],
  precos_ml: [0.77, 0.77, 0.77, 1.11],
  fatores_qualidade: [0.6, 1, 1.3],
};

interface Opcoes {
  perfis?: readonly PerfilDeTinta[];
  rascunho?: RascunhoDoPerfil;
  configuracao?: ConfiguracaoDeTinta;
  selecionado?: string;
  novo?: boolean;
  confirmandoApagar?: boolean;
  salvando?: boolean;
  erro?: string | null;
  aviso?: string | null;
}

function engrenagem(opcoes: Opcoes = {}): string {
  const perfis = opcoes.perfis ?? [PERFIL];
  const rascunho = opcoes.rascunho ?? rascunhoDoPerfil(PERFIL);
  const configuracao = opcoes.configuracao ?? configuracaoPadrao(PERFIL.id);
  return renderToStaticMarkup(
    <PainelDaEngrenagem
      perfis={perfis}
      rascunho={rascunho}
      configuracao={configuracao}
      selecionado={opcoes.selecionado ?? PERFIL.id}
      novo={opcoes.novo ?? false}
      erros={errosDoRascunho(rascunho, perfis.map((perfil) => perfil.id), PERFIL.id)}
      confirmandoApagar={opcoes.confirmandoApagar ?? false}
      salvando={opcoes.salvando ?? false}
      erro={opcoes.erro ?? null}
      aviso={opcoes.aviso ?? null}
      aoSelecionar={() => undefined}
      aoNovo={() => undefined}
      aoDuplicar={() => undefined}
      aoPedirApagar={() => undefined}
      aoConfirmarApagar={() => undefined}
      aoCancelarApagar={() => undefined}
      aoRestaurar={() => undefined}
      aoMudarPerfil={() => undefined}
      aoMudarCanal={() => undefined}
      aoMudarFator={() => undefined}
      aoMudarConfiguracao={() => undefined}
      aoSalvar={() => undefined}
      aoCancelar={() => undefined}
    />,
  );
}

interface Leitor {
  readFileSync(caminho: URL, codificacao: "utf8"): string;
}

const modulofs = "node:fs";

async function lerArquivo(nome: string): Promise<string> {
  const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;
  return fs.readFileSync(new URL(nome, import.meta.url), "utf8").replace(/\r\n/g, "\n");
}

function regra(css: string, seletor: string): string {
  const inicio = css.indexOf(`${seletor} {`);
  expect(inicio, seletor).toBeGreaterThanOrEqual(0);
  return css.slice(inicio, css.indexOf("}", inicio));
}

describe("perfis e tabela por canal", () => {
  const html = engrenagem();

  it("mostra o perfil ativo e as acoes de perfil", () => {
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.perfil"));
    expect(html).toContain(PERFIL.nome);
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.novo"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.duplicar"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.apagar"));
  });

  it("tem uma linha por canal com volume, rendimento e preco por mL", () => {
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.volume"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.rendimento"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.preco"));
    for (const canal of CANAIS_DE_TINTA) {
      expect(html).toContain(traduzir("pt-BR", CHAVE_DO_CANAL[canal]));
    }
    expect(html).toContain('value="65"');
    expect(html).toContain('value="6500"');
    expect(html).toContain('value="0,77"');
    expect(html).toContain('value="0,6"');
  });

  it("mostra a moeda, a ajuda do preco e os fatores de qualidade", () => {
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.moeda"));
    expect(html).toContain('value="BRL"');
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.moedaAjuda"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.fatores"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.ajudaPreco"));
  });
});

describe("configuracao ativa", () => {
  const html = engrenagem();

  it("mostra qualidade, dpi, modo, cinzas e tamanho da imagem", () => {
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.qualidade"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.qualidade.normal"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.dpi"));
    expect(html).toContain("150 dpi");
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.modo"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.modo.simples"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.modo.cmyk"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.cinzas"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.tamanho"));
    expect(html).toContain(traduzir("pt-BR", "plotinfo.tamanho.a4"));
  });

  it("diz que nesta versao o CMYK cai no modo Simples", () => {
    expect(html).toContain("modo Simples (PDFium)");
  });
});

describe("validacao", () => {
  it("um campo invalido fica destacado e o salvar fica desligado", () => {
    const rascunho: RascunhoDoPerfil = {
      ...rascunhoDoPerfil(PERFIL),
      volumes_ml: ["0", "65", "65", "65"],
    };
    const html = engrenagem({ rascunho });
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.invalido"));
    expect(html).toContain('aria-invalid="true"');
    expect(html).toContain('disabled=""');
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.salvar"));
  });

  it("um perfil valido deixa salvar ligado", () => {
    expect(engrenagem()).not.toContain('disabled=""');
  });

  it("sem perfis, explica e nao oferece salvar", () => {
    const html = engrenagem({
      perfis: [],
      rascunho: rascunhoDoPerfil(PERFIL),
    });
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.semPerfis"));
    expect(html).toContain('disabled=""');
  });
});

describe("apagar e erros", () => {
  it("pede confirmacao antes de apagar e oferece manter", () => {
    const html = engrenagem({ confirmandoApagar: true });
    expect(html).toContain(
      traduzir("pt-BR", "plotinfo.engrenagem.confirmarApagar", { nome: PERFIL.nome }),
    );
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.manter"));
  });

  it("a falha de gravacao aparece em alerta e o dialogo continua aberto", () => {
    const html = engrenagem({
      erro: traduzir("pt-BR", "plotinfo.engrenagem.erroSalvar", { motivo: "disco cheio" }),
    });
    expect(html).toContain('role="alert"');
    expect(html).toContain("disco cheio");
    expect(html).toContain(traduzir("pt-BR", "plotinfo.engrenagem.salvar"));
  });

  it("o aviso do arquivo de perfis aparece sem impedir a edicao", () => {
    const html = engrenagem({
      aviso: "O arquivo de perfis está corrompido; os padrões valem.",
    });
    expect(html).toContain("está corrompido");
    expect(html).not.toContain('role="alert"');
  });
});

describe("dialogo da engrenagem", () => {
  it("cabe em 560 px, com o corpo rolando e o rodape fixo", async () => {
    const css = await lerArquivo("./components/PlotInfoEngrenagem.module.css");
    expect(regra(css, ".conteudo")).toMatch(/width:\s*min\(560px/);
    expect(regra(css, ".conteudo")).toMatch(/max-height:\s*calc\(100vh - 32px\)/);
    expect(regra(css, ".corpo")).toMatch(/overflow-y:\s*auto/);
    expect(regra(css, ".rodape")).toMatch(/flex-shrink:\s*0/);
  });

  it("os campos tem alvo de 28 px e o invalido usa borda tracejada", async () => {
    const css = await lerArquivo("./components/PlotInfoEngrenagem.module.css");
    expect(regra(css, ".entrada,\n.entradaDaTabela")).toMatch(/min-height:\s*28px/);
    expect(css).toContain('border-style: dashed');
    expect(css).toContain("@media (forced-colors: active)");
  });
});

describe("tokens da tinta nos dois temas", () => {
  it("cada canal tem a sua cor no escuro e no cinza", async () => {
    const css = await lerArquivo("./theme.css");
    for (const token of ["--ink-cyan", "--ink-magenta", "--ink-yellow", "--ink-key"]) {
      const ocorrencias = css.match(new RegExp(`${token}:`, "g")) ?? [];
      expect(ocorrencias, token).toHaveLength(2);
    }
  });

  it("as duas folhas usam a cor da tinta por variavel, nunca um hexadecimal local", async () => {
    for (const nome of [
      "./components/PlotInfoGaveta.module.css",
      "./components/PlotInfoEngrenagem.module.css",
    ]) {
      const css = await lerArquivo(nome);
      expect(css, nome).not.toMatch(/#[0-9a-f]{6}/i);
      expect(css).toContain("var(--ink-cyan)");
    }
  });
});

describe("idiomas", () => {
  it("a engrenagem tem titulo nos dois idiomas", () => {
    expect(traduzir("pt-BR", "plotinfo.engrenagem.titulo")).toBe("Perfis e configuração");
    expect(traduzir("en", "plotinfo.engrenagem.titulo")).toBe("Profiles and settings");
  });
});