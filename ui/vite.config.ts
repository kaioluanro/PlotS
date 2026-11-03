import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  build: {
    target: "chrome110",
    outDir: "dist",
    emptyOutDir: true,
    // Duas paginas: a janela principal e a tela de abertura.
    rollupOptions: { input: { main: "index.html", abertura: "abertura.html" } },
  },
});
