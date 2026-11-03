import { Check, ChevronDown, CircleAlert, Save, Trash2 } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import { juncaoDaAba, type JuncaoDaAba } from "../confirmacao";
import { NOME_DE_GRUPO_MAXIMO, notaValeNaSelecao, selecaoCoincide } from "../grupos";
import type { UsoDosGrupos } from "../grupos-uso";
import { t } from "../i18n";
import { abrirListaPeloTeclado, ATRIBUTO_MENU_DE_CONTEXTO, teclaAbreALista } from "../impressoras";
import type { Aba, ModoDistribuicao } from "../tabs";
import styles from "./BlocoModo.module.css";
import botoes from "./Botao.module.css";
import { OrdemDaPilha } from "./OrdemDaPilha";
import lateral from "./Sidebar.module.css";

const MODOS: readonly { valor: ModoDistribuicao; rotulo: "modo.espelho" | "modo.divisao" }[] = [
  { valor: "espelho", rotulo: "modo.espelho" },
  { valor: "divisao", rotulo: "modo.divisao" },
];

interface Props {
  aba: Aba | null;
  grupos: UsoDosGrupos;
  aoDefinirModo: (modo: ModoDistribuicao) => void;
}

/** O modo, com duas ou mais marcadas, nunca sai da tela: muda o que cada impressora recebe. Na altura curta, com cartao, so os grupos saem. */
export function BlocoModo({ aba, grupos, aoDefinirModo }: Props) {
  if (aba === null) return null;
  const marcadas = aba.impressorasMarcadas.length;
  return (
    <div className={styles.bloco} data-bloco-modo="">
      {marcadas >= 2 && <EscolhaDoModo modo={aba.modo} aoDefinir={aoDefinirModo} />}
      <LinhaDosGrupos key={aba.id} aba={aba} uso={grupos} />
    </div>
  );
}

function EscolhaDoModo({
  modo,
  aoDefinir,
}: {
  modo: ModoDistribuicao;
  aoDefinir: (modo: ModoDistribuicao) => void;
}) {
  const nome = useId();
  const idDica = useId();
  return (
    <fieldset className={styles.modo} aria-describedby={idDica}>
      <legend className={styles.rotulo}>{t("modo.distribuicao.rotulo")}</legend>
      <div className={styles.opcoes}>
        {MODOS.map(({ valor, rotulo }) => (
          <label key={valor} className={valor === modo ? `${styles.opcao} ${styles.opcaoAtiva}` : styles.opcao}>
            <input
              type="radio"
              className={lateral.ocultoVisual}
              name={nome}
              value={valor}
              checked={valor === modo}
              onChange={() => aoDefinir(valor)}
            />
            {valor === modo && <Check size={12} strokeWidth={3} aria-hidden="true" />}
            <span>{t(rotulo)}</span>
          </label>
        ))}
      </div>
      <p id={idDica} className={styles.dica} data-dica-modo="">
        {t(modo === "espelho" ? "modo.espelho.dica" : "modo.divisao.dica")}
      </p>
    </fieldset>
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
  modo: ModoDistribuicao;
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
    selecaoCoincide(escolhido, aba.impressorasMarcadas, aba.modo);
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
      setEscolhido(
        aplicado === null ? null : { nome, impressoras: aplicado.impressoras, modo: aplicado.modo },
      );
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
            if (await uso.salvar(nome, aba.impressorasMarcadas, aba.modo)) {
              setEscolhido({
                nome: nome.trim(),
                impressoras: aba.impressorasMarcadas,
                modo: aba.modo,
              });
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

/** Quando todos os trabalhos da divisao acabam: o que faltou e como juntar os macos, na ordem fisica. */
export function JuncaoDosMacos({ aba }: { aba: Aba | null }) {
  const juncao = aba === null ? null : juncaoDaAba(aba);
  if (aba === null || juncao === null) return null;
  return <TextoDaJuncao key={aba.id} id={aba.id} juncao={juncao} />;
}

function TextoDaJuncao({ id, juncao }: { id: number; juncao: JuncaoDaAba }) {
  const idTitulo = `juncao-${id}`;
  const raiz = useRef<HTMLElement>(null);
  // Fica acima dos cartoes, e a lista pode estar rolada ate o ultimo deles quando o trabalho acaba.
  // O efeito roda depois de o rodape voltar, entao a altura da lista ja e a final.
  // Com chaves: o scrollIntoView pode devolver uma promessa, e o React a trataria como limpeza.
  useEffect(() => {
    raiz.current?.scrollIntoView({ block: "start" });
  }, []);
  return (
    <section ref={raiz} className={styles.juncao} role="status" aria-labelledby={idTitulo}>
      <h3 id={idTitulo} className={lateral.tituloSecao}>
        {t("modo.juntar.titulo")}
      </h3>
      {juncao.faltas.map((falta) => (
        <p key={falta} className={styles.falta}>
          <CircleAlert size={12} className={styles.iconeFalha} aria-hidden="true" />
          <span>{falta}</span>
        </p>
      ))}
      <OrdemDaPilha ordem={juncao.pilha} />
      <p className={styles.frase}>{juncao.conferencia}</p>
    </section>
  );
}
