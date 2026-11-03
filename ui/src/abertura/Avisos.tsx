import { TriangleAlert, X } from "lucide-react";
import { useRef, type FocusEvent, type KeyboardEvent } from "react";
import { t } from "../i18n";
import { mensagemDoAviso, type AvisoDoToast } from "./textos";

/** Devolve o foco a quem o tinha antes de entrar no aviso, ou ao inicio da interface. */
function devolverFoco(origem: HTMLElement | null) {
  const alvo = origem?.isConnected ? origem : document.getElementById("root");
  if (!alvo) return;
  if (alvo.id === "root") alvo.setAttribute("tabindex", "-1");
  alvo.focus({ preventScroll: true });
}

/** O que falhou ou ficou pendente na abertura, em um aviso que o usuario pode dispensar. */
export function Avisos({
  avisos,
  aoDispensar,
}: {
  avisos: AvisoDoToast[];
  aoDispensar: () => void;
}) {
  const contem = useRef<HTMLDivElement>(null);
  const origem = useRef<HTMLElement | null>(null);

  const aoReceberFoco = (evento: FocusEvent<HTMLDivElement>) => {
    const anterior = evento.relatedTarget;
    if (anterior instanceof HTMLElement && !evento.currentTarget.contains(anterior)) {
      origem.current = anterior;
    }
  };

  const dispensar = () => {
    // So mexe no foco se ele estava dentro do aviso; senao o usuario continua onde estava.
    const estavaDentro = contem.current?.contains(document.activeElement) ?? false;
    aoDispensar();
    if (estavaDentro) devolverFoco(origem.current);
  };

  const aoTeclar = (evento: KeyboardEvent<HTMLDivElement>) => {
    if (evento.key !== "Escape") return;
    evento.preventDefault();
    dispensar();
  };

  return (
    <div
      ref={contem}
      className="abertura-avisos"
      role="alert"
      onFocus={aoReceberFoco}
      onKeyDown={aoTeclar}
    >
      <TriangleAlert size={14} aria-hidden="true" />
      <ul>
        {avisos.map((aviso, indice) => (
          <li key={`${aviso.codigo}-${indice}`}>{mensagemDoAviso(aviso)}</li>
        ))}
      </ul>
      <button
        type="button"
        aria-label={t("erro.dispensar")}
        title={t("erro.dispensar")}
        onClick={dispensar}
      >
        <X size={12} aria-hidden="true" />
      </button>
    </div>
  );
}
