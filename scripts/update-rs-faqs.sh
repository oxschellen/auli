#!/usr/bin/env bash
# update-rs-faqs.sh — atualiza as FAQs do RS de ponta a ponta:
#   re-scrape (cache fresco) → re-vetoriza os packs → sobe o servidor.
#
# O scraper é cache-first SEMPRE (o flag --usecache só muda o que fazer no MISS:
# bail vs. rede). Então "re-scrape de verdade" = apagar o cache de faqs antes,
# para que toda página seja MISS e volte da rede. É o que este script faz.
#
# Uso:   scripts/update-rs-faqs.sh              # scrape fresco + rebuild + sobe servidor
#        scripts/update-rs-faqs.sh --keep-cache # NÃO apaga o cache (roda cache-first)
#        scripts/update-rs-faqs.sh --no-serve   # para antes de subir o servidor
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"   # raiz do repo
SERVER="$ROOT/auli-server"
FAQS_CACHE="$ROOT/data/rs/raw/cache/faqs"

FRESH=1
SERVE=1
for arg in "$@"; do
  case "$arg" in
    --keep-cache) FRESH=0 ;;
    --no-serve)   SERVE=0 ;;
    *) echo "❌ argumento desconhecido: $arg"; exit 1 ;;
  esac
done

# 1. Cache fresco — apaga só o cache de faqs (serviços/pareceres ficam intactos).
if [ "$FRESH" -eq 1 ]; then
  if [ -d "$FAQS_CACHE" ]; then
    n="$(find "$FAQS_CACHE" -type f | wc -l | tr -d ' ')"
    echo "🗑️  apagando cache de faqs ($n arquivos) → re-fetch de tudo"
    rm -rf "$FAQS_CACHE"
  else
    echo "ℹ️  sem cache de faqs em $FAQS_CACHE (será populado do zero)"
  fi
else
  echo "ℹ️  --keep-cache: mantendo o cache (scrape cache-first, rede só no miss)"
fi

# 2. Re-scrape das faqs → grava data/rs/raw/rs-faqs.json (o contrato tipado).
echo "🕸️  scrapeando faqs do RS…"
( cd "$SERVER" && cargo run --release -p auli-scraper-rs -- faqs )

# 3. Binários release que o build-packs.sh exige (auli + auli-collections).
echo "🔧 compilando binários release…"
( cd "$SERVER" && cargo build --release -p auli-cli -p auli-collections )

# 4. Re-vetoriza os packs a partir do raw recém-scrapeado (muda o docs_hash;
#    o boot do servidor recusa subir até os packs baterem — daí ser obrigatório).
echo "📦 re-vetorizando os packs do RS…"
"$ROOT/scripts/build-packs.sh" rs

# 5. Sobe o servidor (recompila + serve na 3000).
if [ "$SERVE" -eq 1 ]; then
  echo "🚀 subindo o servidor…"
  ( cd "$ROOT" && ./start_server.sh )
else
  echo "✅ pronto (--no-serve). Suba quando quiser: ./start_server.sh"
fi
