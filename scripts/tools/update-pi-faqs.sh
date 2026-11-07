#!/usr/bin/env bash
# update-pi-faqs.sh — atualiza as FAQs do PI de ponta a ponta:
#   re-scrape → deriva a árvore → re-vetoriza os packs → sobe o servidor.
#
# Irmão do `update-pa-faqs.sh`. A coleta das FAQs do PI — a seção "Perguntas Frequentes" das 8 páginas de Documentos
# Eletrônicos do portal (Sydle ONE) e o PDF "Perguntas Frequentes EFD ICMS IPI" do repositório de documentos —, 5 GETs
# (o HTML inicial, pelo token anônimo; o `_search` das páginas; a listagem da pasta; o PDF), segundos — vai SEMPRE à rede
# e o cache em data/pi/raw/cache/faqs/ só é lido com `--usecache` (do PDF ele guarda o texto já extraído); mesmo assim
# ele é apagado antes da coleta. Exige o `pdftotext` (poppler-utils). (Os SERVIÇOS do PI são do
# `atualizar-servicos.sh`: este script não os toca.)
#
# Uso:   scripts/tools/update-pi-faqs.sh              # scrape + rebuild + sobe servidor
#        scripts/tools/update-pi-faqs.sh --no-serve   # para antes de subir o servidor
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/../.." && pwd)"   # raiz do repo
SERVER="$ROOT/auli-server"
SNAPSHOT="$ROOT/data/pi/pi-faqs-snapshot.json"
TREE="$ROOT/data/pi/raw/pi-faqs-tree.json"

SERVE=1
for arg in "$@"; do
  case "$arg" in
    --no-serve) SERVE=0 ;;
    *) echo "❌ argumento desconhecido: $arg"; exit 1 ;;
  esac
done

# Cópia do snapshot e da árvore ANTES da coleta: o scraper sobrescreve os dois, e `data/` está fora
# do git — sem a cópia, a queda que o guard do passo 2 pega já não teria volta. Ela só some quando o
# guard passa; enquanto existir, é o último estado bom. Por isso uma rodada que falhou NÃO a recopia:
# a repetição compara com o estado bom, não com o snapshot que acabou de encolher.
if [ ! -f "$SNAPSHOT.anterior" ] && [ -f "$SNAPSHOT" ]; then
  cp -p "$SNAPSHOT" "$SNAPSHOT.anterior"
  [ -f "$TREE" ] && cp -p "$TREE" "$TREE.anterior"
fi

# Quantas FAQs o último snapshot bom tem. É a linha de base do guard do passo 2; sem snapshot
# anterior (primeira coleta) não há com o que comparar.
ANTES=0
[ -f "$SNAPSHOT.anterior" ] && ANTES="$(jq '.coleta.items | length' "$SNAPSHOT.anterior")"

# Cache apagado antes da coleta — regra de todo script de atualização de serviços e de FAQs (decisão do
# mantenedor, 02/11/2026). Este scraper já vai sempre à rede; apagado, o cache guarda só esta coleta,
# que é o que o `--usecache` reproduz. Só o de faqs: os outros namespaces de data/pi/raw/cache/ ficam.
CACHE="$ROOT/data/pi/raw/cache/faqs"
if [ -d "$CACHE" ]; then
  echo "🗑️  apagando o cache de faqs ($(find "$CACHE" -type f | wc -l) arquivos)"
  rm -rf "$CACHE"
fi

# 1. Re-scrape das faqs → grava data/pi/raw/pi-faqs-tree.json e data/pi/pi-faqs-snapshot.json.
command -v pdftotext > /dev/null || { echo "❌ falta o pdftotext (poppler-utils): sudo apt install poppler-utils"; exit 1; }
echo "🕸️  scrapeando faqs do PI…"
( cd "$SERVER" && cargo run --release -p auli-scraper-pi -- faqs )

# 2. Guard de completude. As guardas do scraper (`faqs.rs`) param ANTES de gravar quando o `_search` não traz tudo o
#    que achou, uma página perde a seção "Perguntas Frequentes" ou fica sem pergunta, uma pergunta fica sem resposta,
#    sobra resto do editor ("Título da sanfona", link do `portal-h`), a pasta do PDF tem outro número de PDFs que um
#    (versão nova: revisar o escopo), o sumário do PDF pula um número ou uma entrada dele não está no corpo, ou os
#    totais caem abaixo dos pisos (90 nas páginas, 30 no PDF). Não pegam perguntas que o portal tirou de uma página
#    — e daí em diante tudo é derivação fiel até o servidor subir servindo menos do que existe.
DEPOIS="$(jq '.coleta.items | length' "$SNAPSHOT")"
echo "🔢 FAQs no snapshot: $DEPOIS (antes: $ANTES)"
if [ "$ANTES" -gt 0 ] && [ "$DEPOIS" -lt "$ANTES" ]; then
  echo "❌ o snapshot ENCOLHEU ($ANTES → $DEPOIS). FAQ some do acervo por duas razões — o portal"
  echo "   tirou do ar, ou a coleta falhou —, e o snapshot não distingue as duas."
  echo "   O estado bom está guardado. Para voltar a ele:"
  echo "     mv \"$SNAPSHOT.anterior\" \"$SNAPSHOT\" && mv \"$TREE.anterior\" \"$TREE\""
  echo "   Compare as linhas 'PI FAQs: <página>: N perguntas' do log acima com as da rodada anterior"
  echo "   e rode de novo (a comparação segue contra o estado bom)."
  echo "   Se a queda for real, apague as duas cópias .anterior e rode de novo."
  exit 1
fi
rm -f "$SNAPSHOT.anterior" "$TREE.anterior"

# 3. Binários release que os passos seguintes exigem (auli + auli-collections).
echo "🔧 compilando binários release…"
( cd "$SERVER" && cargo build --release -p auli-cli -p auli-collections )

# Exporta os caminhos para o passo 5: ele compila por conta própria se não receber isto. Como o build
# acabou de rodar aqui, os binários estão em dia por construção.
export AULI_BIN="$SERVER/target/release/auli"
export AULI_COLLECTIONS_BIN="$SERVER/target/release/auli-collections"

# 4. Deriva os artefatos do snapshot. O `auli-collections pi` refaz TODOS os artefatos da entidade —
#    a árvore `docs/faqs/*.md` que o `auli update` lê e também os de serviços. É derivação pura e
#    determinística, então refazer os de serviços não muda um byte. O process resolve `data/` pelo
#    CWD, daí o subshell.
echo "🌳 derivando artefatos do snapshot (árvore docs/faqs)…"
( cd "$SERVER" && "$AULI_COLLECTIONS_BIN" pi )

# 5. Re-vetoriza só as FAQs (rodada parcial: serviços e pareceres ficam como estão no manifesto).
#    Se o snapshot de serviços tiver mudado sem re-vetorizar, o boot recusa nomeando `servicos` —
#    aí é `scripts/build-packs.sh pi servicos`.
echo "📦 re-vetorizando as FAQs do PI (~134 vetores)…"
"$ROOT/scripts/build-packs.sh" pi faqs

# 6. Sobe o servidor. `--no-build`: o passo 3 já compilou o mesmo binário no mesmo target.
#    A aba FAQs lê `pi-faqs-tree.json` do frontend estático: publique com scripts/deploy-frontend.sh.
if [ "$SERVE" -eq 1 ]; then
  echo "🚀 subindo o servidor… (a aba FAQs só muda depois do scripts/deploy-frontend.sh)"
  ( cd "$ROOT" && ./start_server.sh --no-build )
else
  echo "✅ pronto (--no-serve). Suba quando quiser: ./start_server.sh — e publique a aba com scripts/deploy-frontend.sh"
fi
