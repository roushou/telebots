import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import viteReact from "@vitejs/plugin-react";
import { nitro } from "nitro/vite";
import { defineConfig } from "vite";

export default defineConfig({
  server: {
    port: 3000,
  },
  plugins: [
    tanstackStart({ srcDirectory: "src" }),
    // react's vite plugin must come after start's vite plugin
    viteReact(),
    // Pin node-server: the runtime image runs `node .output/server/index.mjs`,
    // and an unpinned preset would auto-detect Bun during `bun run build`.
    nitro({ preset: "node-server" }),
  ],
});
