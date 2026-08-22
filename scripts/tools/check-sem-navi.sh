#!/usr/bin/env bash
# check-sem-navi.sh — guard-rail: o repositório não nomeia a unidade de atendimento onde o Auli é
# usado. O Auli é projeto pessoal e open-source, que serve a quem atende consultas tributárias em
# qualquer órgão de qualquer estado; nomear uma unidade sugere um vínculo que não existe e sugere
# que o projeto é DAQUELA unidade — o oposto do que a arquitetura por entidades foi feita para ser.
#
# A limpeza foi feita de uma vez (22/08/2026); esta guarda existe para o nome não voltar por
# esquecimento num comentário ou num doc novo — que é exatamente como ele entrou.
#
# Barato (só git grep; não compila nada). Bom para CI ou pre-commit.
set -euo pipefail

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/../.." && pwd)"
cd "$ROOT"

# **Por que não `-w`.** Parece o operador certo e não é: para o grep, `-` é separador de palavra,
# então a sigla dentro de `navi-header` (fixture de portal de terceiro) e dentro do nome deste
# próprio guarda contam como palavra inteira e casariam. O lookaround abaixo exige que a sigla não
# seja vizinha de `-` nem de caractere de palavra — isso descarta `navigation`, `navbar`, `navio`,
# `navi-header` e qualquer identificador hifenizado, sem precisar de lista de exceções por caminho.
#
# O único caminho excluído é este arquivo: um guarda tem de escrever o que proíbe, e a linha do
# padrão logo abaixo casaria consigo mesma. O `package-lock.json` sai por ser gerado — a sigla pode
# aparecer dentro de um hash.
PADRAO='(?<![-\w])navi(?![-\w])'
if achados="$(git grep -IinP "$PADRAO" -- . \
      ':!package-lock.json' ':!**/package-lock.json' \
      ':!scripts/tools/check-sem-navi.sh')"; then
  echo "❌ o repositório voltou a nomear a unidade de atendimento:"
  echo "$achados" | sed 's/^/   /'
  echo
  echo "   Troque por uma forma mais genérica — 'quem atende', 'o atendimento', 'estilo de balcão'."
  echo "   NUNCA por nome de órgão, cargo ou estado: a substituição tem de ficar MAIS genérica."
  exit 1
fi

echo "✅ sem menção à unidade de atendimento."
