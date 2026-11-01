# Atualizar serviços e FAQs do RS — e publicar na UI

Runbook de uma rodada: re-coletar do portal da SEFAZ-RS os **serviços** e as **FAQs**, levar o
resultado ao **chat** (packs + API) e às **abas da UI** (site estático). O porquê de cada peça está no
[auli_operations.md](auli_operations.md): §4.1 (packs), §5 (servidor) e §11 (frontend).

## O caminho do dado

```text
portal ─scrape─▶ snapshot ─auli-collections rs─▶ docs/{servicos,faqs}/*.md ─build-packs.sh─▶ packs ─▶ chat
                                             └─▶ raw/*.json ─deploy-frontend.sh─▶ VPS ─▶ abas Serviços / FAQs
```

São **dois consumidores com dois passos finais independentes**: o chat só vê o conteúdo novo depois
do restart do servidor (passo 5); as abas, só depois do deploy do frontend (passo 6). Fazer um sem o
outro deixa o chat e a tela servindo acervos diferentes.

## Escolha o caminho

| quer atualizar | use |
|---|---|
| só os serviços | `scripts/tools/update-rs-servicos.sh --no-serve`, depois os passos 5 e 6 |
| só as FAQs | `scripts/tools/update-rs-faqs.sh --no-serve`, depois os passos 5 e 6 |
| os dois | o roteiro abaixo — rodar os dois scripts vetorizaria duas vezes |

Não use o `scripts/tools/rs-do-zero.sh`: o `rm` do cache e o scrape estão comentados nele, então ele
**não** raspa do zero.

## Roteiro (serviços + FAQs)

Tudo a partir da raiz do repo. `data/` está fora do git: o backup do passo 1 é a única volta.

### 1. Backup e cache fresco

```bash
cp -p data/rs/rs-servicos-snapshot.json{,.anterior}
cp -p data/rs/rs-faqs-snapshot.json{,.anterior}
cp -p data/rs/raw/rs-faqs-tree.json{,.anterior}
mkdir -p data/rs/raw/cache-anterior
mv data/rs/raw/cache/{servicos,faqs} data/rs/raw/cache-anterior/
```

O scraper é cache-first **sempre**, então "do zero" quer dizer sem cache. Mover em vez de apagar
deixa voltar atrás. Os caches de `pareceres` e `tarf` não são tocados.

### 2. Scrape

```bash
(cd auli-server && cargo run --release -p auli-scraper-rs -- servicos)   # ~15 min, ~600 páginas
(cd auli-server && cargo run --release -p auli-scraper-rs -- faqs)       # ~7 min
```

O scrape pode terminar **sem imprimir uma linha** (foi assim em 31/10/2026). Saída vazia não indica
nem sucesso nem falha: quem diz é o snapshot, no passo 3.

### 3. Guarda — o acervo não pode encolher sem explicação

O scraper não falha quando uma página cai: ele segue e grava um snapshot menor. Nas FAQs, uma página
de menu que cai leva junto a subárvore inteira. Daí em diante tudo é derivação fiel, e o servidor
subiria servindo menos do que existe.

**Contagens:**

```bash
for c in servicos faqs; do
  f=data/rs/rs-$c-snapshot.json
  echo "$c: $(jq '.coleta.items|length' $f.anterior) → $(jq '.coleta.items|length' $f)"
done
```

**O que entrou e saiu:**

```bash
# serviços, por link (− saiu, + entrou)
jq -rn --slurpfile a data/rs/rs-servicos-snapshot.json.anterior --slurpfile b data/rs/rs-servicos-snapshot.json '
  ($a[0].coleta.items|map(.link)) as $x | ($b[0].coleta.items|map(.link)) as $y
  | (($x-$y)[]|"− "+.), (($y-$x)[]|"+ "+.)'

# FAQs, saldo por página (só as que mudaram)
jq -rn --slurpfile a data/rs/rs-faqs-snapshot.json.anterior --slurpfile b data/rs/rs-faqs-snapshot.json '
  def n(s): s[0].coleta.items | group_by(.url) | map({(.[0].url): length}) | add;
  n($a) as $x | n($b) as $y | ($x + $y | keys[]) as $k | select($x[$k] != $y[$k])
  | "\($k): \($x[$k] // 0) → \($y[$k] // 0)"'
```

**Se algo saiu, prove que foi o portal antes de seguir:**

1. Nenhuma página do cache novo é bloqueio do F5. Este comando tem que voltar vazio:
   `grep -rlE 'bobcmn|<title>Request Rejected' data/rs/raw/cache/{servicos,faqs}`
2. Para cada página de FAQ que sumiu, nenhuma página do cache novo pode linká-la mais. Também tem que
   voltar vazio: `grep -rlF '/<slug>"' data/rs/raw/cache/faqs`. Se o menu-pai ainda linka o slug,
   a coleta falhou ali.

Não tente provar com `curl` do shell: o F5 devolve `Request Rejected` para ele até em página que
existe.

- **Queda real** (o portal tirou do ar): siga. Em 31/10/2026 as FAQs foram de 1.962 a 1.990 com
  +68/−40, porque o portal reorganizou GIA/EFD. Saída desse tamanho é normal.
- **Falha de coleta:** devolva os `.anterior` (`mv` de volta por cima dos três arquivos) e repita o
  passo 2 **sem** mexer no cache: ele está quente, e só o que faltou volta à rede.
- **Passou:** `rm data/rs/*.anterior data/rs/raw/rs-faqs-tree.json.anterior`

### 4. Derivar a árvore e vetorizar

```bash
(cd auli-server && cargo build --release -p auli-collections && ./target/release/auli-collections rs)
scripts/build-packs.sh rs servicos faqs
rm -rf data/rs/raw/cache-anterior
```

- `auli-collections rs` reescreve `data/rs/docs/{servicos,faqs}/*.md` (a fonte que o `build-packs`
  lê) e os `data/rs/raw/*.json` que a UI serve. Ele resolve `data/` pelo diretório atual, daí o
  `cd auli-server`. Sem este passo o `build-packs` vetorizaria a árvore **antiga**.
- Confira que o número de `.md` bate com o snapshot:
  `find data/rs/docs/servicos data/rs/docs/faqs -name '*.md' | cut -d/ -f4 | sort | uniq -c`
- `build-packs.sh rs servicos faqs` é rodada parcial: as outras coleções do RS ficam como estão. Leia
  a linha `N reaproveitados, M a vetorizar` de cada coleção: `M` deve ser da ordem do que mudou (em
  31/10/2026 foram 1 serviço e 105 FAQs, ~1 min).

### 5. Subir a API (o chat)

```bash
./start_server.sh
```

Rode num terminal só dele: o servidor fica em primeiro plano (Ctrl+C para). O boot recusa subir se
packs e árvore não baterem, e quase sempre isso quer dizer que o passo 4 não rodou.

### 6. Publicar a UI (abas Serviços e FAQs)

```bash
scripts/deploy-frontend.sh --dry-run   # opcional: roda a parte local inteira, não envia nada
scripts/deploy-frontend.sh             # publica; pergunta [s/N] antes de começar
```

Regenera `auli-frontend/public/` a partir de `data/`, monta os zips de download, roda o
`npm run build` e sobe o `dist/` para um staging no VPS. A troca é atômica, com smoke test e
rollback automático (§11 do [auli_operations.md](auli_operations.md)).

⚠️ O deploy publica a working tree **como está**: arquivo modificado e não commitado em
`auli-frontend/` vai junto para produção. Confira `git status auli-frontend` antes.
