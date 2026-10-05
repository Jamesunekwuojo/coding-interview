#!/bin/sh
set -eu
cd /app
for file in package.json pnpm-lock.yaml pnpm-workspace.yaml tsconfig.json vite.config.ts eslint.config.mjs .prettierrc.json .prettierignore; do
  cp "/source/$file" "/app/$file"
done
corepack enable
pnpm install --frozen-lockfile --store-dir /pnpm/store --node-linker=hoisted >&2
exec "$@"
