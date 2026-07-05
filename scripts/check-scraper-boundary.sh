#!/usr/bin/env bash
# check-scraper-boundary.sh — guarda o invariante geográfico da fronteira (D-C1).
#
# NADA fora de `crates/scrapers/` pode referenciar o `auli-scraper-kit` (nem pelo módulo
# `auli_scraper_kit` nem pelo nome do crate `auli-scraper-kit`). A fronteira scraper→collections
# (tipos + versão + caminho + I/O do snapshot) mora no `auli-contract`, o crate leve; o kit é o
# "como raspar" e é exclusivo dos scrapers. Assim o `auli-collections` (derivação offline) e o
# engine nunca arrastam a árvore de rede do scraper (ureq etc.).
#
# Barato: só grep, não compila Rust (fastembed/ort são pesados). Bom para CI ou pre-commit.
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
CRATES="$ROOT/auli-server/crates"

# Referências ao kit (módulo ou crate) em código/manifesto FORA de crates/scrapers/.
hits="$(grep -rn "auli_scraper_kit\|auli-scraper-kit" "$CRATES" \
          --include=*.rs --include=*.toml | grep -v "/scrapers/" || true)"

if [ -n "$hits" ]; then
  echo "❌ invariante da fronteira violado (D-C1): código fora de crates/scrapers/ referencia o"
  echo "   auli-scraper-kit. A fronteira (tipos + I/O do snapshot) mora no auli-contract; o kit é"
  echo "   exclusivo dos scrapers. Mova o que precisar para o contrato."
  echo "$hits"
  exit 1
fi

echo "✅ fronteira OK: nada fora de crates/scrapers/ depende do auli-scraper-kit."
