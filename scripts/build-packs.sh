#!/usr/bin/env bash
# build-packs.sh <entity> — vetoriza os pacotes de uma entidade a partir de data/<id>/raw/.
#
# `auli update` lê o CONTRATO tipado que o scraper grava em data/<id>/raw/
# (<id>-faqs.json e <id>-servicos.json: cada um um `auli_contract::Table<P>`), embedda o campo
# `text_to_embed` de cada registro e escreve os packs em data/<id>/packs/.
#
# `pareceres` é a exceção: desde a G5b a fonte é a ÁRVORE `data/<id>/docs/pareceres/*.md`, lida
# direto pelo `auli update` — não há contrato em `raw/`. Por isso este script, ao final, também deriva
# dela o `<id>-pareceres-index.json` que a tab do frontend consome (ver o bloco no fim do arquivo).
# `notas` ainda não tem fonte struct — o `update` a encontra ausente e simplesmente a pula.
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
export EMBED_CACHE_DIR="${EMBED_CACHE_DIR:-$ROOT/models}"
BIN="${AULI_BIN:-$ROOT/auli-server/target/release/auli}"
[ -x "$BIN" ] || { echo "❌ binário não encontrado: $BIN (compile com cargo build --release)"; exit 1; }

"$BIN" update --entity "$ID" --source "$RAW" --out "$DATA/packs" --version "${VERSION:-1}"
echo "✅ packs de '$ID' em $DATA/packs"

# Índice leve da tab de Pareceres, derivado da MESMA árvore que acabou de ser vetorizada. Fica aqui,
# e não como passo solto, porque este script é inevitável depois de qualquer mexida na árvore (o
# `docs_hash` muda e o boot recusa até ele rodar) — então servidor e frontend nunca divergem de
# estado. Derivação pura: entidade sem árvore de pareceres simplesmente não tem o que derivar.
if [ -d "$DATA/docs/pareceres" ]; then
  COLLECTIONS="${AULI_COLLECTIONS_BIN:-$ROOT/auli-server/target/release/auli-collections}"
  if [ -x "$COLLECTIONS" ]; then
    # O auli-collections resolve `data/` relativo ao CWD, daí o subshell em auli-server/.
    (cd "$ROOT/auli-server" && "$COLLECTIONS" "$ID" indice)
  else
    echo "⚠️  $COLLECTIONS não encontrado — índice de pareceres NÃO foi derivado."
    echo "   Compile (cargo build --release -p auli-collections) e rode: auli-collections $ID indice"
  fi
fi
