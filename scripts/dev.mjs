import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const cwd = fileURLToPath(new URL("..", import.meta.url));
const children = [];
let stopping = false;
const env = {
  ...process.env,
  DATABASE_URL:
    process.env.DATABASE_URL ??
    "postgres://dataroom_interview:dataroom_interview@127.0.0.1:5548/dataroom_interview",
};

function stop(code) {
  if (stopping) return;
  stopping = true;
  for (const child of children) {
    if (child.exitCode !== null || !child.pid) continue;
    try {
      if (process.platform === "win32") child.kill("SIGTERM");
      else process.kill(-child.pid, "SIGTERM");
    } catch (error) {
      if (error.code !== "ESRCH") console.error(error.message);
    }
  }
  process.exitCode = code;
}

for (const [command, args] of [
  ["cargo", ["run", "--locked", "--manifest-path", "api/Cargo.toml", "--bin", "dataroom-api"]],
  ["pnpm", ["dev:web"]],
]) {
  const child = spawn(command, args, {
    cwd,
    stdio: "inherit",
    env,
    detached: process.platform !== "win32",
  });
  children.push(child);
  child.on("error", (error) => {
    console.error(error.message);
    stop(1);
  });
  child.on("exit", (code) => stop(code ?? 1));
}

process.on("SIGINT", () => stop(0));
process.on("SIGTERM", () => stop(0));
