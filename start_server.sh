#!/bin/bash
# start_server.sh — compila e sobe o servidor da Auli (workspace `auli`, modo `server`) em :3000.
# Rode sem sudo, a partir de qualquer lugar:  ./start_server.sh
#
# Sobe também um túnel ngrok (api.auli.com.br) junto do servidor.
#
# Flags:    --no-build   pula o `cargo build` e sobe o binário já compilado (restart rápido).
#           --no-ngrok   sobe só o servidor local, sem o túnel ngrok.
# Variáveis opcionais: PORT (3000), PACKS_DIR (./packs), NGROK_DOMAIN (api.auli.com.br),
#                      CARGO_TARGET_DIR (reuso do build).
set -euo pipefail

NO_BUILD=0
NO_NGROK=0
for arg in "$@"; do
  case "$arg" in
    --no-build) NO_BUILD=1 ;;
    --no-ngrok) NO_NGROK=1 ;;
    *) echo "Flag desconhecida: $arg (use --no-build, --no-ngrok)" >&2; exit 2 ;;
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
NGROK_DOMAIN="${NGROK_DOMAIN:-api.auli.com.br}"
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

# Túnel ngrok em background (se habilitado e instalado); morre junto com o script.
NGROK_PID=""
if [ "$NO_NGROK" -eq 0 ]; then
  if command -v ngrok >/dev/null; then
    echo "🌐 ngrok http --domain=${NGROK_DOMAIN} ${PORT} (log: /tmp/auli-ngrok.log)"
    ngrok http --domain="$NGROK_DOMAIN" "$PORT" >/tmp/auli-ngrok.log 2>&1 &
    NGROK_PID=$!
    trap '[ -n "$NGROK_PID" ] && kill "$NGROK_PID" 2>/dev/null || true' EXIT INT TERM
  else
    echo "⚠️  ngrok não encontrado no PATH — subindo só o servidor local."
  fi
fi

echo "🚀 Subindo 'auli server' em :${PORT} (packs: ${PACKS_DIR}). Ctrl+C para parar."
# Sem `exec`: mantém o script vivo para o trap derrubar o ngrok ao sair (Ctrl+C).
"$BIN" server --port "$PORT" --packs-dir "$PACKS_DIR"
