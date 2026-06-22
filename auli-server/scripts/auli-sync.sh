#!/usr/bin/env bash
#
# auli-sync.sh — sincroniza a cópia de EDIÇÃO (Windows) para a cópia de BUILD (WSL).
#
# Direção: Windows  ->  WSL   (one-way push; o Windows é a fonte da verdade do código)
#
# IMPORTANTE: o caminho do Windows tem acento e espaços ("Área de Trabalho") + OneDrive.
# Por isso ele NUNCA é passado como argumento via PowerShell->wsl.exe — fica literal,
# em UTF-8, DENTRO deste arquivo. Rode este script a partir do WSL (o launcher
# scripts/sync-to-wsl.ps1 faz isso). Veja também a nota de build no CLAUDE.md.
#
# Uso:
#   bash auli-sync.sh             # sincroniza de verdade
#   bash auli-sync.sh --dry-run   # mostra o que mudaria, sem escrever nada
#
set -euo pipefail

SRC="/mnt/c/Users/carlo/OneDrive/Área de Trabalho/auli-server-local"
DST="/home/ubu/auli-server"

DRY=""
if [ "${1:-}" = "--dry-run" ] || [ "${1:-}" = "-n" ]; then
  DRY="--dry-run"
  echo ">> DRY-RUN: nenhuma alteração será gravada"
fi

if [ ! -d "$SRC" ]; then
  echo "ERRO: origem não encontrada: $SRC" >&2
  exit 1
fi
if [ ! -d "$DST" ]; then
  echo "ERRO: destino não encontrado: $DST" >&2
  exit 1
fi

echo ">> Origem (Windows): $SRC"
echo ">> Destino (WSL):    $DST"

# Sincroniza a árvore inteira com --delete, MAS protege tudo que é específico do
# ambiente WSL ou é gerado (esses padrões ficam de fora tanto da cópia quanto da
# remoção, então a cópia do WSL preserva seu próprio .env, chaves, vetores etc.).
#
#   --delete            espelha (remove no destino o que sumiu na origem)
#   excludes ancorados  com "/" => relativos à raiz do projeto
rsync -a --delete --itemize-changes $DRY \
  --exclude='/.git/' \
  --exclude='/target/' \
  --exclude='/vectors/' \
  --exclude='/models/' \
  --exclude='/logs/' \
  --exclude='/.claude/' \
  --exclude='/.vscode-server/' \
  --exclude='/.env' \
  --exclude='jwt_private_key.pem' \
  --exclude='jwt_public_key.pem' \
  --exclude='*.pem' \
  --exclude='.DS_Store' \
  --exclude='Thumbs.db' \
  --exclude='desktop.ini' \
  "$SRC/" "$DST/"

# Normaliza fim de linha dos shell scripts copiados (Windows pode gravar CRLF, que
# quebra a execução no WSL). Só toca em .sh, e só se houver CR.
if [ -z "$DRY" ]; then
  find "$DST/scripts" -name '*.sh' -type f -print0 2>/dev/null \
    | while IFS= read -r -d '' f; do
        if grep -lq $'\r' "$f" 2>/dev/null; then
          sed -i 's/\r$//' "$f"
          echo ">> normalizado CRLF->LF: ${f#$DST/}"
        fi
      done
fi

echo ">> Sincronização concluída."
