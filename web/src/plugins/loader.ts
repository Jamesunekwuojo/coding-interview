import type { Plugin, PluginManifest } from "@interview/plugin-sdk";
export function parseManifest(value: unknown, id: string): PluginManifest {
  if (!value || typeof value !== "object") throw new Error("Invalid plugin manifest");
  const manifest = value as Partial<PluginManifest>;
  const allowed = (path: unknown): path is string =>
    typeof path === "string" && new RegExp(`^/plugins/${id}/[a-zA-Z0-9_-]+\\.(js|css)$`).test(path);
  if (
    !/^[a-z][a-z0-9-]*$/.test(id) ||
    manifest.id !== id ||
    manifest.contract !== 1 ||
    typeof manifest.name !== "string" ||
    !allowed(manifest.entry) ||
    !manifest.entry.endsWith(".js") ||
    !Array.isArray(manifest.styles) ||
    !manifest.styles.every((path) => allowed(path) && path.endsWith(".css"))
  )
    throw new Error("Invalid plugin manifest");
  return manifest as PluginManifest;
}
export async function loadPlugin(manifestUrl: string, id: string, signal: AbortSignal) {
  if (manifestUrl !== `/plugins/${id}/manifest.json`) throw new Error("Invalid manifest location");
  const response = await fetch(manifestUrl, { signal, cache: "no-store" });
  if (!response.ok) throw new Error("Plugin manifest unavailable");
  const manifest = parseManifest(await response.json(), id);
  const module = await import(/* @vite-ignore */ manifest.entry);
  const plugin: Plugin = module.default;
  if (
    !plugin ||
    ![plugin.mount, plugin.update, plugin.unmount].every((value) => typeof value === "function")
  )
    throw new Error("Plugin contract is missing");
  return { manifest, plugin };
}
