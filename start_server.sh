#!/bin/bash
# start_server.sh — compila e sobe o servidor da Auli (workspace `auli`, modo `server`) em :3000.
# Rode sem sudo, a partir de qualquer lugar:  ./start_server.sh
#
# Sobe também o túnel do Cloudflare (cloudflared) que publica api.auli.com.br -> localhost:PORT.
# Configure o túnel UMA vez com ./setup-cloudflared.sh (login + criação + DNS).
#
# Flags:    --no-build    pula o `cargo build` e sobe o binário já compilado (restart rápido).
#           --no-tunnel   sobe só o servidor local, sem o túnel Cloudflare.
#                         (--no-ngrok continua aceito como apelido de --no-tunnel.)
# Variáveis opcionais: PORT (3000), PACKS_DIR (./packs), TUNNEL_NAME (auli-api),
#                      CARGO_TARGET_DIR (reuso do build).
set -euo pipefail

NO_BUILD=0
NO_TUNNEL=0
for arg in "$@"; do
  case "$arg" in
    --no-build) NO_BUILD=1 ;;
    --no-tunnel|--no-ngrok) NO_TUNNEL=1 ;;
    *) echo "Flag desconhecida: $arg (use --no-build, --no-tunnel)" >&2; exit 2 ;;
  esac
done

ROOT="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"   # .../auli_new
WS="$ROOT/auli"                                          # workspace Cargo (vector-store/auli-core/auli-cli)

# cmake desta máquina (instalado via pip em ~/.local/bin) + compat de policy do cmake 4.
# Inócuo onde já houver cmake de sistema.
export PATH="$HOME/.local/bin:$PATH"
export CMAKE_POLICY_VERSION_MINIMUM="${CMAKE_POLICY_VERSION_MINIMUM:-3.5}"

# Reaproveita os artefatos já compilados (fastembed/ort/aws-lc) -> build incremental rápido.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/auli-server/target}"

PORT="${PORT:-3000}"
PACKS_DIR="${PACKS_DIR:-./packs}"
TUNNEL_NAME="${TUNNEL_NAME:-auli-api}"
BIN="$CARGO_TARGET_DIR/release/auli"

cd "$WS"

# O server lê ./entities (entity.json + prompt.txt). Aponta para o baseline se ainda não existir.
[ -e entities ] || ln -s ../auli-server/entities entities

# Derruba uma instância anterior, se houver, para liberar a porta.
pkill -f "release/auli server" 2>/dev/null && sleep 1 || true

if [ "$NO_BUILD" -eq 1 ]; then
  if [ ! -x "$BIN" ]; then
    echo "❌ --no-build, mas o binário não existe em $BIN. Rode uma vez sem a flag." >&2
    exit 1
  fi
  echo "⏭️  Pulando build (--no-build)."
else
  echo "🔨 Compilando (release)..."
  cargo build --release --bin auli
fi

# Túnel Cloudflare (cloudflared) em background; morre junto com o script.
TUNNEL_PID=""
if [ "$NO_TUNNEL" -eq 0 ]; then
  if command -v cloudflared >/dev/null && [ -f "$HOME/.cloudflared/config.yml" ]; then
    echo "🌐 cloudflared tunnel run ${TUNNEL_NAME} (log: /tmp/auli-cloudflared.log)"
    cloudflared tunnel run "$TUNNEL_NAME" >/tmp/auli-cloudflared.log 2>&1 &
    TUNNEL_PID=$!
    trap '[ -n "$TUNNEL_PID" ] && kill "$TUNNEL_PID" 2>/dev/null || true' EXIT INT TERM
  else
    echo "⚠️  Túnel Cloudflare não configurado — subindo só o servidor local."
    echo "    Rode ./setup-cloudflared.sh uma vez (login + criação + DNS), ou use --no-tunnel."
  fi
fi

echo "🚀 Subindo 'auli server' em :${PORT} (packs: ${PACKS_DIR}). Ctrl+C para parar."
# Sem `exec`: mantém o script vivo para o trap derrubar o cloudflared ao sair (Ctrl+C).
"$BIN" server --port "$PORT" --packs-dir "$PACKS_DIR"
