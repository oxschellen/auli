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
#   SMOKE_BASE    URL base do smoke test    (padrão: https://novoauli.vps-kinghost.net)
#   SMOKE_INSECURE=0  exige certificado válido no smoke test (padrão: 1, ignora)
set -euo pipefail

DEPLOY_HOST="${DEPLOY_HOST:-root@novoauli.vps-kinghost.net}"
DEPLOY_PORT="${DEPLOY_PORT:-22}"
WEBROOT="${WEBROOT:-/var/www/html}"
SMOKE_BASE="${SMOKE_BASE:-https://novoauli.vps-kinghost.net}"
SMOKE_INSECURE="${SMOKE_INSECURE:-1}"

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

echo "▶ 1/6  public/ a partir de data/"
scripts/build-frontend-public.sh

echo
echo "▶ 2/6  guarda: toda entidade do registry precisa de public/<id>/ com arquivos"
# O furo que deixou Roraima quebrada em produção: `rm -rf` + `mkdir` por entidade cria o diretório
# mesmo sem fonte, e o `(0 arquivos)` no log passa despercebido. Aqui isso para o deploy.
vazias=()
while read -r id; do
  dir="$FRONT/public/$id"
  if [ ! -d "$dir" ] || [ -z "$(ls -A "$dir" 2>/dev/null)" ]; then
    vazias+=("$id")
  fi
done < <(grep -E '^id[[:space:]]*=' data/registry.toml | sed -E 's/.*"([^"]+)".*/\1/')

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
    echo "     • tire a entidade de data/registry.toml e rode scripts/gen-frontend-entities.mjs"
    echo "   Para publicar assim mesmo, ciente: --allow-vazias"
    exit 1
  fi
  echo "⚠️  --allow-vazias: seguindo com ${vazias[*]} quebrada(s) em produção."
fi
echo "✅ guarda ok"

echo
echo "▶ 3/6  build do app"
(cd "$FRONT" && npm run build)

# Sanidade do que vai subir: sem isso, um build parcial substituiria o site inteiro.
[ -f "$FRONT/dist/index.html" ] || { echo "❌ dist/index.html não existe — build falhou?"; exit 1; }
[ -d "$FRONT/dist/assets" ]     || { echo "❌ dist/assets/ não existe — build falhou?"; exit 1; }
echo "✅ dist/ ok ($(du -sh "$FRONT/dist" | cut -f1))"

echo
echo "▶ 4/6  upload para o staging ($STAGING)"
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
echo "▶ 5/6  troca atômica"
remoto "rm -rf '$ANTIGO' && mv '$WEBROOT' '$ANTIGO' && mv '$STAGING' '$WEBROOT'"
echo "✅ publicado (versão anterior preservada em $ANTIGO)"

echo
echo "▶ 6/6  smoke test"
if [ "$DRY_RUN" = 1 ]; then
  echo "   [dry-run] pulado"
  echo
  echo "✅ dry-run completo — nada foi enviado nem alterado no servidor."
  exit 0
fi

CURL=(curl -sS --max-time 30)
[ "$SMOKE_INSECURE" = 1 ] && CURL+=(-k)

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
