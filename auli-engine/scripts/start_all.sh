#!/bin/bash
# start_all.sh — abre dois gnome-terminals: o `auli server` e o túnel ngrok (api.auli.com.br).
# Para uma execução headless/WSL sem ngrok, use ./scripts/start_local.sh diretamente.
#
# Variáveis de ambiente (opcionais): PORT (default 3000), PACKS_DIR (default ./packs),
# NGROK_DOMAIN (default api.auli.com.br). PORT/PACKS_DIR são repassadas ao start_local.sh.
#
# Requer gnome-terminal e ngrok instalados. Gere os pacotes antes com `auli update`.
set -euo pipefail

ROOT="$(dirname "$(readlink -f "$0")")/.."
cd "$ROOT"
ROOT="$(pwd)" # caminho absoluto para os subshells dos terminais

PORT="${PORT:-3000}"
PACKS_DIR="${PACKS_DIR:-./packs}"
NGROK_DOMAIN="${NGROK_DOMAIN:-api.auli.com.br}"

if ! command -v gnome-terminal >/dev/null; then
  echo "❌ gnome-terminal não encontrado. Para rodar sem ele, use: ./scripts/start_local.sh" >&2
  exit 1
fi

# Terminal 1: o servidor Rust (compila + sobe; vector store + embedder rodam in-process nele).
# Reaproveita o start_local.sh — fonte única da lógica de build/subida.
gnome-terminal -- bash -c "cd '$ROOT' && PORT='$PORT' PACKS_DIR='$PACKS_DIR' ./scripts/start_local.sh; exec bash"

# Terminal 2: túnel ngrok para o domínio público.
gnome-terminal -- bash -c "ngrok http --domain='$NGROK_DOMAIN' '$PORT'; exec bash"

echo "🚀 Abrindo dois terminais: 'auli server' em :${PORT} e ngrok (${NGROK_DOMAIN})."
echo "   (sem gnome-terminal/ngrok? rode ./scripts/start_local.sh no terminal atual.)"
