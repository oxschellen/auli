#!/usr/bin/env bash
# rename-engine.sh — renomeia o subfolder do workspace Cargo `auli/` → `auli-engine/`
# e atualiza TODAS as referências (scripts + docs) na raiz do repo.
#
# Rode UMA vez, a partir da raiz do repositório (a pasta que contém `auli/`, `auli-frontend/`,
# `auli-server/` e os `auli_*.md`):
#
#     bash rename-engine.sh
#
# O que faz:
#   1) `git mv auli auli-engine`  (preserva histórico)
#   2) reescreve as referências ao diretório do workspace nos arquivos da raiz
#
# Os scripts INTERNOS do workspace (auli-engine/scripts/*) usam caminhos relativos
# (`$(dirname "$0")/..`), então continuam funcionando sem edição. Nenhum arquivo .rs
# referencia o nome da pasta. `AULI_DATA_DIR=../data` e `PACKS_DIR=../data` continuam
# corretos a partir de `auli-engine/` (mesmo nível de antes).
#
# Idempotente o suficiente: aborta se `auli-engine/` já existir.
set -euo pipefail

# --- pré-condições -----------------------------------------------------------
[ -d auli ]            || { echo "❌ Rode da raiz do repo: não encontrei ./auli"; exit 1; }
[ -f auli/Cargo.toml ] || { echo "❌ ./auli não parece ser o workspace (sem Cargo.toml)"; exit 1; }
[ ! -e auli-engine ]   || { echo "❌ ./auli-engine já existe — nada a fazer."; exit 1; }
command -v git >/dev/null || { echo "❌ git não encontrado no PATH."; exit 1; }

# --- 1) rename do diretório (com histórico) ----------------------------------
echo "📁 git mv auli → auli-engine"
git mv auli auli-engine

# --- 2) atualização das referências na raiz ----------------------------------
# Prosa/comentários "workspace `auli`" → "workspace `auli-engine`" (start_server.sh + docs).
for f in start_server.sh auli_code.md auli_features.md auli_operations.md; do
  [ -f "$f" ] && sed -i 's|workspace `auli`|workspace `auli-engine`|g' "$f"
done

# start_server.sh: caminho do workspace + comentários "roda em auli/".
sed -i 's|WS="\$ROOT/auli"|WS="$ROOT/auli-engine"|' start_server.sh
sed -i 's|roda em auli/,|roda em auli-engine/,|g'    start_server.sh

# scripts/build-packs.sh: default do cache do modelo.
sed -i 's|\$ROOT/auli/models|$ROOT/auli-engine/models|g' scripts/build-packs.sh

# scripts/check-registry-sync.sh: guarda de diretórios mortos.
sed -i 's|auli/crates/auli-collections/src/entities auli/entities|auli-engine/crates/auli-collections/src/entities auli-engine/entities|' scripts/check-registry-sync.sh

# README.md: link da crate + o `cd auli` do passo de build.
sed -i 's|(auli/crates/vector-store/)|(auli-engine/crates/vector-store/)|' README.md
sed -i 's|^cd auli$|cd auli-engine|'                                       README.md

# auli_code.md / auli_pendencias.md: o `cd auli && cargo run ...` (espaço após auli evita casar auli-frontend).
sed -i 's|cd auli |cd auli-engine |g' auli_code.md auli_pendencias.md

# auli_operations.md / auli_pendencias.md: caminhos `auli/models` e `de \`auli/\``.
sed -i 's|`auli/models`|`auli-engine/models`|g'   auli_operations.md
sed -i 's|de `auli/` com|de `auli-engine/` com|g' auli_operations.md
sed -i 's|\$ROOT/auli/models|$ROOT/auli-engine/models|g' auli_pendencias.md

echo "✅ Rename concluído. Revise com:  git status  &&  git diff --staged"
echo "   Sanidade:  grep -rn --include='*.sh' --include='*.md' -E 'ROOT/auli/|cd auli |\\(auli/crates' . | grep -v auli-engine"
echo "   Build:     (cd auli-engine && cargo build --release --bin auli)"
