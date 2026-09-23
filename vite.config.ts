import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwind from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
import { readFile } from "node:fs/promises";
export default defineConfig({
  root: "web",
  publicDir: "../plugin-dist",
  resolve: {
    alias: {
      "@interview/plugin-sdk": fileURLToPath(new URL("./plugin-sdk", import.meta.url)),
      "@interview/api-client": fileURLToPath(new URL("./api-client/src", import.meta.url)),
    },
  },
  plugins: [
    react(),
    tailwind(),
    {
      name: "local-plugin-assets",
      configureServer(server) {
        server.middlewares.use((request, response, next) => {
          const path = request.url?.split("?")[0] ?? "";
          if (!path.startsWith("/plugins/")) return next();
          if (!/^\/plugins\/[a-z][a-z0-9-]*\/[a-zA-Z0-9_-]+\.(js|css|json)$/.test(path)) {
            response.statusCode = 404;
            response.end();
            return;
          }
          void readFile(new URL(`./plugin-dist${path}`, import.meta.url))
            .then((content) => {
              response.setHeader(
                "Content-Type",
                path.endsWith(".js")
                  ? "text/javascript"
                  : path.endsWith(".css")
                    ? "text/css"
                    : "application/json",
              );
              response.setHeader("Cache-Control", "no-store");
              response.end(content);
            })
            .catch(() => {
              response.statusCode = 404;
              response.end();
            });
        });
      },
    },
  ],
  server: {
    host: "0.0.0.0",
    allowedHosts: ["web"],
    port: Number(process.env.WEB_PORT ?? 5178),
    strictPort: true,
    proxy: {
      "/api": {
        target: process.env.VITE_API_TARGET ?? "http://127.0.0.1:4318",
        changeOrigin: false,
      },
    },
  },
  build: { outDir: "../dist", emptyOutDir: true },
});
