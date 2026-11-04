import react from "@vitejs/plugin-react";
import { resolve } from "node:path";
import { defineConfig } from "vitest/config";

const raiz = import.meta.dirname;

export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  resolve: {
    alias: [
      // Tokens e componentes vêm da interface do PlotS, por importação. A fonte que o tema pede é a
      // instalada aqui, para esta interface compilar sem a outra estar instalada.
      {
        find: /^\.\.\/node_modules\/@fontsource\/inter\/(.*)$/,
        replacement: `${resolve(raiz, "node_modules/@fontsource/inter")}/$1`,
      },
      { find: "@plots", replacement: resolve(raiz, "../../../ui/src") },
    ],
    dedupe: ["react", "react-dom"],
  },
  build: {
    target: "chrome110",
    outDir: "dist",
    emptyOutDir: true,
  },
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
