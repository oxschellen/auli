#!/usr/bin/env bash
# update-pr-faqs.sh — atualiza as FAQs do PR de ponta a ponta:
#   re-scrape → deriva a árvore → re-vetoriza os packs → sobe o servidor.
#
# Irmão do `update-sc-faqs.sh`. A coleta das FAQs do PR vai SEMPRE à rede (D-PRFAQ-7) e o cache em
# data/pr/raw/cache/faqs/ só é lido com `--usecache`; mesmo assim ele é apagado antes da coleta.
#
# Uso:   scripts/tools/update-pr-faqs.sh              # scrape + rebuild + sobe servidor
#        scripts/tools/update-pr-faqs.sh --no-serve   # para antes de subir o servidor
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/../.." && pwd)"   # raiz do repo
SERVER="$ROOT/auli-server"
SNAPSHOT="$ROOT/data/pr/pr-faqs-snapshot.json"
TREE="$ROOT/data/pr/raw/pr-faqs-tree.json"

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
# que é o que o `--usecache` reproduz. Só o de faqs: os outros namespaces de data/pr/raw/cache/ ficam.
CACHE="$ROOT/data/pr/raw/cache/faqs"
if [ -d "$CACHE" ]; then
  echo "🗑️  apagando o cache de faqs ($(find "$CACHE" -type f | wc -l) arquivos)"
  rm -rf "$CACHE"
fi

# 1. Re-scrape das faqs → grava data/pr/raw/pr-faqs-tree.json e data/pr/pr-faqs-snapshot.json.
#    O folha de atendimento.fazenda.pr.gov.br vence em 21/11/2026, e o agent só confia no intermediário
#    embutido (D-PRFAQ-4): se a renovação vier de outro emissor, a coleta para no handshake com a mensagem
#    do rustls, que não diz o que fazer. Daí a orientação abaixo.
echo "🕸️  scrapeando faqs do PR…"
if ! ( cd "$SERVER" && cargo run --release -p auli-scraper-pr -- faqs ); then
  echo "❌ a coleta falhou. Se o erro for de TLS/certificado (o folha de atendimento.fazenda.pr.gov.br"
  echo "   vencia em 21/11/2026): confira o emissor novo com"
  echo "     openssl s_client -connect atendimento.fazenda.pr.gov.br:443 -servername atendimento.fazenda.pr.gov.br </dev/null 2>/dev/null | grep ' i:'"
  echo "   e, se mudou, troque o SECTIGO_DV_R36_PEM em auli-scraper-pr/src/faqs.rs (D-PRFAQ-4)."
  exit 1
fi

# 2. Guard de completude. As guardas do scraper (`faqs.rs`, D-PRFAQ-6) param ANTES de gravar quando
#    um referente do index vem sem pergunta, o parse perde um painel, a identidade não confere ou um
#    número se repete. Não pegam um referente que saiu do index nem um que voltou com menos perguntas
#    do que tinha — o portal pode ter tirado do ar, ou a página veio pela metade, e daí em diante tudo
#    é derivação fiel até o servidor subir servindo menos do que existe.
DEPOIS="$(jq '.coleta.items | length' "$SNAPSHOT")"
echo "🔢 FAQs no snapshot: $DEPOIS (antes: $ANTES)"
if [ "$ANTES" -gt 0 ] && [ "$DEPOIS" -lt "$ANTES" ]; then
  echo "❌ o snapshot ENCOLHEU ($ANTES → $DEPOIS). FAQ some do acervo por duas razões — o portal"
  echo "   tirou do ar, ou a coleta falhou —, e o snapshot não distingue as duas."
  echo "   O estado bom está guardado. Para voltar a ele:"
  echo "     mv \"$SNAPSHOT.anterior\" \"$SNAPSHOT\" && mv \"$TREE.anterior\" \"$TREE\""
  echo "   Compare as linhas 'referente X (Nome): A assuntos, N perguntas' do log acima com as da"
  echo "   rodada anterior e rode de novo (a comparação segue contra o estado bom)."
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

# 4. Deriva os artefatos do snapshot. O `auli-collections pr` refaz TODOS os artefatos da entidade —
#    a árvore `docs/faqs/*.md` que o `auli update` lê e também os de serviços. É derivação pura e
#    determinística, então refazer os de serviços não muda um byte. O process resolve `data/` pelo
#    CWD, daí o subshell.
echo "🌳 derivando artefatos do snapshot (árvore docs/faqs)…"
( cd "$SERVER" && "$AULI_COLLECTIONS_BIN" pr )

# 5. Re-vetoriza só as FAQs (rodada parcial: serviços e pareceres ficam como estão no manifesto).
#    Se o snapshot de serviços tiver mudado sem re-vetorizar, o boot recusa nomeando `servicos` —
#    aí é `scripts/build-packs.sh pr servicos`.
echo "📦 re-vetorizando as FAQs do PR (~1.320 vetores)…"
"$ROOT/scripts/build-packs.sh" pr faqs

# 6. Sobe o servidor. `--no-build`: o passo 3 já compilou o mesmo binário no mesmo target.
#    A aba FAQs lê `pr-faqs-tree.json` do frontend estático: publique com scripts/deploy-frontend.sh.
if [ "$SERVE" -eq 1 ]; then
  echo "🚀 subindo o servidor… (a aba FAQs só muda depois do scripts/deploy-frontend.sh)"
  ( cd "$ROOT" && ./start_server.sh --no-build )
else
  echo "✅ pronto (--no-serve). Suba quando quiser: ./start_server.sh — e publique a aba com scripts/deploy-frontend.sh"
fi
