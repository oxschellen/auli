#!/usr/bin/env bash
# check-registry-sync.sh — guard-rail: data/registry.toml é a FONTE ÚNICA de entidades.
# Falha (exit 1) se:
#   1) o frontend (src/shared/entities.ts) estiver fora de sincronia com o registry, ou
#   2) reaparecer um diretório de definição de entidade fora do registry (a triplicação da §6).
# Barato (node + git; não compila Rust). Bom para CI ou pre-commit.
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
fail=0

# 1) entities.ts regenerado bate com o commitado?
node "$ROOT/scripts/gen-frontend-entities.mjs" >/dev/null
if ! git -C "$ROOT" diff --quiet -- auli-frontend/src/shared/entities.ts; then
  echo "❌ auli-frontend/src/shared/entities.ts fora de sincronia com data/registry.toml."
  echo "   Rode: node scripts/gen-frontend-entities.mjs  (e faça commit)."
  fail=1
fi

# 2) sem cópias de definição de entidade fora do registry
for dead in auli-collections/src/entities auli/entities; do
  if [ -e "$ROOT/$dead" ]; then
    echo "❌ definição de entidade reapareceu fora do registry: $dead"
    fail=1
  fi
done

if [ "$fail" -eq 0 ]; then
  echo "✅ registry sync OK (entities.ts em dia; sem cópias fora do registry)."
fi
exit "$fail"
