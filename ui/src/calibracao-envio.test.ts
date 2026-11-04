import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Acao, PerfilDaRodada } from "./calibracao";

const backend = vi.hoisted(() => ({
  enviarRodada: vi.fn<(id: string, perfil: unknown, papel: string) => Promise<boolean>>(),
}));

vi.mock("./calibracao-backend", async (original) => ({
  ...(await original<typeof import("./calibracao-backend")>()),
  ...backend,
}));

const comandos = vi.hoisted(() => ({
  comandarTrabalho: vi.fn<(id: string, comando: string) => Promise<unknown>>(),
}));

vi.mock("./trabalhos", () => comandos);

import { criarTrava, enviarVerso, iniciarRodada } from "./calibracao-envio";

const PERFIL: PerfilDaRodada = {
  frenteInversa: true,
  versoInverso: true,
  versoGirado: false,
  virada: "lateral",
  borda: "curta",
  driverInverte: false,
};

/** Uma promessa que so resolve quando a teste manda. */
function adiada<T>() {
  let resolver: (valor: T) => void = () => undefined;
  const promessa = new Promise<T>((ok) => {
    resolver = ok;
  });
  return { promessa, resolver };
}

beforeEach(() => {
  backend.enviarRodada.mockReset();
});

describe("trava da tela", () => {
  it("fica ocupada enquanto a tarefa corre, e solta quando ela termina", async () => {
    const mudancas: (string | null)[] = [];
    const trava = criarTrava((ocupacao) => mudancas.push(ocupacao));
    const { promessa, resolver } = adiada<void>();
    expect(trava.executar("gravando", () => promessa)).toBe(true);
    expect(trava.atual()).toBe("gravando");
    resolver();
    await promessa;
    await new Promise((ok) => setTimeout(ok, 0));
    expect(trava.atual()).toBeNull();
    expect(mudancas).toEqual(["gravando", null]);
  });

  it("um segundo pedido no mesmo instante nao roda a tarefa", () => {
    const trava = criarTrava(() => undefined);
    const tarefa = vi.fn(() => new Promise<void>(() => undefined));
    expect(trava.executar("enviando", tarefa)).toBe(true);
    expect(trava.executar("enviando", tarefa)).toBe(false);
    expect(tarefa).toHaveBeenCalledTimes(1);
  });

  it("solta a trava mesmo quando a tarefa falha", async () => {
    const trava = criarTrava(() => undefined);
    trava.executar("gravando", () => Promise.reject(new Error("falhou")));
    await new Promise((ok) => setTimeout(ok, 0));
    expect(trava.atual()).toBeNull();
  });
});

describe("iniciar uma rodada", () => {
  it("clique duplo em Imprimir manda uma so vez, e o segundo nao vira alerta de ocupada", async () => {
    const trava = criarTrava(() => undefined);
    const despachar = vi.fn<(acao: Acao) => void>();
    const { promessa, resolver } = adiada<boolean>();
    backend.enviarRodada.mockReturnValue(promessa);
    const perfil = () => Promise.resolve(PERFIL);
    const primeiro = iniciarRodada({ trava, id: "fila:Sala", papel: "a4", rodada: 1, perfil, despachar });
    const segundo = iniciarRodada({ trava, id: "fila:Sala", papel: "a4", rodada: 1, perfil, despachar });
    expect(primeiro).toBe(true);
    expect(segundo).toBe(false);
    resolver(true);
    await new Promise((ok) => setTimeout(ok, 0));
    expect(backend.enviarRodada).toHaveBeenCalledTimes(1);
    expect(despachar).toHaveBeenCalledWith({ tipo: "rodada_iniciada", perfil: PERFIL, rodada: 1 });
    expect(despachar).not.toHaveBeenCalledWith({ tipo: "recusada", recusa: "ocupada" });
    expect(trava.atual()).toBeNull();
  });

  it("impressora que recusa vira ocupada, e falha de envio vira a sua frase", async () => {
    const trava = criarTrava(() => undefined);
    const despachar = vi.fn<(acao: Acao) => void>();
    backend.enviarRodada.mockResolvedValueOnce(false);
    iniciarRodada({ trava, id: "fila:Sala", papel: "a4", rodada: 1, perfil: () => Promise.resolve(PERFIL), despachar });
    await new Promise((ok) => setTimeout(ok, 0));
    expect(despachar).toHaveBeenLastCalledWith({ tipo: "recusada", recusa: "ocupada" });

    backend.enviarRodada.mockRejectedValueOnce(new Error("sem resposta"));
    iniciarRodada({ trava, id: "fila:Sala", papel: "a4", rodada: 1, perfil: () => Promise.resolve(PERFIL), despachar });
    await new Promise((ok) => setTimeout(ok, 0));
    expect(despachar).toHaveBeenLastCalledWith({ tipo: "recusada", recusa: "falha_envio" });
    expect(trava.atual()).toBeNull();
  });

  it("perfil que nao pode ser preparado vira falha de perfil, e nada e enviado", async () => {
    const trava = criarTrava(() => undefined);
    const despachar = vi.fn<(acao: Acao) => void>();
    iniciarRodada({
      trava,
      id: "fila:Sala",
      papel: "a4",
      rodada: 1,
      perfil: () => Promise.reject(new Error("sem perfil")),
      despachar,
    });
    await new Promise((ok) => setTimeout(ok, 0));
    expect(despachar).toHaveBeenLastCalledWith({ tipo: "recusada", recusa: "falha_perfil" });
    expect(backend.enviarRodada).not.toHaveBeenCalled();
    expect(trava.atual()).toBeNull();
  });
});

describe("enviar o verso", () => {
  it("clique duplo em Imprimir verso manda o comando uma vez so, e o segundo nao vira falha", async () => {
    const trava = criarTrava(() => undefined);
    const despachar = vi.fn<(acao: Acao) => void>();
    const { promessa, resolver } = adiada<unknown>();
    comandos.comandarTrabalho.mockReturnValue(promessa);
    const primeiro = enviarVerso({ trava, id: "fila:Sala", despachar });
    const segundo = enviarVerso({ trava, id: "fila:Sala", despachar });
    expect(primeiro).toBe(true);
    expect(segundo).toBe(false);
    resolver(undefined);
    await new Promise((ok) => setTimeout(ok, 0));
    expect(comandos.comandarTrabalho).toHaveBeenCalledTimes(1);
    expect(comandos.comandarTrabalho).toHaveBeenCalledWith("fila:Sala", "enviar_verso");
    expect(despachar).not.toHaveBeenCalled();
    expect(trava.atual()).toBeNull();
  });

  it("falha ao enviar o verso vira verso_falhou e solta a trava", async () => {
    const trava = criarTrava(() => undefined);
    const despachar = vi.fn<(acao: Acao) => void>();
    comandos.comandarTrabalho.mockRejectedValueOnce(new Error("sem resposta"));
    enviarVerso({ trava, id: "fila:Sala", despachar });
    await new Promise((ok) => setTimeout(ok, 0));
    expect(despachar).toHaveBeenCalledWith({ tipo: "verso_falhou" });
    expect(trava.atual()).toBeNull();
  });
});

describe("cada chamada comeca limpando a mensagem anterior", () => {
  it("iniciar uma rodada despacha a limpeza antes do envio", async () => {
    const trava = criarTrava(() => undefined);
    const despachar = vi.fn<(acao: Acao) => void>();
    backend.enviarRodada.mockResolvedValueOnce(true);
    iniciarRodada({ trava, id: "fila:Sala", papel: "a4", rodada: 1, perfil: () => Promise.resolve(PERFIL), despachar });
    await new Promise((ok) => setTimeout(ok, 0));
    expect(despachar.mock.calls[0]?.[0]).toEqual({ tipo: "chamada_iniciada" });
  });
});
