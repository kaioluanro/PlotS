import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { OpcaoInversao } from "./components/BlocoAjuste";
import { t } from "./i18n";
import type { Impressora } from "./impressoras";

const CALIBRADA: Impressora = {
  id: "fila:Sala",
  nome: "Sala",
  tipo: "fila",
  nivel: "pronta",
  condicoes: [],
  modo: "calibrada",
  driverInverte: true,
  driverNaCalibracao: false,
};

/** Render do zero, como ao desmarcar e remarcar a impressora: nada do que a tela lembrava sobrevive. */
const aoRemontar = (impressora: Impressora) => renderToStaticMarkup(<OpcaoInversao impressora={impressora} />);

describe("aviso de driver trocado depois da calibracao", () => {
  it("avisa quando a opcao difere do valor da calibracao, mesmo depois de remarcar a impressora", () => {
    expect(aoRemontar(CALIBRADA)).toContain(t("calibracao.driverTrocado"));
  });

  it("sem o valor guardado na calibracao, arquivo antigo, nao avisa", () => {
    const antigo = { ...CALIBRADA, driverNaCalibracao: undefined };
    expect(aoRemontar(antigo)).not.toContain(t("calibracao.driverTrocado"));
  });

  it("com a opcao igual a da calibracao, nao avisa", () => {
    expect(aoRemontar({ ...CALIBRADA, driverInverte: false })).not.toContain(t("calibracao.driverTrocado"));
  });
});
