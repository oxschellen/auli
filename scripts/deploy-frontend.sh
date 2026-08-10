#!/usr/bin/env bash
# deploy-frontend.sh [--dry-run] [--allow-vazias] — publica o auli-frontend no servidor.
#
# ─── QUEM ELE CHAMA ────────────────────────────────────────────────────────────────────────────
#   passo 1  cargo build --release -p auli-cli      (pulado se AULI_BIN vier definido)
#   passo 2  scripts/build-frontend-public.sh       regenera public/<id>/ a partir de data/
#   passo 3  target/release/auli bundle             monta os 27 zips em public/downloads/
#   passo 4  (nenhum)                               guarda local: lê config/registry.toml
#   passo 5  npm run build                          em auli-frontend/, produz dist/
#   passo 6  ssh + scp                              sobe dist/ para um staging no servidor
#   passo 7  ssh                                    troca atômica (dois `mv`)
#   passo 8  curl                                   smoke na origem; falhou, reverte sozinho
#
# Publica SÓ o frontend. A API é independente e sobe pelo `./start_server.sh`, na raiz do repo —
# o site é estático e serve as abas do próprio origin; só o chat fala com a API.
#
# ─── POR QUE ELE EXISTE ────────────────────────────────────────────────────────────────────────
# Substitui a sequência manual (build-frontend-public → npm run build → rm -rf remoto → scp), que
# tinha dois furos: (1) entidade sem `data/` gerava `public/<id>/` VAZIO e subia assim, em silêncio;
# (2) o webroot ficava esvaziado durante o upload, e um `index.html` em cache apontando para um
# bundle que o `rm -rf` acabou de apagar deixava a app quebrada até o cache expirar.
#
# Aqui o upload vai para um diretório de staging e a publicação é UM `mv` — atômica. O antigo vira
# `<webroot>.antigo`, então o rollback é outro `mv`. Se o smoke test falhar, o rollback é automático.
#
# ─── USO ───────────────────────────────────────────────────────────────────────────────────────
#   scripts/deploy-frontend.sh --dry-run     # local completo; mostra o que rodaria no servidor
#   scripts/deploy-frontend.sh               # deploy de verdade (pergunta antes de começar)
#   scripts/deploy-frontend.sh --sim         # sem a confirmação (automação)
#
# ─── AJUSTE POR AMBIENTE (variáveis) ───────────────────────────────────────────────────────────
#   DEPLOY_HOST   destino ssh/scp           (padrão: root@novoauli.vps-kinghost.net)
#   DEPLOY_PORT   porta ssh                 (padrão: 22)
#   WEBROOT       raiz servida pelo Apache  (padrão: /var/www/html)
#   SMOKE_HOST    nome que o certificado cobre  (padrão: auli.com.br)
#   SMOKE_ORIGIN  onde o smoke CONECTA          (padrão: o host de DEPLOY_HOST)
#   SMOKE_BASE    URL base do smoke test        (padrão: https://$SMOKE_HOST)
#   SMOKE_INSECURE=1  ignora o certificado no smoke (padrão: 0)
#   AULI_BIN      binário do `auli` a usar nos zips (padrão: compila `target/release/auli`).
#                 Definir a variável PULA a compilação do passo 1 — manter o binário em dia passa a
#                 ser sua responsabilidade.
#
# ─── POR QUE O SMOKE BATE NA ORIGEM, E NÃO NO DOMÍNIO ──────────────────────────────────────────
# O smoke conecta DIRETO na origem (`SMOKE_ORIGIN`, a VPS) e manda o SNI de `SMOKE_HOST` — via
# `curl --resolve`. Assim ele valida a cadeia TLS de verdade E continua testando o servidor, não a
# borda: o domínio público está atrás de Cloudflare, e verificar por ele mediria o cache do CDN
# junto — logo depois de um deploy isso mente nas duas direções. O certificado do servidor cobre
# só `auli.com.br`, então bater no hostname da VPS por HTTPS falharia no nome (era o que o
# `SMOKE_INSECURE=1` contornava, ao custo de deixar o gate cego para problema de certificado).
set -euo pipefail

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PREPARO — variáveis, argumentos e o auxiliar de ssh. Nada sai da máquina aqui.
# ═══════════════════════════════════════════════════════════════════════════════════════════════

DEPLOY_HOST="${DEPLOY_HOST:-root@novoauli.vps-kinghost.net}"
DEPLOY_PORT="${DEPLOY_PORT:-22}"
WEBROOT="${WEBROOT:-/var/www/html}"
SMOKE_HOST="${SMOKE_HOST:-auli.com.br}"
SMOKE_ORIGIN="${SMOKE_ORIGIN:-${DEPLOY_HOST#*@}}"
SMOKE_BASE="${SMOKE_BASE:-https://$SMOKE_HOST}"
SMOKE_INSECURE="${SMOKE_INSECURE:-0}"

STAGING="$WEBROOT.novo"
ANTIGO="$WEBROOT.antigo"

DRY_RUN=0
ALLOW_VAZIAS=0
SIM=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    --allow-vazias) ALLOW_VAZIAS=1 ;;
    --sim) SIM=1 ;;
    *) echo "❌ opção desconhecida: $arg (use --dry-run | --allow-vazias | --sim)"; exit 1 ;;
  esac
done

ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
FRONT="$ROOT/auli-frontend"
cd "$ROOT"

# `ssh`/`scp` só rodam de verdade fora do dry-run; no dry-run o comando é impresso.
remoto() {
  if [ "$DRY_RUN" = 1 ]; then
    echo "   [dry-run] ssh: $*"
  else
    ssh -p "$DEPLOY_PORT" "$DEPLOY_HOST" "$@"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PORTÃO — a confirmação, ANTES de qualquer trabalho
#
# Fica aqui, e não junto do passo 6 (o primeiro que sai da máquina), por uma razão prática: entre
# aqui e lá são vários minutos de compilação, zips e build. Perguntar no começo deixa você responder
# e sair de perto; perguntar no meio obrigaria a ficar de babá esperando a pergunta.
#
# Mora NESTE script, e não num invólucro acima, porque quem publica é este. Portão em invólucro
# protege pela metade: quem chama o script de baixo direto — o caminho normal — não passa por ele.
#
# Não aparece com --dry-run (nada é enviado, não há o que confirmar) nem com --sim (automação).
#
# O resumo é python inline por heredoc com delimitador ENTRE ASPAS (`<<'PY'`): o shell não expande
# nada lá dentro, então as aspas são só do python. A primeira versão disto era um `python3 -c "..."`
# e precisava de três níveis de aspas aninhadas, quebrando a cada ajuste.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
if [ "$DRY_RUN" = 0 ] && [ "$SIM" = 0 ]; then
  echo "──────────────────────────────────────────────────────────────"
  echo " Isto PUBLICA EM PRODUÇÃO ($DEPLOY_HOST:$WEBROOT)."
  echo " A troca é atômica e a versão anterior fica em $ANTIGO; se o smoke falhar, reverte sozinho."
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
  case "$resposta" in [sS]|[sS][iI][mM]) ;; *) echo "Cancelado — nada foi feito."; exit 0 ;; esac
  echo
fi

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 1/8 — COMPILAR O `auli`
#
#   Chama:  cargo build --release -p auli-cli   (ou pula, se AULI_BIN vier de fora)
#   Produz: target/release/auli — usado no passo 3 para montar os zips.
#
# Primeiro de propósito: erro de compilação tem que parar o deploy ANTES de ele gerar artefato
# nenhum. Se quebrar aqui, public/ e dist/ ficam como estavam e o servidor nem foi tocado.
#
# Até 09/08/2026 este script NUNCA compilava, e o resultado foi silencioso: os zips saíram com
# "Coleta em andamento" no README do `tarf/` cinco horas depois de a frase ter sido removida do
# código, porque o binário era anterior ao commit. Nenhum teste pega isso — a suíte roda contra o
# código-fonte, e o artefato publicado vem do binário. Commit em Rust não muda binário nenhum, e o
# que sobe é o binário.
#
# Quem decide se precisa recompilar é o CARGO, não este script: ele resolve por fingerprint do grafo
# inteiro, o que cobre também mudança de dependência no Cargo.lock — um bump de `ort` altera os
# vetores sem tocar num `.rs` (auli_pendencias.md §34.2), e comparar data de arquivo não veria.
# Quando já está fresco, é um no-op de menos de um segundo.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
echo "▶ 1/8  INICIANDO — compilar o auli (release)"
if [ -n "${AULI_BIN:-}" ]; then
  echo "   AULI_BIN definido ($AULI_BIN) — pulando: o binário é seu, e mantê-lo atual também."
else
  (cd "$ROOT/auli-server" && cargo build --release -p auli-cli)
fi
echo "✅ 1/8  CONCLUÍDO — auli em dia"

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 2/8 — REGENERAR `public/` A PARTIR DE `data/`
#
#   Chama:  scripts/build-frontend-public.sh
#   Produz: auli-frontend/public/<id>/ para as 27 entidades + public/manuais/.
#
# O frontend serve conteúdo de referência ESTÁTICO do próprio origin — as abas leem
# `/<id>/<id>-<arquivo>`. Esses arquivos são cópia determinística de `data/<id>/{raw,ref}/`, e não
# entram no git (`auli-frontend/.gitignore` ignora `public/*/`): o que sobe é o `dist/`, montado no
# passo 5 a partir daqui.
#
# Regenerar sempre, e não só quando `data/` mudou, é de propósito: é barato e elimina a classe de
# bug em que o índice servido diverge dos packs — foi o que aconteceu em 09/08/2026, com a aba do
# TARF mostrando 3.800 acórdãos enquanto o chat respondia de 22.476.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
echo
echo "▶ 2/8  INICIANDO — public/ a partir de data/   →  scripts/build-frontend-public.sh"
scripts/build-frontend-public.sh
echo "✅ 2/8  CONCLUÍDO — public/ regenerado"

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 3/8 — MONTAR OS ZIPS DE DOWNLOAD
#
#   Chama:  target/release/auli bundle --data-root data --out auli-frontend/public/downloads
#   Produz: 27 `.zip` (~169 MB) + downloads.json, dentro de public/.
#
# Aqui, e não dentro do build-frontend-public.sh: aquele é rodado sozinho com frequência e não deve
# arrastar ~169 MB de zip junto.
#
# Tem de vir ANTES do passo 5, porque o Vite copia `public/` inteiro para `dist/` — é assim que os
# zips chegam ao servidor, eles não têm caminho de publicação próprio. (É também por isso que o
# `dist/` pesa ~253 MB: os zips vão dentro.)
#
# O bundle é determinístico: quando o conteúdo não muda, ele nem reescreve os arquivos.
#
# O binário vem do passo 1, que garante que está em dia. O ramo do `else` só sobra para quem passou
# um AULI_BIN que não existe — com o padrão, o cargo já teria falhado antes de chegar aqui.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
echo
echo "▶ 3/8  INICIANDO — zips de download por estado (public/downloads/)"
BUNDLE_BIN="${AULI_BIN:-$ROOT/auli-server/target/release/auli}"
if [ -x "$BUNDLE_BIN" ]; then
  "$BUNDLE_BIN" bundle --data-root "$ROOT/data" --out "$FRONT/public/downloads"
else
  echo "⚠️  $BUNDLE_BIN não encontrado — os zips NÃO foram regerados."
  echo "   Aponte o AULI_BIN para um binário que exista, ou tire a variável para o script compilar."
  echo "   Seguindo: public/downloads/ vai com o conteúdo da última geração (ou vazio, na 1ª vez)."
fi
echo "✅ 3/8  CONCLUÍDO — zips prontos em public/downloads/"

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 4/8 — GUARDA: NENHUMA ENTIDADE DO REGISTRY PODE FICAR SEM DADOS
#
#   Chama:  nada — é uma checagem local, lendo config/registry.toml e public/.
#   Aborta: se alguma entidade do registry não tem arquivos em public/<id>/.
#
# O furo que deixou Roraima quebrada em produção: o passo 2 faz `rm -rf` + `mkdir` por entidade, o
# que cria o diretório mesmo sem fonte em `data/`, e o `(0 arquivos)` no log passa despercebido. A
# entidade continua aparecendo no seletor de estados (src/shared/entities.ts, gerado do registry) e
# suas abas quebram para quem a escolher.
#
# Por isso a guarda compara as DUAS listas — o registry e o disco — e para o deploy na divergência.
# O `--allow-vazias` existe para publicar ciente disso, nunca por acidente.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
echo
echo "▶ 4/8  INICIANDO — guarda: toda entidade do registry precisa de public/<id>/ com arquivos"
vazias=()
while read -r id; do
  dir="$FRONT/public/$id"
  if [ ! -d "$dir" ] || [ -z "$(ls -A "$dir" 2>/dev/null)" ]; then
    vazias+=("$id")
  fi
done < <(grep -E '^id[[:space:]]*=' config/registry.toml | sed -E 's/.*"([^"]+)".*/\1/')

if [ "${#vazias[@]}" -gt 0 ]; then
  echo "⚠️  entidades no registry SEM dados em public/: ${vazias[*]}"
  for id in "${vazias[@]}"; do
    [ -d "data/$id" ] || echo "     '$id': data/$id não existe nesta máquina"
  done
  if [ "$ALLOW_VAZIAS" = 0 ]; then
    echo
    echo "❌ deploy abortado. A entidade aparece no seletor de estados (src/shared/entities.ts) e"
    echo "   suas abas falham em produção. Resolva de um dos dois jeitos:"
    echo "     • recupere/colete o data/<id>/ e rode de novo; ou"
    echo "     • tire a entidade de config/registry.toml e rode scripts/tools/gen-frontend-entities.mjs"
    echo "   Para publicar assim mesmo, ciente: --allow-vazias"
    exit 1
  fi
  echo "⚠️  --allow-vazias: seguindo com ${vazias[*]} quebrada(s) em produção."
fi
echo "✅ 4/8  CONCLUÍDO — guarda ok"

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 5/8 — BUILD DO APP
#
#   Chama:  npm run build   (em auli-frontend/ — roda `tsc --noEmit` e depois o Vite)
#   Produz: auli-frontend/dist/ — index.html + assets/ + tudo que estava em public/.
#
# As duas checagens logo depois não são zelo: um build parcial que produzisse um `dist/` incompleto
# substituiria o SITE INTEIRO na troca do passo 7. Aqui é o último ponto barato de descobrir isso.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
echo
echo "▶ 5/8  INICIANDO — build do app   →  npm run build"
(cd "$FRONT" && npm run build)

[ -f "$FRONT/dist/index.html" ] || { echo "❌ dist/index.html não existe — build falhou?"; exit 1; }
[ -d "$FRONT/dist/assets" ]     || { echo "❌ dist/assets/ não existe — build falhou?"; exit 1; }
echo "✅ 5/8  CONCLUÍDO — dist/ ok ($(du -sh "$FRONT/dist" | cut -f1))"

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 6/8 — UPLOAD PARA O STAGING
#
#   Chama:  ssh (limpa o staging) + scp (sobe o dist/) + ssh (preserva os dotfiles)
#   Produz: <webroot>.novo no servidor, pronto para a troca do passo 7.
#
# PRIMEIRO PASSO QUE SAI DA MÁQUINA. Ainda assim é seguro: escreve num diretório PARALELO, e o site
# em produção segue intocado até o passo 7. Abortar aqui não quebra nada.
#
# São ~253 MB por scp (os zips vão dentro do dist/), então leva alguns minutos.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
echo
echo "▶ 6/8  INICIANDO — upload para o staging ($STAGING)"
remoto "rm -rf '$STAGING' && mkdir -p '$STAGING'"
if [ "$DRY_RUN" = 1 ]; then
  echo "   [dry-run] scp: dist/* -> $DEPLOY_HOST:$STAGING/"
else
  # `dist/.` (e não `dist/*`) copia o conteúdo inteiro, inclusive arquivo sem extensão — o `*.*`
  # da sequência antiga pulava esses em silêncio.
  scp -r -P "$DEPLOY_PORT" "$FRONT/dist/." "$DEPLOY_HOST:$STAGING/"
fi

# Dotfiles do webroot atual (tipicamente .htaccess, que faz o roteamento do SPA) NÃO vêm do dist:
# são configuração do servidor. O `rm -rf $WEBROOT/*` da sequência antiga os preservava por acidente
# do glob; aqui a preservação é explícita, senão a troca atômica os perderia.
echo "   preservando dotfiles do webroot atual (ex.: .htaccess)"
remoto "find '$WEBROOT' -maxdepth 1 -name '.*' ! -name '.' ! -name '..' -exec cp -a {} '$STAGING'/ \; 2>/dev/null || true"
echo "✅ 6/8  CONCLUÍDO — staging no servidor, produção ainda intocada"

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 7/8 — TROCA ATÔMICA
#
#   Chama:  ssh — três comandos numa linha: apaga o `.antigo`, move o atual para `.antigo`,
#           move o staging para o webroot.
#   Efeito: É AQUI QUE O SITE MUDA. Dois `mv` de diretório no mesmo filesystem: instantâneos, sem
#           janela em que o webroot esteja pela metade.
#
# É o que substituiu o `rm -rf` remoto da sequência antiga, que esvaziava o webroot durante o upload
# e deixava a app quebrada até o cache expirar.
#
# A versão anterior fica em `<webroot>.antigo`, então o rollback é um `mv` — feito automaticamente
# pelo passo 8 se o smoke falhar, ou à mão com o comando impresso no fim.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
echo
echo "▶ 7/8  INICIANDO — troca atômica"
remoto "rm -rf '$ANTIGO' && mv '$WEBROOT' '$ANTIGO' && mv '$STAGING' '$WEBROOT'"
echo "✅ 7/8  CONCLUÍDO — publicado (versão anterior preservada em $ANTIGO)"

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# PASSO 8/8 — SMOKE TEST (e rollback automático se falhar)
#
#   Chama:  curl contra o site recém-publicado; ssh só se precisar reverter.
#   Verifica: a home, os três índices de referência do rs, e o bundle JS que o index.html novo cita.
#
# Este passo roda DEPOIS da troca — o site já está no ar. É por isso que ele reverte sozinho: o
# valor não está em impedir a publicação ruim, está em não deixá-la de pé.
#
# O bundle é a checagem mais importante das cinco: index.html e assets/ fora de sincronia deixam a
# app BRANCA no navegador, sem erro visível em lugar nenhum.
# ═══════════════════════════════════════════════════════════════════════════════════════════════
echo
echo "▶ 8/8  INICIANDO — smoke test"
if [ "$DRY_RUN" = 1 ]; then
  echo "   [dry-run] pulado"
  echo "✅ 8/8  CONCLUÍDO — (dry-run: nada foi verificado porque nada foi publicado)"
  echo
  echo "✅ dry-run completo — nada foi enviado nem alterado no servidor."
  exit 0
fi

CURL=(curl -sS --max-time 30)

# Fixar na origem e verificar o certificado são ORTOGONAIS: o `--resolve` decide QUEM responde, o
# `-k` decide se a cadeia é conferida. Por isso o fixamento vale sempre — inclusive com
# SMOKE_INSECURE=1, que existe para o cert quebrado e não deveria, de tabela, mandar o smoke medir
# o CDN.
# O `|| true` é obrigatório: com `set -o pipefail`, um `getent` que não resolve faz o pipeline sair
# não-zero e o `set -e` abortaria o script AQUI — depois do upload e antes do rollback automático.
smoke_ip="$(getent ahostsv4 "$SMOKE_ORIGIN" 2>/dev/null | awk 'NR==1{print $1}' || true)"
if [ -n "$smoke_ip" ]; then
  CURL+=(--resolve "$SMOKE_HOST:443:$smoke_ip")
  echo "   🔗 smoke na origem $SMOKE_ORIGIN ($smoke_ip) com o SNI de $SMOKE_HOST."
else
  echo "   ⚠️  não resolvi '$SMOKE_ORIGIN' — o smoke vai pelo DNS público de $SMOKE_HOST"
  echo "      (pode medir o cache do CDN em vez do servidor)."
fi

if [ "$SMOKE_INSECURE" = 1 ]; then
  CURL+=(-k)
  echo "   ⚠️  SMOKE_INSECURE=1: certificado NÃO verificado (o gate fica cego para problema de TLS)."
fi

falhas=0
verificar() { # <caminho> <content-type esperado (substring)>
  local resposta http tipo
  resposta="$("${CURL[@]}" -o /dev/null -w '%{http_code} %{content_type}' "$SMOKE_BASE$1" || echo '000 ')"
  http="${resposta%% *}"
  tipo="${resposta#* }"
  if [ "$http" = 200 ] && [[ "$tipo" == *"$2"* ]]; then
    printf '   ✅ %-34s %s %s\n' "$1" "$http" "$tipo"
  else
    printf '   ❌ %-34s %s %s (esperado 200 + %s)\n' "$1" "$http" "$tipo" "$2"
    falhas=$((falhas + 1))
  fi
}

verificar "/" "text/html"
verificar "/rs/rs-servicos-index.json" "application/json"
verificar "/rs/rs-pareceres-index.json" "application/json"
# O do TARF entra pelo mesmo motivo dos outros dois, e com mais razão: com 22.476 acórdãos ele é o
# MAIOR arquivo do site (35 MB, contra 0,6 MB do de pareceres). Se um upload truncar, é o candidato
# número um — e sem esta linha o smoke passaria verde com a aba quebrada em produção.
verificar "/rs/rs-tarf-index.json" "application/json"

# O bundle que o index.html RECÉM-PUBLICADO referencia precisa existir. É a checagem que pega o modo
# de falha mais traiçoeiro: index.html e assets/ fora de sincronia deixam a app branca no navegador.
bundle="$("${CURL[@]}" "$SMOKE_BASE/" | grep -oE '/assets/[^"]+\.js' | head -1 || true)"
if [ -n "$bundle" ]; then
  verificar "$bundle" "javascript"
else
  echo "   ⚠️  não achei referência a /assets/*.js no index.html publicado"
  falhas=$((falhas + 1))
fi

if [ "$falhas" -gt 0 ]; then
  echo
  echo "❌ 8/8  FALHOU ($falhas). Revertendo para a versão anterior…"
  remoto "rm -rf '$STAGING' && mv '$WEBROOT' '$STAGING' && mv '$ANTIGO' '$WEBROOT'"
  echo "↩️  rollback feito. A versão que falhou ficou em $STAGING para inspeção."
  exit 1
fi
echo "✅ 8/8  CONCLUÍDO — smoke passou"

echo
echo "✅ deploy concluído. Rollback, se precisar:"
echo "   ssh -p $DEPLOY_PORT $DEPLOY_HOST \"rm -rf '$STAGING' && mv '$WEBROOT' '$STAGING' && mv '$ANTIGO' '$WEBROOT'\""
