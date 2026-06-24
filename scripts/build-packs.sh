#!/usr/bin/env bash
# build-packs.sh <entity> — vetoriza os pacotes de uma entidade a partir de data/<id>/raw/.
#
# `auli update` lê o CONTRATO tipado que o scraper grava em data/<id>/raw/
# (<id>-faqs.json e <id>-servicos.json: cada um um `auli_contract::Table<P>`), embedda o campo
# `text_to_embed` de cada registro e escreve os packs em data/<id>/packs/.
#
# `pareceres`/`notas` são autorados (sem scraper) e ainda não têm fonte struct — o `update` os
# encontra ausentes e simplesmente os pula. Por isso não há mais agregação de `ref/` + `raw/`:
# a origem é só a pasta `raw/` com os JSON do contrato.
#
# Uso:   scripts/build-packs.sh rs
#        VERSION=2 scripts/build-packs.sh sc
set -euo pipefail

ID="${1:?uso: build-packs.sh <entity>}"
ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"   # raiz do repo (auli_new)
DATA="$ROOT/data/$ID"
RAW="$DATA/raw"
[ -d "$RAW" ] || { echo "❌ não existe $RAW (rode o scraper antes: gera os <id>-<kind>.json do contrato)"; exit 1; }

echo "📂 origem ($ID): $(ls "$RAW"/*.json 2>/dev/null | xargs -r -n1 basename | tr '\n' ' ')"

mkdir -p "$DATA/packs"
export EMBED_CACHE_DIR="${EMBED_CACHE_DIR:-$ROOT/auli/models}"
BIN="${AULI_BIN:-$ROOT/auli-server/target/release/auli}"
[ -x "$BIN" ] || { echo "❌ binário não encontrado: $BIN (compile com cargo build --release)"; exit 1; }

"$BIN" update --entity "$ID" --source "$RAW" --out "$DATA/packs" --version "${VERSION:-1}"
echo "✅ packs de '$ID' em $DATA/packs"
