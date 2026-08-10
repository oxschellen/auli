#!/usr/bin/env bash
# update-rs-faqs.sh — atualiza as FAQs do RS de ponta a ponta:
#   re-scrape (cache fresco) → re-vetoriza os packs → sobe o servidor.
#
# O scraper é cache-first SEMPRE (o flag --usecache só muda o que fazer no MISS:
# bail vs. rede). Então "re-scrape de verdade" = apagar o cache de faqs antes,
# para que toda página seja MISS e volte da rede. É o que este script faz.
#
# Uso:   scripts/tools/update-rs-faqs.sh              # scrape fresco + rebuild + sobe servidor
#        scripts/tools/update-rs-faqs.sh --keep-cache # NÃO apaga o cache (roda cache-first)
#        scripts/tools/update-rs-faqs.sh --no-serve   # para antes de subir o servidor
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/../.." && pwd)"   # raiz do repo
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

# 2. Re-scrape das faqs → grava data/rs/rs-faqs-snapshot.json (o snapshot da coleta).
echo "🕸️  scrapeando faqs do RS…"
( cd "$SERVER" && cargo run --release -p auli-scraper-rs -- faqs )

# 3. Binários release que os passos seguintes exigem (auli + auli-collections).
echo "🔧 compilando binários release…"
( cd "$SERVER" && cargo build --release -p auli-cli -p auli-collections )

# Exporta os caminhos para os passos 5 e 6: eles compilam por conta própria se não receberem isto,
# e seriam duas invocações a mais do cargo para o mesmo alvo. Como o build acabou de rodar aqui, os
# binários estão em dia por construção — que é a garantia que essas variáveis normalmente dispensam.
export AULI_BIN="$SERVER/target/release/auli"
export AULI_COLLECTIONS_BIN="$SERVER/target/release/auli-collections"

# 4. Deriva os artefatos do snapshot. Sem argumento, o `auli-collections <id>` refaz TODOS os
#    artefatos da entidade — a árvore `docs/faqs/*.md` (a FONTE que o `auli update` lê,
#    TAREFA-FAQS-MD) e também os de serviços. É derivação pura e determinística, então refazer os de
#    serviços não muda um byte; só não se surpreenda com as linhas de `rs-servicos*` no log.
#    Sem este passo o build-packs re-vetorizaria a árvore ANTIGA e o scrape novo não chegaria ao
#    índice. O process resolve `data/` pelo CWD, daí o subshell.
echo "🌳 derivando artefatos do snapshot (árvore docs/faqs)…"
( cd "$SERVER" && "$AULI_COLLECTIONS_BIN" rs )

# 5. Re-vetoriza os packs a partir da árvore recém-derivada (muda o docs_hash;
#    o boot do servidor recusa subir até os packs baterem — daí ser obrigatório).
#
# ⏳ ATENÇÃO AO CUSTO. O `auli update` re-vetoriza a entidade INTEIRA, não só a coleção que mudou:
# desde que a coleta do TARF fechou (09/08/2026, 22.476 acórdãos), atualizar as FAQs do RS custa
# ~72 min e pico de ~42 GB de RAM — e ~99% disso é re-embeddar acórdãos que não mudaram. Antes do
# TARF eram ~16 min. É o caso de uso que mais pede o update incremental (auli_pendencias.md §31).
echo "📦 re-vetorizando os packs do RS…"
echo "   ⏳ a entidade inteira, não só as faqs: ~72 min, pico de ~42 GB de RAM."
"$ROOT/scripts/build-packs.sh" rs

# 6. Sobe o servidor. `--no-build`: o passo 3 já compilou o mesmo binário no mesmo target.
if [ "$SERVE" -eq 1 ]; then
  echo "🚀 subindo o servidor…"
  ( cd "$ROOT" && ./start_server.sh --no-build )
else
  echo "✅ pronto (--no-serve). Suba quando quiser: ./start_server.sh"
fi
