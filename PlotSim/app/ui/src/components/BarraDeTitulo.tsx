import styles from "@plots/components/TitleBar.module.css";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";
import { useEffect, useState } from "react";
import { t } from "../i18n";
import proprios from "./BarraDeTitulo.module.css";

/** A mesma barra do PlotS, com o CSS dela. As abas dela não existem aqui. */
export function BarraDeTitulo() {
  const [maximizada, setMaximizada] = useState(false);

  useEffect(() => {
    const janela = getCurrentWindow();
    const atualizar = () => {
      janela
        .isMaximized()
        .then(setMaximizada)
        .catch(() => {
          // Sem a resposta, o botão continua funcionando; só o ícone pode ficar desatualizado.
        });
    };
    atualizar();
    let ativo = true;
    let cancelar: (() => void) | null = null;
    janela
      .onResized(atualizar)
      .then((funcao) => {
        if (ativo) cancelar = funcao;
        else funcao();
      })
      .catch(() => {
        // Sem o aviso de tamanho, só o ícone pode ficar desatualizado.
      });
    return () => {
      ativo = false;
      cancelar?.();
    };
  }, []);

  const janela = getCurrentWindow();
  const ignorarFalha = (promessa: Promise<void>) => {
    promessa.catch(() => {
      // O sistema recusou a ação; a janela apenas continua como está.
    });
  };

  return (
    <header className={styles.barra} data-tauri-drag-region>
      <div className={styles.marca} data-tauri-drag-region aria-hidden="true">
        <svg viewBox="0 0 16 16" width="11" height="11">
          <rect x="2" y="5" width="9" height="10" rx="1" fill="currentColor" />
          <rect x="5.5" y="1.5" width="9" height="10" rx="1" className={styles.folhaFrente} />
        </svg>
      </div>
      <span className={proprios.nome} data-tauri-drag-region>
        PlotSim
      </span>
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
