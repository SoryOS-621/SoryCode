import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => {
  const isDebug = mode === "development";
  return {
    plugins: [react()],
    clearScreen: false,
    server: {
      port: 5173,
      strictPort: true,
      host: "127.0.0.1",
    },
    envPrefix: ["VITE_", "TAURI_"],
    build: {
      target: "es2021",
      minify: !isDebug,
      sourcemap: isDebug,
      outDir: "dist",
      emptyOutDir: true,
    },
  };
});
