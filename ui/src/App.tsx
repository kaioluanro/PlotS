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
  escolherArquivo,
  limitarLote,
} from "./backend";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { DocumentArea, ID_AREA_DOCUMENTO } from "./components/DocumentArea";
import { ID_LATERAL, Sidebar } from "./components/Sidebar";
import { focarAbaAtiva, TitleBar } from "./components/TitleBar";
import { conteudoDaConfirmacao } from "./confirmacao";
import { mensagemDeErro } from "./erros";
import { t } from "./i18n";
import { abrirLote, type LoteCaminhos } from "./lote";
import {
  decidirFechamentoJanela,
  deveOuvirFechamentoJanela,
  estadoInicial,
  reduzirAbas,
} from "./tabs";

export function App() {
  const [estado, despachar] = useReducer(reduzirAbas, estadoInicial);
  const [lateralAberta, setLateralAberta] = useState(true);
  const [erro, setErro] = useState<string | null>(null);
  const [arrastando, setArrastando] = useState(false);
  const seletorAberto = useRef(false);
  const inicioFeito = useRef(false);
  const focarDepoisDeFechar = useRef(false);

  const confirmacao = conteudoDaConfirmacao(estado);
  const confirmando = confirmacao !== null;
  const abaAtiva = estado.abas.find((aba) => aba.id === estado.ativa) ?? null;

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
    const semMenu = (evento: Event) => evento.preventDefault();
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

  // Compilado apenas no modo e2e: o bundle normal nao contem esta porta de entrada.
  useEffect(() => {
    if (import.meta.env.MODE !== "e2e") return;
    const alvo = window as unknown as { __plotsE2e?: { marcarImpressao(emAndamento: boolean): void } };
    alvo.__plotsE2e = {
      marcarImpressao(emAndamento) {
        if (estado.ativa !== null) {
          despachar({ tipo: "marcar_impressao", id: estado.ativa, emAndamento });
        }
      },
    };
  }, [estado.ativa]);

  const aoConfirmar = () => {
    if (!estado.confirmandoJanela) {
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
        {lateralAberta && <Sidebar />}
        <DocumentArea
          documento={abaAtiva?.documento ?? null}
          idAbaAtiva={estado.ativa}
          erro={erro}
          arrastando={arrastando}
          aoDispensarErro={() => setErro(null)}
          aoAbrir={() => void abrirPeloSeletor()}
        />
      </div>
      <ConfirmDialog
        conteudo={confirmacao}
        aoCancelar={() => despachar({ tipo: "cancelar_fechamento" })}
        aoConfirmar={aoConfirmar}
      />
    </div>
  );
}
