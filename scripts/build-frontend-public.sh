#!/usr/bin/env bash
# build-frontend-public.sh — (re)gera auli-frontend/public/<id>/ a partir de data/<id>/{raw,ref}/.
#
# Substitui a CÓPIA MANUAL (propensa a drift) que abastecia o frontend (roteiro Fase 3). O frontend
# continua servindo conteúdo de referência ESTÁTICO do seu próprio origin (Apache + cache do
# Cloudflare); só a *origem* muda: de cópia à mão para regeneração determinística a partir da fonte
# única em data/. Rode após mudar data/ (e depois faça commit do public/).
#
# O que entra em public/<id>/:
#   - data/<id>/raw/*.json   (servicos-index, servicos-*, faqs.json — o que a UI busca)
#   - data/<id>/ref/*        (portal-pareceres.txt, portal-notas.txt, conteudo_site_tree.json)
# NÃO copia os portal-servicos.txt/portal-faqs.txt de raw/ (grandes; alimentam os packs, a UI não usa).
#
# Cada arquivo é copiado com o nome PREFIXADO por `<id>-` (ex.: faqs.json -> rs-faqs.json), casando
# com `entityPath` no frontend (que busca `/<id>/<id>-<file>`). Arquivos já prefixados (o contrato
# `<id>-servicos.json`) não são duplicados.
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
PUB="$ROOT/auli-frontend/public"

# Copia $1 para o diretório $2, prefixando o nome com "$3-" (sem duplicar se já começar com ele).
copy_prefixed() {
  local base; base="$(basename "$1")"
  case "$base" in "$3-"*) : ;; *) base="$3-$base" ;; esac
  cp "$1" "$2/$base"
}

# ids das entidades a partir do registry (linhas `id = "xx"`).
mapfile -t IDS < <(grep -E '^id[[:space:]]*=' "$ROOT/data/registry.toml" | sed -E 's/.*"([^"]+)".*/\1/')
[ "${#IDS[@]}" -gt 0 ] || { echo "❌ nenhuma entidade em data/registry.toml"; exit 1; }

for id in "${IDS[@]}"; do
  src_raw="$ROOT/data/$id/raw"
  src_ref="$ROOT/data/$id/ref"
  dst="$PUB/$id"
  rm -rf "$dst"; mkdir -p "$dst"

  n=0
  if [ -d "$src_raw" ]; then
    while IFS= read -r -d '' f; do copy_prefixed "$f" "$dst" "$id"; n=$((n+1)); done \
      < <(find "$src_raw" -maxdepth 1 -name '*.json' -print0)
  fi
  if [ -d "$src_ref" ]; then
    while IFS= read -r -d '' f; do copy_prefixed "$f" "$dst" "$id"; n=$((n+1)); done \
      < <(find "$src_ref" -maxdepth 1 -type f -print0)
  fi
  echo "📦 public/$id/  <- data/$id/{raw/*.json, ref/*}  ($n arquivos)"
done
