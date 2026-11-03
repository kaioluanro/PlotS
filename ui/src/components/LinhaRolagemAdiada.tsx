import { ArrowDown, ArrowUp } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";
import { ATRIBUTO_DA_LINHA, espacoAoSumir, rolagensAdiadas, type LeituraDaLinha } from "../cartoes-gesto";
import { t } from "../i18n";
import styles from "./LinhaRolagemAdiada.module.css";

/**
 * Com o ponteiro sobre a lista, ela nao desce sozinha ate o cartao que passou a pedir acao. Esta
 * linha fica fora da area rolavel e diz que ha um cartao a ver, sem mexer em nada sob a mao de quem le.
 */
export function LinhaRolagemAdiada() {
  const quantos = useSyncExternalStore(rolagensAdiadas.assinar, rolagensAdiadas.quantas);
  const posicao = useSyncExternalStore(rolagensAdiadas.assinar, rolagensAdiadas.posicaoDaPrimeira);
  // Altura do espaco que fica sob o ponteiro quando a linha some.
  const [espaco, setEspaco] = useState<number | null>(null);
  const noEspaco = useRef<HTMLDivElement>(null);
  const comEspaco = espaco !== null;
  const botao = useRef<HTMLButtonElement>(null);
  const leitura = useRef<LeituraDaLinha>({ sobPonteiro: false, altura: 0 });

  // O registro avisa antes de a pagina mudar: e a ultima chance de saber se o ponteiro esta na linha.
  useEffect(() => {
    const sair = rolagensAdiadas.assinar(() => {
      const atual = botao.current;
      if (atual) leitura.current = { sobPonteiro: atual.matches(":hover"), altura: atual.offsetHeight };
    });
    return () => {
      sair();
    };
  }, []);

  // Antes de pintar: a lista nao chega a crescer sob o ponteiro.
  useLayoutEffect(() => {
    const altura = espacoAoSumir(quantos, leitura.current);
    if (altura === null) return;
    leitura.current = { sobPonteiro: false, altura: 0 };
    setEspaco(altura);
  }, [quantos]);

  // O ponteiro pode nem ter entrado no espaco: mover em qualquer outro lugar tambem o solta.
  useEffect(() => {
    if (!comEspaco) return;
    const aoMover = (evento: PointerEvent) => {
      if (!noEspaco.current?.contains(evento.target as Node | null)) setEspaco(null);
    };
    document.addEventListener("pointermove", aoMover);
    return () => document.removeEventListener("pointermove", aoMover);
  }, [comEspaco]);

  if (quantos === 0) {
    if (espaco === null) return null;
    return (
      // Inerte de proposito. Sem tirar o foco do cartao, o segundo clique de um clique duplo acaba aqui.
      <div
        ref={noEspaco}
        className={styles.espaco}
        style={{ height: espaco }}
        aria-hidden="true"
        onPointerLeave={() => setEspaco(null)}
        onMouseDown={(evento) => evento.preventDefault()}
      />
    );
  }
  const Seta = posicao === "acima" ? ArrowUp : ArrowDown;
  return (
    <button
      ref={botao}
      type="button"
      className={styles.linha}
      {...{ [ATRIBUTO_DA_LINHA]: "" }}
      onClick={(evento) => rolagensAdiadas.cliqueNaLinha(evento.detail)}
      onMouseDown={(evento) => {
        // O segundo clique de um clique duplo nao tira o foco do cartao que o primeiro abriu.
        if (evento.detail > 1) evento.preventDefault();
      }}
      onPointerLeave={rolagensAdiadas.ponteiroSaiuDaLinha}
    >
      <Seta size={14} aria-hidden="true" />
      <span>
        {quantos === 1 ? t("spooler.rolagem.um") : t("spooler.rolagem.varios", { n: String(quantos) })}
      </span>
    </button>
  );
}
