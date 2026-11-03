// Testes da leitura de PNG do gerador de imagens: entradas forjadas falham com erro claro.
import assert from "node:assert/strict";
import { test } from "node:test";
import { crc32, deflateSync } from "node:zlib";
import { decodificarPng } from "../src-tauri/icons/gerar-imagens.mjs";

const ASSINATURA = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

function bloco(tipo, dados) {
  const nome = Buffer.from(tipo, "latin1");
  const tamanho = Buffer.alloc(4);
  tamanho.writeUInt32BE(dados.length);
  const soma = Buffer.alloc(4);
  soma.writeUInt32BE(crc32(Buffer.concat([nome, dados])));
  return Buffer.concat([tamanho, nome, dados, soma]);
}

function png(largura, altura, linhas = Buffer.alloc(0), tipo = 2) {
  const cabecalho = Buffer.alloc(13);
  cabecalho.writeUInt32BE(largura, 0);
  cabecalho.writeUInt32BE(altura, 4);
  cabecalho.set([8, tipo, 0, 0, 0], 8);
  return Buffer.concat([
    ASSINATURA,
    bloco("IHDR", cabecalho),
    bloco("IDAT", deflateSync(linhas)),
    bloco("IEND", Buffer.alloc(0)),
  ]);
}

test("um PNG de 100000 x 100000 e recusado antes de alocar", () => {
  assert.throws(() => decodificarPng(png(100000, 100000)), /fora do limite/);
});

test("largura ou altura zero sao recusadas", () => {
  assert.throws(() => decodificarPng(png(0, 4)), /fora do limite/);
  assert.throws(() => decodificarPng(png(4, 0)), /fora do limite/);
});

test("area acima de 64 milhoes de pixels e recusada", () => {
  assert.throws(() => decodificarPng(png(8192, 8000)), /fora do limite/);
});

test("sem cabecalho e recusado", () => {
  const sem = Buffer.concat([
    ASSINATURA,
    bloco("IDAT", deflateSync(Buffer.alloc(8))),
    bloco("IEND", Buffer.alloc(0)),
  ]);
  assert.throws(() => decodificarPng(sem), /sem cabecalho/);
});

test("bloco que passa do fim do arquivo e recusado", () => {
  const inteiro = png(2, 2);
  assert.throws(() => decodificarPng(inteiro.subarray(0, inteiro.length - 20)), /cortado|sem cabecalho/);
});

test("dados que descomprimem alem do esperado falham sem estourar a memoria", () => {
  assert.throws(() => decodificarPng(png(2, 2, Buffer.alloc(10_000_000))));
});

test("um PNG valido pequeno e lido", () => {
  const linhas = Buffer.from([0, 10, 20, 30, 40, 50, 60, 0, 1, 2, 3, 4, 5, 6]);
  const { largura, altura, rgba } = decodificarPng(png(2, 2, linhas));
  assert.deepEqual([largura, altura], [2, 2]);
  assert.deepEqual([...rgba.subarray(0, 4)], [10, 20, 30, 255]);
});
