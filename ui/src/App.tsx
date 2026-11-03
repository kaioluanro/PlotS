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
import { BlocoModo } from "./components/BlocoModo";
import { CalibrarDialog } from "./components/CalibrarDialog";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { DocumentArea, ID_AREA_DOCUMENTO } from "./components/DocumentArea";
import { ID_LATERAL, Sidebar } from "./components/Sidebar";
import { focarAbaAtiva, TitleBar } from "./components/TitleBar";
import {
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
  conteudoDaDivisao,
  envioComDivisao,
  conteudoDaReimpressao,
  conteudoDaSubstituicao,
  conteudoDoApagarGrupo,
  conteudoDoCancelamento,
  conteudoDoEnvio,
  divisaoDoResumo,
  type ConteudoConfirmacao,
  type PedidoDeSubstituicao,
} from "./confirmacao";
import { mensagemDeErro } from "./erros";
import { useGrupos } from "./grupos-uso";
import { t } from "./i18n";
import {
  comModeloGravado,
  gravarEscolha,
  liberaMenuDeContexto,
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
import {
  decidirFechamentoJanela,
  deveOuvirFechamentoJanela,
  estadoInicial,
  modoDoEnvio,
  reduzirAbas,
  type Aba,
  type DivisaoDoEnvio,
  type ModoDistribuicao,
} from "./tabs";
import {
  comandarTrabalho,
  iniciarImpressao,
  ouvirCartoes,
  resumoDoEnvio,
  soltarTrabalhos,
} from "./trabalhos";

/** O que o envio manda, alem da aba e dos destinos. No espelho, nao ha lotes a repartir. */
interface PlanoDoEnvio {
  modo: ModoDistribuicao;
  lotes: number;
  /** Paginas do documento na confirmacao: se o arquivo mudar, nada sai. */
  paginas: number;
  divisao: DivisaoDoEnvio | null;
}

const ENVIO_EM_ESPELHO: PlanoDoEnvio = { modo: "espelho", lotes: 0, paginas: 0, divisao: null };

export function App() {
  const [estado, despachar] = useReducer(reduzirAbas, estadoInicial);
  const [lateralAberta, setLateralAberta] = useState(true);
  const [erro, setErro] = useState<string | null>(null);
  const [arrastando, setArrastando] = useState(false);
  const seletorAberto = useRef(false);
  const inicioFeito = useRef(false);
  const focarDepoisDeFechar = useRef(false);

  const confirmacao = conteudoDaConfirmacao(estado);
  const [filaACalibrar, setFilaACalibrar] = useState<string | null>(null);
  const [lista, setLista] = useState<ListaImpressoras>({ impressoras: [], avisos: [] });
  const [lendoLista, setLendoLista] = useState(true);
  const [gravacoes, setGravacoes] = useState<ReadonlyMap<string, Modelo>>(new Map());
  const [falhaAoGravar, setFalhaAoGravar] = useState<Aviso | null>(null);
  const [substituicao, setSubstituicao] = useState<PedidoDeSubstituicao | null>(null);
  const [pedidoDeEnvio, setPedidoDeEnvio] = useState<{
    conteudo: ConteudoConfirmacao;
    aba: Aba;
    destinos: readonly Impressora[];
    plano: PlanoDoEnvio;
  } | null>(null);
  const grupos = useGrupos(despachar);
  const [cartaoACancelar, setCartaoACancelar] = useState<Cartao | null>(null);
  const [cartaoAReimprimir, setCartaoAReimprimir] = useState<Cartao | null>(null);
  const preparandoEnvio = useRef(false);
  const filasGravando = useRef(new Set<string>());
  const ultimaLeitura = useRef(0);
  const contagensPedidas = useRef(new Set<number>());
  const filaDeContagens = useRef<Promise<void>>(Promise.resolve());
  // Qualquer dialogo aberto impede que atalhos e arrastos ajam por tras dele.
  const confirmando =
    confirmacao !== null ||
    filaACalibrar !== null ||
    substituicao !== null ||
    pedidoDeEnvio !== null ||
    cartaoACancelar !== null ||
    cartaoAReimprimir !== null ||
    grupos.aApagar !== null;
  const abaAtiva = estado.abas.find((aba) => aba.id === estado.ativa) ?? null;
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

  const iniciar = useCallback((aba: Aba, destinos: readonly Impressora[], plano: PlanoDoEnvio) => {
    // Antes de pedir: o primeiro cartao chega durante o pedido e ja precisa dos numeros dos lotes.
    despachar({ tipo: "divisao_enviada", id: aba.id, divisao: plano.divisao });
    iniciarImpressao(
      aba.id,
      aba.documento.caminho,
      aba.tamanhoLote,
      destinos.map((destino) => destino.id),
      trechoDoEnvio(lerIntervaloDaAba(aba)),
      plano.modo,
      plano.lotes,
      plano.paginas,
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
      // A thread ja morreu: o cartao encerra sem diálogo e libera a impressora.
      despachar({ tipo: "cartao", cartao: cartaoInterrompido(cartao) });
    });
  }, []);

  const aoComandarCartao = useCallback(
    (cartao: Cartao, comando: Comando) => {
      // Numa fila real, reimprimir cancela o que resta do lote: so depois de perguntar.
      if (comando === "reimprimir" && !cartao.simulada) {
        setCartaoAReimprimir(cartao);
        return;
      }
      comandar(cartao, comando);
    },
    [comandar],
  );

  const dispensar = useCallback((cartao: Cartao) => {
    despachar({ tipo: "dispensar_cartao", id: cartao.aba, impressora: cartao.impressora });
  }, []);

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
        definirDivisao(divisao: DivisaoDoEnvio | null): void;
        abrir(caminho: string): void;
        definirLendo(lendo: boolean): void;
        reterContagens(): void;
        soltarContagens(): void;
      };
    };
    alvo.__plotsE2e = {
      definirLista: setLista,
      definirDivisao(divisao) {
        if (estado.ativa !== null) {
          despachar({ tipo: "divisao_enviada", id: estado.ativa, divisao });
        }
      },
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

  const aoImprimir = () => {
    const { destinos, desabilitado } = envioComDivisao(
      envioComIntervalo(estadoDoEnvio(abaAtiva, lista.impressoras, ocupadas), abaAtiva),
      abaAtiva,
      lista.impressoras,
      ocupadas,
    );
    if (abaAtiva === null || desabilitado || preparandoEnvio.current) return;
    const modo = modoDoEnvio(abaAtiva.modo, destinos.length);
    // A divisao sempre pergunta, mesmo simulada: e la que o usuario le como juntar os macos.
    if (modo === "espelho" && !precisaConfirmar(destinos)) {
      iniciar(abaAtiva, destinos, ENVIO_EM_ESPELHO);
      return;
    }
    const aba = abaAtiva;
    preparandoEnvio.current = true;
    const leitura = lerIntervaloDaAba(aba);
    resumoDoEnvio(
      aba.documento.caminho,
      aba.tamanhoLote,
      trechoDoEnvio(leitura),
      modo,
      destinos.map((destino) => destino.id),
    )
      .then((resumo) => {
        if (modo === "espelho") {
          const { linhas, notas } = linhasDoEnvio(destinos, resumo);
          setPedidoDeEnvio({
            conteudo: conteudoDoEnvio(
              linhasComTrecho(linhas, leitura),
              notas,
              destinos.some((destino) => destino.modo === "automatico"),
              destinos.every((destino) => destino.tipo === "simulada"),
            ),
            aba,
            destinos,
            plano: ENVIO_EM_ESPELHO,
          });
          return;
        }
        // Quem ficou sem lote so aparece na confirmacao: nao recebe trabalho nem cartao.
        const partes = divisaoDoResumo(destinos, resumo).filter((parte) => parte.lotes.length > 0);
        setPedidoDeEnvio({
          conteudo: conteudoDaDivisao(destinos, resumo, leitura),
          aba,
          destinos: destinos.filter((destino) =>
            partes.some((parte) => parte.impressora === destino.id),
          ),
          plano: {
            modo,
            lotes: resumo.lotes,
            paginas: resumo.paginasDoDocumento,
            divisao: { partes, papeis: resumo.papeis },
          },
        });
      })
      .catch((falha: unknown) => setErro(mensagemDeErro(falha)))
      .finally(() => {
        preparandoEnvio.current = false;
      });
  };

  const aoAlternarImpressora = (impressora: string) => {
    if (idAtiva !== null) despachar({ tipo: "alternar_impressora", id: idAtiva, impressora });
  };

  const aoDefinirLote = (tamanho: number) => {
    if (idAtiva !== null) despachar({ tipo: "definir_lote", id: idAtiva, tamanho });
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
            aoAtualizar={() => {
              setFalhaAoGravar(null);
              atualizarLista();
            }}
            aoEscolherModelo={aoEscolherModelo}
            aoPedirSubstituicao={(fila, modelo) => setSubstituicao({ fila, modelo })}
            aoAlternar={aoAlternarImpressora}
            aoDefinirLote={aoDefinirLote}
            aoDefinirIntervalo={aoDefinirIntervalo}
            aoCalibrar={setFilaACalibrar}
            ocupadas={ocupadas}
            aoImprimir={aoImprimir}
            aoComandar={aoComandarCartao}
            aoPedirCancelar={setCartaoACancelar}
            aoDispensar={dispensar}
            blocoModo={
              <BlocoModo
                aba={abaAtiva}
                grupos={grupos}
                aoDefinirModo={(modo) => {
                  if (idAtiva !== null) despachar({ tipo: "definir_modo", id: idAtiva, modo });
                }}
              />
            }
          />
        )}
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
      <CalibrarDialog fila={filaACalibrar} aoFechar={() => setFilaACalibrar(null)} />
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
          if (pedidoDeEnvio !== null) {
            iniciar(pedidoDeEnvio.aba, pedidoDeEnvio.destinos, pedidoDeEnvio.plano);
          }
          setPedidoDeEnvio(null);
        }}
      />
      <ConfirmDialog
        conteudo={
          confirmacao === null && cartaoACancelar !== null
            ? conteudoDoCancelamento(nomeDoCartao(cartaoACancelar), cartaoACancelar.simulada)
            : null
        }
        aoCancelar={() => setCartaoACancelar(null)}
        aoConfirmar={() => {
          if (cartaoACancelar !== null) comandar(cartaoACancelar, "cancelar");
          setCartaoACancelar(null);
        }}
      />
      <ConfirmDialog
        conteudo={
          confirmacao === null && cartaoAReimprimir !== null
            ? conteudoDaReimpressao(
                nomeDoCartao(cartaoAReimprimir),
                cartaoAReimprimir.lote,
                cartaoAReimprimir.total,
                cartaoAReimprimir,
              )
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
