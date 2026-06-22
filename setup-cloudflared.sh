#!/usr/bin/env bash
# setup-cloudflared.sh — one-time setup of the Cloudflare Tunnel that fronts the Auli API.
#
# Replaces the old ngrok tunnel: instead of a public ngrok URL, `cloudflared` dials OUT to
# Cloudflare and routes https://api.auli.com.br -> http://localhost:PORT. Cloudflare becomes the
# ONLY ingress (no public origin to bypass), so edge rate-limiting rules are actually enforced and
# CF-Connecting-IP reaches the app limiter.
#
# Run this ONCE (interactive: it opens a browser to authorize your Cloudflare account). After it
# finishes, `./start_server.sh` brings the tunnel up automatically. Re-running is safe (idempotent).
#
# Requires: cloudflared in PATH (installed at ~/.local/bin/cloudflared).
set -euo pipefail

export PATH="$HOME/.local/bin:$PATH"

TUNNEL_NAME="${TUNNEL_NAME:-auli-api}"
HOSTNAME="${HOSTNAME_CF:-api.auli.com.br}"
SERVICE_PORT="${PORT:-3000}"
CF_DIR="$HOME/.cloudflared"

command -v cloudflared >/dev/null || { echo "❌ cloudflared não está no PATH (~/.local/bin)."; exit 1; }
mkdir -p "$CF_DIR"

# 1) Authorize this machine against your Cloudflare account (writes ~/.cloudflared/cert.pem).
#    Opens a browser; pick the zone 'auli.com.br'. Skipped if already authorized.
if [ ! -f "$CF_DIR/cert.pem" ]; then
  echo "🔐 Autorizando no Cloudflare (vai abrir o navegador; escolha a zona auli.com.br)…"
  cloudflared tunnel login
else
  echo "🔐 Já autorizado (cert.pem presente)."
fi

# 2) Create the named tunnel (skipped if it already exists).
if cloudflared tunnel list 2>/dev/null | awk '{print $2}' | grep -qx "$TUNNEL_NAME"; then
  echo "🚇 Túnel '$TUNNEL_NAME' já existe."
else
  echo "🚇 Criando túnel '$TUNNEL_NAME'…"
  cloudflared tunnel create "$TUNNEL_NAME"
fi

# 3) Resolve the tunnel UUID and its credentials file.
TUNNEL_ID="$(cloudflared tunnel list 2>/dev/null | awk -v n="$TUNNEL_NAME" '$2==n{print $1; exit}')"
[ -n "$TUNNEL_ID" ] || { echo "❌ Não consegui resolver o UUID do túnel."; exit 1; }
CREDS="$CF_DIR/$TUNNEL_ID.json"
[ -f "$CREDS" ] || { echo "❌ Arquivo de credenciais ausente: $CREDS"; exit 1; }
echo "   UUID: $TUNNEL_ID"

# 4) Write the tunnel config (routes the hostname to the local server; 404 for anything else).
cat > "$CF_DIR/config.yml" <<YML
tunnel: $TUNNEL_ID
credentials-file: $CREDS

ingress:
  - hostname: $HOSTNAME
    service: http://localhost:$SERVICE_PORT
  - service: http_status:404
YML
echo "📝 Config escrito em $CF_DIR/config.yml"

# 5) Point the hostname at the tunnel (proxied CNAME -> <UUID>.cfargotunnel.com).
#    --overwrite-dns replaces the old ngrok CNAME on api.auli.com.br.
echo "🌐 Roteando $HOSTNAME -> túnel (substitui o CNAME do ngrok)…"
cloudflared tunnel route dns --overwrite-dns "$TUNNEL_NAME" "$HOSTNAME"

echo
echo "✅ Cloudflare Tunnel configurado."
echo "   Próximos passos:"
echo "   1) No painel Cloudflare: REATIVE a zona auli.com.br (estava em pause) e confira que"
echo "      o registro $HOSTNAME está PROXIED (nuvem laranja)."
echo "   2) Suba tudo com:  ./start_server.sh        (ele agora usa cloudflared, não ngrok)"
echo "   3) Crie a regra de rate limiting (ver auli_operations.md §10)."
