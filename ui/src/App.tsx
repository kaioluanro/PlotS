import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import styles from "./App.module.css";
import { deveBloquearTecla, interpretarAtalho } from "./atalhos";
import {
  abrirDocumento,
  assinar,
  avisarPedidoDeFechamentoRecebido,
  caminhosIniciais,
  contarPaginas,
  definirModelo,
  escolherArquivo,
  limitarLote,
  listarImpressoras,
} from "./backend";
import {
  cartoesComMudanca,
  ESTADO_INICIAL_DOS_AVISOS,
  naoLidos as contarNaoLidos,
  reduzirAvisos,
} from "./avisos";
import { AvisosDoTopo } from "./components/AvisosDoTopo";
import { PainelDeAvisos } from "./components/PainelDeAvisos";
import { BlocoAjuste } from "./components/BlocoAjuste";
import { BlocoFace } from "./components/BlocoFace";
import { CampoCopias } from "./components/CampoCopias";
import { copiasDaFace, temVersoDaAba } from "./tabs";
import { BlocoGrupos } from "./components/BlocoGrupos";
import { HistoricoDialog } from "./components/HistoricoDialog";
import { CalibracaoDialog } from "./components/CalibracaoDialog";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { DocumentArea, ID_AREA_DOCUMENTO } from "./components/DocumentArea";
import { OpcoesDaImpressoraDialog } from "./components/OpcoesDaImpressoraDialog";
import { ID_LATERAL, Sidebar } from "./components/Sidebar";
import { focarAbaAtiva, TitleBar } from "./components/TitleBar";
import {
  aindaPodeCancelar,
  cartaoAtual,
  cartaoInterrompido,
  cartaoSemResposta,
  cartoesRecusados,
  estadoDoEnvio,
  impressorasOcupadas,
  linhasDoEnvio,
  nomeDoCartao,
  precisaConfirmar,
  type Cartao,
  type Comando,
} from "./cartoes";
import {
  conteudoDaConfirmacao,
  conteudoDaReimpressao,
  conteudoDaSubstituicao,
  conteudoDoApagarGrupo,
  conteudoDoCancelamento,
  conteudoDoCancelamentoJaTerminado,
  conteudoDoEnvio,
  type ConteudoConfirmacao,
  type PedidoDeSubstituicao,
} from "./confirmacao";
import { mensagemDeErro } from "./erros";
import { acaoDoEnvio, esperasDoRodape, type EsperaDoRodape } from "./rodape-espera";
import {
  conteudoDoVerso,
  dicaDaFace,
  envioComFace,
  faceDisponivel,
  faceDoEnvio,
  mudouDesdeAFrente,
  notasDaFrente,
  planoDoVerso,
  quemDeuErro,
  quemJaSaiu,
} from "./face-do-envio";
import { useGrupos } from "./grupos-uso";
import { apagarHistorico, conteudoDoApagarHistorico } from "./historico";
import { idiomaAtual, t, traduzir } from "./i18n";
import { useAnaliseDoDocumento } from "./analise-documento";
import { envioSemMarca, notasDasOpcoes, textoDoPapelAjustado } from "./opcoes-envio";
import {
  comModeloGravado,
  gravarEscolha,
  liberaMenuDeContexto,
  nomeDeExibicao,
  type Aviso,
  type Impressora,
  type ListaImpressoras,
  type Modelo,
} from "./impressoras";
import {
  envioComIntervalo,
  lerIntervaloDaAba,
  linhasComTrecho,
  trechoDoEnvio,
  type RascunhoIntervalo,
} from "./intervalo";
import { abrirLote, type LoteCaminhos } from "./lote";
import { usePreferencias } from "./preferencias-estado";
import {
  decidirFechamentoJanela,
  deveOuvirFechamentoJanela,
  estadoInicial,
  reduzirAbas,
  type Aba,
  type FaceDoEnvio,
} from "./tabs";
import {
  comandarTrabalho,
  iniciarImpressao,
  ouvirCartoes,
  resumoDoEnvio,
  soltarTrabalhos,
} from "./trabalhos";

export function App() {
  const [estado, despachar] = useReducer(reduzirAbas, estadoInicial);
  const [lateralAberta, setLateralAberta] = useState(true);
  const [erro, setErro] = useState<string | null>(null);
  // A quantidade digitada e um numero de 1 a 999: enquanto nao for, o envio da face espera.
  const [copiasValidas, setCopiasValidas] = useState(true);
  const [arrastando, setArrastando] = useState(false);
  const seletorAberto = useRef(false);
  const inicioFeito = useRef(false);
  const focarDepoisDeFechar = useRef(false);

  const confirmacao = conteudoDaConfirmacao(estado);
  const [aCalibrar, setACalibrar] = useState<Impressora | null>(null);
  const [lista, setLista] = useState<ListaImpressoras>({ impressoras: [], avisos: [] });
  const [lendoLista, setLendoLista] = useState(true);
  const preferencias = usePreferencias(lendoLista, lista.impressoras);
  // A fila do painel de opcoes, e se ela ja tinha ajuste salvo quando a chave de boca foi usada.
  const [filaDasOpcoes, setFilaDasOpcoes] = useState<string | null>(null);
  const [ajusteDasOpcoes, setAjusteDasOpcoes] = useState(false);
  const [gravacoes, setGravacoes] = useState<ReadonlyMap<string, Modelo>>(new Map());
  const [falhaAoGravar, setFalhaAoGravar] = useState<Aviso | null>(null);
  const [substituicao, setSubstituicao] = useState<PedidoDeSubstituicao | null>(null);
  const [pedidoDeEnvio, setPedidoDeEnvio] = useState<{
    conteudo: ConteudoConfirmacao;
    aba: Aba;
    destinos: readonly Impressora[];
    face: FaceDoEnvio;
    /** O verso que ja espera na fila: confirmado, vai como comando, nao como envio novo. */
    espera?: readonly EsperaDoRodape[];
  } | null>(null);
  const grupos = useGrupos(despachar);
  const [cartaoACancelar, setCartaoACancelar] = useState<Cartao | null>(null);
  const [cartaoAReimprimir, setCartaoAReimprimir] = useState<Cartao | null>(null);
  const [historico, setHistorico] = useState<"lista" | "apagar" | null>(null);
  // Avisos da sessao: cada mudanca de estado de um cartao vira um registro, em memoria.
  const [avisos, despacharAvisos] = useReducer(reduzirAvisos, ESTADO_INICIAL_DOS_AVISOS);
  const [painelAvisos, setPainelAvisos] = useState(false);
  const assinaturasDosCartoes = useRef<ReadonlyMap<string, string>>(new Map());
  const [versaoDoHistorico, setVersaoDoHistorico] = useState(0);
  const [falhaAoApagarHistorico, setFalhaAoApagarHistorico] = useState(false);
  const preparandoEnvio = useRef(false);
  const filasGravando = useRef(new Set<string>());
  const ultimaLeitura = useRef(0);
  const contagensPedidas = useRef(new Set<number>());
  const filaDeContagens = useRef<Promise<void>>(Promise.resolve());
  // Qualquer dialogo aberto impede que atalhos e arrastos ajam por tras dele.
  const confirmando =
    confirmacao !== null ||
    aCalibrar !== null ||
    substituicao !== null ||
    pedidoDeEnvio !== null ||
    cartaoACancelar !== null ||
    cartaoAReimprimir !== null ||
    historico !== null ||
    painelAvisos ||
    grupos.aApagar !== null;
  const abaAtiva = estado.abas.find((aba) => aba.id === estado.ativa) ?? null;
  const analiseDoDocumento = useAnaliseDoDocumento(abaAtiva);
  const idAtiva = estado.ativa;

  // Os eventos do sistema ficam assinados uma vez so e precisam ler o estado mais recente.
  const estadoAtual = useRef(estado);
  estadoAtual.current = estado;
  const confirmandoAgora = useRef(confirmando);
  confirmandoAgora.current = confirmando;

  const abrirCaminhos = useCallback(async (lote: LoteCaminhos) => {
    const mensagem = await abrirLote(lote, abrirDocumento, (documento) =>
      despachar({ tipo: "abrir", documento }),
    );
    setErro(mensagem);
  }, []);

  const abrirPeloSeletor = useCallback(async () => {
    if (seletorAberto.current) return;
    seletorAberto.current = true;
    try {
      const caminho = await escolherArquivo();
      if (caminho !== null) await abrirCaminhos({ caminhos: [caminho], cortado: false });
    } catch {
      setErro(t("erro.seletor"));
    } finally {
      seletorAberto.current = false;
    }
  }, [abrirCaminhos]);

  const pedirFechamento = useCallback((id: number) => {
    focarDepoisDeFechar.current = true;
    despachar({ tipo: "pedir_fechamento", id });
  }, []);

  const atualizarLista = useCallback(() => {
    // So a leitura mais recente vale: um clique repetido nao deixa a resposta antiga por cima.
    ultimaLeitura.current += 1;
    const leitura = ultimaLeitura.current;
    setLendoLista(true);
    listarImpressoras()
      .then((nova) => {
        if (leitura === ultimaLeitura.current) setLista(nova);
      })
      .catch(() => {
        if (leitura === ultimaLeitura.current) {
          setLista((atual) => ({
            impressoras: atual.impressoras,
            avisos: [{ codigo: "comunicacao" }],
          }));
        }
      })
      .finally(() => {
        if (leitura === ultimaLeitura.current) setLendoLista(false);
      });
  }, []);

  useEffect(() => {
    atualizarLista();
  }, [atualizarLista]);

  const aoEscolherModelo = useCallback(
    (fila: string, modelo: Modelo) => {
      // Uma gravacao por fila: a ordem de duas escolhas seguidas nao fica por conta do disco.
      if (filasGravando.current.has(fila)) return;
      filasGravando.current.add(fila);
      setFalhaAoGravar(null);
      setGravacoes((atuais) => new Map(atuais).set(fila, modelo));
      void gravarEscolha(fila, modelo, definirModelo).then((resposta) => {
        filasGravando.current.delete(fila);
        setGravacoes((atuais) => {
          const restantes = new Map(atuais);
          restantes.delete(fila);
          return restantes;
        });
        if (resposta.gravado) setLista((atual) => comModeloGravado(atual, fila, modelo));
        else setFalhaAoGravar(resposta.aviso);
        // Em qualquer caso a linha termina mostrando o que o arquivo diz.
        atualizarLista();
      });
    },
    [atualizarLista],
  );

  // Os trabalhos continuam em qualquer aba: o evento diz a quem cada cartao pertence.
  useEffect(() => assinar(ouvirCartoes((cartao) => despachar({ tipo: "cartao", cartao }))), []);

  const iniciar = useCallback((aba: Aba, destinos: readonly Impressora[], face: FaceDoEnvio) => {
    const trecho = trechoDoEnvio(lerIntervaloDaAba(aba));
    const papel = aba.papelDeAjuste ?? null;
    // Guardado aqui: o verso compara com o que foi pedido, nao com o que a aba mostra depois.
    if (face === "frente") {
      despachar({ tipo: "frente_enviada", id: aba.id, frente: { trecho, papel } });
    }
    iniciarImpressao(
      aba.id,
      aba.documento.caminho,
      destinos.map((destino) => destino.id),
      trecho,
      face,
      papel,
      aba.controle ? idiomaAtual : null,
      copiasDaFace(aba, face),
    )
      .then((aceitas) => {
        // Quem nao foi aceito ja tinha um trabalho: o cartao diz isso, em vez de sumir em silencio.
        for (const cartao of cartoesRecusados(aba.id, destinos, aceitas)) {
          despachar({ tipo: "cartao", cartao });
        }
      })
      .catch(() => {
        for (const destino of destinos) {
          despachar({ tipo: "cartao", cartao: cartaoSemResposta(aba.id, destino) });
        }
      });
  }, []);

  const comandar = useCallback((cartao: Cartao, comando: Comando) => {
    comandarTrabalho(cartao.impressora, comando).catch(() => {
      if (comando === "cancelar") {
        // O trabalho pode ter acabado sozinho: o estado que ele publicou e o real. So o que segue
        // em "Cancelando" depois de a ultima noticia chegar perdeu o acompanhamento.
        window.setTimeout(() => {
          const vivo = cartaoAtual(estadoAtual.current.abas, cartao);
          if (vivo !== null && vivo.estado.tipo === "cancelando") {
            despachar({ tipo: "cartao", cartao: cartaoInterrompido(vivo) });
          }
        }, 800);
        return;
      }
      // A thread ja morreu: o cartao encerra sem diálogo e libera a impressora.
      despachar({ tipo: "cartao", cartao: cartaoInterrompido(cartao) });
    });
  }, []);

  const aoComandarCartao = useCallback(
    (cartao: Cartao, comando: Comando) => {
      // Numa fila real, reimprimir cancela o que resta do trecho: so depois de perguntar.
      if (comando === "reimprimir" && !cartao.simulada) {
        setCartaoAReimprimir(cartao);
        return;
      }
      comandar(cartao, comando);
    },
    [comandar],
  );

  // Cada mudanca de estado de um cartao vira um aviso, uma vez so; o painel guarda a sessao.
  useEffect(() => {
    const { novos, proximo } = cartoesComMudanca(
      assinaturasDosCartoes.current,
      estado.abas.flatMap((aba) => aba.cartoes),
    );
    assinaturasDosCartoes.current = proximo;
    const agora = Date.now();
    for (const cartao of novos) {
      const documento = estado.abas.find((aba) => aba.id === cartao.aba)?.documento.nome ?? null;
      despacharAvisos({ tipo: "publicar", cartao, agora, documento });
    }
  }, [estado.abas]);

  // Com o painel aberto, o que chega ja foi visto: a contagem de nao lidos zera.
  const quantidadeDeAvisos = avisos.itens.length;
  useEffect(() => {
    if (painelAvisos) despacharAvisos({ tipo: "ler_todos" });
  }, [painelAvisos, quantidadeDeAvisos]);

  // Uma contagem por vez: varios PDFs grandes abertos juntos nao disputam a memoria.
  const abas = estado.abas;
  useEffect(() => {
    for (const aba of abas) {
      if (aba.paginas.estado !== "contando" || contagensPedidas.current.has(aba.id)) continue;
      contagensPedidas.current.add(aba.id);
      const { id } = aba;
      const { caminho } = aba.documento;
      filaDeContagens.current = filaDeContagens.current.then(() =>
        contarPaginas(caminho)
          .then((total) => despachar({ tipo: "paginas_contadas", id, total }))
          .catch((falha: unknown) =>
            despachar({ tipo: "paginas_recusadas", id, motivo: mensagemDeErro(falha) }),
          ),
      );
    }
  }, [abas]);

  useEffect(() => {
    if (inicioFeito.current) return;
    inicioFeito.current = true;
    caminhosIniciais()
      .then(abrirCaminhos)
      .catch((falha: unknown) => setErro(mensagemDeErro(falha)));
  }, [abrirCaminhos]);

  useEffect(
    () =>
      assinar(
        getCurrentWebview().onDragDropEvent((evento) => {
          // Com o dialogo aberto, nada abre por tras dele.
          if (confirmandoAgora.current) {
            setArrastando(false);
            return;
          }
          const dados = evento.payload;
          if (dados.type === "enter" || dados.type === "over") {
            setArrastando(true);
            return;
          }
          setArrastando(false);
          if (dados.type === "drop") {
            limitarLote(dados.paths)
              .then(abrirCaminhos)
              .catch((falha: unknown) => setErro(mensagemDeErro(falha)));
          }
        }),
      ),
    [abrirCaminhos],
  );

  // Vale para o x da barra e para Alt+F4: os dois chegam aqui como pedido de fechamento.
  const ouvirFechamento = deveOuvirFechamentoJanela(estado);
  useEffect(() => {
    if (!ouvirFechamento) return undefined;
    return assinar(
      getCurrentWindow().onCloseRequested((evento) => {
        // Sem este aviso o backend destruiria a janela com o dialogo aberto.
        avisarPedidoDeFechamentoRecebido().catch(() => undefined);
        if (decidirFechamentoJanela(estadoAtual.current) === "confirmar") {
          evento.preventDefault();
          despachar({ tipo: "pedir_fechamento_janela" });
        }
      }),
    );
  }, [ouvirFechamento]);

  useEffect(() => {
    const aoPressionar = (evento: KeyboardEvent) => {
      if (deveBloquearTecla(evento)) {
        evento.preventDefault();
        return;
      }
      const acao = interpretarAtalho(evento);
      if (!acao) return;
      evento.preventDefault();
      // Com o dialogo aberto, nenhum atalho age por tras dele; manter a tecla presa so repete a troca.
      if (confirmando || (evento.repeat && acao !== "trocar")) return;
      switch (acao) {
        case "abrir":
          void abrirPeloSeletor();
          break;
        case "fechar":
          if (estado.ativa !== null) pedirFechamento(estado.ativa);
          break;
        case "trocar":
          despachar({ tipo: "trocar" });
          break;
        case "lateral":
          setLateralAberta((aberta) => !aberta);
          break;
      }
    };
    window.addEventListener("keydown", aoPressionar, true);
    return () => window.removeEventListener("keydown", aoPressionar, true);
  }, [abrirPeloSeletor, confirmando, estado.ativa, pedirFechamento]);

  useEffect(() => {
    const semMenu = (evento: Event) => {
      if (!liberaMenuDeContexto(evento.target)) evento.preventDefault();
    };
    window.addEventListener("contextmenu", semMenu);
    return () => window.removeEventListener("contextmenu", semMenu);
  }, []);

  const quantidadeDeAbas = estado.abas.length;
  useEffect(() => {
    if (focarDepoisDeFechar.current && !confirmando) {
      focarDepoisDeFechar.current = false;
      focarAbaAtiva();
    }
  }, [confirmando, quantidadeDeAbas]);

  const soltarContagens = useRef<() => void>(() => {});
  // Compilado apenas no modo e2e: o bundle normal nao contem esta porta de entrada.
  useEffect(() => {
    if (import.meta.env.MODE !== "e2e") return;
    const alvo = window as unknown as {
      __plotsE2e?: {
        marcarImpressao(emAndamento: boolean): void;
        aplicarCartao(cartao: Cartao): void;
        abaAtiva(): number | null;
        definirLista(nova: ListaImpressoras): void;
        abrir(caminho: string): void;
        definirLendo(lendo: boolean): void;
        reterContagens(): void;
        soltarContagens(): void;
      };
    };
    alvo.__plotsE2e = {
      definirLista: setLista,
      aplicarCartao(cartao) {
        despachar({ tipo: "cartao", cartao });
      },
      abaAtiva() {
        return estado.ativa;
      },
      abrir(caminho) {
        void abrirCaminhos({ caminhos: [caminho], cortado: false });
      },
      definirLendo: setLendoLista,
      reterContagens() {
        filaDeContagens.current = filaDeContagens.current.then(
          () => new Promise<void>((soltar) => (soltarContagens.current = soltar)),
        );
      },
      soltarContagens() {
        soltarContagens.current();
      },
      marcarImpressao(emAndamento) {
        if (estado.ativa !== null) {
          despachar({ tipo: "marcar_impressao", id: estado.ativa, emAndamento });
        }
      },
    };
  }, [estado.ativa, abrirCaminhos]);

  const ocupadas = impressorasOcupadas(estado.abas);

  // A frente repetida e o verso sempre perguntam: papel gasto a toa ou maco errado nao tem volta.
  const prepararEnvio = (
    aba: Aba,
    destinos: readonly Impressora[],
    face: FaceDoEnvio,
    espera?: readonly EsperaDoRodape[],
  ) => {
    const repetida = face === "frente" && quemJaSaiu(aba, destinos).length > 0;
    const sobreErro = face === "frente" && quemDeuErro(aba, destinos).length > 0;
    if (
      face === "frente" &&
      !precisaConfirmar(destinos) &&
      !aba.papelDeAjuste &&
      !repetida &&
      !sobreErro
    ) {
      iniciar(aba, destinos, face);
      return;
    }
    preparandoEnvio.current = true;
    const leitura = lerIntervaloDaAba(aba);
    const trecho = trechoDoEnvio(leitura);
    const papel = aba.papelDeAjuste ?? null;
    // O verso em espera manda a passada 2 do lote ja planejado: a confirmacao le o que foi
    // enviado, nao a aba como ela esta agora.
    const emEspera = face === "verso" && (espera?.length ?? 0) > 0;
    const plano = emEspera ? planoDoVerso(aba, (espera ?? []).map((item) => item.cartao)) : null;
    resumoDoEnvio(
      aba.documento.caminho,
      // O plano pode trazer `null` de proposito: e o documento inteiro, nao um valor faltando.
      plano === null ? trecho : plano.trecho,
      destinos.map((destino) => destino.id),
      plano === null ? papel : plano.papel,
      plano === null ? copiasDaFace(aba, face) : plano.copias,
    )
      .then((resumo) => {
        const papelAjustado = textoDoPapelAjustado(plano === null ? aba.papelDeAjuste : plano.papel, resumo.escalas);
        if (face === "verso") {
          const marcadas = lista.impressoras.filter((item) => aba.impressorasMarcadas.includes(item.id));
          setPedidoDeEnvio({
            conteudo: conteudoDoVerso(
              destinos,
              marcadas.filter((item) => !destinos.includes(item) && item.modo === "automatico"),
              resumo,
              plano?.leitura ?? leitura,
              papelAjustado,
              mudouDesdeAFrente(aba.frenteEnviada, trecho, papel),
              idiomaAtual,
              plano === null
                ? { frente: copiasDaFace(aba, "frente"), verso: copiasDaFace(aba, "verso") }
                : { frente: plano.copias, verso: plano.copias },
              quemDeuErro(aba, destinos).length > 0,
            ),
            aba,
            destinos,
            face,
            espera,
          });
          return;
        }
        const notasDaFace = notasDaFrente(aba, destinos, resumo);
        const { linhas, notas } = linhasDoEnvio(destinos, resumo, undefined, undefined, {
          papel: papelAjustado,
          copias: copiasDaFace(aba, face),
          notasDoMeio: [
            ...notasDasOpcoes({
              controle: aba.controle,
              comparacao: resumo.comparacao,
              destinos,
              fluxos: resumo.fluxos,
            }),
            ...notasDaFace.depois,
          ],
        });
        setPedidoDeEnvio({
          conteudo: conteudoDoEnvio(
            [
              ...linhasComTrecho(linhas, leitura),
              ...(copiasDaFace(aba, face) > 1
                ? [traduzir(idiomaAtual, "copias.linha", { n: String(copiasDaFace(aba, face)) })]
                : []),
            ],
            [...notasDaFace.antes, ...notas],
            destinos.some((destino) => destino.modo === "automatico"),
            destinos.every((destino) => destino.tipo === "simulada"),
          ),
          aba,
          destinos,
          face,
        });
      })
      .catch((falha: unknown) => setErro(mensagemDeErro(falha)))
      .finally(() => {
        preparandoEnvio.current = false;
      });
  };

  const marcadasDaAbaAtiva =
    abaAtiva === null
      ? []
      : lista.impressoras.filter((item) => abaAtiva.impressorasMarcadas.includes(item.id));
  const faceDaAbaAtiva = abaAtiva === null ? "frente" : faceDoEnvio(abaAtiva, marcadasDaAbaAtiva);
  // Quem espera a face escolhida (proximo trecho, ou o maco) recebe o comando pelo mesmo botao.
  const esperas = esperasDoRodape(abaAtiva, faceDaAbaAtiva, marcadasDaAbaAtiva);

  const aoImprimir = () => {
    if (abaAtiva === null || preparandoEnvio.current || !copiasValidas) return;
    // O portao e uma decisao pura: verso em espera confirma, frente em espera comanda direto.
    const acao = acaoDoEnvio(esperas);
    if (acao.tipo === "confirmar") {
      prepararEnvio(
        abaAtiva,
        acao.itens.map((item) => item.impressora),
        "verso",
        acao.itens,
      );
      return;
    }
    if (acao.tipo === "comandar") {
      for (const item of acao.itens) comandar(item.cartao, item.comando);
      return;
    }
    const marcadas = lista.impressoras.filter((item) => abaAtiva.impressorasMarcadas.includes(item.id));
    const { destinos, desabilitado } = envioComFace(
      envioComIntervalo(estadoDoEnvio(abaAtiva, lista.impressoras, ocupadas), abaAtiva),
      abaAtiva,
      lista.impressoras,
      analiseDoDocumento,
    );
    if (desabilitado) return;
    prepararEnvio(abaAtiva, destinos, faceDoEnvio(abaAtiva, marcadas));
  };

  const aoAlternarImpressora = (impressora: string) => {
    if (idAtiva !== null) despachar({ tipo: "alternar_impressora", id: idAtiva, impressora });
  };

  const aoDefinirIntervalo = (intervalo: RascunhoIntervalo | null) => {
    if (idAtiva !== null) despachar({ tipo: "definir_intervalo", id: idAtiva, intervalo });
  };

  const aoConfirmar = () => {
    if (!estado.confirmandoJanela) {
      // Os trabalhos da aba param de ser acompanhados; o que ja esta na fila continua la.
      if (estado.aguardandoConfirmacao !== null) {
        soltarTrabalhos(estado.aguardandoConfirmacao).catch(() => undefined);
      }
      despachar({ tipo: "confirmar_fechamento" });
      return;
    }
    // A janela some aqui. Se o sistema recusar, o dialogo fecha e tudo segue como estava.
    getCurrentWindow()
      .destroy()
      .catch(() => despachar({ tipo: "cancelar_fechamento" }));
  };

  const impressoraDasOpcoes = lista.impressoras.find((item) => item.nome === filaDasOpcoes);
  const nomeDasOpcoes = impressoraDasOpcoes ? nomeDeExibicao(impressoraDasOpcoes) : "";

  return (
    <div className={styles.app}>
      <TitleBar
        abas={estado.abas}
        ativa={estado.ativa}
        lateralAberta={lateralAberta}
        aoAlternarLateral={() => setLateralAberta((aberta) => !aberta)}
        aoAtivar={(id) => despachar({ tipo: "ativar", id })}
        aoFechar={pedirFechamento}
        aoAbrir={() => void abrirPeloSeletor()}
        aoAbrirHistorico={() => {
          setFalhaAoApagarHistorico(false);
          setHistorico("lista");
        }}
        naoLidos={contarNaoLidos(avisos.itens)}
        aoAbrirAvisos={() => setPainelAvisos(true)}
        idLateral={ID_LATERAL}
        idPainel={ID_AREA_DOCUMENTO}
      />
      <div className={styles.corpo}>
        {lateralAberta && (
          <Sidebar
            aba={abaAtiva}
            impressoras={lista.impressoras}
            avisos={lista.avisos}
            falhaAoGravar={falhaAoGravar}
            lendo={lendoLista}
            gravacoes={gravacoes}
            preferencias={preferencias}
            aoAbrirOpcoes={(fila, salvo) => {
              setAjusteDasOpcoes(salvo);
              setFilaDasOpcoes(fila);
            }}
            aoAtualizar={() => {
              setFalhaAoGravar(null);
              atualizarLista();
            }}
            aoEscolherModelo={aoEscolherModelo}
            aoPedirSubstituicao={(fila, modelo) => setSubstituicao({ fila, modelo })}
            aoAlternar={aoAlternarImpressora}
            aoDefinirIntervalo={aoDefinirIntervalo}
            aoCalibrar={setACalibrar}
            ocupadas={ocupadas}
            aoImprimir={aoImprimir}
            documento={analiseDoDocumento}
            blocoAjuste={
              <BlocoAjuste
                aba={abaAtiva}
                semMarca={envioSemMarca(
                  abaAtiva?.controle === true,
                  lista.impressoras.filter((item) => abaAtiva?.impressorasMarcadas.includes(item.id)),
                  analiseDoDocumento,
                )}
                aoDefinirPapel={(papel) => {
                  if (idAtiva !== null) despachar({ tipo: "definir_papel_ajuste", id: idAtiva, papel });
                }}
                aoDefinirControle={(ligado) => {
                  if (idAtiva !== null) despachar({ tipo: "definir_controle", id: idAtiva, ligado });
                }}
              />
            }
            blocoGrupos={<BlocoGrupos aba={abaAtiva} grupos={grupos} />}
            campoCopias={
              abaAtiva !== null ? (
                <CampoCopias
                  key={abaAtiva.id}
                  face={faceDaAbaAtiva}
                  valor={copiasDaFace(abaAtiva, faceDaAbaAtiva)}
                  aoDefinir={(valor) => {
                    if (idAtiva !== null)
                      despachar({ tipo: "definir_copias", id: idAtiva, face: faceDaAbaAtiva, valor });
                  }}
                  aoMudarValidade={setCopiasValidas}
                />
              ) : null
            }
            copiasValidas={copiasValidas}
            esperasDoRodape={esperas}
            blocoFace={
              abaAtiva !== null &&
              faceDisponivel(
                lista.impressoras.filter((item) => abaAtiva.impressorasMarcadas.includes(item.id)),
              ) ? (
                <BlocoFace
                  face={faceDaAbaAtiva}
                  semVerso={!temVersoDaAba(abaAtiva)}
                  dicaDoEstado={dicaDaFace(abaAtiva, marcadasDaAbaAtiva)}
                  aoDefinir={(face) => {
                    if (idAtiva !== null) despachar({ tipo: "definir_face", id: idAtiva, face });
                  }}
                />
              ) : null
            }
          />
        )}
        <div className={styles.areaDocumento}>
          <AvisosDoTopo
            itens={avisos.itens}
            oculto={confirmando}
            aoFechar={(id) => despacharAvisos({ tipo: "fechar", id })}
          />
          <DocumentArea
            documento={abaAtiva?.documento ?? null}
            idAbaAtiva={estado.ativa}
            erro={erro}
            recusa={abaAtiva?.paginas.estado === "recusado" ? abaAtiva.paginas.motivo : null}
            arrastando={arrastando}
            aoDispensarErro={() => setErro(null)}
            aoAbrir={() => void abrirPeloSeletor()}
          />
        </div>
      </div>
      <CalibracaoDialog
        impressora={aCalibrar}
        aoFechar={(gravou) => {
          setACalibrar(null);
          if (gravou) atualizarLista();
        }}
      />
      <OpcoesDaImpressoraDialog
        aberto={filaDasOpcoes !== null}
        fila={filaDasOpcoes}
        nome={nomeDasOpcoes}
        temAjusteSalvo={ajusteDasOpcoes}
        aoFechar={() => setFilaDasOpcoes(null)}
        aoAbrirEngrenagem={() => {
          const fila = filaDasOpcoes;
          setFilaDasOpcoes(null);
          if (fila !== null) preferencias.abrir(fila);
        }}
        aoAplicar={() => preferencias.recarregar()}
        aoFilaSumiu={() => {
          preferencias.recarregar();
          atualizarLista();
        }}
      />
      <PainelDeAvisos
        aberto={
          painelAvisos &&
          confirmacao === null &&
          cartaoACancelar === null &&
          cartaoAReimprimir === null
        }
        itens={avisos.itens}
        cartoesVivos={estado.abas.flatMap((aba) => aba.cartoes)}
        aoFechar={() => setPainelAvisos(false)}
        aoComandar={comandar}
        aoPedirCancelar={setCartaoACancelar}
        aoPedirReimprimir={(cartao) => aoComandarCartao(cartao, "reimprimir")}
      />
      <HistoricoDialog
        aberto={confirmacao === null && historico === "lista"}
        impressoras={lista.impressoras}
        falhaAoApagar={falhaAoApagarHistorico}
        versao={versaoDoHistorico}
        aoFechar={() => setHistorico(null)}
        aoPedirApagar={() => setHistorico("apagar")}
      />
      <ConfirmDialog
        conteudo={confirmacao === null && historico === "apagar" ? conteudoDoApagarHistorico() : null}
        aoCancelar={() => setHistorico("lista")}
        aoConfirmar={() => {
          apagarHistorico()
            .then(() => setFalhaAoApagarHistorico(false))
            .catch(() => setFalhaAoApagarHistorico(true))
            .finally(() => {
              setVersaoDoHistorico((versao) => versao + 1);
              setHistorico("lista");
            });
        }}
      />
      <ConfirmDialog
        conteudo={confirmacao}
        aoCancelar={() => despachar({ tipo: "cancelar_fechamento" })}
        aoConfirmar={aoConfirmar}
      />
      {/* O pedido de fechar a janela passa na frente: dois dialogos nunca ficam empilhados. */}
      <ConfirmDialog
        conteudo={
          confirmacao === null && substituicao !== null ? conteudoDaSubstituicao(substituicao) : null
        }
        aoCancelar={() => setSubstituicao(null)}
        aoConfirmar={() => {
          if (substituicao !== null) aoEscolherModelo(substituicao.fila, substituicao.modelo);
          setSubstituicao(null);
        }}
      />
      <ConfirmDialog
        conteudo={confirmacao === null ? (pedidoDeEnvio?.conteudo ?? null) : null}
        aoCancelar={() => setPedidoDeEnvio(null)}
        aoConfirmar={() => {
          if (pedidoDeEnvio?.espera !== undefined) {
            for (const item of pedidoDeEnvio.espera) comandar(item.cartao, item.comando);
          } else if (pedidoDeEnvio !== null) {
            iniciar(pedidoDeEnvio.aba, pedidoDeEnvio.destinos, pedidoDeEnvio.face);
          }
          setPedidoDeEnvio(null);
        }}
      />
      <ConfirmDialog
        conteudo={
          confirmacao === null && cartaoACancelar !== null
            ? aindaPodeCancelar(cartaoAtual(estado.abas, cartaoACancelar))
              ? conteudoDoCancelamento(nomeDoCartao(cartaoACancelar), cartaoACancelar.simulada)
              : conteudoDoCancelamentoJaTerminado(nomeDoCartao(cartaoACancelar))
            : null
        }
        aoCancelar={() => setCartaoACancelar(null)}
        aoConfirmar={() => {
          if (cartaoACancelar !== null) {
            const vivo = cartaoAtual(estado.abas, cartaoACancelar);
            // O trabalho pode ter terminado com o dialogo aberto: ai nao ha o que pedir.
            if (vivo !== null && aindaPodeCancelar(vivo)) {
              despachar({ tipo: "cartao", cartao: { ...vivo, estado: { tipo: "cancelando" } } });
              comandar(vivo, "cancelar");
            }
          }
          setCartaoACancelar(null);
        }}
      />
      <ConfirmDialog
        conteudo={
          confirmacao === null && cartaoAReimprimir !== null
            ? conteudoDaReimpressao(nomeDoCartao(cartaoAReimprimir), cartaoAReimprimir)
            : null
        }
        aoCancelar={() => setCartaoAReimprimir(null)}
        aoConfirmar={() => {
          if (cartaoAReimprimir !== null) comandar(cartaoAReimprimir, "reimprimir");
          setCartaoAReimprimir(null);
        }}
      />
      <ConfirmDialog
        conteudo={
          confirmacao === null && grupos.aApagar !== null ? conteudoDoApagarGrupo(grupos.aApagar) : null
        }
        aoCancelar={grupos.cancelarApagar}
        aoConfirmar={grupos.confirmarApagar}
      />
    </div>
  );
}
