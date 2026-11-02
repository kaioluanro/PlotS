import { describe, expect, it } from "vitest";
import { comandoDeCalibrar, PROGRAMA } from "./calibrar";

describe("comando de calibrar para o PowerShell", () => {
  it("chama o executavel na pasta atual", () => {
    expect(PROGRAMA).toBe("plots-cli");
    expect(comandoDeCalibrar("Laser")).toBe(".\\plots-cli calibrar 'Laser' --saida .");
  });

  it.each([
    ["Canon G3110", ".\\plots-cli calibrar 'Canon G3110' --saida ."],
    ["Custo $HOME", ".\\plots-cli calibrar 'Custo $HOME' --saida ."],
    ["Sala `n 3", ".\\plots-cli calibrar 'Sala `n 3' --saida ."],
    ["\\\\srv\\Laser 1", ".\\plots-cli calibrar '\\\\srv\\Laser 1' --saida ."],
    ["100% tinta", ".\\plots-cli calibrar '100% tinta' --saida ."],
  ])("%s fica entre aspas simples, sem que nada seja interpretado", (nome, esperado) => {
    expect(comandoDeCalibrar(nome)).toBe(esperado);
  });

  it("dobra toda aspa simples, inclusive as tipograficas", () => {
    expect(comandoDeCalibrar("D'Ávila")).toBe(".\\plots-cli calibrar 'D''Ávila' --saida .");
    expect(comandoDeCalibrar("Ana’s")).toBe(".\\plots-cli calibrar 'Ana’’s' --saida .");
  });

  it("aspas duplas passam sem interpretacao e dobradas", () => {
    expect(comandoDeCalibrar('HP "Sala 2"')).toBe('.\\plots-cli --% calibrar "HP ""Sala 2""" --saida .');
  });

  it("uma aspa dupla no nome nao abre espaco para outro comando", () => {
    const comando = comandoDeCalibrar('x"|calc|"');
    expect(comando).toBe('.\\plots-cli --% calibrar "x""|calc|""" --saida .');
    expect(comando).not.toContain('\\"');
  });

  it("barras invertidas antes de aspas e no fim continuam literais", () => {
    expect(comandoDeCalibrar('a\\"b')).toBe('.\\plots-cli --% calibrar "a\\\\""b" --saida .');
    expect(comandoDeCalibrar('a"b\\')).toBe('.\\plots-cli --% calibrar "a""b\\\\" --saida .');
  });

  it("nome com quebra de linha nunca usa a forma que termina na quebra", () => {
    const comando = comandoDeCalibrar('a"\nrm x');
    expect(comando).not.toContain("--%");
    expect(comando).toBe(".\\plots-cli calibrar 'a\"\nrm x' --saida .");
  });

  it.each(['a"100%b', '"%PATH%"', '%"', 'a"\n%b'])(
    "%j tem aspas duplas e por cento: nenhum comando e gerado",
    (nome) => {
      expect(comandoDeCalibrar(nome)).toBeNull();
    },
  );
});
