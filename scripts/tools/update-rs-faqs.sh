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
SNAPSHOT="$ROOT/data/rs/rs-faqs-snapshot.json"

FRESH=1
SERVE=1
for arg in "$@"; do
  case "$arg" in
    --keep-cache) FRESH=0 ;;
    --no-serve)   SERVE=0 ;;
    *) echo "❌ argumento desconhecido: $arg"; exit 1 ;;
  esac
done

# Quantas FAQs o snapshot atual tem — lido ANTES de qualquer coisa ser sobrescrita. É a linha de
# base do guard do passo 3; sem snapshot anterior (primeira coleta) não há com o que comparar.
ANTES=0
[ -f "$SNAPSHOT" ] && ANTES="$(jq '.coleta.items | length' "$SNAPSHOT")"

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

# 3. Guard de completude. O walker NÃO falha quando uma página cai: erro ao caminhar vira
#    `eprintln!` e o laço segue (`faqs/mod.rs`, `Error walking …`), e falha no AJAX devolve o nó
#    VAZIO. Como o portal é uma árvore de menus, uma página de menu que caia leva junto a subárvore
#    inteira — dezenas de FAQs somem de uma vez. Daí em diante tudo é derivação fiel: a árvore
#    encolhe, o pack encolhe, o boot valida e o servidor sobe servindo menos do que existe. O
#    `set -e` não pega nada disso, então o portão é aqui.
DEPOIS="$(jq '.coleta.items | length' "$SNAPSHOT")"
echo "🔢 FAQs no snapshot: $DEPOIS (antes: $ANTES)"
if [ "$ANTES" -gt 0 ] && [ "$DEPOIS" -lt "$ANTES" ]; then
  echo "❌ o snapshot ENCOLHEU ($ANTES → $DEPOIS). FAQ some do acervo por duas razões — o portal"
  echo "   tirou do ar, ou a coleta falhou —, e o snapshot não distingue as duas."
  echo "   Procure 'Error walking' / 'Error fetching body' no log acima e rode de novo (o cache"
  echo "   agora está quente: só o que faltou volta à rede). Se a queda for real, siga com"
  echo "   --keep-cache."
  exit 1
fi

# 4. Binários release que os passos seguintes exigem (auli + auli-collections).
echo "🔧 compilando binários release…"
( cd "$SERVER" && cargo build --release -p auli-cli -p auli-collections )

# Exporta os caminhos para os passos 5 e 6: eles compilam por conta própria se não receberem isto,
# e seriam duas invocações a mais do cargo para o mesmo alvo. Como o build acabou de rodar aqui, os
# binários estão em dia por construção — que é a garantia que essas variáveis normalmente dispensam.
export AULI_BIN="$SERVER/target/release/auli"
export AULI_COLLECTIONS_BIN="$SERVER/target/release/auli-collections"

# 5. Deriva os artefatos do snapshot. Sem argumento, o `auli-collections <id>` refaz TODOS os
#    artefatos da entidade — a árvore `docs/faqs/*.md` (a FONTE que o `auli update` lê,
#    TAREFA-FAQS-MD) e também os de serviços. É derivação pura e determinística, então refazer os de
#    serviços não muda um byte; só não se surpreenda com as linhas de `rs-servicos*` no log.
#    Sem este passo o build-packs re-vetorizaria a árvore ANTIGA e o scrape novo não chegaria ao
#    índice. O process resolve `data/` pelo CWD, daí o subshell.
echo "🌳 derivando artefatos do snapshot (árvore docs/faqs)…"
( cd "$SERVER" && "$AULI_COLLECTIONS_BIN" rs )

# 6. Re-vetoriza os packs a partir da árvore recém-derivada (muda o docs_hash;
#    o boot do servidor recusa subir até os packs baterem — daí ser obrigatório).
#
# O custo NÃO é mais o da entidade inteira. Até o PR #140 era: atualizar as FAQs re-embeddava os
# 22.476 acórdãos do TARF que não tinham mudado, ~72 min e pico de ~42 GB de RAM. Com o update
# incremental, `pareceres` e `tarf` reaproveitam o vetor de quem tem a mesma key, e sobram as
# reescritas totais por doutrina (D-INC-8): `faqs` + `servicos`. O log imprime quantos vetores
# reaproveitou — é a linha a conferir se a rodada parecer cara demais.
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
