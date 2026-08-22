#!/usr/bin/env bash
# build-frontend-public.sh [<id>] — (re)gera auli-frontend/public/<id>/ a partir de data/<id>/{raw,ref}/.
#
# Substitui a CÓPIA MANUAL (propensa a drift) que abastecia o frontend (roteiro Fase 3). O frontend
# continua servindo conteúdo de referência ESTÁTICO do seu próprio origin (Apache + cache do
# Cloudflare); só a *origem* muda: de cópia à mão para regeneração determinística a partir da fonte
# única em data/. Rode após mudar data/.
#
# NÃO se commita o resultado: `auli-frontend/.gitignore` ignora `public/*/` desde que a geração
# passou a ser determinística — o que sobe para produção é o `dist/`, montado no deploy a partir
# daqui. (O cabeçalho pedia "faça commit do public/" até 09/08/2026, de quando a cópia era manual.)
#
# Sem argumento, regenera TODAS as entidades do registry. Com `<id>` (ex.: `build-frontend-public.sh
# rs`), regenera só aquela — útil quando o data/ das outras não está fresco (evita sobrescrevê-las).
#
# O que entra em public/<id>/ (todos com o nome já prefixado por `<id>-` na origem, casando com
# `entityPath` no frontend, que busca `/<id>/<id>-<file>`):
#   - data/<id>/raw/*.json   (<id>-servicos-index, <id>-servicos-*, <id>-faqs-tree.json,
#                             <id>-pareceres-index.json — o que a UI busca)
#   - data/<id>/ref/*        (<id>-portal-notas.txt, <id>-conteudo_site_tree.json)
# NÃO copia os <id>-portal-servicos.txt/<id>-portal-faqs.txt de raw/ (o filtro `*.json` já os exclui;
# são grandes, alimentam os packs, a UI não usa) nem os contratos do engine `<id>-{faqs,servicos}.json`
# (lidos pelo `auli update`, a UI não os busca).
#
# Tanto `raw/` (gerado) quanto `ref/` (autorado, versionado) já são prefixados por `<id>-` na origem,
# então os arquivos são copiados COMO ESTÃO — o prefixo deixou de ser adicionado aqui.
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
PUB="$ROOT/auli-frontend/public"

# raw/: já prefixado por "$3-" na origem. Copia como está, pulando (retorna 1) os contratos do engine
# `<id>-servicos.json` / `<id>-faqs.json`, que a UI não consome.
copy_raw() {
  local base; base="$(basename "$1")"
  case "$base" in "$3-servicos.json"|"$3-faqs.json") return 1 ;; esac
  cp "$1" "$2/$base"
}

# ref/: também já prefixado por "$3-" na origem (arquivos autorados/versionados). Copia como está,
# pulando (retorna 1) o `<id>-portal-pareceres.txt`: a tab de Pareceres migrou para o índice leve
# `<id>-pareceres-index.json` (vem de raw/, derivado da árvore por `auli-collections <id> indice`).
# O `.txt` continua em data/<id>/ref/ como fonte de bootstrap do `auli-collections <id> pareceres`,
# mas não é mais servido — eram 176 MB de corpo integral em public/, 147 deles só do SP.
#
# Pelo mesmo motivo pula os `<id>-*-texto-vigente.md`: são os textos integrais das leis, insumo de
# origem dos pares P/R de `docs/legislacao/` (a lei é pública, mas o corpo servido é o do par, e o
# integral seriam centenas de KB mortos em cada deploy). O sufixo marca a família — virão CF, CTN e
# Kandir. Ficam em ref/, não em docs/, porque a árvore docs/ é "um .md = uma unidade vetorizável".
copy_ref() {
  local base; base="$(basename "$1")"
  case "$base" in "$3-portal-pareceres.txt"|"$3"-*-texto-vigente.md) return 1 ;; esac
  cp "$1" "$2/$base"
}

# ids das entidades a partir do registry (linhas `id = "xx"`).
mapfile -t IDS < <(grep -E '^id[[:space:]]*=' "$ROOT/config/registry.toml" | sed -E 's/.*"([^"]+)".*/\1/')
[ "${#IDS[@]}" -gt 0 ] || { echo "❌ nenhuma entidade em config/registry.toml"; exit 1; }

# Argumento opcional: restringe a uma entidade (precisa existir no registry).
if [ "$#" -gt 0 ]; then
  want="$1"
  printf '%s\n' "${IDS[@]}" | grep -qx "$want" || { echo "❌ '$want' não está em config/registry.toml (entidades: ${IDS[*]})"; exit 1; }
  IDS=("$want")
fi

for id in "${IDS[@]}"; do
  src_raw="$ROOT/data/$id/raw"
  src_ref="$ROOT/data/$id/ref"
  dst="$PUB/$id"
  rm -rf "$dst"; mkdir -p "$dst"

  n=0
  if [ -d "$src_raw" ]; then
    while IFS= read -r -d '' f; do copy_raw "$f" "$dst" "$id" && n=$((n+1)); done \
      < <(find "$src_raw" -maxdepth 1 -name '*.json' -print0)
  fi
  if [ -d "$src_ref" ]; then
    while IFS= read -r -d '' f; do copy_ref "$f" "$dst" "$id" && n=$((n+1)); done \
      < <(find "$src_ref" -maxdepth 1 -type f -print0)
  fi
  echo "📦 public/$id/  <- data/$id/{raw/*.json, ref/*}  ($n arquivos)"
done

# manuais/: os manuais de instalação do conector MCP são GLOBAIS (não escopados a entidade) e
# autorados em manuais/ na raiz. Copiados aqui — não à mão — pela mesma razão do resto do script:
# fonte única, sem drift. O README.md fica de fora: a aba MCP do frontend cumpre o papel de índice.
# Fora do laço de propósito: mesmo chamado com um `<id>`, este passo roda.
dst_manuais="$PUB/manuais"
rm -rf "$dst_manuais"; mkdir -p "$dst_manuais"
cp "$ROOT"/manuais/manual-*.md "$dst_manuais/"
echo "📦 public/manuais/  <- manuais/manual-*.md  ($(find "$dst_manuais" -name '*.md' | wc -l) arquivos)"

# tributum/: os arquivos HOSPEDADOS da seção editorial (D-TRIB-8) — PDFs autorais ou com autorização
# expressa, e datasets públicos. Autorados em tributum/ na raiz e copiados aqui pela MESMA razão dos
# manuais, e por uma a mais: `auli-frontend/.gitignore` ignora as subpastas de `public/`, então um
# arquivo posto direto em `public/tributum/` não seria versionado e sumiria no próximo clone.
# (O CATÁLOGO não passa por aqui: `tributum.json` vive na RAIZ de `public/`, com o `about.md`, que é
# o único lugar de `public/` que o gitignore preserva. Ver o doc do `useTributum.ts`.)
# Fora do laço de propósito, como os manuais: mesmo chamado com um `<id>`, este passo roda.
dst_tributum="$PUB/tributum"
rm -rf "$dst_tributum"; mkdir -p "$dst_tributum"
if compgen -G "$ROOT/tributum/*" > /dev/null; then
  cp "$ROOT"/tributum/* "$dst_tributum/"
fi
echo "📦 public/tributum/  <- tributum/*  ($(find "$dst_tributum" -type f | wc -l) arquivos)"
