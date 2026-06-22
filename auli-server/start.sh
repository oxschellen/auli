#!/bin/bash
# start.sh — derruba o servidor auli-server em execução, recompila e sobe um novo.
# Uso: ./start.sh            (build release + run em foreground)
#      ./start.sh --debug    (build debug)
set -euo pipefail

# Diretório raiz do projeto = onde este script está (funciona em qualquer caminho).
cd "$(dirname "$(readlink -f "$0")")"

BIN="auli-server"
PROFILE="release"
BUILD_FLAG="--release"
if [[ "${1:-}" == "--debug" ]]; then
  PROFILE="debug"
  BUILD_FLAG=""
fi

# 1) Derruba o servidor existente (se houver).
echo "🛑 Derrubando instância existente de ${BIN}..."
if pkill -x "$BIN"; then
  # Espera o processo encerrar e libera a porta 3000.
  for _ in $(seq 1 10); do
    pgrep -x "$BIN" >/dev/null || break
    sleep 0.5
  done
  pkill -9 -x "$BIN" 2>/dev/null || true
  echo "   instância anterior encerrada."
else
  echo "   nenhuma instância rodando."
fi

# 2) Compila a nova versão.
echo "🔨 Compilando (${PROFILE})..."
cargo build ${BUILD_FLAG}

# 3) Sobe o novo servidor.
echo "🚀 Subindo ${BIN} (${PROFILE}) em :3000..."
exec "./target/${PROFILE}/${BIN}"
