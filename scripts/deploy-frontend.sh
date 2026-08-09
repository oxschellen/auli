#!/usr/bin/env bash
# deploy-frontend.sh [--dry-run] [--allow-vazias] — publica o auli-frontend no servidor.
#
# Substitui a sequência manual (build-frontend-public → npm run build → rm -rf remoto → scp), que
# tinha dois furos: (1) entidade sem `data/` gerava `public/<id>/` VAZIO e subia assim, em silêncio;
# (2) o webroot ficava esvaziado durante o upload, e um `index.html` em cache apontando para um
# bundle que o `rm -rf` acabou de apagar deixava a app quebrada até o cache expirar.
#
# Aqui o upload vai para um diretório de staging e a publicação é UM `mv` — atômica. O antigo vira
# `<webroot>.antigo`, então o rollback é outro `mv`. Se o smoke test falhar, o rollback é automático.
#
# Uso:
#   scripts/deploy-frontend.sh --dry-run     # local completo; mostra o que rodaria no servidor
#   scripts/deploy-frontend.sh               # deploy de verdade
#
# Ajuste por ambiente (variáveis):
#   DEPLOY_HOST   destino ssh/scp           (padrão: root@novoauli.vps-kinghost.net)
#   DEPLOY_PORT   porta ssh                 (padrão: 22)
#   WEBROOT       raiz servida pelo Apache  (padrão: /var/www/html)
#   SMOKE_HOST    nome que o certificado cobre  (padrão: auli.com.br)
#   SMOKE_ORIGIN  onde o smoke CONECTA          (padrão: o host de DEPLOY_HOST)
#   SMOKE_BASE    URL base do smoke test        (padrão: https://$SMOKE_HOST)
#   SMOKE_INSECURE=1  ignora o certificado no smoke (padrão: 0)
#
# O smoke conecta DIRETO na origem (`SMOKE_ORIGIN`, a VPS) e manda o SNI de `SMOKE_HOST` — via
# `curl --resolve`. Assim ele valida a cadeia TLS de verdade E continua testando o servidor, não a
# borda: o domínio público está atrás de Cloudflare, e verificar por ele mediria o cache do CDN
# junto — logo depois de um deploy isso mente nas duas direções. O certificado do servidor cobre
# só `auli.com.br`, então bater no hostname da VPS por HTTPS falharia no nome (era o que o
# `SMOKE_INSECURE=1` contornava, ao custo de deixar o gate cego para problema de certificado).
set -euo pipefail

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
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    --allow-vazias) ALLOW_VAZIAS=1 ;;
    *) echo "❌ opção desconhecida: $arg (use --dry-run | --allow-vazias)"; exit 1 ;;
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

echo "▶ 1/7  public/ a partir de data/"
scripts/build-frontend-public.sh

echo
echo "▶ 2/7  zips de download por estado (public/downloads/)"
# Aqui, e não no build-frontend-public.sh: aquele é rodado sozinho com frequência e não deve
# arrastar ~70 MB de zip junto. Tem de vir ANTES do build, porque o Vite copia public/ para dist/ —
# é assim que os zips sobem, sem caminho de publicação próprio. O bundle é determinístico: quando o
# conteúdo não muda, ele nem reescreve os arquivos.
# ⚠️ Este passo usa o binário COMO ESTÁ no disco — este script nunca compila. Mudou o `bundle.rs`?
# Rode `cargo build --release -p auli-cli` ANTES, ou os zips saem do código velho, sem aviso. Foi o
# que aconteceu em 09/08/2026: README do `tarf/` publicado com "Coleta em andamento" cinco horas
# depois de a frase ter sido removida do código. Ver auli_operations.md §11.
BUNDLE_BIN="${AULI_BIN:-$ROOT/auli-server/target/release/auli}"
if [ -x "$BUNDLE_BIN" ]; then
  "$BUNDLE_BIN" bundle --data-root "$ROOT/data" --out "$FRONT/public/downloads"
else
  echo "⚠️  $BUNDLE_BIN não encontrado — os zips NÃO foram regerados."
  echo "   Compile (cargo build --release -p auli-cli) e rode de novo, ou publique ciente de que"
  echo "   public/downloads/ vai com o conteúdo da última geração (ou vazio, na primeira vez)."
fi

echo
echo "▶ 3/7  guarda: toda entidade do registry precisa de public/<id>/ com arquivos"
# O furo que deixou Roraima quebrada em produção: `rm -rf` + `mkdir` por entidade cria o diretório
# mesmo sem fonte, e o `(0 arquivos)` no log passa despercebido. Aqui isso para o deploy.
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
    echo "     • tire a entidade de config/registry.toml e rode scripts/gen-frontend-entities.mjs"
    echo "   Para publicar assim mesmo, ciente: --allow-vazias"
    exit 1
  fi
  echo "⚠️  --allow-vazias: seguindo com ${vazias[*]} quebrada(s) em produção."
fi
echo "✅ guarda ok"

echo
echo "▶ 4/7  build do app"
(cd "$FRONT" && npm run build)

# Sanidade do que vai subir: sem isso, um build parcial substituiria o site inteiro.
[ -f "$FRONT/dist/index.html" ] || { echo "❌ dist/index.html não existe — build falhou?"; exit 1; }
[ -d "$FRONT/dist/assets" ]     || { echo "❌ dist/assets/ não existe — build falhou?"; exit 1; }
echo "✅ dist/ ok ($(du -sh "$FRONT/dist" | cut -f1))"

echo
echo "▶ 5/7  upload para o staging ($STAGING)"
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

echo
echo "▶ 6/7  troca atômica"
remoto "rm -rf '$ANTIGO' && mv '$WEBROOT' '$ANTIGO' && mv '$STAGING' '$WEBROOT'"
echo "✅ publicado (versão anterior preservada em $ANTIGO)"

echo
echo "▶ 7/7  smoke test"
if [ "$DRY_RUN" = 1 ]; then
  echo "   [dry-run] pulado"
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
  echo "❌ smoke test falhou ($falhas). Revertendo para a versão anterior…"
  remoto "rm -rf '$STAGING' && mv '$WEBROOT' '$STAGING' && mv '$ANTIGO' '$WEBROOT'"
  echo "↩️  rollback feito. A versão que falhou ficou em $STAGING para inspeção."
  exit 1
fi

echo
echo "✅ deploy concluído. Rollback, se precisar:"
echo "   ssh -p $DEPLOY_PORT $DEPLOY_HOST \"rm -rf '$STAGING' && mv '$WEBROOT' '$STAGING' && mv '$ANTIGO' '$WEBROOT'\""
