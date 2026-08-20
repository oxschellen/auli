#!/usr/bin/env bash
# build-packs.sh <entity> — vetoriza os pacotes de uma entidade a partir de data/<id>/docs/.
#
# `auli update` lê as ÁRVORES `data/<id>/docs/{servicos,faqs,legislacao,pareceres,tarf}/*.md` — um
# `.md` por documento (a de legislação tem uma subpasta por lei) —, embedda o campo `text_to_embed`
# de cada registro e escreve os packs em data/<id>/packs/. Coleção sem árvore é pulada.
#
# Este script, ao final, também deriva de cada árvore com índice leve o `<id>-<colecao>-index.json`
# que a tab do frontend consome (ver o bloco no fim do arquivo).
# `notas` ainda não tem fonte struct — o `update` a encontra ausente e simplesmente a pula.
#
# Uso:   scripts/build-packs.sh rs                    # rodada completa (como sempre)
#        scripts/build-packs.sh rs legislacao         # só a coleção legislacao (auli update --kind)
#        scripts/build-packs.sh rs faqs legislacao    # duas coleções
#        VERSION=2 scripts/build-packs.sh sc
#
# A rodada PARCIAL preserva verbatim as entradas e hashes das coleções que não rodaram, e o boot
# continua conferindo TODAS contra o disco: esquecer uma árvore mexida vira recusa NOMEADA no
# próximo restart, não silêncio. O `update` avisa na hora, ao fim da rodada.
set -euo pipefail

ID="${1:?uso: build-packs.sh <entity> [colecao...]}"
shift || true
KINDS=("$@")
ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"   # raiz do repo
DATA="$ROOT/data/$ID"
DOCS="$DATA/docs"
[ -d "$DOCS" ] || { echo "❌ não existe $DOCS (rode o scraper e o \`auli-collections $ID process\` antes: geram as árvores .md)"; exit 1; }

echo "📂 origem ($ID): $(for d in "$DOCS"/*/; do [ -d "$d" ] && printf '%s=%s ' "$(basename "$d")" "$(find "$d" -name '*.md' | wc -l)"; done)"

# Compila antes de usar. Binário velho aqui é o caso mais traiçoeiro do repositório: `auli update`
# grava os packs e carimba o mapa `docs_hashes`, que é hash da ÁRVORE — não do código. Um `text_to_embed`
# de fórmula antiga produz vetores errados que PASSAM na validação de boot, porque o manifesto e o
# disco continuam coerentes entre si. Não há erro em lugar nenhum; só recuperação pior.
# (O trio de identidade pega mudança de STRATEGY_VERSION, mas só se alguém tiver lembrado do bump.)
#
# Quem decide se há o que recompilar é o cargo, por fingerprint do grafo — inclusive mudança no
# Cargo.lock, que data de arquivo não veria. Em dia, é um no-op de menos de um segundo.
# Cada binário é pulado se veio pronto por variável — é o que o `tools/update-rs-faqs.sh` faz:
# compila uma vez no começo do ciclo e exporta os dois.
pkgs=()
[ -n "${AULI_BIN:-}" ]             || pkgs+=(-p auli-cli)
[ -n "${AULI_COLLECTIONS_BIN:-}" ] || pkgs+=(-p auli-collections)
if [ "${#pkgs[@]}" -gt 0 ]; then
  echo "🔨 compilando: ${pkgs[*]}"
  (cd "$ROOT/auli-server" && cargo build --release "${pkgs[@]}")
fi

mkdir -p "$DATA/packs"
export EMBED_CACHE_DIR="${EMBED_CACHE_DIR:-$ROOT/models}"
BIN="${AULI_BIN:-$ROOT/auli-server/target/release/auli}"
[ -x "$BIN" ] || { echo "❌ binário não encontrado: $BIN (AULI_BIN aponta para algo que não existe?)"; exit 1; }

KIND_FLAGS=()
for K in "${KINDS[@]}"; do KIND_FLAGS+=(--kind "$K"); done
"$BIN" update --entity "$ID" --out "$DATA/packs" --version "${VERSION:-1}" "${KIND_FLAGS[@]}"
echo "✅ packs de '$ID' em $DATA/packs"

# Índices leves das tabs de jurisprudência, derivados das MESMAS árvores que acabaram de ser
# vetorizadas. Ficam aqui, e não como passo solto, porque este script é inevitável depois de qualquer
# mexida na árvore (o `docs_hashes` daquela coleção muda e o boot recusa até ele rodar) — então servidor e frontend
# nunca divergem de estado. Derivação pura: coleção sem árvore simplesmente não tem o que derivar.
#
# Falha DURA se o binário não estiver lá — antes isto era só um aviso, e o aviso mentia por omissão:
# sem o índice derivado, os packs ficam novos e a aba do frontend continua lendo o índice velho. Foi
# exatamente o que aconteceu em 09/08/2026 (aba mostrando 3.800 acórdãos, chat respondendo de
# 22.476), e nada no boot pega — o `docs_hashes` cobre packs×árvore, não índice×árvore.
COLLECTIONS="${AULI_COLLECTIONS_BIN:-$ROOT/auli-server/target/release/auli-collections}"
[ -x "$COLLECTIONS" ] || { echo "❌ $COLLECTIONS não encontrado (AULI_COLLECTIONS_BIN aponta para algo que não existe?)"; exit 1; }
# `legislacao` entra na mesma lista: o índice dela tem shape e ordenação próprios (D-LEG-11/12),
# mas o subcomando é o mesmo e o motivo de estar aqui também — árvore mexida sem índice derivado
# deixa a aba lendo o artefato velho, e nada no boot pega isso.
# Na rodada parcial, só os das coleções pedidas — as outras árvores não mudaram por definição do
# `--kind` (e se mudaram, o `update` acabou de avisar e o boot vai recusar até serem recarimbadas).
if [ "${#KINDS[@]}" -gt 0 ]; then LISTA=("${KINDS[@]}"); else LISTA=(pareceres tarf legislacao); fi
for COLECAO in "${LISTA[@]}"; do
  case "$COLECAO" in pareceres | tarf | legislacao) ;; *) continue ;; esac
  [ -d "$DATA/docs/$COLECAO" ] || continue
  # O auli-collections resolve `data/` relativo ao CWD, daí o subshell em auli-server/.
  (cd "$ROOT/auli-server" && "$COLLECTIONS" "$ID" indice "$COLECAO")
done
