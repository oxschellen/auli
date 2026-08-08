#!/usr/bin/env bash
# build-packs.sh <entity> — vetoriza os pacotes de uma entidade a partir de data/<id>/docs/.
#
# `auli update` lê as ÁRVORES `data/<id>/docs/{servicos,faqs,pareceres,tarf}/*.md` — um `.md` por
# documento —, embedda o campo `text_to_embed` de cada registro e escreve os packs em
# data/<id>/packs/. Coleção sem árvore é pulada.
#
# Este script, ao final, também deriva de cada árvore de jurisprudência o `<id>-<colecao>-index.json`
# que a tab do frontend consome (ver o bloco no fim do arquivo).
# `notas` ainda não tem fonte struct — o `update` a encontra ausente e simplesmente a pula.
#
# Uso:   scripts/build-packs.sh rs
#        VERSION=2 scripts/build-packs.sh sc
set -euo pipefail

ID="${1:?uso: build-packs.sh <entity>}"
ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"   # raiz do repo (auli_new)
DATA="$ROOT/data/$ID"
DOCS="$DATA/docs"
[ -d "$DOCS" ] || { echo "❌ não existe $DOCS (rode o scraper e o \`auli-collections $ID process\` antes: geram as árvores .md)"; exit 1; }

echo "📂 origem ($ID): $(for d in "$DOCS"/*/; do [ -d "$d" ] && printf '%s=%s ' "$(basename "$d")" "$(find "$d" -name '*.md' | wc -l)"; done)"

mkdir -p "$DATA/packs"
export EMBED_CACHE_DIR="${EMBED_CACHE_DIR:-$ROOT/models}"
BIN="${AULI_BIN:-$ROOT/auli-server/target/release/auli}"
[ -x "$BIN" ] || { echo "❌ binário não encontrado: $BIN (compile com cargo build --release)"; exit 1; }

"$BIN" update --entity "$ID" --out "$DATA/packs" --version "${VERSION:-1}"
echo "✅ packs de '$ID' em $DATA/packs"

# Índices leves das tabs de jurisprudência, derivados das MESMAS árvores que acabaram de ser
# vetorizadas. Ficam aqui, e não como passo solto, porque este script é inevitável depois de qualquer
# mexida na árvore (o `docs_hash` muda e o boot recusa até ele rodar) — então servidor e frontend
# nunca divergem de estado. Derivação pura: coleção sem árvore simplesmente não tem o que derivar.
COLLECTIONS="${AULI_COLLECTIONS_BIN:-$ROOT/auli-server/target/release/auli-collections}"
for COLECAO in pareceres tarf; do
  [ -d "$DATA/docs/$COLECAO" ] || continue
  if [ -x "$COLLECTIONS" ]; then
    # O auli-collections resolve `data/` relativo ao CWD, daí o subshell em auli-server/.
    (cd "$ROOT/auli-server" && "$COLLECTIONS" "$ID" indice "$COLECAO")
  else
    echo "⚠️  $COLLECTIONS não encontrado — índice de $COLECAO NÃO foi derivado."
    echo "   Compile (cargo build --release -p auli-collections) e rode: auli-collections $ID indice $COLECAO"
  fi
done
