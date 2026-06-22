#!/bin/bash
# start_local.sh — derruba a instância existente, recompila e sobe o auli-server em :3000 (sem ngrok).
# Roda no terminal atual; não depende de gnome-terminal (funciona no WSL/headless).
set -euo pipefail

# Raiz do projeto = pasta-pai deste script (scripts/ -> raiz).
cd "$(dirname "$(readlink -f "$0")")/.."

BIN="auli-server"

# 1) Derruba o servidor existente (se houver) e libera a porta 3000.
# Casa pelo CAMINHO do binário (-f), não pelo nome exato (-x): assim pega tanto
# "auli-server" quanto binários antigos com sufixo (ex. "auli-server-local", de
# antes do rename do pacote) que ainda estejam segurando a porta 3000.
PAT="target/(debug|release)/auli-server"
echo "🛑 Derrubando instância existente (${PAT})..."
if pgrep -af "$PAT" >/dev/null; then
  pkill -f "$PAT" 2>/dev/null || true
  for _ in $(seq 1 10); do
    pgrep -f "$PAT" >/dev/null || break
    sleep 0.5
  done
  pkill -9 -f "$PAT" 2>/dev/null || true
  echo "   instância anterior encerrada."
else
  echo "   nenhuma instância rodando."
fi

# O vector store roda in-process (Rust puro, persistido em ./vectors); não há banco
# separado para iniciar — sobe junto com o servidor.

# 2) Compila e 3) sobe o novo servidor (Ctrl+C derruba limpo).
echo "🔨 Compilando (release)..."
cargo build --release
echo "🚀 Subindo ${BIN} em :3000..."
exec ./target/release/auli-server
