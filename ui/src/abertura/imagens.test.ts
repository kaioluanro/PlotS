import { describe, expect, it } from "vitest";
import { traducoes } from "../i18n";

// Os arquivos de imagem sao lidos do disco; o tipo minimo evita depender dos tipos do Node.
interface Leitor {
  readFileSync(caminho: URL): Uint8Array;
}
interface Descompressor {
  inflateSync(dados: Uint8Array): Uint8Array;
}
const modulofs = "node:fs";
const modulozlib = "node:zlib";
const fs = (await import(/* @vite-ignore */ modulofs)) as Leitor;
const zlib = (await import(/* @vite-ignore */ modulozlib)) as Descompressor;

const abertura = fs.readFileSync(new URL("../../assets/splash.png", import.meta.url));
const marca = fs.readFileSync(new URL("../../assets/marca.png", import.meta.url));
const icone = fs.readFileSync(new URL("../../../src-tauri/icons/icon.ico", import.meta.url));

function u32(dados: Uint8Array, posicao: number, grande: boolean): number {
  const visao = new DataView(dados.buffer, dados.byteOffset, dados.byteLength);
  return visao.getUint32(posicao, !grande);
}

/** Largura, altura e tipo de cor do cabecalho de um PNG. */
function cabecalho(png: Uint8Array) {
  return { largura: u32(png, 16, true), altura: u32(png, 20, true), tipo: png[25] };
}

/** Primeiro pixel RGBA de um PNG de tipo 6: na primeira linha nada o antecede, entao o filtro nao muda o valor. */
function primeiroPixel(png: Uint8Array): number[] {
  const partes: Uint8Array[] = [];
  for (let p = 8; p + 8 <= png.length; ) {
    const tamanho = u32(png, p, true);
    const nome = String.fromCharCode(...png.subarray(p + 4, p + 8));
    if (nome === "IDAT") partes.push(png.subarray(p + 8, p + 8 + tamanho));
    p += 12 + tamanho;
  }
  const todos = new Uint8Array(partes.reduce((soma, parte) => soma + parte.length, 0));
  let deslocamento = 0;
  for (const parte of partes) {
    todos.set(parte, deslocamento);
    deslocamento += parte.length;
  }
  return [...zlib.inflateSync(todos).subarray(1, 5)];
}

describe("imagem da tela de abertura", () => {
  it("tem 960 x 600, na proporcao da janela de 480 x 300", () => {
    const { largura, altura } = cabecalho(abertura);
    expect([largura, altura]).toEqual([960, 600]);
    expect(largura / altura).toBe(480 / 300);
  });

  it("pesa menos de 400 KB", () => {
    expect(abertura.length).toBeLessThan(400 * 1024);
  });

  it("o texto alternativo existe nos dois idiomas e nomeia o programa e o autor", () => {
    expect(traducoes["pt-BR"]["abertura.imagem.alt"]).toBe(
      "PlotS: frente e verso, sempre na ordem. Por Kaio Luan.",
    );
    expect(traducoes.en["abertura.imagem.alt"]).toContain("PlotS");
    expect(traducoes.en["abertura.imagem.alt"]).toContain("Kaio Luan");
    expect(traducoes.en["abertura.imagem.alt"]).not.toBe(
      traducoes["pt-BR"]["abertura.imagem.alt"],
    );
  });
});

// O suavizado da reducao deixa no maximo um traco de cobertura no pixel do canto.
const ALFA_DO_SUAVIZADO = 8;

describe("icone do programa", () => {
  const quantidade = icone[4] === undefined ? 0 : icone[4] + (icone[5] ?? 0) * 256;
  const entradas = Array.from({ length: quantidade }, (_, n) => {
    const base = 6 + 16 * n;
    const lado = icone[base] === 0 ? 256 : (icone[base] ?? 0);
    const tamanho = u32(icone, base + 8, false);
    const inicio = u32(icone, base + 12, false);
    return { lado, png: icone.subarray(inicio, inicio + tamanho) };
  });

  it("traz os sete tamanhos do Windows, cada um em PNG com transparencia", () => {
    expect(entradas.map((e) => e.lado)).toEqual([16, 24, 32, 48, 64, 128, 256]);
    for (const { lado, png } of entradas) {
      expect(cabecalho(png), `${lado} px`).toEqual({ largura: lado, altura: lado, tipo: 6 });
    }
  });

  it("os cantos fora do quadrado arredondado sao transparentes", () => {
    for (const { lado, png } of entradas) {
      expect(primeiroPixel(png)[3] ?? 255, `${lado} px`).toBeLessThanOrEqual(ALFA_DO_SUAVIZADO);
    }
  });
});

describe("marca da barra superior", () => {
  it("e um quadrado com cantos transparentes, nitido ate 24 px em tela de alta resolucao", () => {
    const { largura, altura, tipo } = cabecalho(marca);
    expect(largura).toBe(altura);
    expect(largura).toBeGreaterThanOrEqual(40);
    expect(tipo).toBe(6);
    expect(primeiroPixel(marca)[3] ?? 255).toBeLessThanOrEqual(ALFA_DO_SUAVIZADO);
  });
});
