#!/usr/bin/env bash
# publicar.sh [--packs <id>] [--dry-run] [--no-tunnel] [--sim]
#
# Compila tudo, publica o frontend e sobe a API — nesta ordem, com UMA compilação no começo.
#
# ─── QUEM ELE CHAMA ────────────────────────────────────────────────────────────────────────────
#   passo 1  (nenhum script)              cargo build --release -p auli-cli -p auli-collections
#   passo 2  scripts/build-packs.sh       só com --packs; re-vetoriza uma entidade
#   passo 3  scripts/deploy-frontend.sh   que por sua vez chama scripts/build-frontend-public.sh
#   passo 4  ./start_server.sh            na RAIZ do repo, não em scripts/
#
# Sem --packs são 3 passos (o 2 some e os demais renumeram).
#
# ─── POR QUE ESTE SCRIPT EXISTE ────────────────────────────────────────────────────────────────
# O repositório tem UM binário (`auli`) com TRÊS modos (`server`, `update`, `bundle`), chamados por
# três scripts diferentes que tinham três políticas de compilação diferentes — e um segundo binário
# (`auli-collections`) que nenhum script compilava. O resultado era publicar artefato gerado por
# código velho, sem aviso: em 09/08/2026 os zips saíram com uma frase removida do código cinco horas
# antes (ver auli_operations.md §11).
#
# Aqui a compilação acontece UMA vez, no passo 1, e os scripts de baixo recebem os caminhos prontos
# em AULI_BIN / AULI_COLLECTIONS_BIN. Eles honram essas variáveis e pulam a própria compilação —
# então não há binário velho possível, e também não se compila quatro vezes.
#
# ─── USO ───────────────────────────────────────────────────────────────────────────────────────
#   scripts/publicar.sh                    # compila, publica o frontend, sobe a API
#   scripts/publicar.sh --packs rs         # + re-vetoriza o rs antes (~72 min, pico ~42 GB)
#                                          #   a entidade é obrigatória: não há modo "todas"
#   scripts/publicar.sh --dry-run          # local inteiro; nada sai da máquina, API não sobe
#   scripts/publicar.sh --sim              # sem o portão de confirmação (para automação)
#   scripts/publicar.sh --no-tunnel        # sobe a API só em localhost, sem o túnel Cloudflare
#
# ─── O QUE ELE NÃO FAZ, DE PROPÓSITO ───────────────────────────────────────────────────────────
#   • coleta (`auli-scraper-<uf>`) — bate em portal externo e leva horas; não pertence a publicação
#   • re-vetorizar sem você pedir — daí o `--packs` ser opt-in
set -euo pipefail

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PREPARO — argumentos, caminhos e a contagem de passos. Nada acontece com o sistema aqui.
# ═══════════════════════════════════════════════════════════════════════════════════════════════

PACKS=0
PACKS_ID=""
DRY_RUN=0
NO_TUNNEL=0
SIM=0

while [ $# -gt 0 ]; do
  case "$1" in
    --packs)
      PACKS=1
      # A entidade é OBRIGATÓRIA: o `build-packs.sh` vetoriza uma por vez e não tem modo "todas".
      # Nem deveria — re-vetorizar o acervo inteiro passa de duas horas (só o SP são ~49 min).
      case "${2:-}" in
        -*|"") echo "❌ --packs exige a entidade: --packs rs" >&2; exit 1 ;;
        *) PACKS_ID="$2"; shift ;;
      esac
      ;;
    --dry-run)   DRY_RUN=1 ;;
    --no-tunnel) NO_TUNNEL=1 ;;
    --sim)       SIM=1 ;;
    *) echo "❌ opção desconhecida: $1 (use --packs <id> | --dry-run | --no-tunnel | --sim)" >&2; exit 1 ;;
  esac
  shift
done

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
cd "$ROOT"

TARGET="${CARGO_TARGET_DIR:-$ROOT/auli-server/target}"
BIN_AULI="$TARGET/release/auli"
BIN_COLLECTIONS="$TARGET/release/auli-collections"

# O passo 2 (packs) é opcional, então o denominador muda: 4 passos com --packs, 3 sem.
TOTAL=4
[ "$PACKS" = 1 ] || TOTAL=3
PASSO=1

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 1 — COMPILAR
#
#   Chama:  cargo (nenhum script).
#   Produz: target/release/auli e target/release/auli-collections.
#
# Primeiro de propósito: erro de compilação tem que parar o deploy ANTES de ele gerar artefato
# nenhum. Se falhar aqui, nada foi tocado — nem public/, nem os zips, nem o servidor.
#
# Os dois pacotes numa chamada só: o cargo resolve o grafo comum uma única vez. E é o CARGO — não
# uma comparação de datas de arquivo — quem decide se há o que recompilar: ele usa fingerprint do
# grafo inteiro, o que cobre também mudança no Cargo.lock. Isso importa: um bump de `ort` altera os
# vetores sem tocar num único `.rs` (auli_pendencias.md §34.2), e mtime não veria.
#
# Com tudo em dia, leva menos de um segundo.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
echo "═══ $PASSO/$TOTAL  INICIANDO — compilar (release): auli + auli-collections"
(cd "$ROOT/auli-server" && cargo build --release -p auli-cli -p auli-collections)
[ -x "$BIN_AULI" ]        || { echo "❌ $BIN_AULI não existe depois do build" >&2; exit 1; }
[ -x "$BIN_COLLECTIONS" ] || { echo "❌ $BIN_COLLECTIONS não existe depois do build" >&2; exit 1; }
echo "✅  $PASSO/$TOTAL  CONCLUÍDO — binários em dia"
PASSO=$((PASSO + 1))

# A partir daqui NINGUÉM compila de novo. Os scripts de baixo honram estas duas variáveis e pulam o
# próprio passo de build — é isto que garante que packs, zips e API saem do MESMO código.
export AULI_BIN="$BIN_AULI"
export AULI_COLLECTIONS_BIN="$BIN_COLLECTIONS"

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 2 — RE-VETORIZAR   (opcional: só com --packs <id>)
#
#   Chama:  scripts/build-packs.sh <id>
#   Produz: data/<id>/packs/ (vetores + manifesto) e os índices das abas de jurisprudência.
#
# É CARO: o `auli update` re-vetoriza a entidade inteira, não a coleção que mudou. No rs isso são
# ~72 min e pico de ~42 GB de RAM. Por isso é opt-in e nunca roda sozinho.
#
# Quando é obrigatório: sempre que a árvore `data/<id>/docs/` mudar. O manifesto carimba um
# `docs_hash` sobre ela e o servidor RECUSA subir se divergir — então pular este passo depois de
# mexer na árvore não dá erro sutil, dá boot recusado.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
if [ "$PACKS" = 1 ]; then
  echo
  echo "═══ $PASSO/$TOTAL  INICIANDO — re-vetorizar '$PACKS_ID'   →  scripts/build-packs.sh"
  echo "    ⏳ caro: a entidade INTEIRA. No rs, ~72 min e pico de ~42 GB (o TARF são 22.476 docs)."
  scripts/build-packs.sh "$PACKS_ID"
  echo "✅  $PASSO/$TOTAL  CONCLUÍDO — packs e índices de '$PACKS_ID' regerados"
  PASSO=$((PASSO + 1))
fi

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PORTÃO — a última chance de desistir
#
#   Chama:  nada — o resumo dos packs é um python inline logo abaixo.
#
# Daqui para frente as ações SAEM DA MÁQUINA. O portão mostra o que vai subir — versão do frontend
# e contagem por coleção de cada pack — para a decisão ser informada, e não confiante.
#
# Não aparece com --dry-run (nada é enviado, não há o que confirmar) nem com --sim (automação).
#
# O resumo já morou em `scripts/resumo-packs.py`, extraído porque a primeira versão era um
# `python3 -c "..."` e precisava de três níveis de aspas aninhadas (bash → python → f-string),
# quebrando ao menor ajuste. O heredoc com delimitador ENTRE ASPAS (`<<'PY'`) resolve isso na raiz:
# o shell não expande nada lá dentro, então o python é literal e as aspas são só do python.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
if [ "$DRY_RUN" = 0 ] && [ "$SIM" = 0 ]; then
  echo
  echo "──────────────────────────────────────────────────────────────"
  echo " O que vem a seguir SAI DESTA MÁQUINA:"
  echo "   • publica o frontend em produção (troca atômica, com rollback automático no smoke)"
  echo "   • derruba a API que estiver rodando e sobe a nova$([ "$NO_TUNNEL" = 1 ] && echo " (sem túnel)" || echo " + túnel Cloudflare")"
  echo
  echo "   frontend: v$(grep -m1 '"version"' auli-frontend/package.json | sed -E 's/.*"([^"]+)".*/\1/')"
  # Só lê. Manifesto ilegível vira uma linha dizendo isso, não um traceback: o portão existe para
  # informar a decisão, e um erro de leitura aqui não é motivo para abortar o deploy.
  python3 - <<'PY' || echo "   (resumo dos packs indisponível)"
import json
import pathlib

manifestos = sorted(pathlib.Path("data").glob("*/packs/*.manifest.json"))
if not manifestos:
    print("   packs: nenhum manifesto em data/*/packs/")
for m in manifestos:
    entidade = m.parent.parent.name
    try:
        d = json.loads(m.read_text(encoding="utf-8"))
        partes = " ".join(f"{c['kind']}={c['count']}" for c in d["collections"])
        print(f"   packs {entidade}: {partes} · {d['built_at'][:10]}")
    except (OSError, ValueError, KeyError) as e:
        print(f"   packs {entidade}: ilegível ({type(e).__name__})")
PY
  echo "──────────────────────────────────────────────────────────────"
  read -r -p " Publicar? [s/N] " resposta
  case "$resposta" in [sS]|[sS][iI][mM]) ;; *) echo "Cancelado — nada foi enviado."; exit 0 ;; esac
fi

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 3 — PUBLICAR O FRONTEND
#
#   Chama:  scripts/deploy-frontend.sh  →  que internamente chama scripts/build-frontend-public.sh
#   Produz: o site novo em produção; a versão anterior fica preservada em /var/www/html.antigo.
#
# São 8 sub-passos com numeração própria: os `═══` abaixo são deste script, os `▶ n/8` são de lá.
# Em resumo, o que ele faz: regenera public/ a partir de data/, monta os 27 zips de download,
# confere que nenhuma entidade do registry ficou sem dados, builda o app, sobe para um staging
# remoto, troca por `mv` (atômico) e roda o smoke. Se o smoke falhar, ele mesmo desfaz a troca.
#
# O `deploy-frontend.sh` compila sozinho quando rodado à mão; aqui ele recebe AULI_BIN e pula,
# porque o passo 1 já garantiu o binário.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
echo
echo "═══ $PASSO/$TOTAL  INICIANDO — publicar o frontend   →  scripts/deploy-frontend.sh"
# Array, não expansão condicional. Duas armadilhas evitadas aqui, as duas silenciosas:
#   • `[ x = y ] && arr+=(...)` como ÚLTIMO comando retorna 1 quando o teste falha e, com `set -e`,
#     derruba o script no caso normal. Por isso o `if`.
#   • `"${arr[@]:-}"` com array vazio expande para UM argumento vazio, que o deploy rejeitaria como
#     "opção desconhecida: ". A forma `${arr[@]+"${arr[@]}"}` não expande para nada.
deploy_flags=()
if [ "$DRY_RUN" = 1 ]; then deploy_flags+=(--dry-run); fi
scripts/deploy-frontend.sh ${deploy_flags[@]+"${deploy_flags[@]}"}
echo "✅  $PASSO/$TOTAL  CONCLUÍDO — frontend publicado"
PASSO=$((PASSO + 1))

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 4 — SUBIR A API
#
#   Chama:  ./start_server.sh --no-build   (na RAIZ do repo, não em scripts/)
#   Produz: `auli server` na :3000 + o túnel do Cloudflare publicando api.auli.com.br.
#
# É o ÚLTIMO por construção, não por gosto: o `start_server.sh` não daemoniza. Ele fica em primeiro
# plano para que o `trap` dele consiga derrubar o cloudflared no Ctrl+C. Usamos `exec`, então este
# script é SUBSTITUÍDO pelo servidor — nada roda depois daquela linha, e é por isso que a mensagem
# de conclusão deste passo vem ANTES dela, e não depois.
#
# `--no-build` porque o passo 1 já compilou o mesmo binário, no mesmo target.
#
# Para não perder o terminal, rode o publicar.sh inteiro sob `nohup ... &` — ver
# auli_operations.md §5.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
echo
echo "═══ $PASSO/$TOTAL  INICIANDO — subir a API   →  ./start_server.sh"
if [ "$DRY_RUN" = 1 ]; then
  echo "    [dry-run] pulado — a API não é derrubada nem subida."
  echo "✅  $PASSO/$TOTAL  CONCLUÍDO — (dry-run: nada foi feito neste passo)"
  echo
  echo "✅ dry-run completo: compilou, gerou tudo localmente, não enviou nada."
  exit 0
fi

server_flags=(--no-build)
if [ "$NO_TUNNEL" = 1 ]; then server_flags+=(--no-tunnel); fi
echo "✅  $PASSO/$TOTAL  entregando o terminal ao servidor — Ctrl+C encerra API e túnel juntos"
echo
exec ./start_server.sh "${server_flags[@]}"
