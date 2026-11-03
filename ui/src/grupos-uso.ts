import { useCallback, useEffect, useRef, useState, type Dispatch } from "react";
import {
  apagarGrupo,
  aplicarGrupo,
  listarGrupos,
  salvarGrupo,
  textoDaFalhaDoGrupo,
  textoDasAusentes,
  textoDosIgnorados,
  type GrupoAplicado,
  type GrupoSalvo,
  type NotaDaSelecao,
} from "./grupos";
import { t } from "./i18n";
import type { AcaoAbas, ModoDistribuicao } from "./tabs";

export interface UsoDosGrupos {
  grupos: readonly GrupoSalvo[];
  /** O que falhou por ultimo, ja traduzido. */
  falha: string | null;
  /** Quais impressoras do grupo aplicado nao existem mais. */
  ausentes: NotaDaSelecao | null;
  /** O aviso de grupos do arquivo que nao puderam ser usados, ja traduzido. */
  ignorados: string | null;
  /** O anuncio do ultimo grupo apagado, ja traduzido; sai como a nota de quem sumiu. */
  apagado: NotaDaSelecao | null;
  /** Grupo cuja exclusao espera a resposta do usuario. */
  aApagar: string | null;
  /** Devolve se gravou: o campo do nome so fecha quando deu certo. */
  salvar: (nome: string, impressoras: readonly string[], modo: ModoDistribuicao) => Promise<boolean>;
  /** Devolve o que foi aplicado, ou `null` se falhou. */
  aplicar: (aba: number, nome: string) => Promise<GrupoAplicado | null>;
  /** Guarda a selecao de onde se pediu, porque o anuncio fala dela. */
  pedirApagar: (nome: string, aba: number, impressoras: readonly string[]) => void;
  cancelarApagar: () => void;
  confirmarApagar: () => void;
  limparFalha: () => void;
  limparAusentes: () => void;
  limparApagado: () => void;
}

export function useGrupos(despachar: Dispatch<AcaoAbas>): UsoDosGrupos {
  const [grupos, setGrupos] = useState<readonly GrupoSalvo[]>([]);
  const [falha, setFalha] = useState<string | null>(null);
  const [ausentes, setAusentes] = useState<NotaDaSelecao | null>(null);
  const [ignorados, setIgnorados] = useState<string | null>(null);
  const [apagado, setApagado] = useState<NotaDaSelecao | null>(null);
  const [aApagar, setAApagar] = useState<string | null>(null);
  const origemDoApagar = useRef<Pick<NotaDaSelecao, "aba" | "impressoras"> | null>(null);

  useEffect(() => {
    listarGrupos()
      .then((lista) => {
        setGrupos(lista.grupos);
        setIgnorados(textoDosIgnorados(lista.ignorados));
      })
      .catch((erro: unknown) => setFalha(textoDaFalhaDoGrupo(erro)));
  }, []);

  const salvar = useCallback(
    (nome: string, impressoras: readonly string[], modo: ModoDistribuicao) => {
      setAusentes(null);
      setApagado(null);
      return salvarGrupo(nome, impressoras, modo)
        .then((novos) => {
          setGrupos(novos);
          setFalha(null);
          return true;
        })
        .catch((erro: unknown) => {
          setFalha(textoDaFalhaDoGrupo(erro));
          return false;
        });
    },
    [],
  );

  const aplicar = useCallback(
    (aba: number, nome: string) => {
      setAusentes(null);
      setApagado(null);
      return aplicarGrupo(nome)
        .then((aplicado) => {
          setFalha(null);
          const texto = textoDasAusentes(aplicado.ausentes);
          setAusentes(texto === null ? null : { texto, aba, impressoras: aplicado.impressoras });
          despachar({
            tipo: "aplicar_grupo",
            id: aba,
            impressoras: aplicado.impressoras,
            modo: aplicado.modo,
          });
          return aplicado;
        })
        .catch((erro: unknown) => {
          setFalha(textoDaFalhaDoGrupo(erro));
          return null;
        });
    },
    [despachar],
  );

  const confirmarApagar = useCallback(() => {
    if (aApagar === null) return;
    const nome = aApagar;
    const origem = origemDoApagar.current;
    setAApagar(null);
    setAusentes(null);
    setApagado(null);
    apagarGrupo(nome)
      .then((novos) => {
        setGrupos(novos);
        setFalha(null);
        if (origem !== null) setApagado({ texto: t("grupo.apagado", { nome }), ...origem });
      })
      .catch((erro: unknown) => setFalha(textoDaFalhaDoGrupo(erro)));
  }, [aApagar]);

  return {
    grupos,
    falha,
    ausentes,
    ignorados,
    apagado,
    aApagar,
    salvar,
    aplicar,
    pedirApagar: useCallback((nome: string, aba: number, impressoras: readonly string[]) => {
      origemDoApagar.current = { aba, impressoras };
      setAApagar(nome);
    }, []),
    cancelarApagar: useCallback(() => setAApagar(null), []),
    confirmarApagar,
    limparFalha: useCallback(() => setFalha(null), []),
    limparAusentes: useCallback(() => setAusentes(null), []),
    limparApagado: useCallback(() => setApagado(null), []),
  };
}
