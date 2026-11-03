import { describe, expect, it } from "vitest";
import { FilaDeDesenho, PRIORIDADE_MINIATURA, PRIORIDADE_PAGINA, type Desenho } from "./fila";

function desenhoManual(registro: string[], nome: string) {
  let terminar: () => void = () => undefined;
  let falhar: (motivo: Error) => void = () => undefined;
  const concluido = new Promise<void>((resolver, rejeitar) => {
    terminar = resolver;
    falhar = rejeitar;
  });
  const iniciar = (): Desenho => {
    registro.push(`inicio ${nome}`);
    return { concluido, cancelar: () => registro.push(`cancela ${nome}`) };
  };
  return { iniciar, terminar: () => terminar(), falhar: (motivo: Error) => falhar(motivo) };
}

const passar = () => new Promise((resolver) => setTimeout(resolver, 0));

describe("fila de desenho", () => {
  it("roda um por vez, e a pagina passa na frente da miniatura", async () => {
    const registro: string[] = [];
    const fila = new FilaDeDesenho();
    const a = desenhoManual(registro, "a");
    const miniatura = desenhoManual(registro, "miniatura");
    const pagina = desenhoManual(registro, "pagina");
    fila.pedir(PRIORIDADE_PAGINA, a.iniciar);
    fila.pedir(PRIORIDADE_MINIATURA, miniatura.iniciar);
    fila.pedir(PRIORIDADE_PAGINA, pagina.iniciar);
    expect(registro).toEqual(["inicio a"]);
    a.terminar();
    await passar();
    expect(registro).toEqual(["inicio a", "inicio pagina"]);
    pagina.terminar();
    await passar();
    expect(registro).toEqual(["inicio a", "inicio pagina", "inicio miniatura"]);
  });

  it("desistir de quem espera tira da fila sem iniciar", async () => {
    const registro: string[] = [];
    const fila = new FilaDeDesenho();
    const a = desenhoManual(registro, "a");
    const b = desenhoManual(registro, "b");
    fila.pedir(PRIORIDADE_PAGINA, a.iniciar);
    fila.pedir(PRIORIDADE_PAGINA, b.iniciar)();
    a.terminar();
    await passar();
    expect(registro).toEqual(["inicio a"]);
  });

  it("uma pagina que nunca termina libera a vez ao ser cancelada", () => {
    const registro: string[] = [];
    const fila = new FilaDeDesenho();
    const presa = desenhoManual(registro, "presa");
    const proxima = desenhoManual(registro, "proxima");
    const desistir = fila.pedir(PRIORIDADE_PAGINA, presa.iniciar);
    fila.pedir(PRIORIDADE_PAGINA, proxima.iniciar);
    desistir();
    expect(registro).toEqual(["inicio presa", "cancela presa", "inicio proxima"]);
  });

  it("uma falha no desenho nao para a fila", async () => {
    const registro: string[] = [];
    const fila = new FilaDeDesenho();
    const ruim = desenhoManual(registro, "ruim");
    const boa = desenhoManual(registro, "boa");
    fila.pedir(PRIORIDADE_PAGINA, ruim.iniciar);
    fila.pedir(PRIORIDADE_PAGINA, boa.iniciar);
    ruim.falhar(new Error("pagina quebrada"));
    await passar();
    expect(registro).toEqual(["inicio ruim", "inicio boa"]);
  });

  it("um pedido que falha ao iniciar nao para a fila", () => {
    const registro: string[] = [];
    const fila = new FilaDeDesenho();
    const boa = desenhoManual(registro, "boa");
    fila.pedir(PRIORIDADE_PAGINA, () => {
      throw new Error("sem canvas");
    });
    fila.pedir(PRIORIDADE_PAGINA, boa.iniciar);
    expect(registro).toContain("inicio boa");
  });

  it("a limpeza espera o desenho terminar e segura os novos ate acabar", async () => {
    const registro: string[] = [];
    const fila = new FilaDeDesenho();
    const a = desenhoManual(registro, "a");
    const b = desenhoManual(registro, "b");
    let soltarLimpeza: () => void = () => undefined;
    fila.pedir(PRIORIDADE_PAGINA, a.iniciar);
    fila.quandoOciosa(() => {
      registro.push("limpa");
      return new Promise<void>((resolver) => (soltarLimpeza = resolver));
    });
    fila.pedir(PRIORIDADE_PAGINA, b.iniciar);
    expect(registro).toEqual(["inicio a"]);
    a.terminar();
    await passar();
    expect(registro).toEqual(["inicio a", "limpa"]);
    soltarLimpeza();
    await passar();
    expect(registro).toEqual(["inicio a", "limpa", "inicio b"]);
  });

  it("uma limpeza que falha nao prende a fila", async () => {
    const registro: string[] = [];
    const fila = new FilaDeDesenho();
    const a = desenhoManual(registro, "a");
    fila.quandoOciosa(() => Promise.reject(new Error("ocupada")));
    fila.pedir(PRIORIDADE_PAGINA, a.iniciar);
    await passar();
    expect(registro).toEqual(["inicio a"]);
  });

  it("encerrar cancela o atual e ignora o que vier depois", async () => {
    const registro: string[] = [];
    const fila = new FilaDeDesenho();
    const a = desenhoManual(registro, "a");
    const b = desenhoManual(registro, "b");
    const c = desenhoManual(registro, "c");
    fila.pedir(PRIORIDADE_PAGINA, a.iniciar);
    fila.pedir(PRIORIDADE_PAGINA, b.iniciar);
    fila.encerrar();
    fila.pedir(PRIORIDADE_PAGINA, c.iniciar);
    a.terminar();
    await passar();
    expect(registro).toEqual(["inicio a", "cancela a"]);
  });
});
