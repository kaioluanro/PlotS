import { CircleAlert, RotateCcw } from "lucide-react";
import { useId, useRef } from "react";
import { t } from "../i18n";
import { ATRIBUTO_MENU_DE_CONTEXTO } from "../impressoras";
import {
  ariaDoCampo,
  leituraDoErro,
  limparCampo,
  lerIntervaloDaAba,
  motivoDosCamposFechados,
  passoNoCampo,
  textoDoErro,
  valoresDosCampos,
  type CampoIntervalo,
  type RascunhoIntervalo,
} from "../intervalo";
import type { ControleDaEspera } from "../intervalo-espera";
import type { Aba } from "../tabs";
import estilos from "./BlocoIntervalo.module.css";
import styles from "./Sidebar.module.css";

interface Props {
  aba: Aba | null;
  espera: ControleDaEspera;
  aoDefinir: (intervalo: RascunhoIntervalo | null) => void;
}

/** De qual a qual pagina imprimir. Os campos aceitam qualquer texto; o erro explica, sem travar. */
export function BlocoIntervalo({ aba, espera, aoDefinir }: Props) {
  const idRotulo = useId();
  const idErro = useId();
  const primeiro = useRef<HTMLInputElement>(null);
  const aoEntrar = useRef<RascunhoIntervalo | null>(null);
  const leitura = lerIntervaloDaAba(aba);
  const valores = valoresDosCampos(aba);
  const fechado = motivoDosCamposFechados(aba);
  const naTela = leituraDoErro(leitura, espera.estado);
  const erro = naTela === null ? null : textoDoErro(naTela);
  const total = aba?.paginas.estado === "conhecido" ? aba.paginas.total : 0;
  const errados = naTela?.campos ?? [];
  const digitar = (proximo: RascunhoIntervalo) => {
    espera.aoDigitar(proximo);
    aoDefinir(proximo);
  };
  const concluir = (nome: CampoIntervalo) => {
    espera.aoConcluir();
    const limpo = limparCampo(valores[nome]);
    if (limpo !== valores[nome]) aoDefinir({ ...valores, [nome]: limpo });
  };

  const campo = (nome: CampoIntervalo) => {
    const invalido = errados.includes(nome);
    const atual = valores[nome];
    return (
      <input
        ref={nome === "inicio" ? primeiro : undefined}
        className={invalido ? `${styles.campoLote} ${styles.campoInvalido}` : styles.campoLote}
        type="text"
        inputMode="numeric"
        role="spinbutton"
        autoComplete="off"
        spellCheck={false}
        title={fechado ?? undefined}
        disabled={fechado !== null}
        value={atual}
        aria-label={t(nome === "inicio" ? "intervalo.primeira" : "intervalo.ultima")}
        aria-valuemin={1}
        {...ariaDoCampo(atual, total)}
        aria-invalid={invalido}
        aria-describedby={invalido ? idErro : undefined}
        {...{ [ATRIBUTO_MENU_DE_CONTEXTO]: "" }}
        onFocus={() => {
          aoEntrar.current = aba?.intervalo ?? null;
        }}
        onChange={(evento) => digitar({ ...valores, [nome]: evento.target.value })}
        onBlur={() => concluir(nome)}
        onKeyDown={(evento) => {
          if (evento.key === "Enter") concluir(nome);
          if (evento.key === "Escape") {
            espera.aoConcluir();
            aoDefinir(aoEntrar.current);
          }
          if (evento.key === "ArrowUp" || evento.key === "ArrowDown") {
            evento.preventDefault();
            digitar(passoNoCampo(valores, nome, evento.key === "ArrowUp" ? 1 : -1, total));
          }
        }}
      />
    );
  };

  return (
    <div className={estilos.bloco}>
      <div
        className={`${styles.loteLinha} ${estilos.linhaPaginas}`}
        role="group"
        aria-labelledby={idRotulo}
      >
        <span id={idRotulo} className={styles.rotuloLote}>
          {t("intervalo.rotulo")}
        </span>
        <span className={estilos.linha}>
          {aba?.intervalo != null && (
            <button
              type="button"
              className={estilos.inteiro}
              title={t("intervalo.inteiro")}
              aria-label={t("intervalo.inteiro")}
              onClick={() => {
                espera.aoConcluir();
                aoDefinir(null);
                // O botao some ao ser usado: o foco fica no campo que mostra o resultado.
                primeiro.current?.focus();
                aoEntrar.current = null;
              }}
            >
              <RotateCcw size={14} aria-hidden="true" />
            </button>
          )}
          <span className={estilos.campos}>
            {campo("inicio")}
            <span className={estilos.ate} aria-hidden="true">
              {t("intervalo.ate")}
            </span>
            {campo("fim")}
          </span>
        </span>
      </div>
      {erro !== null && (
        <p id={idErro} className={styles.erroLote} role="alert">
          <CircleAlert size={12} aria-hidden="true" />
          <span>{erro}</span>
        </p>
      )}
    </div>
  );
}
