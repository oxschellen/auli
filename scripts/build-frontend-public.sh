#!/usr/bin/env bash
# build-frontend-public.sh [<id>] — (re)gera auli-frontend/public/<id>/ a partir de data/<id>/{raw,ref}/.
#
# Substitui a CÓPIA MANUAL (propensa a drift) que abastecia o frontend (roteiro Fase 3). O frontend
# continua servindo conteúdo de referência ESTÁTICO do seu próprio origin (Apache + cache do
# Cloudflare); só a *origem* muda: de cópia à mão para regeneração determinística a partir da fonte
# única em data/. Rode após mudar data/ (e depois faça commit do public/).
#
# Sem argumento, regenera TODAS as entidades do registry. Com `<id>` (ex.: `build-frontend-public.sh
# rs`), regenera só aquela — útil quando o data/ das outras não está fresco (evita sobrescrevê-las).
#
# O que entra em public/<id>/:
#   - data/<id>/raw/*.json   (servicos-index, servicos-*, faqs-tree.json — o que a UI busca)
#   - data/<id>/ref/*        (portal-pareceres.txt, portal-notas.txt, conteudo_site_tree.json)
# NÃO copia os portal-servicos.txt/portal-faqs.txt de raw/ (grandes; alimentam os packs, a UI não usa).
#
# Cada arquivo é copiado com o nome PREFIXADO por `<id>-` (ex.: faqs-tree.json -> rs-faqs-tree.json),
# casando com `entityPath` no frontend (que busca `/<id>/<id>-<file>`). Arquivos em data/<id>/raw que
# JÁ começam com `<id>-` são os contratos do engine (`<id>-faqs.json` / `<id>-servicos.json`, lidos
# pelo `auli update`); a UI não os busca, então NÃO vão para public/.
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
PUB="$ROOT/auli-frontend/public"

# Copia $1 para $2/ prefixando o nome com "$3-". Pula (retorna 1) os contratos do engine, que já
# vêm prefixados com "$3-" — a UI não os consome.
copy_prefixed() {
  local base; base="$(basename "$1")"
  case "$base" in "$3-"*) return 1 ;; esac
  cp "$1" "$2/$3-$base"
}

# ids das entidades a partir do registry (linhas `id = "xx"`).
mapfile -t IDS < <(grep -E '^id[[:space:]]*=' "$ROOT/data/registry.toml" | sed -E 's/.*"([^"]+)".*/\1/')
[ "${#IDS[@]}" -gt 0 ] || { echo "❌ nenhuma entidade em data/registry.toml"; exit 1; }

# Argumento opcional: restringe a uma entidade (precisa existir no registry).
if [ "$#" -gt 0 ]; then
  want="$1"
  printf '%s\n' "${IDS[@]}" | grep -qx "$want" || { echo "❌ '$want' não está em data/registry.toml (entidades: ${IDS[*]})"; exit 1; }
  IDS=("$want")
fi

for id in "${IDS[@]}"; do
  src_raw="$ROOT/data/$id/raw"
  src_ref="$ROOT/data/$id/ref"
  dst="$PUB/$id"
  rm -rf "$dst"; mkdir -p "$dst"

  n=0
  if [ -d "$src_raw" ]; then
    while IFS= read -r -d '' f; do copy_prefixed "$f" "$dst" "$id" && n=$((n+1)); done \
      < <(find "$src_raw" -maxdepth 1 -name '*.json' -print0)
  fi
  if [ -d "$src_ref" ]; then
    while IFS= read -r -d '' f; do copy_prefixed "$f" "$dst" "$id" && n=$((n+1)); done \
      < <(find "$src_ref" -maxdepth 1 -type f -print0)
  fi
  echo "📦 public/$id/  <- data/$id/{raw/*.json, ref/*}  ($n arquivos)"
done
