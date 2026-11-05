import botoes from "@plots/components/Botao.module.css";
import { EmptyState } from "@plots/components/EmptyState";
import {
  Circle,
  CircleAlert,
  CircleCheck,
  CircleX,
  FlipHorizontal2,
  FolderClock,
  LoaderCircle,
  Plus,
  Printer,
  RefreshCw,
  Trash2,
  Undo2,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import styles from "./App.module.css";
import * as backend from "./backend";
import { BarraDeTitulo } from "./components/BarraDeTitulo";
import { DialogoHistorico, DialogoNova, DialogoRecolocar, DialogoRemover } from "./components/Dialogos";
import { esquecerDesenhos, Folha } from "./components/Folha";
import { t, type Chave } from "./i18n";
import {
  acaoLimpar,
  acaoRecolocar,
  acaoVirar,
  dicaDaEspera,
  folhaEscolhida,
  iconeDoResumo,
  nomeDaFila,
  passadaDoVersoRecebida,
  rotuloDeVirar,
  pontoDoEstado,
  resumoDaBandeja,
  temBandeja,
  textoComNumero,
  textoDasOperacoes,
  textoDoAviso,
  textoDoErro,
  type Acao,
  type BandejaTela,
  type EstadoFila,
  type FilaTela,
} from "./tela";

/** O evento avisa na hora; a consulta cobre o aviso que se perder. */
const INTERVALO_DE_CONSULTA = 1500;

type Aberto =
  | { tipo: "nova"; nomeFixo: string | null }
  | { tipo: "remover" }
  | { tipo: "historico" }
  | { tipo: "recolocar" }
  | null;

function Ponto({ estado }: { estado: EstadoFila }) {
  const { forma, cor } = pontoDoEstado(estado);
  return <span aria-hidden="true" className={`${styles.ponto} ${styles[forma]} ${styles[cor]}`} />;
}

function BotaoDoRodape({
  acao,
  rotulo,
  parametros,
  icone,
  primario,
  descricao,
  aoClicar,
}: {
  acao: Acao;
  rotulo: Chave;
  parametros?: Record<string, string>;
  icone: ReactNode;
  primario?: boolean;
  /** O que o botão faz, mesmo quando ligado; o motivo de estar desligado vem junto. */
  descricao?: string;
  aoClicar: () => void;
}) {
  const motivo = acao.motivo === null ? undefined : t(acao.motivo);
  const titulo = [descricao, motivo].filter((parte) => parte !== undefined).join(" ") || undefined;
  return (
    <button
      type="button"
      className={`${botoes.botao} ${primario ? botoes.primario : botoes.secundario}`}
      disabled={!acao.ativa}
      title={titulo}
      aria-description={titulo}
      onClick={aoClicar}
    >
      {icone}
      {t(rotulo, parametros)}
    </button>
  );
}

/** Marcas que o texto traduzido carrega no lugar dos caminhos, para trocá-las por elementos. */
const MARCA_DE_CAMINHO = /\u0000(\w+)\u0000/;

/** Põe cada caminho do texto em um trecho próprio, em fonte monoespaçada. */
function comCaminhos(texto: string, caminhos: Record<string, string>): ReactNode[] {
  return texto.split(MARCA_DE_CAMINHO).map((parte, indice) => {
    if (indice % 2 === 0) return parte;
    const valor = caminhos[parte] ?? "";
    return (
      <span key={indice} className={styles.caminho} title={valor}>
        {valor}
      </span>
    );
  });
}

const marca = (nome: string) => `\u0000${nome}\u0000`;

function CaixaDeErro({ erro, acao }: { erro: { texto: string; detalhe: string | null }; acao?: ReactNode }) {
  return (
    <div role="alert" className={styles.aviso}>
      <CircleAlert size={14} aria-hidden="true" className={styles.avisoIcone} />
      <div className={styles.avisoTexto}>
        <p>{erro.texto}</p>
        {erro.detalhe !== null && <p className={styles.avisoDetalhe}>{erro.detalhe}</p>}
      </div>
      {acao}
    </div>
  );
}

function iconeVazio(): ReactNode {
  return <Printer size={18} />;
}

export function App() {
  const [filas, setFilas] = useState<FilaTela[] | null>(null);
  const [erroDaLista, setErroDaLista] = useState<unknown>(null);
  const [atualizando, setAtualizando] = useState(false);
  const [escolhida, setEscolhida] = useState<string | null>(null);
  const [bandeja, setBandeja] = useState<BandejaTela | null>(null);
  const [erroDaBandeja, setErroDaBandeja] = useState<unknown>(null);
  const [folha, setFolha] = useState<number | null>(null);
  const [viradas, setViradas] = useState<ReadonlySet<number>>(new Set());
  const [aberto, setAberto] = useState<Aberto>(null);
  const [canalFechado, setCanalFechado] = useState(false);
  const escolhidaAgora = useRef(escolhida);
  escolhidaAgora.current = escolhida;

  const fila = filas?.find((candidata) => candidata.nome === escolhida) ?? null;
  const acompanha = temBandeja(fila);
  const acompanhaAgora = useRef(acompanha);
  acompanhaAgora.current = acompanha;

  const aplicarFilas = useCallback((lidas: FilaTela[]) => {
    setFilas(lidas);
    setErroDaLista(null);
    setEscolhida((atual) =>
      atual !== null && lidas.some((candidata) => candidata.nome === atual)
        ? atual
        : (lidas[0]?.nome ?? null),
    );
  }, []);

  const lerFilas = useCallback(() => {
    setAtualizando(true);
    backend
      .listarFilas()
      .then(aplicarFilas)
      .catch(setErroDaLista)
      .finally(() => setAtualizando(false));
  }, [aplicarFilas]);

  const lerBandeja = useCallback(() => {
    const nome = escolhidaAgora.current;
    if (nome === null) return;
    backend
      .verBandeja(nome)
      .then((lida) => {
        // A resposta pode chegar depois de a pessoa trocar de impressora.
        if (escolhidaAgora.current !== nome) return;
        setBandeja(lida);
        setErroDaBandeja(null);
        setFolha((atual) => folhaEscolhida(lida.folhas, atual));
      })
      .catch((falha: unknown) => {
        if (escolhidaAgora.current === nome) setErroDaBandeja(falha);
      });
  }, []);

  const lerEstados = useCallback(() => {
    backend
      .canalFechado()
      .then(setCanalFechado)
      .catch(() => {
        // Sem a resposta, o aviso segue como estava até a próxima consulta.
      });
    backend
      .estadosDasFilas()
      .then((estados) => {
        setFilas((atuais) => {
          if (atuais === null) return atuais;
          let mudou = false;
          const novas = atuais.map((candidata) => {
            const estado = estados[candidata.nome.toLowerCase()];
            if (estado === undefined || estado === candidata.estado) return candidata;
            mudou = true;
            return { ...candidata, estado };
          });
          return mudou ? novas : atuais;
        });
      })
      .catch(() => {
        // Falha ao ler a memória do programa: a próxima consulta tenta de novo.
      });
  }, []);

  useEffect(lerFilas, [lerFilas]);

  useEffect(() => {
    setBandeja(null);
    setErroDaBandeja(null);
    setFolha(null);
    setViradas(new Set());
    if (acompanha) lerBandeja();
  }, [escolhida, acompanha, lerBandeja]);

  useEffect(() => {
    const atualizar = () => {
      lerEstados();
      if (acompanhaAgora.current) lerBandeja();
    };
    let ativo = true;
    let cancelar: (() => void) | null = null;
    backend
      .aoMudar(atualizar)
      .then((funcao) => {
        if (ativo) cancelar = funcao;
        else funcao();
      })
      .catch(() => {
        // Sem o aviso, a consulta periódica mantém a tela em dia.
      });
    const relogio = window.setInterval(atualizar, INTERVALO_DE_CONSULTA);
    return () => {
      ativo = false;
      cancelar?.();
      window.clearInterval(relogio);
    };
  }, [lerBandeja, lerEstados]);

  const limparVista = () => {
    esquecerDesenhos();
    setViradas(new Set());
    lerBandeja();
    lerEstados();
  };

  const virarFolha = () => {
    if (folha === null) return;
    setViradas((atuais) => {
      const novas = new Set(atuais);
      if (!novas.delete(folha)) novas.add(folha);
      return novas;
    });
  };

  const esvaziarBandeja = () => {
    if (escolhida === null) return;
    backend
      .limparBandeja(escolhida)
      .then(limparVista)
      .catch(setErroDaBandeja);
  };

  const folhas = bandeja?.folhas ?? [];
  const escolherFolha = useCallback((numero: number) => setFolha(numero), []);
  const icone = iconeDoResumo(bandeja);
  const versoRecebido = passadaDoVersoRecebida(bandeja);
  const folhaVirada = folha !== null && viradas.has(folha);
  const rotuloVirar = rotuloDeVirar(bandeja, folha, folhaVirada);
  const dica = dicaDaEspera(bandeja);
  const avisoDaBandeja = bandeja?.aviso ? textoDoAviso(bandeja.aviso) : null;
  const notas: string[] = [];
  if (bandeja) {
    if (bandeja.etapa === "na_entrada") notas.push(textoDasOperacoes(bandeja.operacoes));
    if (bandeja.leitura !== null) notas.push(t(`centro.leitura.${bandeja.leitura}`));
    if (bandeja.semInstrucao) notas.push(t("centro.semInstrucao"));
    if (bandeja.conferenciaPorFolha) notas.push(t("centro.porFolha"));
    if (bandeja.paginasSemFolha > 0) {
      notas.push(
        textoComNumero(bandeja.paginasSemFolha, "centro.paginasSemFolha.um", "centro.paginasSemFolha"),
      );
    }
    if (bandeja.folhasSemVerso > 0) {
      notas.push(
        textoComNumero(bandeja.folhasSemVerso, "centro.folhasSemVerso.um", "centro.folhasSemVerso"),
      );
    }
  }

  const conteudoDoCentro = (): ReactNode => {
    if (filas !== null && filas.length === 0 && erroDaLista === null) {
      return (
        <EmptyState icone={iconeVazio()} titulo={t("centro.semFilas.titulo")} texto={t("centro.semFilas.texto")}>
          <button
            type="button"
            className={`${botoes.botao} ${botoes.primario}`}
            onClick={() => setAberto({ tipo: "nova", nomeFixo: null })}
          >
            <Plus size={14} aria-hidden="true" />
            {t("lista.nova")}
          </button>
        </EmptyState>
      );
    }
    if (fila === null) {
      return (
        <EmptyState icone={iconeVazio()} titulo={t("centro.escolha.titulo")} texto={t("centro.escolha.texto")} />
      );
    }
    if (fila.estado === "fora_do_windows") {
      return (
        <EmptyState
          icone={<CircleAlert size={18} />}
          titulo={t("centro.foraDoWindows.titulo")}
          texto={t("centro.foraDoWindows.texto")}
        />
      );
    }
    if (fila.estado === "diferente") {
      return (
        <EmptyState
          icone={<CircleAlert size={18} />}
          titulo={t("centro.diferente.titulo")}
          texto={t("centro.diferente.texto")}
        />
      );
    }
    if (fila.estado === "sem_modo") {
      return (
        <EmptyState
          icone={<CircleAlert size={18} />}
          titulo={t("centro.semModo.titulo")}
          texto={t("centro.semModo.texto")}
        >
          <button
            type="button"
            className={`${botoes.botao} ${botoes.primario}`}
            onClick={() => setAberto({ tipo: "nova", nomeFixo: fila.nome })}
          >
            {t("centro.semModo.acao")}
          </button>
        </EmptyState>
      );
    }
    if (folhas.length === 0) {
      return bandeja?.recebendo ? (
        <EmptyState
          icone={<LoaderCircle size={18} className={styles.girando} />}
          titulo={t("centro.recebendo.titulo")}
          texto={t("centro.recebendo.texto")}
        />
      ) : (
        <EmptyState
          icone={iconeVazio()}
          titulo={t("centro.vazia.titulo")}
          texto={t("centro.vazia.texto", { fila: nomeDaFila(fila.nome) })}
        />
      );
    }
    return null;
  };

  const vazioDoCentro = conteudoDoCentro();

  return (
    <div className={styles.app}>
      <BarraDeTitulo />
      <div className={styles.corpo}>
        <aside className={styles.lateral}>
          <div className={styles.lateralCabecalho}>
            <h2 className={styles.tituloDaSecao}>{t("lista.titulo")}</h2>
            {filas !== null && <span className={styles.contador}>{filas.length}</span>}
            <button
              type="button"
              className={styles.botaoIcone}
              aria-label={t("lista.atualizar")}
              title={t("lista.atualizar")}
              disabled={atualizando}
              onClick={lerFilas}
            >
              <RefreshCw size={14} className={atualizando ? styles.girando : undefined} />
            </button>
          </div>
          <div className={styles.lista}>
            {filas === null && erroDaLista === null && (
              <p className={styles.notaDaLista} role="status">
                {t("lista.lendo")}
              </p>
            )}
            {erroDaLista !== null && (
              <div className={styles.erroDaLista}>
                <CaixaDeErro erro={textoDoErro(erroDaLista)} />
                <button
                  type="button"
                  className={`${botoes.botao} ${botoes.secundario} ${botoes.pequeno}`}
                  onClick={lerFilas}
                >
                  {t("lista.tentar")}
                </button>
              </div>
            )}
            {filas?.length === 0 && erroDaLista === null && (
              <p className={styles.notaDaLista}>{t("lista.vazia")}</p>
            )}
            {filas !== null && filas.length > 0 && (
              <ul className={styles.itens}>
                {filas.map((candidata) => {
                  const estado = t(`estado.${candidata.estado}`);
                  const modo = t(`modo.${candidata.modo}`);
                  const ativa = candidata.nome === escolhida;
                  return (
                    <li key={candidata.nome}>
                      <button
                        type="button"
                        className={`${styles.item} ${ativa ? styles.itemAtivo : ""}`}
                        aria-current={ativa ? "true" : undefined}
                        aria-label={t("lista.rotulo", { nome: candidata.nome, estado, modo })}
                        title={nomeDaFila(candidata.nome)}
                        onClick={() => setEscolhida(candidata.nome)}
                      >
                        <span className={styles.itemNome}>{candidata.nome}</span>
                        <span className={styles.itemEstado}>
                          <Ponto estado={candidata.estado} />
                          {estado}
                        </span>
                        <span className={styles.itemModo}>{modo}</span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
          <div className={styles.lateralRodape}>
            <button
              type="button"
              className={`${botoes.botao} ${botoes.secundario} ${styles.botaoNova}`}
              onClick={() => setAberto({ tipo: "nova", nomeFixo: null })}
            >
              <Plus size={14} aria-hidden="true" />
              {t("lista.nova")}
            </button>
          </div>
        </aside>

        <main className={styles.centro}>
          {fila !== null && (
            <div className={styles.cabecalho}>
              <h1 className={styles.nomeDaFila} title={nomeDaFila(fila.nome)}>
                {fila.nome}
              </h1>
              <span className={styles.modoDaFila}>{t(`modo.${fila.modo}`)}</span>
              <div className={styles.acoesDaFila}>
                <button
                  type="button"
                  className={`${botoes.botao} ${botoes.secundario} ${botoes.pequeno}`}
                  onClick={() => setAberto({ tipo: "historico" })}
                >
                  <FolderClock size={12} aria-hidden="true" />
                  {t("centro.limparHistorico")}
                </button>
                <button
                  type="button"
                  className={`${botoes.botao} ${botoes.secundario} ${botoes.pequeno}`}
                  onClick={() => setAberto({ tipo: "remover" })}
                >
                  <Trash2 size={12} aria-hidden="true" />
                  {t("centro.remover")}
                </button>
              </div>
            </div>
          )}

          <div className={styles.rolagem}>
            {canalFechado && <CaixaDeErro erro={{ texto: t("centro.canalFechado"), detalhe: null }} />}
            {erroDaBandeja !== null && <CaixaDeErro erro={textoDoErro(erroDaBandeja)} />}
            {avisoDaBandeja !== null && <CaixaDeErro erro={avisoDaBandeja} />}
            {bandeja?.gerais.map((motivo) => (
              <CaixaDeErro key={motivo} erro={{ texto: motivo, detalhe: null }} />
            ))}
            {notas.length > 0 && (
              <p className={styles.notas} aria-live="polite">
                {notas.join(" ")}
              </p>
            )}
            {vazioDoCentro !== null ? (
              <div className={styles.vazio}>{vazioDoCentro}</div>
            ) : (
              <>
                <h2 className={styles.tituloDaSecao}>{t("centro.folhas")}</h2>
                {dica !== null && (
                  <p className={styles.notas} aria-live="polite">
                    {t(dica)}
                  </p>
                )}
                <div className={styles.grade}>
                  {fila !== null &&
                    folhas.map((item) => (
                      <Folha
                        key={item.numero}
                        fila={fila.nome}
                        folha={item}
                        selecionada={item.numero === folha}
                        virada={viradas.has(item.numero)}
                        versoRecebido={versoRecebido}
                        aoEscolher={escolherFolha}
                      />
                    ))}
                </div>
              </>
            )}
          </div>

          {fila !== null && (
            <p className={styles.local}>
              {comCaminhos(
                t("centro.pastaDosPdfs", { historico: marca("historico"), porta: marca("porta") }),
                { historico: fila.historico, porta: fila.porta },
              )}{" "}
              {t("centro.semEnvio")}
            </p>
          )}

          <footer className={styles.rodape}>
            <span className={styles.resumo} role="status">
              {bandeja?.recebendo && folhas.length > 0 && (
                <LoaderCircle size={12} aria-hidden="true" className={styles.girando} />
              )}
              {icone === "certo" && (
                <CircleCheck size={14} aria-hidden="true" className={styles.iconeCerto} />
              )}
              {icone === "com_erro" && (
                <CircleX size={14} aria-hidden="true" className={styles.iconeErrado} />
              )}
              {icone === "neutro" && (
                <Circle size={14} aria-hidden="true" className={styles.iconeNeutro} />
              )}
              {resumoDaBandeja(bandeja)}
            </span>
            {icone !== null && <span className={styles.notaDoModelo}>{t("centro.conferidoPeloModelo")}</span>}
            <div className={styles.acoes}>
              <BotaoDoRodape
                acao={acaoVirar(bandeja, folha)}
                rotulo={rotuloVirar.chave}
                parametros={rotuloVirar.parametros}
                icone={<FlipHorizontal2 size={14} aria-hidden="true" />}
                descricao={t("rodape.virar.dica")}
                aoClicar={virarFolha}
              />
              <BotaoDoRodape
                acao={acaoRecolocar(bandeja)}
                rotulo="rodape.recolocar"
                icone={<Undo2 size={14} aria-hidden="true" />}
                primario={acaoRecolocar(bandeja).ativa}
                aoClicar={() => setAberto({ tipo: "recolocar" })}
              />
              <BotaoDoRodape
                acao={acaoLimpar(bandeja)}
                rotulo="rodape.limpar"
                icone={<Trash2 size={14} aria-hidden="true" />}
                aoClicar={esvaziarBandeja}
              />
            </div>
          </footer>
        </main>
      </div>

      {aberto?.tipo === "nova" && (
        <DialogoNova
          nomeFixo={aberto.nomeFixo}
          aoFechar={() => setAberto(null)}
          aoCriar={(lidas, nome) => {
            aplicarFilas(lidas);
            const criada = lidas.find((c) => c.nome.toLowerCase() === nome.toLowerCase());
            if (criada) setEscolhida(criada.nome);
            setAberto(null);
          }}
        />
      )}
      {aberto?.tipo === "remover" && fila !== null && (
        <DialogoRemover
          fila={fila}
          aoFechar={() => setAberto(null)}
          aoRemover={(lidas) => {
            esquecerDesenhos();
            aplicarFilas(lidas);
            setAberto(null);
          }}
        />
      )}
      {aberto?.tipo === "historico" && fila !== null && (
        <DialogoHistorico
          fila={fila}
          aoFechar={() => setAberto(null)}
          aoLimpar={() => {
            limparVista();
            setAberto(null);
          }}
        />
      )}
      {aberto?.tipo === "recolocar" && escolhida !== null && (
        <DialogoRecolocar
          nome={escolhida}
          aoFechar={() => setAberto(null)}
          aoRecolocar={() => {
            setAberto(null);
            lerBandeja();
            lerEstados();
          }}
        />
      )}
    </div>
  );
}
