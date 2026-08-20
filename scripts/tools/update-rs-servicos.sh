#!/usr/bin/env bash
# update-rs-servicos.sh — atualiza os SERVIÇOS do RS de ponta a ponta:
#   re-scrape (cache fresco) → deriva a árvore → re-vetoriza os packs → sobe o servidor.
#
# Irmão do `update-rs-faqs.sh`, mesma forma e mesmas flags, trocando a coleção. O scraper é
# cache-first SEMPRE (o `--usecache` só muda o que fazer no MISS: bail vs. rede), então "scrape do
# zero" = apagar o cache de serviços antes, para que as ~600 páginas voltem todas da rede.
#
# Uso:   scripts/tools/update-rs-servicos.sh              # scrape do zero + rebuild + sobe servidor
#        scripts/tools/update-rs-servicos.sh --keep-cache # NÃO apaga o cache (roda cache-first)
#        scripts/tools/update-rs-servicos.sh --no-serve   # para antes de subir o servidor
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/../.." && pwd)"   # raiz do repo
SERVER="$ROOT/auli-server"
CACHE="$ROOT/data/rs/raw/cache/servicos"
SNAPSHOT="$ROOT/data/rs/rs-servicos-snapshot.json"

FRESH=1
SERVE=1
for arg in "$@"; do
  case "$arg" in
    --keep-cache) FRESH=0 ;;
    --no-serve)   SERVE=0 ;;
    *) echo "❌ argumento desconhecido: $arg"; exit 1 ;;
  esac
done

# Quantos serviços o snapshot atual tem — lido ANTES de qualquer coisa ser sobrescrita. É a linha de
# base do guard do passo 3; sem snapshot anterior (primeira coleta) não há com o que comparar.
ANTES=0
[ -f "$SNAPSHOT" ] && ANTES="$(jq '.coleta.items | length' "$SNAPSHOT")"

# 1. Cache fresco — apaga só o cache de serviços. Os irmãos `faqs`, `pareceres` e `tarf` (1,5 GB)
#    são namespaces separados e ficam intactos.
if [ "$FRESH" -eq 1 ]; then
  if [ -d "$CACHE" ]; then
    n="$(find "$CACHE" -type f | wc -l | tr -d ' ')"
    echo "🗑️  apagando cache de serviços ($n arquivos) → re-fetch de tudo"
    rm -rf "$CACHE"
  else
    echo "ℹ️  sem cache de serviços em $CACHE (será populado do zero)"
  fi
else
  echo "ℹ️  --keep-cache: mantendo o cache (scrape cache-first, rede só no miss)"
fi

# 2. Re-scrape → grava data/rs/rs-servicos-snapshot.json (o snapshot da coleta).
echo "🕸️  scrapeando serviços do RS…"
( cd "$SERVER" && cargo run --release -p auli-scraper-rs -- servicos )

# 3. Guard de completude. O scraper NÃO falha quando páginas de detalhe caem: ele lista as falhas em
#    stderr e grava o snapshot mesmo assim (`report_failed_detail_urls`), e um público cujo arquivo
#    per-tipo não existir é simplesmente pulado na agregação (`load_per_tipo`). Com o cache apagado
#    são ~600 idas à rede, ou seja, ~600 chances de perder serviço em silêncio — e daí em diante
#    tudo é derivação fiel: a árvore encolhe, o pack encolhe, o boot valida e o servidor sobe
#    servindo menos do que existe. O `set -e` não pega nada disso, então o portão é aqui.
DEPOIS="$(jq '.coleta.items | length' "$SNAPSHOT")"
echo "🔢 serviços no snapshot: $DEPOIS (antes: $ANTES)"
if [ "$ANTES" -gt 0 ] && [ "$DEPOIS" -lt "$ANTES" ]; then
  echo "❌ o snapshot ENCOLHEU ($ANTES → $DEPOIS). Serviço some do acervo por duas razões — o portal"
  echo "   tirou do ar, ou a coleta falhou —, e o snapshot não distingue as duas."
  echo "   Confira as URLs que falharam no log acima e rode de novo (o cache agora está quente:"
  echo "   só o que faltou volta à rede). Se a queda for real, repita com --keep-cache para seguir."
  exit 1
fi

# 4. Binários release que os passos seguintes exigem (auli + auli-collections).
echo "🔧 compilando binários release…"
( cd "$SERVER" && cargo build --release -p auli-cli -p auli-collections )

export AULI_BIN="$SERVER/target/release/auli"
export AULI_COLLECTIONS_BIN="$SERVER/target/release/auli-collections"

# 5. Deriva os artefatos do snapshot — inclusive a árvore `docs/servicos/*.md`, que é a FONTE que o
#    `auli update` lê. Sem este passo o build-packs re-vetorizaria a árvore ANTIGA e o scrape novo
#    não chegaria ao índice. O `process` resolve `data/` pelo CWD, daí o subshell.
echo "🌳 derivando artefatos do snapshot (árvore docs/servicos)…"
( cd "$SERVER" && "$AULI_COLLECTIONS_BIN" rs )

# 6. Re-vetoriza os packs (muda o `docs_hashes` de `servicos`; o boot recusa subir até packs e árvore baterem).
#
# O custo NÃO é o da entidade inteira. `servicos` e `faqs` são reescrita total por doutrina
# (D-INC-8) — 586 + 1.947 vetores —, mas `pareceres` e `tarf` são incrementais desde o PR #140: as
# keys não mudaram, então os 22.848 vetores da jurisprudência vêm do pack anterior. O log imprime
# quantos reaproveitou; é a linha a conferir se a rodada parecer cara demais.
echo "📦 re-vetorizando os packs do RS…"
echo "   ⏳ ~2.500 vetores novos; a jurisprudência (22.848) é reaproveitada do pack anterior."
"$ROOT/scripts/build-packs.sh" rs

# 7. Sobe o servidor. `--no-build`: o passo 4 já compilou o mesmo binário no mesmo target.
if [ "$SERVE" -eq 1 ]; then
  echo "🚀 subindo o servidor…"
  ( cd "$ROOT" && ./start_server.sh --no-build )
else
  echo "✅ pronto (--no-serve). Suba quando quiser: ./start_server.sh"
fi
