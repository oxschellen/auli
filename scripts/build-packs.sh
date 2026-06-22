#!/usr/bin/env bash
# build-packs.sh <entity> — vetoriza os pacotes de uma entidade a partir de data/<id>/.
#
# Decisão #3a do roteiro: os `portal-*.txt` ficam divididos entre `raw/` (gerado pelo scraper:
# servicos, faqs) e `ref/` (autorado: pareceres, notas). Este script **agrega** os dois num
# diretório temporário e chama `auli update --source <agregado> --out data/<id>/packs`. Assim o
# binário `auli` não precisa saber do split — o plumbing fica no script.
#
# Uso:   scripts/build-packs.sh rs
#        VERSION=2 scripts/build-packs.sh sc
set -euo pipefail

ID="${1:?uso: build-packs.sh <entity>}"
ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"   # raiz do repo (auli_new)
DATA="$ROOT/data/$ID"
[ -d "$DATA" ] || { echo "❌ não existe $DATA"; exit 1; }

SRC="$(mktemp -d)"
trap 'rm -rf "$SRC"' EXIT

# Agrega os portal-*.txt de raw/ (gerado) e ref/ (autorado) num único dir de origem.
for d in raw ref; do
  if [ -d "$DATA/$d" ]; then
    cp -n "$DATA/$d"/portal-*.txt "$SRC"/ 2>/dev/null || true
  fi
done
echo "📂 origem agregada ($ID): $(ls "$SRC" | tr '\n' ' ')"

mkdir -p "$DATA/packs"
export EMBED_CACHE_DIR="${EMBED_CACHE_DIR:-$ROOT/auli/models}"
BIN="${AULI_BIN:-$ROOT/auli-server/target/release/auli}"
[ -x "$BIN" ] || { echo "❌ binário não encontrado: $BIN (compile com cargo build --release)"; exit 1; }

"$BIN" update --entity "$ID" --source "$SRC" --out "$DATA/packs" --version "${VERSION:-1}"
echo "✅ packs de '$ID' em $DATA/packs"
