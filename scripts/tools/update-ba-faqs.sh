#!/usr/bin/env bash
# update-ba-faqs.sh — atualiza as FAQs da BA de ponta a ponta:
#   re-scrape → deriva a árvore → re-vetoriza os packs → sobe o servidor.
#
# Irmão do `update-pi-faqs.sh`. A coleta das FAQs da BA — o "Perguntas e Respostas" do Plantão Fiscal no portal antigo
# (portal.sefaz.ba.gov.br, os menus Inspetoria, Legislação e Contencioso: 16 páginas HTML e 17 PDFs, mais o menu) e três
# do site novo (www.sefaz.ba.gov.br: o Balcão Virtual e o Cadastro Sincronizado pela API do WordPress, o PDF da taxa de
# incêndio) —, 37 GETs com 1 s entre eles, menos de um minuto — vai SEMPRE à rede e o cache em data/ba/raw/cache/faqs/ só
# é lido com `--usecache` (dos PDFs ele guarda o texto já extraído e a data de criação); mesmo assim ele é apagado antes
# da coleta. Exige o `pdftotext` e o `pdfinfo` (poppler-utils). (Os SERVIÇOS da BA são do `atualizar-servicos.sh`: este
# script não os toca.)
#
# Uso:   scripts/tools/update-ba-faqs.sh              # scrape + rebuild + sobe servidor
#        scripts/tools/update-ba-faqs.sh --no-serve   # para antes de subir o servidor
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/../.." && pwd)"   # raiz do repo
SERVER="$ROOT/auli-server"
SNAPSHOT="$ROOT/data/ba/ba-faqs-snapshot.json"
TREE="$ROOT/data/ba/raw/ba-faqs-tree.json"

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
# que é o que o `--usecache` reproduz. Só o de faqs: os outros namespaces de data/ba/raw/cache/ ficam.
CACHE="$ROOT/data/ba/raw/cache/faqs"
if [ -d "$CACHE" ]; then
  echo "🗑️  apagando o cache de faqs ($(find "$CACHE" -type f | wc -l) arquivos)"
  rm -rf "$CACHE"
fi

# 1. Re-scrape das faqs → grava data/ba/raw/ba-faqs-tree.json e data/ba/ba-faqs-snapshot.json.
command -v pdftotext > /dev/null && command -v pdfinfo > /dev/null \
  || { echo "❌ falta o pdftotext/pdfinfo (poppler-utils): sudo apt install poppler-utils"; exit 1; }
echo "🕸️  scrapeando faqs da BA…"
( cd "$SERVER" && cargo run --release -p auli-scraper-ba -- faqs )

# 2. Guard de completude. As guardas do scraper (`faqs.rs`) param ANTES de gravar quando uma página ou PDF falha ou
#    fica sem pergunta, uma pergunta fica sem resposta ou vem repetida, uma seção da NF-e perde o título, o Balcão
#    Virtual muda de layout (abas ≠ acordeões), ou o total cai abaixo do piso (700). Opção nova no menu do Plantão
#    Fiscal é só aviso no log. Não pegam perguntas que o portal tirou de uma página
#    — e daí em diante tudo é derivação fiel até o servidor subir servindo menos do que existe.
DEPOIS="$(jq '.coleta.items | length' "$SNAPSHOT")"
echo "🔢 FAQs no snapshot: $DEPOIS (antes: $ANTES)"
if [ "$ANTES" -gt 0 ] && [ "$DEPOIS" -lt "$ANTES" ]; then
  echo "❌ o snapshot ENCOLHEU ($ANTES → $DEPOIS). FAQ some do acervo por duas razões — o portal"
  echo "   tirou do ar, ou a coleta falhou —, e o snapshot não distingue as duas."
  echo "   O estado bom está guardado. Para voltar a ele:"
  echo "     mv \"$SNAPSHOT.anterior\" \"$SNAPSHOT\" && mv \"$TREE.anterior\" \"$TREE\""
  echo "   Compare as linhas 'BA FAQs: <menu> | <página>: N perguntas' do log acima com as da rodada anterior"
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

# 4. Deriva os artefatos do snapshot. O `auli-collections ba` refaz TODOS os artefatos da entidade —
#    a árvore `docs/faqs/*.md` que o `auli update` lê e também os de serviços. É derivação pura e
#    determinística, então refazer os de serviços não muda um byte. O process resolve `data/` pelo
#    CWD, daí o subshell.
echo "🌳 derivando artefatos do snapshot (árvore docs/faqs)…"
( cd "$SERVER" && "$AULI_COLLECTIONS_BIN" ba )

# 5. Re-vetoriza só as FAQs (rodada parcial: serviços e pareceres ficam como estão no manifesto).
#    Se o snapshot de serviços tiver mudado sem re-vetorizar, o boot recusa nomeando `servicos` —
#    aí é `scripts/build-packs.sh ba servicos`.
echo "📦 re-vetorizando as FAQs da BA (~830 vetores)…"
"$ROOT/scripts/build-packs.sh" ba faqs

# 6. Sobe o servidor. `--no-build`: o passo 3 já compilou o mesmo binário no mesmo target.
#    A aba FAQs lê `ba-faqs-tree.json` do frontend estático: publique com scripts/deploy-frontend.sh.
if [ "$SERVE" -eq 1 ]; then
  echo "🚀 subindo o servidor… (a aba FAQs só muda depois do scripts/deploy-frontend.sh)"
  ( cd "$ROOT" && ./start_server.sh --no-build )
else
  echo "✅ pronto (--no-serve). Suba quando quiser: ./start_server.sh — e publique a aba com scripts/deploy-frontend.sh"
fi
