#!/bin/bash
# start_local.sh — derruba a instância existente, recompila e sobe `auli server` em :3000 (sem ngrok).
# Roda no terminal atual; não depende de gnome-terminal (funciona no WSL/headless).
#
# Variáveis de ambiente (todas opcionais, com defaults):
#   PORT             porta HTTP                         (default 3000)
#   PACKS_DIR        pasta dos pacotes de vetores       (default ./packs)
#   EMBED_CACHE_DIR  cache do modelo BGE-M3 (ONNX)      (default ./models)
#
# Pré-requisitos de build: cmake + compilador C (para aws-lc-sys) e rede no primeiro build
# (`ort` baixa o ONNX Runtime; o BGE-M3 baixa do Hugging Face para EMBED_CACHE_DIR no 1º uso).
# O server também precisa de um `.env` na raiz do workspace (LLM_API_*, JWT_*, DATABASE_URL):
# ele conecta no Postgres para auth no boot. Gere os pacotes antes com:
#   auli update --entity rs --source ../auli-server/entities/rs --out ./packs
set -euo pipefail

# Raiz do workspace = pasta-pai deste script (scripts/ -> raiz).
cd "$(dirname "$(readlink -f "$0")")/.."

PORT="${PORT:-3000}"
PACKS_DIR="${PACKS_DIR:-./packs}"
export EMBED_CACHE_DIR="${EMBED_CACHE_DIR:-./models}"

BIN="./target/release/auli"

# 1) Derruba o `auli server` existente (se houver) e libera a porta.
# Casa pelo CAMINHO do binário + subcomando `server` (-f), nunca um `auli update` em curso.
PAT="target/(debug|release)/auli server"
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

# O vector store e o embedder rodam in-process; o server só LÊ os pacotes de $PACKS_DIR.
# Não há serviço de vetor/embedding separado para subir.

# 2) Compila e 3) sobe o servidor (Ctrl+C derruba limpo).
echo "🔨 Compilando (release)..."
cargo build --release --bin auli
echo "🚀 Subindo 'auli server' em :${PORT} (packs: ${PACKS_DIR})..."
exec "$BIN" server --port "$PORT" --packs-dir "$PACKS_DIR"
