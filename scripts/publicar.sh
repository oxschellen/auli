#!/usr/bin/env bash
# publicar.sh [--packs [<id>]] [--dry-run] [--no-tunnel] [--sim] — compila tudo, publica o frontend
# e sobe a API, nesta ordem, com UMA compilação no começo.
#
# Existe porque o repositório tem UM binário (`auli`) com TRÊS modos (`server`, `update`, `bundle`),
# chamados por três scripts diferentes que tinham três políticas de compilação diferentes — e um
# segundo binário (`auli-collections`) que nenhum script compilava. O resultado era publicar artefato
# gerado por código velho sem aviso: em 09/08/2026 os zips saíram com uma frase que já tinha sido
# removida do código cinco horas antes (ver auli_operations.md §11).
#
# Aqui a compilação acontece UMA vez, no passo 1, e os scripts de baixo recebem os caminhos prontos
# em AULI_BIN / AULI_COLLECTIONS_BIN. Eles já sabem respeitar essas variáveis e pulam a própria
# compilação — então não há binário velho possível, e também não se compila quatro vezes.
#
# Uso:
#   scripts/publicar.sh                    # compila, publica o frontend, sobe a API
#   scripts/publicar.sh --packs rs         # + re-vetoriza o rs antes (72 min, ~42 GB de RAM)
#   scripts/publicar.sh --dry-run          # local inteiro; nada sai da máquina, API não sobe
#   scripts/publicar.sh --sim              # sem o portão de confirmação (para automação)
#
# O que este script NÃO faz, de propósito:
#   • coleta (`auli-scraper-<uf>`) — bate em portal externo, leva horas, não pertence a publicação;
#   • re-vetorizar sem você pedir — daí o `--packs` ser opt-in.
#
# O passo 4 usa `exec`: o `start_server.sh` fica em primeiro plano (ele segura o terminal para o
# `trap` derrubar o `cloudflared` no Ctrl+C), então ele é necessariamente o último.
set -euo pipefail

PACKS=0
PACKS_ID=""
DRY_RUN=0
NO_TUNNEL=0
SIM=0

while [ $# -gt 0 ]; do
  case "$1" in
    --packs)
      PACKS=1
      # `--packs` sozinho vale para todas as entidades; `--packs rs` só para uma. O argumento é
      # opcional, daí o teste: só consome o próximo se ele não for outra flag.
      case "${2:-}" in -*|"") ;; *) PACKS_ID="$2"; shift ;; esac
      ;;
    --dry-run)   DRY_RUN=1 ;;
    --no-tunnel) NO_TUNNEL=1 ;;
    --sim)       SIM=1 ;;
    *) echo "❌ opção desconhecida: $1 (use --packs [<id>] | --dry-run | --no-tunnel | --sim)" >&2; exit 1 ;;
  esac
  shift
done

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
cd "$ROOT"

TARGET="${CARGO_TARGET_DIR:-$ROOT/auli-server/target}"
BIN_AULI="$TARGET/release/auli"
BIN_COLLECTIONS="$TARGET/release/auli-collections"

TOTAL=4
[ "$PACKS" = 1 ] || TOTAL=3

echo "═══ 1/$TOTAL  compilar (release): auli + auli-collections"
# Os dois de uma vez: o cargo resolve o grafo comum uma única vez, e é o cargo — não uma comparação
# de datas — quem decide se há o que recompilar. Fingerprint cobre também mudança no Cargo.lock, que
# um teste de mtime não veria (um bump de `ort` altera os vetores sem tocar num `.rs`; §34.2).
# Com tudo em dia isso leva menos de um segundo.
(cd "$ROOT/auli-server" && cargo build --release -p auli-cli -p auli-collections)
[ -x "$BIN_AULI" ]        || { echo "❌ $BIN_AULI não existe depois do build" >&2; exit 1; }
[ -x "$BIN_COLLECTIONS" ] || { echo "❌ $BIN_COLLECTIONS não existe depois do build" >&2; exit 1; }
echo "✅ binários em dia"

# A partir daqui NINGUÉM compila de novo: os scripts de baixo honram estas variáveis e pulam o
# próprio passo de build. É isto que garante que os quatro artefatos saem do MESMO código.
export AULI_BIN="$BIN_AULI"
export AULI_COLLECTIONS_BIN="$BIN_COLLECTIONS"

PASSO=2

if [ "$PACKS" = 1 ]; then
  echo
  echo "═══ $PASSO/$TOTAL  re-vetorizar ${PACKS_ID:-todas as entidades}"
  echo "   ⏳ caro: ~72 min e pico de ~42 GB de RAM só para o rs (o TARF é 22.476 documentos)."
  scripts/build-packs.sh ${PACKS_ID:+"$PACKS_ID"}
  PASSO=$((PASSO + 1))
fi

# ── Portão: daqui para frente sai da máquina ──────────────────────────────────────────────────
# O `--dry-run` não precisa de portão: ele não envia nada. O `--sim` existe para automação.
if [ "$DRY_RUN" = 0 ] && [ "$SIM" = 0 ]; then
  echo
  echo "──────────────────────────────────────────────────────────────"
  echo " O que vem a seguir SAI DESTA MÁQUINA:"
  echo "   • publica o frontend em produção (troca atômica, com rollback automático no smoke)"
  echo "   • derruba a API que estiver rodando e sobe a nova$([ "$NO_TUNNEL" = 1 ] && echo " (sem túnel)" || echo " + túnel Cloudflare")"
  echo
  echo "   frontend: v$(grep -m1 '"version"' auli-frontend/package.json | sed -E 's/.*"([^"]+)".*/\1/')"
  python3 scripts/resumo-packs.py 2>/dev/null || echo "   (resumo dos packs indisponível)"
  echo "──────────────────────────────────────────────────────────────"
  read -r -p " Publicar? [s/N] " resposta
  case "$resposta" in [sS]|[sS][iI][mM]) ;; *) echo "Cancelado — nada foi enviado."; exit 0 ;; esac
fi

echo
echo "═══ $PASSO/$TOTAL  publicar o frontend"
# Array, não expansão condicional. Duas armadilhas evitadas aqui, as duas silenciosas:
#   • `[ x = y ] && arr+=(...)` como ÚLTIMO comando retorna 1 quando o teste falha e, com `set -e`,
#     derruba o script no caso normal. Por isso o `if`.
#   • `"${arr[@]:-}"` com array vazio expande para UM argumento vazio, que o deploy rejeitaria como
#     "opção desconhecida: ". A forma `${arr[@]+"${arr[@]}"}` não expande para nada.
deploy_flags=()
if [ "$DRY_RUN" = 1 ]; then deploy_flags+=(--dry-run); fi
scripts/deploy-frontend.sh ${deploy_flags[@]+"${deploy_flags[@]}"}
PASSO=$((PASSO + 1))

echo
echo "═══ $PASSO/$TOTAL  subir a API"
if [ "$DRY_RUN" = 1 ]; then
  echo "   [dry-run] pulado — a API não é derrubada nem subida."
  echo
  echo "✅ dry-run completo: compilou, gerou tudo localmente, não enviou nada."
  exit 0
fi

# `exec`: o start_server.sh não daemoniza (fica vivo para o trap derrubar o cloudflared no Ctrl+C),
# então ele é o último passo por construção. `--no-build` porque o passo 1 já compilou o mesmo
# binário, no mesmo target — recompilar aqui seria só um no-op ruidoso.
server_flags=(--no-build)
if [ "$NO_TUNNEL" = 1 ]; then server_flags+=(--no-tunnel); fi
exec ./start_server.sh "${server_flags[@]}"
