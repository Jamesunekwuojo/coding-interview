export function pluginPath(base: string, path: string) {
  if (!path.startsWith("/") || path.startsWith("//") || path.includes("\\"))
    throw new Error("Use a plugin-relative path");
  const url = new URL(`${base}${path}`, "http://plugin.local");
  if (!url.pathname.startsWith(`${base}/`))
    throw new Error("Navigation must stay inside the plugin");
  return `${url.pathname}${url.search}${url.hash}`;
}
