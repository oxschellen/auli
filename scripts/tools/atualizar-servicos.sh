#!/usr/bin/env bash
# atualizar-servicos.sh <id>... — re-raspa os serviços de uma ou mais entidades e reconstrói tudo
# até os packs. Portal muda: serviço novo, removido, renomeado, texto editado.
#
# Nasceu como `migrar-arvore-servicos.sh`, a campanha que levou as 27 entidades para a árvore
# `docs/servicos/*.md` (TAREFA-SERVICOS-MD, PR #107). A campanha terminou em 02/08/2026 com o `ap`;
# o ciclo ficou, porque re-raspar continua sendo necessário — só o nome mudou.
#
# Ciclo por entidade:
#   1. backup de `data/<id>` (para o diff do passo 4 e para poder voltar atrás)
#   2. scrape        → grava o snapshot
#   3. process       → materializa `docs/servicos/*.md` + reescreve os artefatos de `raw/`
#   4. diff de `raw/` contra o backup
#   5. build-packs   → OBRIGATÓRIO: mexer em `docs/` muda o `docs_hashes` daquela coleção e o boot recusa até re-vetorizar
#
# O passo 4 é o gate de leitura humana: o que ele acusa é mudança do PORTAL, porque o re-processamento
# em si não altera conteúdo. Vale ler o que mudou antes de dar a entidade por atualizada.
#
# Uso:   scripts/tools/atualizar-servicos.sh sc
#        scripts/tools/atualizar-servicos.sh mg pe ba      # para na primeira que falhar
#        BACKUP_DIR=/var/tmp/auli scripts/tools/atualizar-servicos.sh sp
set -uo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/../.." && pwd)"
SERVER="$ROOT/auli-server"
BACKUP_DIR="${BACKUP_DIR:-/tmp/auli-atualizacao}"
[ $# -gt 0 ] || { echo "uso: atualizar-servicos.sh <id>..."; exit 1; }

atualizar() {
  local ID="$1"
  local BK="$BACKUP_DIR/$ID"
  [ -d "$ROOT/data/$ID" ] || { echo "❌ $ID: não existe $ROOT/data/$ID"; return 1; }
  cd "$SERVER" || return 1

  echo "════════ $ID: backup em $BK"
  rm -rf "$BK"; mkdir -p "$BACKUP_DIR"
  # Checado: este script roda sem `set -e` (o laço lá embaixo é quem trata a falha), então um `cp`
  # que falhasse seguiria adiante — o scrape sobrescreveria `data/<id>` e a instrução de rollback
  # apontaria para um backup inexistente. É o pior desfecho possível aqui, e o mais silencioso.
  cp -a "$ROOT/data/$ID" "$BK" || { echo "❌ $ID: backup falhou — abortando ANTES de tocar em data/"; return 1; }

  echo "════════ $ID: build do scraper + auli-collections"
  # O `auli-collections` entra junto: o passo `process` logo abaixo usa o binário do
  # `target/release/` como estiver, e compilar só o scraper deixava o derivador para trás — a
  # árvore `docs/servicos/*.md` sairia do código antigo, sem aviso. Ver auli_operations.md §11.
  #
  # Com log em arquivo, e não `| tail -1`: sem `set -e`, o pipe engolia a falha de compilação e o
  # script seguia para rodar o binário ANTIGO. Mesmo formato do scrape logo abaixo — silencioso
  # quando dá certo, com as últimas linhas quando não dá.
  if ! cargo build --release -p "auli-scraper-$ID" -p auli-collections > "/tmp/$ID-build.log" 2>&1; then
    echo "❌ $ID: build falhou — últimas linhas:"; tail -15 "/tmp/$ID-build.log"; return 1
  fi
  tail -1 "/tmp/$ID-build.log"

  echo "════════ $ID: scrape (rede)"
  if ! "./target/release/auli-scraper-$ID" servicos > "/tmp/$ID-scrape.log" 2>&1; then
    echo "❌ $ID: scrape falhou — últimas linhas:"; tail -5 "/tmp/$ID-scrape.log"; return 1
  fi
  tail -2 "/tmp/$ID-scrape.log"

  echo "════════ $ID: process"
  ./target/release/auli-collections "$ID" process 2>&1 | tail -3 || return 1

  echo "════════ $ID: raw/ mudou?"
  local mudou=0 a b
  for a in "$BK"/raw/*.json "$BK"/raw/*.txt; do
    [ -f "$a" ] || continue
    b="$ROOT/data/$ID/raw/$(basename "$a")"
    diff -q "$a" "$b" > /dev/null 2>&1 || { echo "  ≠ $(basename "$a")"; mudou=1; }
  done
  [ "$mudou" -eq 0 ] && echo "  ✅ todos idênticos — o portal não mudou"

  echo "════════ $ID: build-packs"
  # O `❌` entra no filtro: sem ele, uma falha do build-packs (binário ausente, árvore sumida) era
  # descartada pelo grep e o usuário via só o "🛑 PAROU" no fim, sem o motivo.
  "$ROOT/scripts/build-packs.sh" "$ID" 2>&1 | grep -E "docs:|⚠️|❌|✅|📝"
}

for id in "$@"; do
  if ! atualizar "$id"; then
    echo "🛑 PAROU em '$id'."
    if [ -d "$BACKUP_DIR/$id" ]; then
      echo "   Backup intacto — para voltar ao estado anterior:"
      echo "   rm -rf $ROOT/data/$id && cp -a $BACKUP_DIR/$id $ROOT/data/$id"
    fi
    exit 1
  fi
done
echo "✅ atualização completa: $*"
