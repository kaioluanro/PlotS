import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import {
  apagarPerfilDeTinta,
  cancelarAnalise,
  configuracaoDeTinta,
  definirConfiguracaoDeTinta,
  escolherCaminhoCsv,
  exportarCsv,
  iniciarAnalise,
  lerCopiasDoPainel,
  mensagemDeAviso,
  motivoDaFalha,
  normalizarResultado,
  ouvirAnaliseDeTinta,
  perfisDeTinta,
  recalcularAnalise,
  salvarPerfilDeTinta,
  textoDePaginas,
  type ConfiguracaoDeTinta,
  type PerfilDeTinta,
  type ProgressoDaAnalise,
  type ResultadoDaAnalise,
  type ResultadoDaPagina,
} from "./analise-tinta";
import { t } from "./i18n";
import {
  ESTADO_INICIAL_DO_PAINEL,
  reduzirPainelDeTinta,
  type EstadoDoPainelDeTinta,
} from "./plotinfo-estado";
import type { Aba } from "./tabs";

export type SituacaoDaTinta =
  | "fechado"
  | "sem_documento"
  | "contando"
  | "recusado"
  | "analisando"
  | "pronto"
  | "erro";

export interface UsoDaTinta {
  situacao: SituacaoDaTinta;
  erro: string | null;
  aviso: string | null;
  mensagem: string | null;
  exportando: boolean;
  resultado: ResultadoDaAnalise | null;
  paginas: ReadonlyMap<number, ResultadoDaPagina>;
  progresso: ProgressoDaAnalise | null;
  recalculando: boolean;
  painel: EstadoDoPainelDeTinta;
  copias: number | null;
  configuracao: ConfiguracaoDeTinta | null;
  perfis: readonly PerfilDeTinta[];
  perfilAtivo: PerfilDeTinta | null;
  avisoDosPerfis: string | null;
  definirPaginas: (valor: string) => void;
  definirCopias: (valor: string) => void;
  tentarDeNovo: () => void;
  aplicarConfiguracao: (nova: ConfiguracaoDeTinta) => Promise<void>;
  salvarPerfil: (perfil: PerfilDeTinta) => Promise<void>;
  apagarPerfil: (id: string) => Promise<void>;
  exportar: () => Promise<void>;
}

function juntarPaginas(
  atuais: ReadonlyMap<number, ResultadoDaPagina>,
  novas: readonly ResultadoDaPagina[],
): ReadonlyMap<number, ResultadoDaPagina> {
  if (novas.length === 0) return atuais;
  const mapa = new Map(atuais);
  for (const pagina of novas) mapa.set(pagina.numero, pagina);
  return mapa;
}

function chaveDaSelecao(
  paginas: string | null,
  copias: number,
  configuracao: ConfiguracaoDeTinta,
): string {
  return JSON.stringify([
    paginas,
    copias,
    configuracao.perfil,
    configuracao.qualidade,
    configuracao.dpi,
    configuracao.modo,
    configuracao.cinzas_so_preto,
    configuracao.tamanho_da_imagem,
  ]);
}

/**
 * Orquestra a analise da aba ativa enquanto a gaveta esta aberta. Fechar ou trocar de aba cancela
 * o trabalho; o ultimo resultado fica para o resumo da barra. Campo de paginas e de copias so
 * alimentam este estado e o backend.
 */
export function useTintaDoDocumento(
  aba: Aba | null,
  aberto: boolean,
  paginaAtual: number | null,
): UsoDaTinta {
  const [painel, despacharPainel] = useReducer(reduzirPainelDeTinta, ESTADO_INICIAL_DO_PAINEL);
  const [situacao, setSituacao] = useState<SituacaoDaTinta>("fechado");
  const [erro, setErro] = useState<string | null>(null);
  const [aviso, setAviso] = useState<string | null>(null);
  const [mensagem, setMensagem] = useState<string | null>(null);
  const [exportando, setExportando] = useState(false);
  const [resultado, setResultado] = useState<ResultadoDaAnalise | null>(null);
  const [paginas, setPaginas] = useState<ReadonlyMap<number, ResultadoDaPagina>>(new Map());
  const [progresso, setProgresso] = useState<ProgressoDaAnalise | null>(null);
  const [recalculando, setRecalculando] = useState(false);
  const [configuracao, setConfiguracao] = useState<ConfiguracaoDeTinta | null>(null);
  const [perfis, setPerfis] = useState<readonly PerfilDeTinta[]>([]);
  const [avisoDosPerfis, setAvisoDosPerfis] = useState<string | null>(null);
  const [prontoParaAnalisar, setProntoParaAnalisar] = useState(false);
  const [tentativa, setTentativa] = useState(0);

  const idAtual = useRef<string | null>(null);
  const selecaoEnviada = useRef<string | null>(null);
  const coberturaAnalisada = useRef<string | null>(null);
  const copiasAtuais = lerCopiasDoPainel(painel.copiasTexto);

  const painelRef = useRef(painel);
  painelRef.current = painel;
  const copiasRef = useRef(copiasAtuais);
  copiasRef.current = copiasAtuais;
  const configuracaoRef = useRef(configuracao);
  configuracaoRef.current = configuracao;
  const paginaAtualRef = useRef(paginaAtual);
  paginaAtualRef.current = paginaAtual;
  const abaRef = useRef(aba);
  abaRef.current = aba;

  const idDaAba = aba?.id ?? null;
  const chaveDaCobertura = useMemo(() => {
    if (configuracao === null) return null;
    return `${configuracao.dpi}|${configuracao.modo}|${configuracao.cinzas_so_preto}`;
  }, [configuracao]);

  // Trocar de documento zera a leitura: a selecao do painel vale para o documento aberto.
  useEffect(() => {
    setResultado(null);
    setPaginas(new Map());
    setProgresso(null);
    setErro(null);
    setAviso(null);
    setMensagem(null);
    setExportando(false);
    setRecalculando(false);
    idAtual.current = null;
    selecaoEnviada.current = null;
    coberturaAnalisada.current = null;
    despacharPainel({ tipo: "reiniciar" });
  }, [idDaAba]);

  // Le perfis e configuracao ao abrir a gaveta, com ou sem documento: a engrenagem vale para os dois.
  useEffect(() => {
    if (!aberto) {
      setProntoParaAnalisar(false);
      setSituacao(idDaAba === null ? "sem_documento" : "fechado");
      return undefined;
    }
    const documento = abaRef.current;
    const semDocumento = documento === null;
    if (semDocumento) {
      setSituacao("sem_documento");
    } else if (documento.paginas.estado === "recusado") {
      setSituacao("recusado");
    } else if (documento.paginas.estado === "contando") {
      setSituacao("contando");
    } else {
      setSituacao("analisando");
    }

    let ativo = true;
    setProntoParaAnalisar(false);
    setErro(null);
    setAviso(null);
    setMensagem(null);
    void Promise.all([perfisDeTinta(), configuracaoDeTinta()])
      .then(([estadoDosPerfis, estadoDaConfiguracao]) => {
        if (!ativo) return;
        setPerfis(estadoDosPerfis.perfis);
        setAvisoDosPerfis(
          mensagemDeAviso(estadoDosPerfis.aviso ?? estadoDaConfiguracao.aviso),
        );
        const primeiro = estadoDosPerfis.perfis[0]?.id ?? "";
        const carregada = estadoDaConfiguracao.configuracao;
        const perfil = estadoDosPerfis.perfis.some((item) => item.id === carregada.perfil)
          ? carregada.perfil
          : primeiro;
        if (perfil === "" || estadoDosPerfis.perfis.length === 0) {
          setErro(t("plotinfo.engrenagem.semPerfis"));
          if (!semDocumento) setSituacao("erro");
          setProntoParaAnalisar(false);
          return;
        }
        setConfiguracao({ ...carregada, perfil });
        setProntoParaAnalisar(true);
      })
      .catch((falha: unknown) => {
        if (!ativo) return;
        setErro(motivoDaFalha(falha));
        if (!semDocumento) setSituacao("erro");
        setProntoParaAnalisar(false);
      });
    return () => {
      ativo = false;
    };
  }, [aberto, idDaAba, aba?.documento.caminho, aba?.paginas.estado, tentativa]);

  // Uma analise por abertura. O cleanup cancela ao fechar, trocar de aba ou tentar de novo.
  useEffect(() => {
    if (!aberto || !prontoParaAnalisar) return undefined;
    const documento = abaRef.current;
    const configuracaoAtual = configuracaoRef.current;
    if (documento === null || documento.paginas.estado !== "conhecido") return undefined;
    if (configuracaoAtual === null || configuracaoAtual.perfil === "") return undefined;
    if (idAtual.current !== null) return undefined;
    const copiasIniciais = copiasRef.current;
    if (copiasIniciais === null) {
      setErro(t("plotinfo.erro.copias"));
      setSituacao("erro");
      return undefined;
    }

    let ativo = true;
    let cancelarEventos: (() => void) | null = null;
    coberturaAnalisada.current = chaveDaCobertura;
    setSituacao("analisando");
    setErro(null);
    setProgresso(null);

    void (async () => {
      cancelarEventos = await ouvirAnaliseDeTinta(
        {
          aoProgresso(progressoNovo) {
            if (!ativo) return;
            setProgresso(progressoNovo);
            setSituacao((atual) => (atual === "erro" ? atual : "analisando"));
          },
          aoPagina(pagina) {
            if (!ativo) return;
            setPaginas((atuais) => juntarPaginas(atuais, [pagina]));
          },
          aoFim(resultadoNovo) {
            if (!ativo) return;
            setResultado(resultadoNovo);
            setAviso(mensagemDeAviso(resultadoNovo.aviso));
            setProgresso(null);
            setRecalculando(false);
            setSituacao("pronto");
            setErro(null);
          },
          aoFalha(motivo) {
            if (!ativo) return;
            setErro(motivo);
            setRecalculando(false);
            setSituacao("erro");
          },
        },
        () => idAtual.current,
        documento.documento.nome,
      );
      if (!ativo) {
        cancelarEventos();
        return;
      }
      const paginasIniciais = textoDePaginas(painelRef.current.paginasTexto);
      try {
        const id = await iniciarAnalise(
          documento.documento.caminho,
          paginasIniciais,
          copiasIniciais,
          configuracaoAtual,
          paginaAtualRef.current,
        );
        if (!ativo) {
          void cancelarAnalise(id).catch(() => undefined);
          return;
        }
        idAtual.current = id;
        selecaoEnviada.current = chaveDaSelecao(
          paginasIniciais,
          copiasIniciais,
          configuracaoAtual,
        );
      } catch (falha) {
        if (!ativo) return;
        setErro(motivoDaFalha(falha));
        setSituacao("erro");
      }
    })();

    return () => {
      ativo = false;
      cancelarEventos?.();
      const id = idAtual.current;
      idAtual.current = null;
      if (id !== null) void cancelarAnalise(id).catch(() => undefined);
    };
    // A configuracao entra por ref: trocar perfil, qualidade ou dpi recalcula, nao reanalisa.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [aberto, prontoParaAnalisar, idDaAba, aba?.documento.caminho, aba?.paginas.estado, tentativa, chaveDaCobertura]);

  const chaveAtual = useMemo(() => {
    if (configuracao === null) return null;
    const copias = copiasAtuais ?? -1;
    return chaveDaSelecao(textoDePaginas(painel.paginasTexto), copias, configuracao);
  }, [configuracao, copiasAtuais, painel.paginasTexto]);

  // Cada mudanca de campo ou de configuracao chama o recalculo; o backend continua a fonte da selecao.
  // Dpi, modo e cinzas mudam a cobertura: nesse caso a analise e refeita, nao recalculada.
  useEffect(() => {
    if (!aberto || chaveAtual === null || idAtual.current === null) return undefined;
    if (situacao !== "pronto") {
      setRecalculando(false);
      return undefined;
    }
    if (coberturaAnalisada.current !== chaveDaCobertura) {
      setRecalculando(false);
      return undefined;
    }
    if (copiasAtuais === null) {
      setErro(t("plotinfo.erro.copias"));
      setRecalculando(false);
      return undefined;
    }
    if (selecaoEnviada.current === chaveAtual) {
      setRecalculando(false);
      return undefined;
    }
    const id = idAtual.current;
    const configuracaoPedida = configuracaoRef.current;
    if (configuracaoPedida === null) return undefined;
    const paginasPedidas = textoDePaginas(painelRef.current.paginasTexto);
    setRecalculando(true);
    const prazo = window.setTimeout(() => {
      if (idAtual.current !== id) return;
      void recalcularAnalise(id, configuracaoPedida, paginasPedidas, copiasAtuais)
        .then((resposta) => {
          if (idAtual.current !== id) return;
          const novo = normalizarResultado(
            resposta,
            id,
            abaRef.current?.documento.nome ?? "",
          );
          if (novo === null) {
            setErro(t("plotinfo.erro.resposta"));
            setRecalculando(false);
            return;
          }
          setResultado(novo);
          setAviso(mensagemDeAviso(novo.aviso));
          setSituacao("pronto");
          setProgresso(null);
          selecaoEnviada.current = chaveAtual;
          setErro(null);
          setRecalculando(false);
        })
        .catch((falha: unknown) => {
          if (idAtual.current !== id) return;
          setErro(motivoDaFalha(falha));
          setRecalculando(false);
        });
    }, 250);
    return () => window.clearTimeout(prazo);
  }, [aberto, chaveAtual, copiasAtuais, situacao, chaveDaCobertura]);

  const definirPaginas = useCallback((valor: string) => {
    despacharPainel({ tipo: "paginas", valor });
  }, []);

  const definirCopias = useCallback((valor: string) => {
    despacharPainel({ tipo: "copias", valor });
    const erroDeCopias = t("plotinfo.erro.copias");
    if (lerCopiasDoPainel(valor) === null) {
      setErro(erroDeCopias);
    } else {
      // So o erro do campo sai; uma falha da analise continua visivel.
      setErro((atual) => (atual === erroDeCopias ? null : atual));
      if (
        aberto &&
        idAtual.current === null &&
        abaRef.current?.paginas.estado === "conhecido"
      ) {
        setTentativa((atual) => atual + 1);
      }
    }
  }, [aberto]);

  const tentarDeNovo = useCallback(() => {
    setErro(null);
    setProgresso(null);
    setRecalculando(false);
    setTentativa((valor) => valor + 1);
  }, []);

  const aplicarConfiguracao = useCallback(async (nova: ConfiguracaoDeTinta) => {
    const estado = await definirConfiguracaoDeTinta(nova);
    setConfiguracao(estado.configuracao);
    setAvisoDosPerfis(mensagemDeAviso(estado.aviso));
  }, []);

  const salvarPerfil = useCallback(async (perfil: PerfilDeTinta) => {
    const estado = await salvarPerfilDeTinta(perfil);
    setPerfis(estado.perfis);
    const avisoTraduzido = mensagemDeAviso(estado.aviso);
    setAvisoDosPerfis(avisoTraduzido);
    if (avisoTraduzido !== null) setAviso(avisoTraduzido);
  }, []);

  const apagarPerfil = useCallback(async (id: string) => {
    const estado = await apagarPerfilDeTinta(id);
    setPerfis(estado.perfis);
    setAvisoDosPerfis(mensagemDeAviso(estado.aviso));
    // Apagar o perfil ativo troca a configuracao no backend; a leitura mantem a gaveta coerente.
    const configuracaoLida = await configuracaoDeTinta();
    const primeiro = estado.perfis[0]?.id ?? "";
    const perfil = estado.perfis.some((item) => item.id === configuracaoLida.configuracao.perfil)
      ? configuracaoLida.configuracao.perfil
      : primeiro;
    setConfiguracao({ ...configuracaoLida.configuracao, perfil });
  }, []);

  const exportar = useCallback(async () => {
    const id = idAtual.current;
    if (id === null) {
      setMensagem(t("plotinfo.csv.semResultado"));
      return;
    }
    setExportando(true);
    setMensagem(null);
    try {
      const caminho = await escolherCaminhoCsv(abaRef.current?.documento.nome ?? "");
      if (caminho === null) return;
      await exportarCsv(id, caminho);
      setMensagem(t("plotinfo.aviso.csvExportado", { caminho }));
    } catch {
      setErro(t("plotinfo.erro.csvSimples"));
    } finally {
      setExportando(false);
    }
  }, []);

  const perfilAtivo =
    configuracao === null
      ? null
      : (perfis.find((perfil) => perfil.id === configuracao.perfil) ?? null);

  return {
    situacao,
    erro,
    aviso,
    mensagem,
    exportando,
    resultado,
    paginas,
    progresso,
    recalculando,
    painel,
    copias: copiasAtuais,
    configuracao,
    perfis,
    perfilAtivo,
    avisoDosPerfis,
    definirPaginas,
    definirCopias,
    tentarDeNovo,
    aplicarConfiguracao,
    salvarPerfil,
    apagarPerfil,
    exportar,
  };
}