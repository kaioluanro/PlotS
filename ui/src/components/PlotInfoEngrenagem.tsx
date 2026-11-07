import * as Dialog from "@radix-ui/react-dialog";
import { CircleAlert, LoaderCircle, Settings } from "lucide-react";
import { useEffect, useRef, useState, type RefObject } from "react";
import {
  CANAIS_DE_TINTA,
  CHAVE_DO_CANAL,
  CHAVE_DO_MODO,
  CHAVE_DO_TAMANHO,
  CHAVE_DA_QUALIDADE,
  configuracaoPadrao,
  motivoDaFalha,
  type CanalDeTinta,
  type ConfiguracaoDeTinta,
  type PerfilDeTinta,
} from "../analise-tinta";
import { deveRolarPeloTeclado, rolarPeloTeclado } from "../dialogo";
import { t, type Chave } from "../i18n";
import {
  DPIS,
  MODOS,
  QUALIDADES,
  TAMANHOS,
  configuracaoValida,
  errosDoRascunho,
  idDePerfil,
  perfilDoRascunho,
  rascunhoDoPerfil,
  rascunhoNovo,
  type ErrosDoRascunho,
  type RascunhoDoPerfil,
} from "../plotinfo-estado";
import botoes from "./Botao.module.css";
import styles from "./PlotInfoEngrenagem.module.css";

const ID_TITULO = "plotinfo-engrenagem-titulo";

interface PropsDoPainel {
  perfis: readonly PerfilDeTinta[];
  rascunho: RascunhoDoPerfil;
  configuracao: ConfiguracaoDeTinta;
  selecionado: string;
  novo: boolean;
  erros: ErrosDoRascunho;
  confirmandoApagar: boolean;
  salvando: boolean;
  erro: string | null;
  aviso: string | null;
  aoSelecionar: (id: string) => void;
  aoNovo: () => void;
  aoDuplicar: () => void;
  aoPedirApagar: () => void;
  aoConfirmarApagar: () => void;
  aoCancelarApagar: () => void;
  aoRestaurar: () => void;
  aoMudarPerfil: (mudanca: Partial<RascunhoDoPerfil>) => void;
  aoMudarCanal: (
    campo: "volumes_ml" | "rendimentos_paginas" | "precos_ml",
    indice: number,
    valor: string,
  ) => void;
  aoMudarFator: (indice: number, valor: string) => void;
  aoMudarConfiguracao: (mudanca: Partial<ConfiguracaoDeTinta>) => void;
  aoSalvar: () => void;
  aoCancelar: () => void;
  corpoRef?: RefObject<HTMLDivElement | null>;
}

function idDoCampo(campo: string, indice: number): string {
  return `plotinfo-${campo}-${indice}`;
}

function textoDoCampo(campo: Chave, canal: CanalDeTinta): string {
  return `${t(campo)}: ${t(CHAVE_DO_CANAL[canal])}`;
}

/** Corpo do dialogo, sem o Radix: os estados e a validacao sao testados sem abrir a janela. */
export function PainelDaEngrenagem({
  perfis,
  rascunho,
  configuracao,
  selecionado,
  novo,
  erros,
  confirmandoApagar,
  salvando,
  erro,
  aviso,
  aoSelecionar,
  aoNovo,
  aoDuplicar,
  aoPedirApagar,
  aoConfirmarApagar,
  aoCancelarApagar,
  aoRestaurar,
  aoMudarPerfil,
  aoMudarCanal,
  aoMudarFator,
  aoMudarConfiguracao,
  aoSalvar,
  aoCancelar,
  corpoRef,
}: PropsDoPainel) {
  const nomeDoPerfil = perfis.find((perfil) => perfil.id === selecionado)?.nome ?? rascunho.nome;
  const perfilExistente = !novo && perfis.some((perfil) => perfil.id === rascunho.id);
  const temPerfis = perfis.length > 0;
  const podeSalvar = !salvando && !erros.algum && temPerfis;

  return (
    <>
      <div ref={corpoRef} className={styles.corpo}>
        {erro !== null && (
          <p className={styles.erro} role="alert">
            <CircleAlert size={14} aria-hidden="true" />
            <span>{erro}</span>
          </p>
        )}
        {aviso !== null && <p className={styles.aviso}>{aviso}</p>}
        {!temPerfis ? (
          <p className={styles.aviso}>{t("plotinfo.engrenagem.semPerfis")}</p>
        ) : (
          <>
            {erros.algum && (
              <p className={styles.erro} role="alert">
                <CircleAlert size={14} aria-hidden="true" />
                <span>{t("plotinfo.engrenagem.invalido")}</span>
              </p>
            )}

            <label className={styles.campo}>
              <span className={styles.rotulo}>{t("plotinfo.engrenagem.perfil")}</span>
              <select
                className={styles.seletor}
                value={selecionado}
                onChange={(evento) => aoSelecionar(evento.target.value)}
              >
                {perfis.map((perfil) => (
                  <option key={perfil.id} value={perfil.id}>
                    {perfil.nome}
                  </option>
                ))}
                {novo && (
                  <option value={rascunho.id}>{t("plotinfo.engrenagem.novoNome")}</option>
                )}
              </select>
            </label>

            <div className={styles.acoesDePerfil}>
              <button type="button" className={`${botoes.botao} ${botoes.secundario} ${botoes.pequeno}`} onClick={aoNovo}>
                {t("plotinfo.engrenagem.novo")}
              </button>
              <button
                type="button"
                className={`${botoes.botao} ${botoes.secundario} ${botoes.pequeno}`}
                onClick={aoDuplicar}
              >
                {t("plotinfo.engrenagem.duplicar")}
              </button>
              <button
                type="button"
                className={`${botoes.botao} ${botoes.secundario} ${botoes.pequeno}`}
                disabled={!perfilExistente}
                onClick={aoPedirApagar}
              >
                {t("plotinfo.engrenagem.apagar")}
              </button>
            </div>

            {confirmandoApagar && (
              <div className={styles.confirmarApagar} role="group" aria-label={t("plotinfo.engrenagem.apagar")}>
                <p className={styles.aviso}>
                  {t("plotinfo.engrenagem.confirmarApagar", { nome: nomeDoPerfil })}
                </p>
                <div className={styles.acoesDePerfil}>
                  <button
                    type="button"
                    className={`${botoes.botao} ${botoes.perigo} ${botoes.pequeno}`}
                    onClick={aoConfirmarApagar}
                  >
                    {t("plotinfo.engrenagem.apagar")}
                  </button>
                  <button
                    type="button"
                    className={`${botoes.botao} ${botoes.secundario} ${botoes.pequeno}`}
                    onClick={aoCancelarApagar}
                  >
                    {t("plotinfo.engrenagem.manter")}
                  </button>
                </div>
              </div>
            )}

            <div className={styles.duasColunas}>
              <label className={styles.campo}>
                <span className={styles.rotulo}>{t("plotinfo.engrenagem.nome")}</span>
                <input
                  className={styles.entrada}
                  value={rascunho.nome}
                  aria-invalid={erros.nome}
                  onChange={(evento) => aoMudarPerfil({ nome: evento.target.value })}
                />
              </label>
              <label className={styles.campo}>
                <span className={styles.rotulo}>{t("plotinfo.engrenagem.moeda")}</span>
                <input
                  className={styles.entrada}
                  value={rascunho.moeda}
                  maxLength={3}
                  aria-invalid={erros.moeda}
                  onChange={(evento) => aoMudarPerfil({ moeda: evento.target.value.toUpperCase() })}
                />
                <span className={styles.dica}>{t("plotinfo.engrenagem.moedaAjuda")}</span>
              </label>
            </div>

            <table className={styles.tabela}>
              <thead>
                <tr>
                  <th scope="col">{t("plotinfo.engrenagem.canal")}</th>
                  <th scope="col">{t("plotinfo.engrenagem.volume")}</th>
                  <th scope="col">{t("plotinfo.engrenagem.rendimento")}</th>
                  <th scope="col">{t("plotinfo.engrenagem.preco")}</th>
                </tr>
              </thead>
              <tbody>
                {CANAIS_DE_TINTA.map((canal, indice) => (
                  <tr key={canal}>
                    <th scope="row">
                      <span className={`${styles.ponto} ${styles[canal]}`} aria-hidden="true" />
                      <span>{t(CHAVE_DO_CANAL[canal])}</span>
                    </th>
                    <td>
                      <input
                        id={idDoCampo("volume", indice)}
                        className={styles.entradaDaTabela}
                        value={rascunho.volumes_ml[indice] ?? ""}
                        inputMode="decimal"
                        aria-label={textoDoCampo("plotinfo.engrenagem.volume", canal)}
                        aria-invalid={erros.volumes[indice] ?? false}
                        onChange={(evento) => aoMudarCanal("volumes_ml", indice, evento.target.value)}
                      />
                    </td>
                    <td>
                      <input
                        id={idDoCampo("rendimento", indice)}
                        className={styles.entradaDaTabela}
                        value={rascunho.rendimentos_paginas[indice] ?? ""}
                        inputMode="numeric"
                        aria-label={textoDoCampo("plotinfo.engrenagem.rendimento", canal)}
                        aria-invalid={erros.rendimentos[indice] ?? false}
                        onChange={(evento) =>
                          aoMudarCanal("rendimentos_paginas", indice, evento.target.value)
                        }
                      />
                    </td>
                    <td>
                      <input
                        id={idDoCampo("preco", indice)}
                        className={styles.entradaDaTabela}
                        value={rascunho.precos_ml[indice] ?? ""}
                        inputMode="decimal"
                        aria-label={textoDoCampo("plotinfo.engrenagem.preco", canal)}
                        aria-invalid={erros.precos[indice] ?? false}
                        onChange={(evento) => aoMudarCanal("precos_ml", indice, evento.target.value)}
                      />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            <p className={styles.dica}>{t("plotinfo.engrenagem.ajudaPreco")}</p>

            <fieldset className={styles.grupo}>
              <legend className={styles.rotulo}>{t("plotinfo.engrenagem.fatores")}</legend>
              <div className={styles.tresColunas}>
                {(["rascunho", "normal", "alta"] as const).map((qualidade, indice) => (
                  <label key={qualidade} className={styles.campo}>
                    <span className={styles.rotulo}>{t(CHAVE_DA_QUALIDADE[qualidade])}</span>
                    <input
                      className={styles.entrada}
                      value={rascunho.fatores_qualidade[indice] ?? ""}
                      inputMode="decimal"
                      aria-invalid={erros.fatores[indice] ?? false}
                      onChange={(evento) => aoMudarFator(indice, evento.target.value)}
                    />
                  </label>
                ))}
              </div>
            </fieldset>

            <div className={styles.duasColunas}>
              <label className={styles.campo}>
                <span className={styles.rotulo}>{t("plotinfo.engrenagem.qualidade")}</span>
                <select
                  className={styles.seletor}
                  value={configuracao.qualidade}
                  onChange={(evento) =>
                    aoMudarConfiguracao({
                      qualidade: evento.target.value as ConfiguracaoDeTinta["qualidade"],
                    })
                  }
                >
                  {QUALIDADES.map((qualidade) => (
                    <option key={qualidade} value={qualidade}>
                      {t(CHAVE_DA_QUALIDADE[qualidade])}
                    </option>
                  ))}
                </select>
              </label>
              <label className={styles.campo}>
                <span className={styles.rotulo}>{t("plotinfo.engrenagem.dpi")}</span>
                <select
                  className={styles.seletor}
                  value={String(configuracao.dpi)}
                  onChange={(evento) =>
                    aoMudarConfiguracao({ dpi: Number(evento.target.value) })
                  }
                >
                  {DPIS.map((dpi) => (
                    <option key={dpi} value={String(dpi)}>
                      {t("plotinfo.dpi.valor", { dpi: String(dpi) })}
                    </option>
                  ))}
                </select>
              </label>
            </div>

            <fieldset className={styles.grupo}>
              <legend className={styles.rotulo}>{t("plotinfo.engrenagem.modo")}</legend>
              {MODOS.map((modo) => (
                <label key={modo} className={styles.opcao}>
                  <input
                    type="radio"
                    name="plotinfo-modo"
                    checked={configuracao.modo === modo}
                    onChange={() => aoMudarConfiguracao({ modo })}
                  />
                  <span>{t(CHAVE_DO_MODO[modo])}</span>
                </label>
              ))}
              <p className={styles.dica}>{t("plotinfo.engrenagem.modoAjuda")}</p>
            </fieldset>

            <label className={styles.opcao}>
              <input
                type="checkbox"
                checked={configuracao.cinzas_so_preto}
                onChange={(evento) =>
                  aoMudarConfiguracao({ cinzas_so_preto: evento.target.checked })
                }
              />
              <span>{t("plotinfo.engrenagem.cinzas")}</span>
            </label>

            <label className={styles.campo}>
              <span className={styles.rotulo}>{t("plotinfo.engrenagem.tamanho")}</span>
              <select
                className={styles.seletor}
                value={configuracao.tamanho_da_imagem}
                onChange={(evento) =>
                  aoMudarConfiguracao({
                    tamanho_da_imagem:
                      evento.target.value as ConfiguracaoDeTinta["tamanho_da_imagem"],
                  })
                }
              >
                {TAMANHOS.map((tamanho) => (
                  <option key={tamanho} value={tamanho}>
                    {t(CHAVE_DO_TAMANHO[tamanho])}
                  </option>
                ))}
              </select>
            </label>
          </>
        )}
      </div>

      <div className={styles.rodape}>
        <button
          type="button"
          className={`${botoes.botao} ${botoes.secundario}`}
          onClick={aoRestaurar}
          disabled={salvando || !temPerfis}
          title={t("plotinfo.engrenagem.restaurarAjuda")}
        >
          {t("plotinfo.engrenagem.restaurar")}
        </button>
        <button
          type="button"
          className={`${botoes.botao} ${botoes.secundario}`}
          onClick={aoCancelar}
          disabled={salvando}
        >
          {t("plotinfo.engrenagem.cancelar")}
        </button>
        <button
          type="button"
          className={`${botoes.botao} ${botoes.primario}`}
          onClick={aoSalvar}
          disabled={!podeSalvar}
          aria-busy={salvando}
        >
          {salvando && <LoaderCircle size={14} className={styles.girando} aria-hidden="true" />}
          {t(salvando ? "plotinfo.engrenagem.salvando" : "plotinfo.engrenagem.salvar")}
        </button>
      </div>
    </>
  );
}

interface Props {
  aberto: boolean;
  perfis: readonly PerfilDeTinta[];
  configuracao: ConfiguracaoDeTinta | null;
  aviso: string | null;
  aoFechar: () => void;
  aoSalvarPerfil: (perfil: PerfilDeTinta) => Promise<void>;
  aoApagarPerfil: (id: string) => Promise<void>;
  aoAplicarConfiguracao: (configuracao: ConfiguracaoDeTinta) => Promise<void>;
}

/** Dialogo da engrenagem: foco preso, ESC e devolucao do foco ao gatilho pelo Radix. */
export function PlotInfoEngrenagem({
  aberto,
  perfis,
  configuracao,
  aviso,
  aoFechar,
  aoSalvarPerfil,
  aoApagarPerfil,
  aoAplicarConfiguracao,
}: Props) {
  const propsAtuais = useRef({ perfis, configuracao });
  propsAtuais.current = { perfis, configuracao };

  const [selecionado, setSelecionado] = useState(configuracao?.perfil ?? "");
  const perfilInicial = perfis.find((perfil) => perfil.id === selecionado) ?? perfis[0] ?? null;
  const [rascunho, setRascunho] = useState<RascunhoDoPerfil>(() =>
    perfilInicial === null
      ? rascunhoNovo("", [])
      : rascunhoDoPerfil(perfilInicial),
  );
  const [config, setConfig] = useState<ConfiguracaoDeTinta>(() =>
    configuracao ?? configuracaoPadrao(perfilInicial?.id ?? ""),
  );
  const [novo, setNovo] = useState(false);
  const [confirmandoApagar, setConfirmandoApagar] = useState(false);
  const [salvando, setSalvando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  const corpo = useRef<HTMLDivElement>(null);

  // Cada abertura parte do perfil ativo; os campos voltam ao que esta salvo.
  useEffect(() => {
    if (!aberto) return;
    const atuais = propsAtuais.current;
    const perfil =
      atuais.perfis.find((item) => item.id === atuais.configuracao?.perfil) ??
      atuais.perfis[0] ??
      null;
    setSelecionado(perfil?.id ?? "");
    setRascunho(perfil === null ? rascunhoNovo("", []) : rascunhoDoPerfil(perfil));
    setConfig(
      atuais.configuracao ?? configuracaoPadrao(perfil?.id ?? ""),
    );
    setNovo(false);
    setConfirmandoApagar(false);
    setSalvando(false);
    setErro(null);
  }, [aberto]);

  // Perfil apagado ou ainda nao carregado: a selecao cai no primeiro, sem deixar id orfao.
  useEffect(() => {
    if (!aberto || perfis.length === 0) return;
    if (perfis.some((perfil) => perfil.id === selecionado)) return;
    const primeiro = perfis[0];
    if (primeiro === undefined) return;
    setSelecionado(primeiro.id);
    setRascunho(rascunhoDoPerfil(primeiro));
    setConfig((atual) => ({ ...atual, perfil: primeiro.id }));
    setNovo(false);
    setConfirmandoApagar(false);
  }, [aberto, perfis, selecionado]);

  const ids = perfis.map((perfil) => perfil.id);
  const erros = errosDoRascunho(rascunho, ids, novo ? null : rascunho.id);

  const selecionar = (id: string) => {
    if (id === rascunho.id && novo) return;
    const perfil = perfis.find((item) => item.id === id);
    if (perfil === undefined) return;
    setSelecionado(id);
    setRascunho(rascunhoDoPerfil(perfil));
    setConfig((atual) => ({ ...atual, perfil: id }));
    setNovo(false);
    setConfirmandoApagar(false);
    setErro(null);
  };

  const criarNovo = () => {
    const criado = rascunhoNovo(t("plotinfo.engrenagem.novoNome"), ids);
    setSelecionado(criado.id);
    setRascunho(criado);
    setConfig((atual) => ({ ...atual, perfil: criado.id }));
    setNovo(true);
    setConfirmandoApagar(false);
    setErro(null);
  };

  const duplicar = () => {
    const nome = t("plotinfo.engrenagem.copiaDe", { nome: rascunho.nome });
    const copia: RascunhoDoPerfil = {
      ...rascunho,
      id: idDePerfil(nome, ids),
      nome,
    };
    setSelecionado(copia.id);
    setRascunho(copia);
    setConfig((atual) => ({ ...atual, perfil: copia.id }));
    setNovo(true);
    setConfirmandoApagar(false);
    setErro(null);
  };

  const apagar = async () => {
    setSalvando(true);
    setErro(null);
    try {
      await aoApagarPerfil(rascunho.id);
      setConfirmandoApagar(false);
      const restante = perfis.find((perfil) => perfil.id !== rascunho.id);
      if (restante !== undefined) {
        setSelecionado(restante.id);
        setRascunho(rascunhoDoPerfil(restante));
        setConfig((atual) => ({ ...atual, perfil: restante.id }));
      }
    } catch (falha) {
      setErro(
        t("plotinfo.engrenagem.erroApagar", { motivo: motivoDaFalha(falha) }),
      );
    } finally {
      setSalvando(false);
    }
  };

  const salvar = async () => {
    const perfil = perfilDoRascunho(rascunho, ids, novo ? null : rascunho.id);
    if (perfil === null) {
      setErro(t("plotinfo.engrenagem.invalido"));
      return;
    }
    const configuracaoNova = { ...config, perfil: perfil.id };
    if (!configuracaoValida(configuracaoNova, [...perfis, perfil])) {
      setErro(t("plotinfo.engrenagem.invalido"));
      return;
    }
    setSalvando(true);
    setErro(null);
    try {
      await aoSalvarPerfil(perfil);
      await aoAplicarConfiguracao(configuracaoNova);
      aoFechar();
    } catch (falha) {
      setErro(
        t("plotinfo.engrenagem.erroSalvar", { motivo: motivoDaFalha(falha) }),
      );
    } finally {
      setSalvando(false);
    }
  };

  const restaurar = () => {
    const perfil = perfis.find((item) => item.id === selecionado) ?? null;
    setRascunho(perfil === null ? rascunhoNovo("", []) : rascunhoDoPerfil(perfil));
    setConfig(configuracaoPadrao(perfil?.id ?? ""));
    setNovo(false);
    setConfirmandoApagar(false);
    setErro(null);
  };

  const mudarPerfil = (mudanca: Partial<RascunhoDoPerfil>) => {
    setRascunho((atual) => ({ ...atual, ...mudanca }));
  };

  const mudarCanal = (
    campo: "volumes_ml" | "rendimentos_paginas" | "precos_ml",
    indice: number,
    valor: string,
  ) => {
    setRascunho((atual) => {
      const lista = [...atual[campo]] as [string, string, string, string];
      if (indice < 0 || indice >= lista.length) return atual;
      lista[indice] = valor;
      return { ...atual, [campo]: lista };
    });
  };

  const mudarFator = (indice: number, valor: string) => {
    setRascunho((atual) => {
      const lista = [...atual.fatores_qualidade] as [string, string, string];
      if (indice < 0 || indice >= lista.length) return atual;
      lista[indice] = valor;
      return { ...atual, fatores_qualidade: lista };
    });
  };

  return (
    <Dialog.Root
      open={aberto}
      onOpenChange={(estaAberto) => {
        if (!estaAberto) aoFechar();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className={styles.veu} />
        <Dialog.Content
          className={styles.conteudo}
          aria-describedby="plotinfo-engrenagem-descricao"
          onCloseAutoFocus={(evento) => {
            evento.preventDefault();
            document.getElementById("plotinfo-abrir-engrenagem")?.focus();
          }}
          onKeyDown={(evento) => {
            const alvo = evento.target instanceof Element ? evento.target : null;
            if (deveRolarPeloTeclado(alvo) && rolarPeloTeclado(evento.key, corpo.current)) {
              evento.preventDefault();
            }
          }}
        >
          <div className={styles.cabecalho}>
            <Settings size={16} aria-hidden="true" />
            <Dialog.Title id={ID_TITULO} className={styles.titulo}>
              {t("plotinfo.engrenagem.titulo")}
            </Dialog.Title>
          </div>
          <Dialog.Description id="plotinfo-engrenagem-descricao" className={styles.descricao}>
            {t("plotinfo.engrenagem.descricao")}
          </Dialog.Description>
          <div className={styles.corpoRolavel}>
            <PainelDaEngrenagem
              corpoRef={corpo}
              perfis={perfis}
              rascunho={rascunho}
              configuracao={config}
              selecionado={selecionado}
              novo={novo}
              erros={erros}
              confirmandoApagar={confirmandoApagar}
              salvando={salvando}
              erro={erro}
              aviso={aviso}
              aoSelecionar={selecionar}
              aoNovo={criarNovo}
              aoDuplicar={duplicar}
              aoPedirApagar={() => setConfirmandoApagar(true)}
              aoConfirmarApagar={() => void apagar()}
              aoCancelarApagar={() => setConfirmandoApagar(false)}
              aoRestaurar={restaurar}
              aoMudarPerfil={mudarPerfil}
              aoMudarCanal={mudarCanal}
              aoMudarFator={mudarFator}
              aoMudarConfiguracao={(mudanca) =>
                setConfig((atual) => ({ ...atual, ...mudanca }))
              }
              aoSalvar={() => void salvar()}
              aoCancelar={aoFechar}
            />
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}