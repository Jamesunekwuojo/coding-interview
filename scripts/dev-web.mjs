import { createServer } from "vite";
import { buildPlugins } from "./build-plugins.mjs";
const watchers = await buildPlugins(true);
const server = await createServer();
await server.listen();
server.printUrls();
let stopping = false;
async function stop() {
  if (stopping) return;
  stopping = true;
  await Promise.all([server.close(), ...watchers.map((watcher) => watcher.close())]);
}
process.on("SIGINT", stop);
process.on("SIGTERM", stop);
