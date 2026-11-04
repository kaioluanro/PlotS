import botoes from "@plots/components/Botao.module.css";
import confirmar from "@plots/components/ConfirmDialog.module.css";
import { LoaderCircle } from "lucide-react";
import { useEffect, useId, useState } from "react";
import * as backend from "../backend";
import { t, type Chave } from "../i18n";
import {
  ESCOLHA_VAZIA,
  escolherEntrada,
  facesDaEntrada,
  formatarTamanho,
  gestoCompleto,
  nomeDaFila,
  textoComNumero,
  type BordaGesto,
  type EntradaGesto,
  type EscolhaDoGesto,
  type FaceGesto,
  type FilaTela,
} from "../tela";
import { Dialogo, ErroDoDialogo } from "./Dialogo";
import estilos from "./Dialogo.module.css";

const girando = <LoaderCircle size={14} aria-hidden="true" className={estilos.andamento} />;

function Opcao({
  nome,
  marcada,
  aoEscolher,
  rotulo,
  dica,
}: {
  nome: string;
  marcada: boolean;
  aoEscolher: () => void;
  rotulo: string;
  dica?: string;
}) {
  return (
    <label className={estilos.opcao}>
      <input type="radio" name={nome} checked={marcada} onChange={aoEscolher} />
      <span className={estilos.opcaoTexto}>
        {rotulo}
        {dica && <span className={estilos.dica}>{dica}</span>}
      </span>
    </label>
  );
}

/** Nova impressora, ou o modo de uma fila que está sem ele: no Windows, criar de novo só guarda o modo. */
export function DialogoNova({
  nomeFixo,
  aoFechar,
  aoCriar,
}: {
  nomeFixo: string | null;
  aoFechar: () => void;
  aoCriar: (filas: FilaTela[], nome: string) => void;
}) {
  const [nome, setNome] = useState(nomeFixo ?? "");
  const [jato, setJato] = useState(true);
  const [criando, setCriando] = useState(false);
  const [erro, setErro] = useState<unknown>(null);
  const idNome = useId();
  const idDica = useId();
  const grupo = useId();

  const criar = () => {
    setCriando(true);
    setErro(null);
    backend
      .criarFila(nome, jato)
      .then((filas) => aoCriar(filas, nome))
      .catch((falha: unknown) => {
        setErro(falha);
        setCriando(false);
      });
  };

  return (
    <Dialogo
      titulo={nomeFixo === null ? t("nova.titulo") : t("nova.tituloDefinir", { nome: nomeFixo })}
      aoFechar={aoFechar}
      setasRolam={false}
      rodape={
        <>
          <button type="button" className={`${botoes.botao} ${botoes.secundario}`} onClick={aoFechar}>
            {t("nova.cancelar")}
          </button>
          <button
            type="submit"
            form="formulario-nova"
            className={`${botoes.botao} ${botoes.primario}`}
            disabled={criando || nome.length === 0}
          >
            {criando && girando}
            {criando ? t("nova.criando") : t(nomeFixo === null ? "nova.criar" : "nova.salvar")}
          </button>
        </>
      }
    >
      <form
        id="formulario-nova"
        className={estilos.formulario}
        onSubmit={(evento) => {
          evento.preventDefault();
          if (!criando && nome.length > 0) criar();
        }}
      >
        <p className={confirmar.texto}>{t("nova.texto")}</p>
        {nomeFixo === null && (
          <div className={estilos.campo}>
            <label className={estilos.rotulo} htmlFor={idNome}>
              {t("nova.nome")}
            </label>
            <div className={estilos.prefixo}>
              <span className={estilos.prefixoTexto} aria-hidden="true">
                {nomeDaFila("")}
              </span>
              <input
                id={idNome}
                className={`${estilos.entrada} ${erro ? estilos.entradaComErro : ""}`}
                value={nome}
                maxLength={64}
                autoComplete="off"
                spellCheck={false}
                aria-describedby={idDica}
                aria-invalid={erro !== null}
                onChange={(evento) => setNome(evento.target.value)}
              />
            </div>
            <p id={idDica} className={estilos.dica}>
              {t("nova.nome.dica")}
            </p>
          </div>
        )}
        <fieldset className={estilos.campo}>
          <legend className={estilos.rotulo}>{t("nova.modo")}</legend>
          <div className={`${estilos.opcoes} ${estilos.opcoesEmColuna}`}>
            <Opcao
              nome={grupo}
              marcada={jato}
              aoEscolher={() => setJato(true)}
              rotulo={t("modo.jato")}
              dica={t("modo.jato.dica")}
            />
            <Opcao
              nome={grupo}
              marcada={!jato}
              aoEscolher={() => setJato(false)}
              rotulo={t("modo.laser")}
              dica={t("modo.laser.dica")}
            />
          </div>
        </fieldset>
        {erro !== null && <ErroDoDialogo erro={erro} />}
      </form>
    </Dialogo>
  );
}

/** Lê quantos PDFs a fila guarda ao abrir. Sem resposta, o diálogo não trava: mostra o que sabe. */
function useResumoDoHistorico(nome: string) {
  const [resumo, setResumo] = useState<{ arquivos: number; bytes: number } | null>(null);
  const [erro, setErro] = useState<unknown>(null);
  useEffect(() => {
    let ativo = true;
    backend
      .resumoDoHistorico(nome)
      .then((lido) => {
        if (ativo) setResumo(lido);
      })
      .catch((falha: unknown) => {
        if (ativo) setErro(falha);
      });
    return () => {
      ativo = false;
    };
  }, [nome]);
  return { resumo, erro };
}

export function DialogoRemover({
  fila,
  aoFechar,
  aoRemover,
}: {
  fila: FilaTela;
  aoFechar: () => void;
  aoRemover: (filas: FilaTela[]) => void;
}) {
  const { resumo, erro: erroDoResumo } = useResumoDoHistorico(fila.nome);
  const [apagar, setApagar] = useState(false);
  const [removendo, setRemovendo] = useState(false);
  const [erro, setErro] = useState<unknown>(null);
  const temPdfs = resumo !== null && resumo.arquivos > 0;

  const remover = () => {
    setRemovendo(true);
    setErro(null);
    backend
      .removerFila(fila.nome, apagar && temPdfs)
      .then(aoRemover)
      .catch((falha: unknown) => {
        setErro(falha);
        setRemovendo(false);
      });
  };

  return (
    <Dialogo
      titulo={t("remover.titulo", { nome: fila.nome })}
      aoFechar={aoFechar}
      rodape={
        <>
          <button type="button" className={`${botoes.botao} ${botoes.secundario}`} onClick={aoFechar}>
            {t("remover.manter")}
          </button>
          <button
            type="button"
            className={`${botoes.botao} ${botoes.perigo}`}
            disabled={removendo}
            onClick={remover}
          >
            {removendo && girando}
            {removendo ? t("remover.removendo") : t("remover.confirmar")}
          </button>
        </>
      }
    >
      <p className={confirmar.texto}>{t("remover.texto", { fila: nomeDaFila(fila.nome) })}</p>
      {resumo === null && erroDoResumo === null && (
        <p className={confirmar.nota}>{t("historico.lendo")}</p>
      )}
      {resumo !== null && !temPdfs && (
        <p className={confirmar.nota}>{t("remover.historicoVazio")}</p>
      )}
      {temPdfs && (
        <>
          <p className={confirmar.nota}>
            {textoComNumero(resumo.arquivos, "remover.historico.um", "remover.historico", {
              tamanho: formatarTamanho(resumo.bytes),
              pasta: fila.historico,
            })}
          </p>
          <label className={estilos.opcao}>
            <input
              type="checkbox"
              checked={apagar}
              onChange={(evento) => setApagar(evento.target.checked)}
            />
            {t("remover.apagarHistorico")}
          </label>
        </>
      )}
      {erroDoResumo !== null && <ErroDoDialogo erro={erroDoResumo} />}
      {erro !== null && <ErroDoDialogo erro={erro} />}
    </Dialogo>
  );
}

export function DialogoHistorico({
  fila,
  aoFechar,
  aoLimpar,
}: {
  fila: FilaTela;
  aoFechar: () => void;
  aoLimpar: () => void;
}) {
  const { resumo, erro: erroDoResumo } = useResumoDoHistorico(fila.nome);
  const [apagando, setApagando] = useState(false);
  const [erro, setErro] = useState<unknown>(null);
  const temPdfs = resumo !== null && resumo.arquivos > 0;

  const limpar = () => {
    setApagando(true);
    setErro(null);
    backend
      .limparHistorico(fila.nome)
      .then(aoLimpar)
      .catch((falha: unknown) => {
        setErro(falha);
        setApagando(false);
      });
  };

  return (
    <Dialogo
      titulo={t("historico.titulo", { nome: fila.nome })}
      aoFechar={aoFechar}
      rodape={
        temPdfs ? (
          <>
            <button
              type="button"
              className={`${botoes.botao} ${botoes.secundario}`}
              onClick={aoFechar}
            >
              {t("historico.manter")}
            </button>
            <button
              type="button"
              className={`${botoes.botao} ${botoes.perigo}`}
              disabled={apagando}
              onClick={limpar}
            >
              {apagando && girando}
              {apagando ? t("historico.apagando") : t("historico.confirmar")}
            </button>
          </>
        ) : (
          <button type="button" className={`${botoes.botao} ${botoes.secundario}`} onClick={aoFechar}>
            {t("historico.fechar")}
          </button>
        )
      }
    >
      {resumo === null && erroDoResumo === null && (
        <p className={confirmar.texto}>{t("historico.lendo")}</p>
      )}
      {resumo !== null && !temPdfs && <p className={confirmar.texto}>{t("historico.vazio")}</p>}
      {temPdfs && (
        <p className={confirmar.texto}>
          {textoComNumero(resumo.arquivos, "historico.texto.um", "historico.texto", {
            tamanho: formatarTamanho(resumo.bytes),
            pasta: fila.historico,
          })}
        </p>
      )}
      {erroDoResumo !== null && <ErroDoDialogo erro={erroDoResumo} />}
      {erro !== null && <ErroDoDialogo erro={erro} />}
    </Dialogo>
  );
}

const ENTRADAS: readonly EntradaGesto[] = ["traseira", "gaveta", "manual"];
const BORDAS: readonly BordaGesto[] = ["topo", "base"];

/** A pessoa lê o cartão no PlotS e marca o gesto aqui. Nada vem marcado: isso testa o cartão. */
export function DialogoRecolocar({
  nome,
  aoFechar,
  aoRecolocar,
}: {
  nome: string;
  aoFechar: () => void;
  aoRecolocar: () => void;
}) {
  const [escolha, setEscolha] = useState<EscolhaDoGesto>(ESCOLHA_VAZIA);
  const [erro, setErro] = useState<unknown>(null);
  const grupo = useId();
  const gesto = gestoCompleto(escolha);
  const faces = facesDaEntrada(escolha.entrada);

  const confirmarGesto = () => {
    if (gesto === null) return;
    setErro(null);
    backend.recolocar(nome, gesto).then(aoRecolocar).catch(setErro);
  };

  return (
    <Dialogo
      titulo={t("gesto.titulo")}
      aoFechar={aoFechar}
      setasRolam={false}
      largura="larga"
      rodape={
        <>
          {gesto === null && <p className={confirmar.retorno}>{t("gesto.falta")}</p>}
          <button type="button" className={`${botoes.botao} ${botoes.secundario}`} onClick={aoFechar}>
            {t("gesto.cancelar")}
          </button>
          <button
            type="button"
            className={`${botoes.botao} ${botoes.primario}`}
            disabled={gesto === null}
            onClick={confirmarGesto}
          >
            {t("gesto.confirmar")}
          </button>
        </>
      }
    >
      <p className={confirmar.texto}>{t("gesto.texto")}</p>
      <fieldset className={estilos.campo}>
        <legend className={estilos.rotulo}>{t("gesto.entrada")}</legend>
        <div className={estilos.opcoes}>
          {ENTRADAS.map((entrada) => (
            <Opcao
              key={entrada}
              nome={`${grupo}-entrada`}
              marcada={escolha.entrada === entrada}
              aoEscolher={() => setEscolha((atual) => escolherEntrada(atual, entrada))}
              rotulo={t(`gesto.entrada.${entrada}` as Chave)}
            />
          ))}
        </div>
      </fieldset>
      <fieldset className={estilos.campo}>
        <legend className={estilos.rotulo}>{t("gesto.face")}</legend>
        {faces.length === 0 ? (
          <p className={estilos.dica}>{t("gesto.face.semEntrada")}</p>
        ) : (
          <div className={estilos.opcoes}>
            {faces.map((face: FaceGesto) => (
              <Opcao
                key={face}
                nome={`${grupo}-face`}
                marcada={escolha.faceImpressa === face}
                aoEscolher={() => setEscolha((atual) => ({ ...atual, faceImpressa: face }))}
                rotulo={t(`gesto.face.${face}` as Chave)}
              />
            ))}
          </div>
        )}
      </fieldset>
      <fieldset className={estilos.campo}>
        <legend className={estilos.rotulo}>{t("gesto.borda")}</legend>
        <div className={estilos.opcoes}>
          {BORDAS.map((borda) => (
            <Opcao
              key={borda}
              nome={`${grupo}-borda`}
              marcada={escolha.bordaQueEntra === borda}
              aoEscolher={() => setEscolha((atual) => ({ ...atual, bordaQueEntra: borda }))}
              rotulo={t(`gesto.borda.${borda}` as Chave)}
            />
          ))}
        </div>
      </fieldset>
      <fieldset className={estilos.campo}>
        <legend className={estilos.rotulo}>{t("gesto.ordem")}</legend>
        <div className={estilos.opcoes}>
          <Opcao
            nome={`${grupo}-ordem`}
            marcada={escolha.inverterOrdem === false}
            aoEscolher={() => setEscolha((atual) => ({ ...atual, inverterOrdem: false }))}
            rotulo={t("gesto.ordem.manter")}
          />
          <Opcao
            nome={`${grupo}-ordem`}
            marcada={escolha.inverterOrdem === true}
            aoEscolher={() => setEscolha((atual) => ({ ...atual, inverterOrdem: true }))}
            rotulo={t("gesto.ordem.inverter")}
          />
        </div>
      </fieldset>
      {erro !== null && <ErroDoDialogo erro={erro} />}
    </Dialogo>
  );
}
