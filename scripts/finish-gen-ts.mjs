import { readdir, readFile, mkdir, writeFile, rm } from "node:fs/promises";
import path from "node:path";
import prettier from "prettier";
const [source, target, mode] = process.argv.slice(2);
for (const kind of ["handlers", "types"]) {
  const names = (await readdir(path.join(source, kind)))
    .filter((name) => name.endsWith(".ts"))
    .sort();
  if (!names.length) throw new Error(`Gen-TS produced no ${kind}`);
  const generated = new Map();
  for (const name of names) {
    const content = await readFile(path.join(source, kind, name), "utf8");
    generated.set(name, await prettier.format(content, { parser: "typescript", printWidth: 100 }));
  }
  if (mode === "check") {
    const existing = (await readdir(path.join(target, kind)).catch(() => [])).sort();
    if (JSON.stringify(existing) !== JSON.stringify(names))
      throw new Error(`Generated ${kind} file list differs; run make gen-ts`);
    for (const [name, content] of generated) {
      if ((await readFile(path.join(target, kind, name), "utf8")) !== content)
        throw new Error(`Generated ${kind}/${name} differs; run make gen-ts`);
    }
  } else {
    await rm(path.join(target, kind), { recursive: true, force: true });
    await mkdir(path.join(target, kind), { recursive: true });
    for (const [name, content] of generated)
      await writeFile(path.join(target, kind, name), content);
  }
}
console.log(
  mode === "check"
    ? "Gen-TS matches Rust declarations."
    : "Generated TypeScript handlers and DTOs.",
);
