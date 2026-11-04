import { Check, ChevronDown, CircleAlert, Save, Trash2 } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import { NOME_DE_GRUPO_MAXIMO, notaValeNaSelecao, selecaoCoincide } from "../grupos";
import type { UsoDosGrupos } from "../grupos-uso";
import { t } from "../i18n";
import { abrirListaPeloTeclado, ATRIBUTO_MENU_DE_CONTEXTO, teclaAbreALista } from "../impressoras";
import type { Aba } from "../tabs";
import styles from "./BlocoGrupos.module.css";
import botoes from "./Botao.module.css";
import lateral from "./Sidebar.module.css";

interface Props {
  aba: Aba | null;
  grupos: UsoDosGrupos;
}

/** Os grupos salvos: um nome para uma selecao de impressoras. */
export function BlocoGrupos({ aba, grupos }: Props) {
  if (aba === null) return null;
  return (
    <div className={styles.bloco}>
      <LinhaDosGrupos key={aba.id} aba={aba} uso={grupos} />
    </div>
  );
}

/** O foco so pode entrar depois que o dialogo de apagar terminou de sair e devolveu o dele. */
function focarDepoisDoDialogo(alvo: () => HTMLElement | null, tentativas = 60) {
  const aberto = document.querySelector('[role="dialog"], [role="alertdialog"]') !== null;
  if (aberto && tentativas > 0) {
    requestAnimationFrame(() => focarDepoisDoDialogo(alvo, tentativas - 1));
    return;
  }
  // Mais um quadro: quem fecha o dialogo devolve o foco logo depois de desmontar.
  requestAnimationFrame(() => alvo()?.focus());
}

interface GrupoEscolhido {
  nome: string;
  impressoras: readonly string[];
}

function LinhaDosGrupos({ aba, uso }: { aba: Aba; uso: UsoDosGrupos }) {
  const [escolhido, setEscolhido] = useState<GrupoEscolhido | null>(null);
  const [aplicando, setAplicando] = useState<string | null>(null);
  const [nomeando, setNomeando] = useState(false);
  const botaoSalvar = useRef<HTMLButtonElement>(null);
  const seletor = useRef<HTMLSelectElement>(null);
  const voltarFoco = useRef(false);
  const idMotivo = useId();
  const semGrupos = uso.grupos.length === 0;
  const coincide =
    escolhido !== null &&
    uso.grupos.some((grupo) => grupo.nome === escolhido.nome) &&
    selecaoCoincide(escolhido, aba.impressorasMarcadas);
  // Grupo apagado, que sumiu do arquivo ou selecao mudada: a lista volta ao convite e aplica de novo.
  const valor = aplicando ?? (coincide ? escolhido.nome : "");
  const semMarcadas = aba.impressorasMarcadas.length === 0;
  const { ausentes, limparAusentes, apagado, limparApagado } = uso;

  useEffect(() => {
    if (!nomeando && voltarFoco.current) {
      voltarFoco.current = false;
      botaoSalvar.current?.focus();
    }
  }, [nomeando]);

  // As notas falam de uma selecao: outra aba ou outra marcacao ja nao e aquela.
  const notaVale = notaValeNaSelecao(ausentes, aba.id, aba.impressorasMarcadas);
  useEffect(() => {
    if (ausentes !== null && !notaVale) limparAusentes();
  }, [ausentes, notaVale, limparAusentes]);

  const apagadoVale = notaValeNaSelecao(apagado, aba.id, aba.impressorasMarcadas);
  useEffect(() => {
    if (apagado !== null && !apagadoVale) limparApagado();
  }, [apagado, apagadoVale, limparApagado]);

  // Quem so voltou a esta aba nao perde o foco: o anuncio antigo ja nao vale aqui.
  useEffect(() => {
    if (!apagadoVale) return;
    focarDepoisDoDialogo(() =>
      seletor.current && !seletor.current.disabled ? seletor.current : botaoSalvar.current,
    );
  }, [apagado, apagadoVale]);

  const fecharNome = () => {
    voltarFoco.current = true;
    uso.limparFalha();
    setNomeando(false);
  };

  const aplicar = (nome: string) => {
    if (nome === "") return;
    setAplicando(nome);
    void uso.aplicar(aba.id, nome).then((aplicado) => {
      setAplicando(null);
      setEscolhido(aplicado === null ? null : { nome, impressoras: aplicado.impressoras });
    });
  };

  const abrirNome = () => {
    uso.limparFalha();
    setNomeando(true);
  };

  return (
    <div className={styles.grupos} data-bloco-grupos="" data-nome-aberto={nomeando ? "" : undefined}>
      {nomeando ? (
        <CampoDoNome
          aoCancelar={fecharNome}
          falha={uso.falha}
          aoEditar={uso.limparFalha}
          aoSalvar={async (nome) => {
            if (await uso.salvar(nome, aba.impressorasMarcadas)) {
              setEscolhido({ nome: nome.trim(), impressoras: aba.impressorasMarcadas });
              voltarFoco.current = true;
              setNomeando(false);
            }
          }}
        />
      ) : semGrupos ? (
        <>
          <button
            ref={botaoSalvar}
            type="button"
            className={`${botoes.botao} ${botoes.secundario} ${styles.salvarGrupo}`}
            disabled={semMarcadas}
            title={semMarcadas ? t("grupo.salvar.semMarcada") : undefined}
            aria-describedby={semMarcadas ? idMotivo : undefined}
            onClick={abrirNome}
          >
            {t("grupo.salvarComoGrupo")}
          </button>
          {semMarcadas && (
            <span id={idMotivo} className={lateral.ocultoVisual}>
              {t("grupo.salvar.semMarcada")}
            </span>
          )}
        </>
      ) : (
        <div className={styles.linhaGrupo}>
          <span className={styles.caixaDoGrupo}>
            <select
              ref={seletor}
              className={`${lateral.seletor} ${styles.seletorGrupo}`}
              value={valor}
              aria-label={t("grupo.rotulo")}
              title={valor === "" ? t("grupo.rotulo") : valor}
              // As setas abrem a lista em vez de aplicar o grupo vizinho por engano.
              onKeyDown={(evento) => {
                if (teclaAbreALista(evento.key) && abrirListaPeloTeclado(evento.currentTarget)) {
                  evento.preventDefault();
                }
              }}
              onChange={(evento) => aplicar(evento.target.value)}
            >
              <option value="">{t("grupo.escolher")}</option>
              {uso.grupos.map((grupo) => (
                <option key={grupo.nome} value={grupo.nome}>
                  {grupo.nome}
                </option>
              ))}
            </select>
            <ChevronDown size={10} className={`${lateral.setaSeletor} ${styles.setaGrupo}`} aria-hidden="true" />
          </span>
          <button
            ref={botaoSalvar}
            type="button"
            className={styles.botaoIcone}
            disabled={semMarcadas}
            aria-label={t("grupo.salvar")}
            title={t(semMarcadas ? "grupo.salvar.semMarcada" : "grupo.salvar")}
            onClick={abrirNome}
          >
            <Save size={14} aria-hidden="true" />
          </button>
          <button
            type="button"
            className={styles.botaoIcone}
            disabled={valor === ""}
            aria-label={valor === "" ? t("grupo.apagar.semEscolha") : t("grupo.apagar", { nome: valor })}
            title={valor === "" ? t("grupo.apagar.semEscolha") : t("grupo.apagar", { nome: valor })}
            onClick={() => uso.pedirApagar(valor, aba.id, aba.impressorasMarcadas)}
          >
            <Trash2 size={14} aria-hidden="true" />
          </button>
        </div>
      )}
      {!nomeando && uso.falha !== null && <Falha texto={uso.falha} />}
      {uso.ignorados !== null && (
        <p className={styles.nota}>
          <CircleAlert size={12} className={styles.iconeNota} aria-hidden="true" />
          <span>{uso.ignorados}</span>
        </p>
      )}
      {notaVale && (
        <p className={styles.nota} role="status">
          <CircleAlert size={12} className={styles.iconeNota} aria-hidden="true" />
          <span>{ausentes.texto}</span>
        </p>
      )}
      {/* Fica sempre montado: o leitor de tela so anuncia o que muda dentro de uma regiao existente. */}
      <p role="status" className={styles.anuncio}>
        {apagadoVale && (
          <>
            <Check size={12} className={styles.iconeNota} aria-hidden="true" />
            <span>{apagado.texto}</span>
          </>
        )}
      </p>
    </div>
  );
}

function Falha({ texto, id }: { texto: string; id?: string }) {
  return (
    <p id={id} className={styles.falha} role="alert">
      <CircleAlert size={12} className={styles.iconeFalha} aria-hidden="true" />
      <span>{texto}</span>
    </p>
  );
}

function CampoDoNome({
  falha,
  aoSalvar,
  aoCancelar,
  aoEditar,
}: {
  falha: string | null;
  aoSalvar: (nome: string) => Promise<void>;
  aoCancelar: () => void;
  aoEditar: () => void;
}) {
  const [nome, setNome] = useState("");
  const [gravando, setGravando] = useState(false);
  const idFalha = useId();
  const campo = useRef<HTMLInputElement>(null);

  useEffect(() => {
    campo.current?.focus();
  }, []);

  const enviar = () => {
    if (gravando) return;
    setGravando(true);
    void aoSalvar(nome).finally(() => setGravando(false));
  };

  return (
    <form
      className={styles.formulario}
      onSubmit={(evento) => {
        evento.preventDefault();
        enviar();
      }}
      onKeyDown={(evento) => {
        if (evento.key === "Escape") aoCancelar();
      }}
    >
      <label className={styles.rotulo}>
        {t("grupo.nome.rotulo")}
        <input
          ref={campo}
          className={falha !== null ? `${styles.campo} ${styles.campoInvalido}` : styles.campo}
          value={nome}
          maxLength={NOME_DE_GRUPO_MAXIMO}
          autoComplete="off"
          spellCheck={false}
          aria-invalid={falha !== null}
          aria-describedby={falha !== null ? idFalha : undefined}
          {...{ [ATRIBUTO_MENU_DE_CONTEXTO]: "" }}
          onChange={(evento) => {
            setNome(evento.target.value);
            aoEditar();
          }}
        />
      </label>
      {falha !== null && <Falha texto={falha} id={idFalha} />}
      <div className={styles.acoesDoNome}>
        <button type="button" className={`${botoes.botao} ${botoes.secundario} ${styles.acao}`} onClick={aoCancelar}>
          {t("grupo.nome.cancelar")}
        </button>
        <button
          type="submit"
          className={`${botoes.botao} ${botoes.secundario} ${styles.acao}`}
          aria-busy={gravando}
        >
          {t("grupo.nome.salvar")}
        </button>
      </div>
    </form>
  );
}
