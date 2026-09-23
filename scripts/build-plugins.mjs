import { readdir, readFile, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "vite";
import react from "@vitejs/plugin-react";
import tailwind from "@tailwindcss/vite";
const root = fileURLToPath(new URL("..", import.meta.url));
export async function buildPlugins(watch = false) {
  await rm(path.join(root, "plugin-dist"), { recursive: true, force: true });
  const directories = await readdir(path.join(root, "plugins"), { withFileTypes: true });
  return Promise.all(
    directories
      .filter((entry) => entry.isDirectory())
      .map(async ({ name }) => {
        const directory = path.join(root, "plugins", name);
        const manifestPath = path.join(directory, "manifest.json");
        const readMeta = async () => {
          const value = JSON.parse(await readFile(manifestPath, "utf8"));
          if (value.id !== name || !/^[a-z][a-z0-9-]*$/.test(name) || value.contract !== 1)
            throw new Error(`Invalid plugin: ${name}`);
          return value;
        };
        let meta = await readMeta();
        const output = await build({
          configFile: false,
          root: directory,
          publicDir: false,
          resolve: { alias: { "@interview/plugin-sdk": path.join(root, "plugin-sdk") } },
          define: { "process.env.NODE_ENV": JSON.stringify("production") },
          plugins: [
            react(),
            tailwind(),
            {
              name: "local-plugin-manifest",
              async buildStart() {
                this.addWatchFile(manifestPath);
                meta = await readMeta();
              },
              generateBundle: {
                order: "post",
                handler(_, bundle) {
                  const entry = Object.values(bundle).find(
                    (item) => item.type === "chunk" && item.isEntry,
                  );
                  if (!entry || entry.imports.length || entry.dynamicImports.length)
                    throw new Error("Plugin must be self-contained");
                  const styles = Object.keys(bundle)
                    .filter((file) => file.endsWith(".css"))
                    .map((file) => `/plugins/${name}/${file}`);
                  this.emitFile({
                    type: "asset",
                    fileName: "manifest.json",
                    source: JSON.stringify({
                      ...meta,
                      entry: `/plugins/${name}/${entry.fileName}`,
                      styles,
                    }),
                  });
                },
              },
            },
          ],
          build: {
            outDir: path.join(root, "plugin-dist", "plugins", name),
            emptyOutDir: true,
            watch: watch ? {} : null,
            minify: true,
            cssCodeSplit: false,
            lib: {
              entry: path.join(directory, "ui/index.tsx"),
              formats: ["es"],
              fileName: "entry",
            },
            rollupOptions: {
              output: {
                entryFileNames: "entry-[hash].js",
                assetFileNames: "[name]-[hash][extname]",
                inlineDynamicImports: true,
              },
            },
          },
        });
        if (watch) {
          await new Promise((resolve, reject) => {
            const onEvent = (event) => {
              if (event.code === "END") {
                output.off("event", onEvent);
                resolve();
              }
              if (event.code === "ERROR") {
                output.off("event", onEvent);
                reject(event.error);
              }
            };
            output.on("event", onEvent);
          });
        }
        return output;
      }),
  );
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  await buildPlugins();
