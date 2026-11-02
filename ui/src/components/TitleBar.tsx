import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, PanelLeftClose, PanelLeftOpen, Plus, Square, X } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { assinar } from "../backend";
import { t } from "../i18n";
import { indiceDeTabulacao, vizinhaPorSeta, type Aba } from "../tabs";
import styles from "./TitleBar.module.css";

const ATRIBUTO_ABA_ATIVA = "data-aba-ativa";
const ID_BOTAO_ABRIR = "botao-abrir-pdf";
/** Altura de uma linha de roda quando o sistema informa o giro em linhas, e nao em pixels. */
const PIXELS_POR_LINHA = 16;

export function idDaAba(id: number): string {
  return `aba-${id}`;
}

/** Devolve o foco para a aba ativa ou, sem abas, para o botao de abrir. */
export function focarAbaAtiva(): void {
  const alvo =
    document.querySelector<HTMLElement>(`[${ATRIBUTO_ABA_ATIVA}="true"]`) ??
    document.getElementById(ID_BOTAO_ABRIR);
  alvo?.focus();
}

interface Props {
  abas: readonly Aba[];
  ativa: number | null;
  lateralAberta: boolean;
  aoAlternarLateral: () => void;
  aoAtivar: (id: number) => void;
  aoFechar: (id: number) => void;
  aoAbrir: () => void;
  idLateral: string;
  idPainel: string;
}

export function TitleBar({
  abas,
  ativa,
  lateralAberta,
  aoAlternarLateral,
  aoAtivar,
  aoFechar,
  aoAbrir,
  idLateral,
  idPainel,
}: Props) {
  const [maximizada, setMaximizada] = useState(false);
  const [ocultas, setOcultas] = useState({ esquerda: false, direita: false });
  const lista = useRef<HTMLDivElement>(null);
  const haAbas = abas.length > 0;

  useEffect(() => {
    const janela = getCurrentWindow();
    const atualizar = () => {
      janela
        .isMaximized()
        .then(setMaximizada)
        .catch(() => {
          // Sem a resposta, o botao continua funcionando; so o icone pode ficar desatualizado.
        });
    };
    atualizar();
    return assinar(janela.onResized(atualizar));
  }, []);

  useEffect(() => {
    lista.current
      ?.querySelector(`[${ATRIBUTO_ABA_ATIVA}="true"]`)
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [ativa, abas.length]);

  // Marca as bordas com conteudo oculto e faz a roda vertical andar a lista na horizontal.
  useEffect(() => {
    const elemento = lista.current;
    if (!elemento) return;
    const medir = () => {
      const esquerda = elemento.scrollLeft > 0;
      const direita = elemento.scrollLeft + elemento.clientWidth < elemento.scrollWidth - 1;
      setOcultas((atual) =>
        atual.esquerda === esquerda && atual.direita === direita ? atual : { esquerda, direita },
      );
    };
    const aoRodar = (evento: WheelEvent) => {
      if (Math.abs(evento.deltaY) <= Math.abs(evento.deltaX)) return;
      evento.preventDefault();
      const escala = evento.deltaMode === WheelEvent.DOM_DELTA_LINE ? PIXELS_POR_LINHA : 1;
      elemento.scrollLeft += evento.deltaY * escala;
    };
    medir();
    const observador = new ResizeObserver(medir);
    observador.observe(elemento);
    elemento.addEventListener("scroll", medir, { passive: true });
    elemento.addEventListener("wheel", aoRodar, { passive: false });
    return () => {
      observador.disconnect();
      elemento.removeEventListener("scroll", medir);
      elemento.removeEventListener("wheel", aoRodar);
    };
  }, [haAbas, abas.length]);

  const aoTeclarNaAba = (evento: KeyboardEvent<HTMLButtonElement>, posicao: number) => {
    const vizinha = vizinhaPorSeta(abas, posicao, evento.key);
    if (!vizinha) return;
    evento.preventDefault();
    aoAtivar(vizinha.id);
    document.getElementById(idDaAba(vizinha.id))?.focus();
  };

  const janela = getCurrentWindow();
  const ignorarFalha = (promessa: Promise<void>) => {
    promessa.catch(() => {
      // O sistema recusou a acao; a janela apenas continua como esta.
    });
  };

  const classesDaLista = [
    styles.abas,
    ocultas.esquerda ? styles.ocultaEsquerda : "",
    ocultas.direita ? styles.ocultaDireita : "",
  ].join(" ");

  return (
    <header className={styles.barra} data-tauri-drag-region>
      <div className={styles.marca} data-tauri-drag-region aria-hidden="true">
        <svg viewBox="0 0 16 16" width="11" height="11">
          <rect x="2" y="5" width="9" height="10" rx="1" fill="currentColor" />
          <rect x="5.5" y="1.5" width="9" height="10" rx="1" className={styles.folhaFrente} />
        </svg>
      </div>
      <button
        type="button"
        className={styles.botaoIcone}
        aria-label={t(lateralAberta ? "barra.lateral.ocultar" : "barra.lateral.mostrar")}
        title={t(lateralAberta ? "barra.lateral.ocultar" : "barra.lateral.mostrar")}
        aria-expanded={lateralAberta}
        aria-controls={idLateral}
        onClick={aoAlternarLateral}
      >
        {lateralAberta ? <PanelLeftClose size={14} /> : <PanelLeftOpen size={14} />}
      </button>
      {haAbas && (
        <div ref={lista} className={classesDaLista} role="tablist" aria-label={t("barra.abas")}>
          {abas.map((aba, posicao) => {
            const selecionada = aba.id === ativa;
            return (
              <div
                key={aba.id}
                role="presentation"
                className={selecionada ? `${styles.aba} ${styles.abaAtiva}` : styles.aba}
              >
                <button
                  type="button"
                  role="tab"
                  id={idDaAba(aba.id)}
                  className={styles.rotuloAba}
                  aria-selected={selecionada}
                  aria-controls={idPainel}
                  tabIndex={indiceDeTabulacao(aba.id, ativa)}
                  title={aba.documento.nome}
                  {...{ [ATRIBUTO_ABA_ATIVA]: selecionada }}
                  onClick={() => aoAtivar(aba.id)}
                  onKeyDown={(evento) => aoTeclarNaAba(evento, posicao)}
                >
                  <span className={styles.textoAba}>{aba.documento.nome}</span>
                </button>
                <button
                  type="button"
                  className={styles.fecharAba}
                  aria-label={t("aba.fechar", { nome: aba.documento.nome })}
                  title={t("aba.fechar", { nome: aba.documento.nome })}
                  tabIndex={indiceDeTabulacao(aba.id, ativa)}
                  onClick={() => aoFechar(aba.id)}
                >
                  <X size={12} />
                </button>
              </div>
            );
          })}
        </div>
      )}
      <button
        type="button"
        id={ID_BOTAO_ABRIR}
        className={styles.botaoIcone}
        aria-label={t("barra.abrir")}
        title={t("barra.abrir")}
        onClick={aoAbrir}
      >
        <Plus size={14} />
      </button>
      <div className={styles.arrasto} data-tauri-drag-region />
      <div className={styles.controles}>
        <button
          type="button"
          className={styles.controle}
          aria-label={t("barra.minimizar")}
          title={t("barra.minimizar")}
          onClick={() => ignorarFalha(janela.minimize())}
        >
          <Minus size={12} />
        </button>
        <button
          type="button"
          className={styles.controle}
          aria-label={t(maximizada ? "barra.restaurar" : "barra.maximizar")}
          title={t(maximizada ? "barra.restaurar" : "barra.maximizar")}
          onClick={() => ignorarFalha(janela.toggleMaximize())}
        >
          {maximizada ? <Copy size={12} className={styles.espelhado} /> : <Square size={12} />}
        </button>
        <button
          type="button"
          className={`${styles.controle} ${styles.fecharJanela}`}
          aria-label={t("barra.fechar")}
          title={t("barra.fechar")}
          onClick={() => ignorarFalha(janela.close())}
        >
          <X size={12} />
        </button>
      </div>
    </header>
  );
}
