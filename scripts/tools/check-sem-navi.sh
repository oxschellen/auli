#!/usr/bin/env bash
# check-sem-navi.sh — guard-rail: o repositório não nomeia a unidade de atendimento onde o Auli é
# usado. O Auli é projeto pessoal e open-source, que serve a quem atende consultas tributárias em
# qualquer órgão de qualquer estado; nomear uma unidade sugere um vínculo que não existe e sugere
# que o projeto é DAQUELA unidade — o oposto do que a arquitetura por entidades foi feita para ser.
#
# A limpeza foi feita de uma vez (TAREFA-SEM-NAVI, 22/08/2026); esta guarda existe para o nome não
# voltar por esquecimento num comentário ou num doc novo — que é exatamente como ele entrou.
#
# Barato (só git grep; não compila nada). Bom para CI ou pre-commit.
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/../.." && pwd)"
cd "$ROOT"

# `-w` pega `navigation`, `navbar`, `NavItem` e `navio` — mas NÃO resolve `navi-header`: para o
# grep, `-` é separador de palavra, então `navi` ali é palavra inteira. Daí a exceção explícita às
# fixtures dos scrapers, que são páginas CAPTURADAS de portais de terceiros: markup que não
# escrevemos e não editamos (editar quebra o que a fixture prova). Não é exceção de conveniência —
# é a fronteira entre a nossa prosa e o HTML alheio.
# O `package-lock.json` fica de fora porque é gerado e pode carregar o nome dentro de um hash.
if achados="$(git grep -Iinw "navi" -- . \
      ':!package-lock.json' ':!**/package-lock.json' \
      ':!auli-server/crates/scrapers/*/tests/fixtures/**')"; then
  echo "❌ o repositório voltou a nomear a unidade de atendimento:"
  echo "$achados" | sed 's/^/   /'
  echo
  echo "   Troque por uma forma mais genérica — 'quem atende', 'o atendimento', 'estilo de balcão'."
  echo "   NUNCA por nome de órgão, cargo ou estado: a substituição tem de ficar MAIS genérica."
  exit 1
fi

echo "✅ sem menção à unidade de atendimento."
