import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import { realpathSync } from "node:fs";
import path from "node:path";

/**
 * Edits to the symlinked `substrate-platform-ui` reload like the app's own:
 * its real folder is watched and each change passed on under the linked path.
 */
function watchSubstrate(): Plugin {
  return {
    name: "yog-installer:watch-substrate",
    configureServer(server) {
      const link = path.resolve("node_modules/substrate-platform-ui");
      const real = realpathSync(link);
      server.watcher.add(path.join(real, "src"));
      for (const event of ["change", "add", "unlink"] as const) {
        server.watcher.on(event, (file) => {
          if (file.startsWith(real + path.sep)) server.watcher.emit(event, link + file.slice(real.length));
        });
      }
    },
  };
}

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react(), watchSubstrate()],
  // One React for the app and the symlinked platform UI.
  resolve: { preserveSymlinks: true, dedupe: ["react", "react-dom"] },
  // The platform UI's CommonJS dependencies, reached only through the
  // symlinked `substrate-platform-ui`: prebundled, or the dev server serves
  // them as ES modules they aren't ("default cannot be resolved").
  optimizeDeps: {
    include: [
      "substrate-platform-ui > markdown-it",
      "substrate-platform-ui > markdown-it-footnote",
      "substrate-platform-ui > markdown-it-github-alerts",
      "substrate-platform-ui > @vscode/markdown-it-katex",
      "substrate-platform-ui > katex",
      "substrate-platform-ui > highlight.js",
      "substrate-platform-ui > dompurify",
      "substrate-platform-ui > mermaid",
    ],
  },
  clearScreen: false,
  server: {
    // Not Yog IDLE's port: both can run in development side by side.
    port: 1430,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1431 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
    headers: { "Cache-Control": "no-store" },
  },
}));
