# Auli — Operação (compilar, subir, cloudflared, logs)

Runbook prático para **compilar, gerar os dados e subir o servidor da Auli** (workspace `auli-server`,
modo `server`) com o túnel do **Cloudflare** (cloudflared), e para saber **onde ficam os logs**. Para a descrição
técnica do código, ver [auli_code.md](auli_code.md) (§3 cobre o workspace `auli-server`).

> TL;DR — numa máquina já preparada (build feito, packs gerados):
>
> ```bash
> ./start_server.sh                  # compila (incremental) + sobe server + túnel
> ./start_server.sh --no-build       # restart rápido, sem recompilar
> ./start_server.sh --no-tunnel       # só o servidor local, sem túnel
> ```

---

## 1. O que sobe

O binário único `auli` tem dois modos:

- **`auli server`** — sobe a API HTTP (axum) em `:3000`, **somente leitura**: carrega os pacotes
  de vetores, valida o manifesto e responde perguntas (RAG). Embeda só a pergunta; nunca escreve.
- **`auli update`** — lê o **contrato tipado** (`data/<id>/raw/<id>-<kind>.json` =
  `auli_contract::Table<P>`, **derivado pelo `auli-collections <id>`** a partir do snapshot — ver
  §4.1), embeda o `text_to_embed` de cada registro e escreve os **pacotes** (`<id>-<kind>.json` +
  `<id>.manifest.json`). É o **único do engine** que escreve dados.

Embeddings (fastembed/BGE-M3) e busca vetorial rodam **in-process** — não há Ollama, ChromaDB nem
serviço de embedding para subir à parte.

---

## 2. Pré-requisitos

| Item                     | Detalhe                                                                                                                                                           |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Rust**                 | toolchain estável (`cargo`, `rustc`).                                                                                                                             |
| **cmake + compilador C** | exigidos por `aws-lc-sys` (TLS rustls). **Nesta máquina não há cmake de sistema**: foi instalado via `pip install --user cmake` (fica em `~/.local/bin`). Ver §3. |
| **Rede (1º build/run)**  | `ort` baixa o ONNX Runtime no build; o modelo **BGE-M3** baixa do Hugging Face para `EMBED_CACHE_DIR` no 1º uso.                                                  |
| **cloudflared**          | túnel do Cloudflare que publica `api.auli.com.br` (em `~/.local/bin`). Configure 1× com `./setup-cloudflared.sh`. Opcional ao rodar: `--no-tunnel`.               |
| **`.env`**               | na raiz do repo `auli/` (ver §4).                                                                                                                                 |

---

## 3. Compilar (build)

```bash
cd /home/ubu/Desktop/auli/auli-server     # o workspace Cargo (raiz do repo: /home/ubu/Desktop/auli)

# cmake desta máquina (pip) + compat de policy do cmake 4 — INÓCUO onde já houver cmake de sistema:
export PATH="$HOME/.local/bin:$PATH"
export CMAKE_POLICY_VERSION_MINIMUM=3.5

cargo build --release --workspace      # ou só o engine: cargo build --release --bin auli
```

- Binários em `auli-server/target/release/`: **`auli`** (server/update) e **um scraper por entidade**
  (9): `auli-scraper-{rs,sc,sp,pr,mg,pe,ba,rj,ce}`. **Nenhum usa headless Chrome** — todos são ureq +
  HTML/JSON (o RS migrou para API JSON `tudofacil`; o BA usa `native-tls`/OpenSSL por causa do TLS
  1.2-CBC do portal). A técnica de cada um está em
  [`crates/scrapers/SCRAPERS.md`](auli-server/crates/scrapers/SCRAPERS.md).
- 1º build recompila fastembed/ort/aws-lc (alguns minutos); depois é incremental (segundos). Os
  scrapers **não** dependem de fastembed/ort — compilam leves (invariante do `crates/scrapers/`).
- Numa máquina com cmake de sistema, **nenhuma** das `export` é necessária.
- Testes: `cargo test --workspace`.

> Se faltar cmake: `python3 -m pip install --user cmake` (sem sudo) e garanta `~/.local/bin` no PATH.

---

## 4. Dados necessários para servir

Tudo vive na pasta única **`data/`** na raiz (`AULI_DATA_DIR`, default `../data` a partir de
`auli-server/`). O `auli server` lê de lá: `registry.toml`, `prompts/`, os packs por entidade e —
desde a G3 — a **árvore `data/<id>/docs/pareceres/*.md`**, de onde o corpo das consultas é lido
**na hora da query** (o pack não carrega mais o corpo).

> ⚠️ **A árvore `docs/` é requisito de serving, não artefato intermediário.** O manifesto carimba um
> `docs_hash` e o boot **recusa subir** se a árvore divergir dele. Ao copiar dados entre máquinas,
> leve `packs/` **e** `docs/` juntos — pack sem árvore não serve.

**O repositório guarda código + config, não dado coletado.** Só `data/registry.toml` e
`data/prompts/` são versionados; todo o resto de `data/<id>/**` é gitignored e reconstruído pelo
pipeline. Um clone novo, portanto, **não traz dados** — rode o pipeline para popular.

> ⚠️ **Ao sincronizar a mudança que tirou os dados do git, arquivos locais em `data/<id>/ref/`
> somem.** Os arquivos do RS (`rs-portal-pareceres.txt`, `rs-portal-notas.txt`,
> `rs-conteudo_site_tree.json`) eram versionados até então. O commit que os destrackeia registra
> uma **deleção**; ao trocar de branch ou mergear, o git os trata como "rastreados e removidos" e
> **apaga do working tree** — mesmo que `git rm --cached` os tenha preservado no momento da
> operação. Perda observada na prática nesta migração.
>
> **Recuperação** (o conteúdo continua no histórico — não houve rewrite):
>
> ```bash
> PRE=<commit-do-merge-que-destrackeou>^
> mkdir -p data/rs/ref
> for f in rs-portal-pareceres.txt rs-portal-notas.txt rs-conteudo_site_tree.json; do
>   git show "$PRE:data/rs/ref/$f" > "data/rs/ref/$f"
> done
> ```
>
> Conferir depois: `grep -cE '^// [0-9]+' data/rs/ref/rs-portal-pareceres.txt` (372 blocos) e
> `grep -c '### Descrição Resumida' …` (372 sinopses). **Faça backup de `data/<id>/ref/` antes de
> sincronizar** numa máquina que tenha dados locais — é a única parte de `data/` que já esteve no
> git e, por isso, a única sujeita a esse efeito. `raw/`, `packs/` e o cache nunca foram
> rastreados e não são afetados.

### 4.1 Pacotes de vetores (`data/<id>/packs/`)

Pipeline em **três passos** (a coleta virou binários próprios na fase 2; tudo roda de `auli-server/`):

1. **Raspar** (rede, **sem headless**) → grava um snapshot por coleção `data/<id>/<id>-<kind>-snapshot.json` (v3):
   `auli-scraper-<id> servicos` para cada uma das **27 entidades** (rs/sc/sp/pr/mg/pe/ba/rj/ce/ms/mt/go/pi/am/pa/es/ro/to/ma/ap/ac/df/rn/pb/al/se/rr); o RS
   também aceita `faqs`/`all`. `--usecache` reusa o cache de páginas (offline, sem rede).
   > **Dependência de runtime do `go`, `df` e `se`:** os scrapers `auli-scraper-go`, `auli-scraper-df` e
   > `auli-scraper-se` chamam o binário **`curl`** (no PATH) para os GETs de catálogo — GO e DF ficam
   > atrás de um WAF que bloqueia o fingerprint TLS (JA3) do `ureq`, e o SharePoint do SE encerra a
   > conexão de um jeito que o `ureq` rejeita (`unexpected end of file`) mas o curl tolera
   > (ver `go_waf.md`/pendências §11). Garantir `curl` instalado no **desktop de coleta E no host
   > do túnel**, se a coleta rodar lá. No modo `--usecache` o curl não é chamado (lê do cache).
2. **Derivar** (offline) → o contrato `<id>-faqs.json`/`<id>-servicos.json` + prints + index +
   per-público (e a árvore `faqs-tree.json` p/ a UI, no RS) em `data/<id>/raw/`:
   `./target/release/auli-collections <id>`.
3. **Vetorizar** → `scripts/build-packs.sh <id>` (aponta o `auli update --source` para `raw/`).

```bash
cd auli-server
# RS (FAQs + serviços):
./target/release/auli-scraper-rs all && ./target/release/auli-collections rs && (cd .. && scripts/build-packs.sh rs)
# Demais (só serviços) — a partir de auli-server/:
for id in sc sp pr mg pe ba rj ce; do
  ./target/release/auli-scraper-$id servicos && ./target/release/auli-collections $id && (cd .. && scripts/build-packs.sh $id)
done
```

Produz, por entidade, `data/<id>/packs/<id>-servicos.json` + `<id>.manifest.json` (`strategy_version: 2`,
kind `servicos`) — e `<id>-faqs.json` onde houver FAQs (hoje só RS). Contagens atuais: **rs** serviços
586 + FAQs 1937, **sc** 208, **sp** 537, **pr** 141, **mg** 148, **pe** 38, **ba** 204, **rj** 91,
**ce** 382. `notas` é autorada (sem scraper) e ainda **não** tem fonte struct no contrato — fica
**ausente** até ser modelada; o server tolera packs ausentes (sobe com a coleção vazia). `pareceres`
também é autorado, mas já tem um passo de ingestão do `.txt` de referência (ver nota abaixo). **Só
precisa rodar de novo quando o conteúdo ou a estratégia de embedding mudar.**

> **Pareceres / Consultas — pipeline próprio.** Os pareceres (RS) e consultas tributárias (SC/SP/PR)
> têm scraper dedicado **e** um passo de **sinopse por LLM** entre o derive e a vetorização. O fluxo é
> mais longo que o de serviços e tem regras próprias de cota, cache e idempotência: **ver §4.5**.

> Regenerar **sem re-raspar** (ex.: após bump de `STRATEGY_VERSION`): com o snapshot já em disco,
> `auli-collections <id>` re-deriva os contratos offline e `build-packs.sh` regera os packs
> (substitui o antigo subcomando `rebuild`, removido).

> ⚠️ **O cache é sempre lido primeiro — mesmo sem `--usecache`.** Uma página já cacheada **não** é
> re-buscada num novo scrape; `--usecache` apenas transforma um cache-miss em erro (modo offline).
> Para forçar o refetch (novos serviços do portal, conteúdo alterado), **apague o cache antes de
> raspar**: `rm -rf data/<id>/raw/cache/`. No SC isso inclui a listagem paginada e o `buildId`, então
> serviços novos do portal só aparecem depois de limpar o cache.
>
> **Namespace por tipo.** O cache fica em `data/<id>/raw/cache/<kind>/`, um diretório por tipo de
> conteúdo — `servicos/`, `faqs/`, `pareceres/`. Cada caller do kit declara o seu:
> `auli_scraper_kit::cache::{read,write,read_or_bail}(data_dir, kind, url, …)`. Assim uma entidade que
> raspa mais de um tipo (o RS raspa os três) não mistura nem colide. Para limpar só um tipo:
> `rm -rf data/<id>/raw/cache/pareceres/`.

### 4.2 Entidades (`data/registry.toml`)

A lista de entidades e o caminho do prompt de cada uma vêm do **registro único**
`data/registry.toml` (não há mais symlink `./entities`). O system prompt é lido de
`data/prompts/<id>.txt`. Adicionar um estado = uma entrada `[[entities]]` no registry + os dados em
`data/<id>/`.

### 4.3 Modelo (`<raiz>/models`)

Cache do BGE-M3. Os lançadores definem `EMBED_CACHE_DIR=<raiz>/models` (caminho **absoluto**,
CWD-independente — fonte única). Baixa do Hugging Face no 1º uso; depois é reaproveitado (sem rede).

### 4.4 `.env` (raiz do repo `auli/`)

Carregado via `dotenv` a partir do CWD para cima — por isso um `.env` na raiz do repo (`auli/`) serve
para o server rodando em `auli-server/`. Variáveis:

| Variável                                        | Obrigatória?                                                 | Uso                                             |
| ----------------------------------------------- | ------------------------------------------------------------ | ----------------------------------------------- |
| `LLM_API_URL` / `LLM_API_KEY` / `LLM_API_MODEL` | sim                                                          | LLM externo (Groq-compat) que redige a resposta |
| `EMBED_CACHE_DIR`                               | não (lançadores: `<raiz>/models`; def. do código `./models`) | cache do modelo                                 |
| `EMBED_THREADS`                                 | não (def. `16`)                                              | threads do ONNX Runtime                         |
| `AULI_LOG_DIR`                                  | não (lançadores: `<raiz>/logs`; def. do código `./logs`)     | dir dos logs de Q&A do RAG (§7)                 |

> Faltando uma variável **obrigatória**, o server dá `panic` no boot com mensagem clara.
>
> O server **não tem auth nem banco**: só precisa das variáveis de LLM + embedding acima. Não há
> `DATABASE_URL`, chaves `JWT_*` nem Postgres no boot.

---

### 4.5 Pareceres / Consultas — pipeline completo (scrape → sinopse → vetorizar)

Vale para as 4 entidades com acervo de consultas formais: **rs** (Pareceres), **sc** (Consultas
COPAT), **sp** (Respostas a Consultas), **pr** (Consultas SEFA). São **4 passos**: o scraper já
emite a árvore, a sinopse a preenche, o build vetoriza e o índice abastece o frontend.

```text
auli-scraper-<id> pareceres      → docs/pareceres/<slug>.md   (rede; um .md por consulta INÉDITA,
                                                               nasce pendente = sem `## sinopse`)
auli-collections <id> sinopse    → edita os .md               (LLM; preenche os pendentes)
scripts/build-packs.sh <id>      → packs/<id>-pareceres.json  (embedding; lê a árvore)
auli-collections <id> indice     → raw/<id>-pareceres-index.json  (índice leve da tab; lê a árvore)
```

Os dois últimos são irmãos: **o mesmo acervo, dois consumidores**. O `build-packs.sh` serve o RAG
(vetores + corpo lido tarde); o `indice` serve a tab de Pareceres do frontend (numero/assunto/
resumo/link, **sem corpo**, copiado para `public/` pelo `build-frontend-public.sh`). Rode o `indice`
depois de qualquer passo que mexa na árvore — é derivação pura, então re-rodar nunca faz mal.

> **A árvore `.md` É a fonte** (G5b). Não há mais `.txt` intermediário, promoção manual, JSON de
> contrato nem passo de materialização — o `auli update` lê `docs/pareceres/*.md` direto.

**Incremental de graça:** o scraper grava **só o que é inédito** — arquivo que já existe é pulado,
sem sequer ser aberto. Re-raspar é barato e **nunca** destrói uma sinopse já gerada. O corolário é
que **correção de conteúdo não chega sozinha**: se o portal corrigir o corpo de uma consulta já
coletada, é preciso apagar o `.md` (decisão humana, porque isso descarta a sinopse) e recoletar.

> ⚠️ **Um scrape mexe direto na árvore de serving.** Como só acrescenta, uma coleta ruim adiciona
> lixo mas não destrói acervo. Mas o `docs_hash` muda na hora — o boot passa a recusar até o próximo
> `build-packs.sh`. Não reinicie o servidor entre o scrape e o build.

**Bootstrap a partir de `.txt` legado:** `auli-collections <id> pareceres` reconstrói a árvore a
partir de um `ref/<id>-portal-pareceres.txt` antigo (mesma regra: só o inédito). Serve para acervos
coletados antes da G5; no fluxo normal não é usado.

#### Por que existe o passo `sinopse`

A **ementa oficial** do parecer é curta e em juridiquês denso — péssima chave de busca. O `sinopse`
faz **uma** passada de LLM por documento (o corpo é imutável, então é one-shot) gerando uma
**descrição em linguagem natural + palavras-chave**, que passam a ser o texto vetorizado. Resultado
medido: perguntas em linguagem natural passam a recuperar as consultas certas (ver §4.5.4).

#### 4.5.1 Passo 1 — Raspar (emite a árvore)

```bash
cd auli-server
./target/release/auli-scraper-<id> pareceres        # rede; sem --usecache = dados frescos
```

- Grava **um `.md` por consulta inédita** em `data/<id>/docs/pareceres/<slug>.md` (frontmatter
  `numero`/`assunto`/`link` + `## corpo`), **sem sinopse** — o documento nasce pendente.
- **Existe ⇒ pula**, sem abrir o arquivo. É o incremental, e é o que protege sinopses já geradas.
- Relatório ao fim: `N novo(s), M já existente(s) (pulados)`.
- **Colisão de slug é erro**: dois `numero` distintos que gerariam o mesmo arquivo abortam a coleta
  nomeando os dois. Sem isso o segundo sumiria em silêncio (o produtor o veria como "já existe").
- Cada scraper tem uma **guarda de truncamento** (ex.: RS aborta com < 250 consultas) para não
  gravar a partir de uma coleta parcial.
- Cache em `data/<id>/raw/cache/pareceres/` (namespace por tipo; ver o aviso de cache em §4.1).
- O RS exige **`curl`** no PATH (paginação por postback ASP.NET atrás de WAF) e usa UA
  institucional com cortesia de 500 ms entre requisições.

#### 4.5.2 Passo 2 — Sinopse (LLM)

```bash
./target/release/auli-collections <id> sinopse [flags]
```

| Flag               | Efeito                                                                    |
| ------------------ | ------------------------------------------------------------------------- |
| `--dry-run`        | conta pendentes e estima tokens de entrada; **não escreve nada**          |
| `--limit N`        | processa no máximo N documentos nesta rodada (gerados + falhas) — batches |
| `--force <numero>` | re-gera a sinopse de um documento específico (ignora a existente)         |
| `--fake`           | dev-only: preenche resumo sintético, sem tocar rede                       |

**Fonte e destino são os `.md`**: pendente = arquivo **sem a seção `## sinopse`**. O passo lê cada
documento, gera o que falta e **regrava o próprio `.md`** (frontmatter `sinopse_*` + seção),
atomicamente. Exige a árvore já no disco — quem a cria é o scraper (passo 1); sem ela, erro claro.

- **Env**: `SINOPSE_API_URL` / `SINOPSE_API_KEY` / `SINOPSE_API_MODEL`, com **fallback** para os
  `LLM_*`. Isso permite apontar o lote para um **projeto/quota dedicado**, sem competir com o chat do
  RAG.
- **Prompt**: `data/prompts/sinopse.txt` (versão gravada em `SinopseInfo.prompt_versao`;
  `SINOPSE_PROMPT_VERSION = 1`). Saída validada: as duas seções
  `### Descrição Resumida do Assunto` + `### Palavras Chave do Tema`, na ordem, descrição ≤ 2000
  chars e ≥ 3 palavras-chave. Falha de validação → **1 re-tentativa**; persistindo, conta como falha
  e o lote segue.
- **Entrada truncada** em `CORPO_MAX_CHARS = 24_000` chars (v1 sem chunking; avisa no log).
- **Idempotente**: documento que já tem `## sinopse` é **pulado** (zero chamadas). Re-rodar é seguro
  e barato; é assim que se retoma um lote interrompido — a retomada é implícita, sem estado externo.
- **Grava documento a documento** (proteção contra queda): escrita atômica (`.tmp` + rename) no
  próprio `.md`. Uma queda perde no máximo o documento em voo.
- **Memória constante**: processa um a um (lê → gera → grava), nunca carrega a árvore inteira — SP
  tem 15,6 mil arquivos.
- **Relatório final** com invariante de guarda:
  `reaproveitados + gerados + falhas + pendentes-restantes == total`.

##### Cota / rate limit (o ponto operacional mais importante)

O provedor (Groq-compat) impõe **RPD** (requisições/dia) por projeto. Duas defesas, em camadas:

1. **Proactive-stop (primário, zero rejeição).** A cada resposta o cliente lê
   `x-ratelimit-remaining-requests`. Quando o headroom cai a `≤ RPD_MARGEM_PARADA (5)`, o lote **para
   antes** de mandar a requisição que seria rejeitada, e loga o `x-ratelimit-reset-requests` (quando a
   cota volta). A margem não é 0 de propósito: reserva para o chat do RAG, que divide a mesma quota.
2. **Early-abort (rede de segurança).** Se o header não vier e um `429 rate_limit_exceeded` chegar, o
   lote aborta no **primeiro** — em vez de disparar centenas de requisições condenadas.

> ⚠️ **Requisição rejeitada também consome RPD.** Antes dessas defesas, um lote que batia no teto no
> meio seguia disparando e queimava a cota do dia inteiro em 429s — o dia rendia pouquíssimas
> sinopses. Hoje isso não acontece: pior caso é **1** rejeição-probe.

Nos dois casos o documento que parou e os restantes ficam em `pendentes-restantes` (**não** contam
como falha) — basta **re-rodar** depois do reset (ou com o teto elevado) que o merge reaproveita tudo.

Para inspecionar a cota sem rodar o lote:

```bash
set -a; . .env; set +a
curl -sS -D - -o /dev/null -X POST "$LLM_API_URL" \
  -H "Authorization: Bearer $LLM_API_KEY" -H "Content-Type: application/json" \
  -d "{\"model\":\"$LLM_API_MODEL\",\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"max_completion_tokens\":1}" \
  | grep -i x-ratelimit
```

##### Acervos grandes: batches de 1000

Para entidades com milhares de consultas (SP tem **15.605**), rode em batches e audite no caminho:

```bash
./target/release/auli-collections sp sinopse --limit 1000   # repita até pendentes-restantes = 0
```

Cada rodada reaproveita as prontas e ataca as próximas 1000; o proactive-stop encerra antes se a cota
acabar. Dá para automatizar com um driver que repete `--limit 1000`, detecta o proactive-stop no log,
dorme até o reset e retoma — com teto de iterações e abort se um batch não progredir (evita loop
infinito).

#### 4.5.3 Passo 3 — Vetorizar

```bash
scripts/build-packs.sh <id>
```

- **A fonte é a árvore** (G5b): o `auli update` lê `docs/pareceres/*.md` em ordem de nome de arquivo
  (estável, para os `id-N` do pack não dançarem entre rodadas) e monta o registro dali — frontmatter,
  `## sinopse` e `## corpo`. Não há JSON de pareceres no caminho.
- **Guarda de ingestão:** **recusa** vetorizar se **qualquer** documento estiver sem `## sinopse`,
  listando os números. A árvore em si segue válida — pendência é estado legal dela; só a vetorização
  é barrada.
- **O que é embedado** (`text_to_embed`): `numero` + `assunto` + `resumo` (a sinopse), unidos por
  quebra de linha, vazios pulados. O **corpo integral NÃO é embedado**.
- **O que o pack guarda** (mudou na G3): não é mais o bloco pronto com o corpo, e sim um **payload
  leve** — `numero`/`assunto`/`resumo`/`link`/`doc_path`. O corpo é lido da árvore **na query**, só
  para os documentos selecionados. Se um `.md` sumir, o servidor **degrada** (serve a sinopse com o
  aviso `[corpo indisponível — ver link]` e loga `ERROR`) em vez de derrubar a consulta.
- **`STRATEGY_VERSION`** (hoje `4`) é carimbado no manifesto e validado no boot: pacote com versão
  diferente ⇒ o server **recusa subir**. Regenerar sinopses **em massa** (mudança de
  `SINOPSE_PROMPT_VERSION` ou troca do modelo + re-geração) muda os textos embedados ⇒ **bump
  obrigatório**. Sinopses novas convivendo com antigas (append-only) **não** exigem bump.

  > ⚠️ **O bump é global e o boot é fatal para o servidor inteiro.** `packs::load_all` valida
  > **todas** as entidades; a primeira com versão antiga derruba o boot. Ao bumpar, re-rode
  > `build-packs.sh` em **todas** as entidades antes de subir o servidor — não dá para migrar uma de
  > cada vez com o serviço no ar.

#### 4.5.4 Validar o retrieval (sem LLM)

```bash
EMBED_CACHE_DIR=$PWD/models \
  cargo run --release -p auli-cli --example retrieval_test -- data/<id>/packs/<id>-pareceres.json
```

Carrega o embedder + o pack e imprime o **top-5 por proximidade** de um conjunto de perguntas em
linguagem natural (score = distância; **menor = mais próximo**). É o jeito rápido de confirmar que a
key nova está funcionando, sem gastar quota de LLM.

#### 4.5.5 Estado atual

| Entidade | Consultas | Sinopses | Observação                                       |
| -------- | --------- | -------- | ------------------------------------------------ |
| **rs**   | 372       | 372      | —                                                |
| **sc**   | 1.743     | 1.743    | —                                                |
| **pr**   | 2.060     | 2.060    | —                                                |
| **sp**   | 15.605    | 15.605   | fechado em ~7 h de lote; use `--limit` (§4.5.2)   |

Todas as 4 completas. As sinopses vivem em `data/<id>/docs/pareceres/*.md` — **é essa árvore que
precisa de backup**, não o `.txt`.

#### 4.5.6 Trabalhar com um lote em curso — e o que acontece se a máquina cair

Um lote de sinopse roda por **horas** (o SP são ~15 mil documentos). Não é preciso ficar parado: dá
para desenvolver e até rodar o pipeline de **outras** entidades em paralelo, desde que se respeite o
que o lote está usando.

##### O que o lote está usando

| Recurso                                                 | Em uso pelo lote?                        | Consequência                                                    |
| ------------------------------------------------------- | ---------------------------------------- | --------------------------------------------------------------- |
| `target/release/auli-collections`                       | **sim** — o driver o invoca a cada batch | não pode ser substituído: o próximo batch rodaria outro binário |
| `target/release/auli`                                   | só se houver `auli server` no ar         | livre para rebuild/execução quando o server está parado         |
| `data/<id>/raw/<id>-pareceres.json` da entidade em lote | **sim**, reescrito a cada documento      | não ler nem regenerar essa entidade                             |
| `data/<outra>/**`                                       | não                                      | **disjunto** — outras entidades são seguras                     |

##### Regras para buildar sem derrubar o lote

- **Escopar o build**: `cargo build --release -p <pacote>` produz **apenas** o binário daquele
  pacote. As libs compartilhadas (`auli-contract`, `auli-core`) viram `.rlib` novos, mas um
  executável já linkado é estático — o processo em execução e o arquivo no disco **não mudam**.
- **Nunca** `cargo build --release` do workspace inteiro enquanto um lote roda: isso relinka
  **todos** os binários, inclusive o que está em uso.
- Testes/clippy em **perfil debug** (`cargo test -p <pacote>`) nunca encostam em `target/release/`.
- Alternativa de risco zero, ao custo de um rebuild do zero (fastembed/ort/aws-lc, alguns minutos):
  `CARGO_TARGET_DIR=/tmp/target-paralelo cargo build --release …`.

##### Regra das entidades

Os dados são **disjuntos por entidade** (`data/<id>/`). Rodar `auli update --entity rs` ou
`scripts/build-packs.sh rs` **não toca em nada** do SP. A regra é simples: **mexa em qualquer
entidade exceto a que está em lote**.

##### Se a máquina cair no meio de um lote de sinopse

**Nada a reconciliar — o passo foi desenhado para isso.**

- O `sinopse` grava **atomicamente por documento** (`.tmp` + rename): tudo que já foi gerado está no
  disco. Perde-se, no máximo, o documento em voo.
- O driver de batches **não tem estado próprio**: ele deriva "quantos faltam" do próprio JSON. Não há
  lock, arquivo de progresso nem transação pendente para limpar.
- **Recuperação:** relançar o driver (ou rodar `auli-collections <id> sinopse --limit N` à mão). O
  merge por `numero` reaproveita todos os prontos e ataca só os pendentes.

Para ver onde parou:

```bash
python3 -c "import json;d=json.load(open('data/<id>/raw/<id>-pareceres.json'));i=next(v for v in d.values() if isinstance(v,list));c=sum(1 for r in i if (r.get('resumo') or '').strip());print(f'{c}/{len(i)} prontas, {len(i)-c} pendentes')"
```

##### Se cair durante `auli update` / `build-packs.sh`

O pack pode ficar truncado — mas isso **não vira serviço quebrado**: a validação do manifesto no boot
(identidade de embedding + hash de integridade) faz o server **recusar subir** em vez de responder a
partir de um pacote corrompido. Recuperação: rodar `scripts/build-packs.sh <id>` de novo (idempotente).

##### Se cair com trabalho de código não commitado

O de sempre: só sobrevive o que está commitado (e, de preferência, pushado). Com um lote longo em
paralelo, a tentação é acumular mudanças por horas — commite cedo e com frequência.

---

## 5. Subir o servidor + túnel Cloudflare

O jeito recomendado é o script [start_server.sh](start_server.sh) (na raiz do repo `auli/`):

```bash
./start_server.sh                       # build incremental + server + túnel
./start_server.sh --no-build            # pula o cargo build (restart rápido) + túnel
./start_server.sh --no-tunnel            # só o servidor local, sem túnel
./start_server.sh --no-build --no-tunnel # restart local puro
```

O que ele faz: exporta o env de cmake desta máquina, reusa `auli-server/target`, entra em `auli-server/`,
exporta `AULI_DATA_DIR=../data`, derruba uma instância anterior na porta, compila (a menos de
`--no-build`), sobe o **túnel cloudflared em background** e o **server em foreground**. **Ctrl+C**
encerra os dois (um `trap` derruba o cloudflared junto). O túnel precisa ter sido configurado 1× com
`./setup-cloudflared.sh` (ver §10); sem isso, sobe só o servidor local.

Variáveis de ambiente para sobrescrever:

```bash
PORT=8080 ./start_server.sh                       # outra porta
AULI_DATA_DIR=/dados ./start_server.sh            # outra raiz de data/ (registry+prompts+packs)
TUNNEL_NAME=outro-tunel ./start_server.sh         # outro túnel cloudflared
./start_server.sh --no-tunnel                     # só o servidor local, sem túnel
```

**Boot saudável** se parecer com (5 entidades ativas):

```text
🏛️  Entidades carregadas: [mg, pr, rs, sc, sp]
🔎 Manifesto de 'rs' validado contra a identidade local.
📦 rs-servicos — 586 registros
📦 rs-faqs — 1937 registros
🔎 Manifesto de 'sp' validado contra a identidade local.
📦 sp-servicos — 537 registros
🔎 Manifesto de 'pr' validado contra a identidade local.
📦 pr-servicos — 141 registros
🔎 Manifesto de 'sc' validado contra a identidade local.
📦 sc-servicos — 208 registros
🔎 Manifesto de 'mg' validado contra a identidade local.
📦 mg-servicos — 148 registros
✅ Server started successfully at 0.0.0.0:3000
```

### Sem o script (comando direto)

```bash
cd /home/ubu/Desktop/auli/auli-server
AULI_DATA_DIR=../data target/release/auli server --packs-dir ../data
```

> **Nunca use `sudo`.** A porta 3000 não exige root, e sob `sudo` o server procura `.env`/cache no
> HOME do root e não acha os dados.

---

## 6. Smoke tests

Com o server no ar, em outro terminal:

```bash
curl -s localhost:3000/v1/health
curl -s -X POST localhost:3000/v1/question -H 'Content-Type: application/json' \
  -d '{"entity":"rs","question":"Como obtenho certidão negativa de débitos?"}'
# entidade desconhecida -> erro amigável, sem panic, HTTP 200:
curl -s -X POST localhost:3000/v1/question -H 'Content-Type: application/json' \
  -d '{"entity":"zz","question":"x"}'
```

---

## 7. Logs — onde ficam

Há **três** destinos distintos:

| Tipo                    | Onde                                                                   | Conteúdo                                                                                                                                                                                                                |
| ----------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Q&A do RAG**          | `logs/<AAAA-MM-DD_HH-MM-SS>.txt` na raiz do repo (um arquivo por pergunta) | `Pergunta:` + contexto RAG recuperado + `Resposta:`. Gravado por [rag.rs](auli-server/crates/auli-cli/src/rag.rs) em `$AULI_LOG_DIR` (default `./logs` do CWD). O `start_server.sh` exporta `AULI_LOG_DIR=<raiz>/logs` (absoluto), então caem na **raiz do repo**, não em `auli-server/`. |
| **cloudflared**         | `/tmp/auli-cloudflared.log`                                            | saída do túnel Cloudflare (redirecionada pelo `start_server.sh`).                                                                                                                                                       |
| **Console (`tracing`)** | **stdout/stderr** do terminal (não vai a arquivo)                      | boot, scores, `info/debug/warn`. Controlado por `RUST_LOG`.                                                                                                                                                             |

Exemplos:

```bash
ls -lt logs/ | head                             # últimas consultas (raiz do repo)
tail -f /tmp/auli-cloudflared.log              # acompanhar o túnel
RUST_LOG=auli_cli=debug ./start_server.sh   # ver arrays de score + prompt RAG completo no console
```

> Para gravar também o console em arquivo: `./start_server.sh --no-tunnel 2>&1 | tee logs/console.log`.

---

## 8. Parar / reiniciar

- **Parar:** `Ctrl+C` no terminal do server (encerra server + cloudflared pelo `trap`).
- **Matar à força (porta presa):** `pkill -f "release/auli server"`.
- **Reiniciar rápido (sem recompilar):** `./start_server.sh --no-build`.

---

## 9. Troubleshooting

| Sintoma                                                                     | Causa / correção                                                                                                                                                                                                                    |
| --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Não foi possível ler o registro de entidades` / `Entidades carregadas: []` | `AULI_DATA_DIR` não aponta para a pasta `data/` (com `registry.toml`). Rode via `start_server.sh` (exporta `../data`).                                                                                                              |
| `📦 Pacotes carregados de …` (e nada carregado)                             | `--packs-dir`/`AULI_DATA_DIR` aponta para a raiz errada (sem `<id>/packs/`). Sem `--packs-dir`, o server usa `AULI_DATA_DIR` (default `./data`); o `start_server.sh` exporta `../data`.                                             |
| Rebaixando `model_quantized.onnx` toda vez                                  | Cache não encontrado. Os lançadores já apontam `EMBED_CACHE_DIR=<raiz>/models` (absoluto), então o CWD não importa — confira se os pesos estão em `<raiz>/models`. Em execução manual, exporte `EMBED_CACHE_DIR` para esse caminho. |
| Erro de cmake / `aws-lc-sys` no build                                       | Sem cmake no PATH ou cmake 4 reclamando de policy. `export PATH="$HOME/.local/bin:$PATH"` e `export CMAKE_POLICY_VERSION_MINIMUM=3.5` (o `start_server.sh` já faz).                                                                 |
| `Variável de ambiente obrigatória ausente: ...`                             | Falta variável de LLM no `.env` (`LLM_API_URL`/`LLM_API_KEY`/`LLM_API_MODEL`). Ver §4.4.                                                                                                                                            |
| `Manifest incompatível ...` no boot                                         | Pacotes gerados com modelo/dim/`strategy_version` diferente do binário. Re-gere com `auli update`.                                                                                                                                  |
| `Permission denied` ao rodar o script                                       | Faltou `chmod +x start_server.sh`, ou usou `sudo` (não use).                                                                                                                                                                        |
| cloudflared com "connection refused"/"context deadline" no início           | Normal: o túnel tenta conectar enquanto o server ainda carrega o modelo; conecta quando o boot termina.                                                                                                                             |
| `failed to create tunnel ... already exists`                                | O túnel `auli-api` já existe. O `setup-cloudflared.sh` é idempotente; rode de novo (ele detecta e reaproveita).                                                                                                                     |
| `An A, AAAA, or CNAME record with that host already exists`                 | O CNAME do ngrok ainda está em `api.auli.com.br`. O script usa `--overwrite-dns`; se persistir, apague o registro no painel e rode de novo.                                                                                         |
| HTTP 1015 / "rate limited" no cliente                                       | A regra de rate limiting do Cloudflare disparou (§10.2). Ajuste o limite ou o período.                                                                                                                                              |

---

## 10. Túnel Cloudflare + rate limiting

O `api.auli.com.br` é publicado por um **Cloudflare Tunnel** (`cloudflared`) — substitui o ngrok.
O `cloudflared` roda nesta máquina e disca **para fora**, então **o Cloudflare é o único caminho de
entrada** (não há URL pública de origem para burlar) e as regras de rate limiting do Cloudflare são
de fato aplicadas. O `CF-Connecting-IP` chega ao limitador interno do app (1 req/s, burst 2 por IP).

### 10.1 Configuração do túnel (uma vez)

```bash
./setup-cloudflared.sh        # login (abre navegador) + cria o túnel 'auli-api' + roteia o DNS
```

O script: autoriza a conta (`~/.cloudflared/cert.pem`), cria o túnel, escreve
`~/.cloudflared/config.yml` (ingress `api.auli.com.br` -> `http://localhost:3000`) e aponta o
**CNAME proxied** `api.auli.com.br` -> `<uuid>.cfargotunnel.com` (substituindo o CNAME do ngrok).
Depois disso, **`./start_server.sh` sobe o túnel automaticamente** (`cloudflared tunnel run auli-api`).

No painel Cloudflare, **reative a zona `auli.com.br`** (estava em _pause_) e confira que
`api.auli.com.br` está **proxied** (nuvem laranja).

### 10.2 Regra de rate limiting (no painel Cloudflare)

**Security -> WAF -> Rate limiting rules -> Create rule**:

| Campo                          | Valor                                                                                                                               |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| **If incoming requests match** | `Hostname` equals `api.auli.com.br` **AND** `URI Path` equals `/v1/question`                                                        |
| **(opcional)**                 | **AND** `Request Method` equals `POST`                                                                                              |
| **Rate**                       | ex.: **60** requests per **1 minute** (ajuste à realidade de um órgão; lembre que NAT faz um escritório inteiro sair por **um IP**) |
| **Counting characteristic**    | `IP` (com proxy, é o IP real do cliente)                                                                                            |
| **Then**                       | **Block** por **10s**-**60s**, resposta **429** (corpo custom opcional em pt-BR)                                                    |

Defesa em camadas: a regra do Cloudflare é a barreira de borda; o limitador interno do app
(`/v1/question`, por IP) é a segunda linha. Não há mais URL de ngrok pública para contornar.

### 10.3 Verificação

```bash
tail -f /tmp/auli-cloudflared.log                 # túnel: deve logar "Registered tunnel connection"
curl -s https://api.auli.com.br/v1/health         # 200 via Cloudflare
# dispare > limite no /v1/question e espere 429 (vinda do Cloudflare; veja em Security -> Events)
```
