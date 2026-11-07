import { Calculator, Check, CircleAlert, Download, Settings, X } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  CANAIS_DE_TINTA,
  CHAVE_DO_CANAL,
  CHAVE_DO_MODO,
  CHAVE_DA_QUALIDADE,
  formatarMl,
  formatarMoeda,
  formatarNumero,
  formatarPercentual,
  rotuloDoPapel,
  type CanalDeTinta,
  type ConfiguracaoDeTinta,
  type PerfilDeTinta,
  type ProgressoDaAnalise,
  type ResultadoDaAnalise,
  type ResultadoDaPagina,
} from "../analise-tinta";
import { t } from "../i18n";
import type { EstadoDoPainelDeTinta } from "../plotinfo-estado";
import type { Aba } from "../tabs";
import styles from "./PlotInfoGaveta.module.css";
import { PlotInfoEngrenagem } from "./PlotInfoEngrenagem";

export const ID_GAVETA_PLOTINFO = "gaveta-plotinfo";
export const ID_BOTAO_ENGENHAGEM_PLOTINFO = "plotinfo-abrir-engrenagem";

export interface ConteudoDaTinta {
  situacao:
    | "fechado"
    | "sem_documento"
    | "contando"
    | "recusado"
    | "analisando"
    | "pronto"
    | "erro";
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
  perfilAtivo: PerfilDeTinta | null;
  perfis: readonly PerfilDeTinta[];
  avisoDosPerfis: string | null;
  definirPaginas: (valor: string) => void;
  definirCopias: (valor: string) => void;
  tentarDeNovo: () => void;
  exportar: () => Promise<void>;
}

function PontoDoCanal({ canal }: { canal: CanalDeTinta }) {
  return <span className={`${styles.ponto} ${styles[canal]}`} aria-hidden="true" />;
}

function Secao({ titulo, children }: { titulo: string; children: ReactNode }) {
  return (
    <section className={styles.secao}>
      <h3 className={styles.tituloSecao}>{titulo}</h3>
      {children}
    </section>
  );
}

function fraseDoTotal(paginas: number, copias: number, total: number): string {
  const numeros = {
    paginas: formatarNumero(paginas),
    copias: formatarNumero(copias),
    total: formatarNumero(total),
  };
  if (paginas === 1) {
    return copias === 1
      ? t("plotinfo.selecao.total.um")
      : t("plotinfo.selecao.total.umaPagina", { copias: numeros.copias, total: numeros.total });
  }
  return copias === 1
    ? t("plotinfo.selecao.total.umaCopia", { paginas: numeros.paginas, total: numeros.total })
    : t("plotinfo.selecao.total", numeros);
}

function LinhaDeArquivo({ rotulo, valor, mono = false }: { rotulo: string; valor: string; mono?: boolean }) {
  return (
    <div className={styles.linha}>
      <span className={styles.rotuloDaLinha}>{rotulo}</span>
      <span className={mono ? `${styles.valor} ${styles.mono}` : styles.valor}>{valor}</span>
    </div>
  );
}

interface PropsDoPainel {
  aba: Aba | null;
  paginaAtual: number | null;
  tinta: ConteudoDaTinta;
  aoFechar: () => void;
  aoAbrirEngrenagem: () => void;
  engrenagemAberta: boolean;
}

/** O painel inteiro, sem o dialogo da engrenagem: os estados sao testados sem abrir a janela. */
export function PainelDeTinta({
  aba,
  paginaAtual,
  tinta,
  aoFechar,
  aoAbrirEngrenagem,
  engrenagemAberta,
}: PropsDoPainel) {
  const fecharRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    fecharRef.current?.focus();
  }, []);

  const { resultado, configuracao, perfilAtivo, progresso } = tinta;
  const arquivo = resultado?.arquivo ?? null;
  const totalDoArquivo =
    arquivo?.paginas ??
    (aba?.paginas.estado === "conhecido" ? aba.paginas.total : null);
  const pagina = paginaAtual === null ? null : (tinta.paginas.get(paginaAtual) ?? null);
  const paginaDaMedida = pagina ?? [...tinta.paginas.values()][0] ?? null;
  const medida =
    arquivo?.largura_mm != null && arquivo.altura_mm != null
      ? { largura: arquivo.largura_mm, altura: arquivo.altura_mm }
      : paginaDaMedida === null
        ? null
        : { largura: paginaDaMedida.largura_mm, altura: paginaDaMedida.altura_mm };
  const papel = medida === null ? "—" : rotuloDoPapel(medida.largura, medida.altura);
  const dpi = arquivo?.dpi ?? configuracao?.dpi ?? null;
  const modo = arquivo?.modo_usado ?? configuracao?.modo ?? "simples";
  const qualidade = resultado?.qualidade ?? configuracao?.qualidade ?? "normal";
  const selecao = resultado?.selecao ?? progresso?.selecao ?? null;
  const totalDaSelecao = selecao?.total_com_copias ?? null;
  const percentualDoProgresso =
    progresso === null || progresso.total <= 0
      ? 0
      : Math.min(100, Math.max(0, (progresso.feitas / progresso.total) * 100));
  const analisando = tinta.situacao === "analisando";

  return (
    <aside
      id={ID_GAVETA_PLOTINFO}
      className={styles.gaveta}
      aria-label={t("plotinfo.titulo")}
      aria-busy={analisando || tinta.recalculando}
      onKeyDown={(evento) => {
        if (evento.key === "Escape" && !engrenagemAberta) {
          evento.preventDefault();
          aoFechar();
        }
      }}
    >
      <header className={styles.cabecalho}>
        <div className={styles.titulo}>
          <Calculator size={14} aria-hidden="true" />
          <span>{t("plotinfo.titulo")}</span>
        </div>
        <div className={styles.acoesDoCabecalho}>
          <button
            type="button"
            id={ID_BOTAO_ENGENHAGEM_PLOTINFO}
            className={styles.botaoIcone}
            aria-label={t("plotinfo.engrenagem")}
            title={t("plotinfo.engrenagem")}
            aria-haspopup="dialog"
            onClick={aoAbrirEngrenagem}
          >
            <Settings size={14} aria-hidden="true" />
          </button>
          <button
            ref={fecharRef}
            type="button"
            className={styles.botaoIcone}
            aria-label={t("plotinfo.fechar")}
            title={t("plotinfo.fechar")}
            onClick={aoFechar}
          >
            <X size={14} aria-hidden="true" />
          </button>
        </div>
      </header>

      {(analisando || tinta.recalculando) && (
        <div className={styles.progresso} role="status">
          <p className={styles.progressoTexto}>
            {analisando
              ? progresso === null
                ? t("plotinfo.progresso.preparando")
                : t("plotinfo.progresso", {
                    feitas: String(progresso.feitas),
                    total: String(progresso.total),
                  })
              : t("plotinfo.recalculando")}
          </p>
          {analisando && (
            <div
              className={styles.trilho}
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={progresso?.total ?? 1}
              aria-valuenow={progresso?.feitas ?? 0}
            >
              <span
                className={styles.preenchimento}
                style={{ width: `${percentualDoProgresso}%` }}
              />
            </div>
          )}
        </div>
      )}

      <div className={styles.corpo}>
        {tinta.erro !== null && (
          <div className={styles.erro} role="alert">
            <CircleAlert size={14} aria-hidden="true" />
            <div className={styles.erroTexto}>
              <p className={styles.textoDoErro}>{tinta.erro}</p>
              {tinta.situacao === "erro" && (
                <button
                  type="button"
                  className={styles.tentarDeNovo}
                  onClick={tinta.tentarDeNovo}
                >
                  {t("plotinfo.tentarDeNovo")}
                </button>
              )}
            </div>
          </div>
        )}
        {(tinta.avisoDosPerfis ?? tinta.aviso) !== null && (
          <p className={styles.aviso}>{tinta.avisoDosPerfis ?? tinta.aviso}</p>
        )}
        {tinta.mensagem !== null && (
          <p className={styles.sucesso} role="status">
            <Check size={12} aria-hidden="true" />
            <span>{tinta.mensagem}</span>
          </p>
        )}

        {tinta.situacao === "sem_documento" ? (
          <div className={styles.vazio}>
            <Calculator size={18} aria-hidden="true" />
            <p className={styles.vazioTitulo}>{t("plotinfo.estado.vazioTitulo")}</p>
            <p className={styles.vazioTexto}>{t("plotinfo.estado.vazioTexto")}</p>
          </div>
        ) : tinta.situacao === "contando" ? (
          <p className={styles.aviso}>{t("plotinfo.estado.contando")}</p>
        ) : tinta.situacao === "recusado" ? (
          <p className={styles.erro} role="alert">
            <CircleAlert size={14} aria-hidden="true" />
            <span>
              {t("plotinfo.estado.recusado", {
                motivo: aba?.paginas.estado === "recusado" ? aba.paginas.motivo : "",
              })}
            </span>
          </p>
        ) : null}

        {tinta.situacao !== "sem_documento" && tinta.situacao !== "contando" && (
          <>
            <Secao titulo={t("plotinfo.secao.arquivo")}>
              <LinhaDeArquivo
                rotulo={t("plotinfo.arquivo.nome")}
                valor={arquivo?.nome ?? aba?.documento.nome ?? "—"}
              />
              <LinhaDeArquivo
                rotulo={t("plotinfo.arquivo.paginas")}
                valor={totalDoArquivo === null ? "—" : formatarNumero(totalDoArquivo)}
                mono
              />
              <LinhaDeArquivo rotulo={t("plotinfo.arquivo.papel")} valor={papel} />
              <LinhaDeArquivo
                rotulo={t("plotinfo.arquivo.dpi")}
                valor={dpi === null ? "—" : t("plotinfo.dpi.valor", { dpi: String(dpi) })}
                mono
              />
              <LinhaDeArquivo
                rotulo={t("plotinfo.arquivo.modo")}
                valor={t(CHAVE_DO_MODO[modo])}
              />
            </Secao>

            <Secao titulo={t("plotinfo.secao.pagina")}>
              {paginaAtual === null ? (
                <p className={styles.aviso}>{t("plotinfo.pagina.semDocumento")}</p>
              ) : pagina === null ? (
                <p className={styles.aviso}>
                  {analisando
                    ? t("plotinfo.pagina.analisando", { n: String(paginaAtual) })
                    : t("plotinfo.pagina.semResultado")}
                </p>
              ) : (
                <>
                  <div className={styles.tituloDaPagina}>
                    <span className={styles.tituloLinha}>
                      {t("plotinfo.pagina.titulo", {
                        atual: String(paginaAtual),
                        total: String(totalDoArquivo ?? paginaAtual),
                      })}
                    </span>
                    <span className={styles.marca}>{t("plotinfo.pagina.umaCopia")}</span>
                  </div>
                  {pagina.em_branco && (
                    <p className={styles.paginaEmBranco}>
                      <span className={styles.pontoVazado} aria-hidden="true" />
                      {t("plotinfo.pagina.emBranco")}
                    </p>
                  )}
                  <div className={styles.cabecalhoDeCanais} aria-hidden="true">
                    <span>{t("plotinfo.engrenagem.canal")}</span>
                    <span>{t("plotinfo.cobertura")}</span>
                    <span>{t("plotinfo.ml")}</span>
                  </div>
                  {CANAIS_DE_TINTA.map((canal, indice) => (
                    <div key={canal} className={styles.linhaDeCanal}>
                      <span className={styles.nomeDoCanal}>
                        <PontoDoCanal canal={canal} />
                        <span>{t(CHAVE_DO_CANAL[canal])}</span>
                      </span>
                      <span className={`${styles.numero} ${styles.cobertura}`}>
                        {formatarPercentual(pagina.cobertura_pct[indice] ?? 0)}
                      </span>
                      <span className={`${styles.numero} ${styles.ml}`}>
                        {formatarMl(pagina.tinta_ml[indice] ?? 0)}
                      </span>
                    </div>
                  ))}
                  <div className={`${styles.linhaTotal} ${styles.linhaDeCanal}`}>
                    <span className={styles.nomeDoCanal}>{t("plotinfo.total")}</span>
                    <span className={`${styles.numero} ${styles.cobertura}`} />
                    <span className={`${styles.numero} ${styles.ml}`}>
                      {formatarMl(pagina.total_ml)}
                    </span>
                  </div>
                  <LinhaDeArquivo
                    rotulo={t("plotinfo.custo")}
                    valor={formatarMoeda(
                      pagina.total_custo,
                      resultado?.moeda ?? perfilAtivo?.moeda ?? "BRL",
                    )}
                    mono
                  />
                </>
              )}
            </Secao>

            <Secao titulo={t("plotinfo.secao.selecao")}>
              <div className={styles.campos}>
                <label className={styles.campo}>
                  <span className={styles.rotulo}>{t("plotinfo.selecao.campoPaginas")}</span>
                  <input
                    className={styles.entrada}
                    value={tinta.painel.paginasTexto}
                    inputMode="text"
                    placeholder={t("plotinfo.selecao.dicaPaginas")}
                    onChange={(evento) => tinta.definirPaginas(evento.target.value)}
                  />
                  <span className={styles.dica}>{t("plotinfo.selecao.dicaPaginas")}</span>
                </label>
                <label className={styles.campo}>
                  <span className={styles.rotulo}>{t("plotinfo.selecao.campoCopias")}</span>
                  <input
                    className={styles.entrada}
                    value={tinta.painel.copiasTexto}
                    inputMode="numeric"
                    aria-invalid={tinta.copias === null}
                    aria-describedby="plotinfo-dica-copias"
                    onChange={(evento) => tinta.definirCopias(evento.target.value)}
                  />
                  <span id="plotinfo-dica-copias" className={styles.dica}>
                    {t("plotinfo.selecao.dicaCopias")}
                  </span>
                </label>
              </div>
              {tinta.copias === null && (
                <p className={styles.campoInvalido} id="plotinfo-erro-copias">
                  {t("plotinfo.erro.copias")}
                </p>
              )}
              <p className={styles.totalDaSelecao} aria-live="polite">
                {selecao === null || totalDaSelecao === null
                  ? t("plotinfo.estado.semNumeros")
                  : fraseDoTotal(
                      selecao.quantidade,
                      resultado?.copias ?? tinta.copias ?? 1,
                      totalDaSelecao,
                    )}
              </p>
              {selecao !== null && (
                <div className={styles.resumoDaSelecao}>
                  <LinhaDeArquivo
                    rotulo={t("plotinfo.selecao.tinta")}
                    valor={formatarMl(selecao.total_ml)}
                    mono
                  />
                  <LinhaDeArquivo
                    rotulo={t("plotinfo.selecao.custo")}
                    valor={formatarMoeda(
                      selecao.total_custo,
                      resultado?.moeda ?? perfilAtivo?.moeda ?? "BRL",
                    )}
                    mono
                  />
                </div>
              )}
            </Secao>

            <Secao titulo={t("plotinfo.secao.documento")}>
              {resultado === null ? (
                <p className={styles.aviso}>{t("plotinfo.documento.semDados")}</p>
              ) : (
                <>
                  <p className={styles.marca}>{t("plotinfo.documento.umaCopia")}</p>
                  {CANAIS_DE_TINTA.map((canal, indice) => (
                    <div key={canal} className={styles.linhaDeCanal}>
                      <span className={styles.nomeDoCanal}>
                        <PontoDoCanal canal={canal} />
                        <span>{t(CHAVE_DO_CANAL[canal])}</span>
                      </span>
                      <span className={`${styles.numero} ${styles.cobertura}`}>
                        {formatarMl(resultado.documento.tinta[indice] ?? 0)}
                      </span>
                      <span className={`${styles.numero} ${styles.ml}`}>
                        {formatarMoeda(
                          resultado.documento.custo[indice] ?? 0,
                          resultado.moeda,
                        )}
                      </span>
                    </div>
                  ))}
                  <div className={`${styles.linhaTotal} ${styles.linhaDeCanal}`}>
                    <span className={styles.nomeDoCanal}>{t("plotinfo.total")}</span>
                    <span className={`${styles.numero} ${styles.cobertura}`}>
                      {formatarMl(resultado.documento.total_ml)}
                    </span>
                    <span className={`${styles.numero} ${styles.ml}`}>
                      {formatarMoeda(resultado.documento.total_custo, resultado.moeda)}
                    </span>
                  </div>
                  <LinhaDeArquivo
                    rotulo={t("plotinfo.documento.media")}
                    valor={formatarMl(resultado.documento.ml_medio_por_pagina)}
                    mono
                  />
                  <p className={styles.aviso}>
                    {t("plotinfo.documento.maisCara", {
                      n:
                        resultado.documento.pagina_mais_cara === null
                          ? "—"
                          : formatarNumero(resultado.documento.pagina_mais_cara.numero),
                    })}
                  </p>
                </>
              )}
            </Secao>

            <Secao titulo={t("plotinfo.secao.cartucho")}>
              {selecao === null ? (
                <p className={styles.aviso}>{t("plotinfo.estado.semNumeros")}</p>
              ) : (
                <>
                  <div className={styles.linhaDoCartucho}>
                    <span className={styles.nomeDoCanal}>
                      <PontoDoCanal canal={selecao.cartucho.canal_que_acaba_primeiro} />
                      <span>{t("plotinfo.cartucho.consumoTotal")}</span>
                    </span>
                    <span className={styles.barraDoCanal} aria-hidden="true">
                      <span
                        className={`${styles.barraPreenchida} ${styles[selecao.cartucho.canal_que_acaba_primeiro]}`}
                        style={{
                          width: `${Math.min(
                            100,
                            Math.max(0, selecao.cartucho.percentual_consumido),
                          )}%`,
                        }}
                      />
                    </span>
                    <span className={`${styles.numero} ${styles.percentual}`}>
                      {formatarPercentual(selecao.cartucho.percentual_consumido)}
                    </span>
                  </div>
                  <div className={styles.linhaDeCanal}>
                    <span className={styles.nomeDoCanal}>
                      <PontoDoCanal canal={selecao.cartucho.canal_que_acaba_primeiro} />
                      <span>
                        {t("plotinfo.cartucho.primeiro", {
                          canal: t(CHAVE_DO_CANAL[selecao.cartucho.canal_que_acaba_primeiro]),
                        })}
                      </span>
                    </span>
                  </div>
                  <p className={styles.totalDaSelecao}>
                    {selecao.cartucho.copias_ate_esvaziar === null
                      ? t("plotinfo.cartucho.semConsumo")
                      : t("plotinfo.cartucho.copias", {
                          n: formatarNumero(selecao.cartucho.copias_ate_esvaziar, undefined, 0),
                        })}
                  </p>
                </>
              )}
            </Secao>
          </>
        )}
      </div>

      <footer className={styles.rodape}>
        <button
          type="button"
          className={styles.exportar}
          disabled={
            resultado === null || tinta.situacao === "analisando" || tinta.exportando
          }
          aria-busy={tinta.exportando}
          title={resultado === null ? t("plotinfo.csv.semResultado") : t("plotinfo.csv")}
          onClick={() => void tinta.exportar()}
        >
          <Download size={12} aria-hidden="true" />
          {t(tinta.exportando ? "plotinfo.csv.exportando" : "plotinfo.csv")}
        </button>
        <span>
          {t("plotinfo.rodape", {
            estimativa: t("plotinfo.estimativa"),
            naoAltera: t("plotinfo.naoAltera"),
            perfil: perfilAtivo?.nome ?? resultado?.perfil ?? "—",
            qualidade: t(CHAVE_DA_QUALIDADE[qualidade]),
            modo: t(CHAVE_DO_MODO[modo]),
          })}
        </span>
        <p className={styles.somenteLeitor} role="status" aria-live="polite">
          {tinta.situacao === "pronto" && selecao !== null
            ? t("plotinfo.analise.concluida", {
                n: formatarNumero(selecao.total_com_copias),
              })
            : ""}
        </p>
      </footer>
    </aside>
  );
}

interface Props {
  aberto: boolean;
  aba: Aba | null;
  paginaAtual: number | null;
  tinta: ConteudoDaTinta;
  aoFechar: () => void;
  aoMudarEngrenagem: (aberta: boolean) => void;
  aoSalvarPerfil: (perfil: PerfilDeTinta) => Promise<void>;
  aoApagarPerfil: (id: string) => Promise<void>;
  aoAplicarConfiguracao: (configuracao: ConfiguracaoDeTinta) => Promise<void>;
}

/** Gaveta da estimativa. O dialogo da engrenagem vive aqui, mas o estado da analise vem de cima. */
export function PlotInfoGaveta({
  aberto,
  aba,
  paginaAtual,
  tinta,
  aoFechar,
  aoMudarEngrenagem,
  aoSalvarPerfil,
  aoApagarPerfil,
  aoAplicarConfiguracao,
}: Props) {
  const [engrenagemAberta, setEngrenagemAberta] = useState(false);

  useEffect(() => {
    if (!aberto) setEngrenagemAberta(false);
  }, [aberto]);

  const abrirEngrenagem = () => {
    setEngrenagemAberta(true);
    aoMudarEngrenagem(true);
  };
  const fecharEngrenagem = () => {
    setEngrenagemAberta(false);
    aoMudarEngrenagem(false);
  };
  const fechar = () => {
    setEngrenagemAberta(false);
    aoMudarEngrenagem(false);
    aoFechar();
  };

  if (!aberto) return null;

  return (
    <>
      <PainelDeTinta
        aba={aba}
        paginaAtual={paginaAtual}
        tinta={tinta}
        aoFechar={fechar}
        aoAbrirEngrenagem={abrirEngrenagem}
        engrenagemAberta={engrenagemAberta}
      />
      <PlotInfoEngrenagem
        aberto={engrenagemAberta}
        perfis={tinta.perfis}
        configuracao={tinta.configuracao}
        aviso={tinta.avisoDosPerfis}
        aoFechar={fecharEngrenagem}
        aoSalvarPerfil={aoSalvarPerfil}
        aoApagarPerfil={aoApagarPerfil}
        aoAplicarConfiguracao={aoAplicarConfiguracao}
      />
    </>
  );
}