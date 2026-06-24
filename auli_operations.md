# Auli — Operação (compilar, subir, cloudflared, logs)

Runbook prático para **compilar, gerar os dados e subir o servidor da Auli** (workspace `auli-engine`,
modo `server`) com o túnel do **Cloudflare** (cloudflared), e para saber **onde ficam os logs**. Para a descrição
técnica do código, ver [auli_code.md](auli_code.md) (§9 cobre o workspace `auli-engine`).

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
- **`auli update`** — lê o **contrato tipado** do scraper (`data/<id>/raw/<id>-<kind>.json` =
  `auli_contract::Table<P>`), embeda o `text_to_embed` de cada registro e escreve os **pacotes**
  (`<id>-<kind>.json` + `<id>.manifest.json`). É o **único** que escreve dados.

Embeddings (fastembed/BGE-M3) e busca vetorial rodam **in-process** — não há Ollama, ChromaDB nem
serviço de embedding para subir à parte.

---

## 2. Pré-requisitos

| Item | Detalhe |
| --- | --- |
| **Rust** | toolchain estável (`cargo`, `rustc`). |
| **cmake + compilador C** | exigidos por `aws-lc-sys` (TLS rustls). **Nesta máquina não há cmake de sistema**: foi instalado via `pip install --user cmake` (fica em `~/.local/bin`). Ver §3. |
| **Rede (1º build/run)** | `ort` baixa o ONNX Runtime no build; o modelo **BGE-M3** baixa do Hugging Face para `EMBED_CACHE_DIR` no 1º uso. |
| **cloudflared** | túnel do Cloudflare que publica `api.auli.com.br` (em `~/.local/bin`). Configure 1× com `./setup-cloudflared.sh`. Opcional ao rodar: `--no-tunnel`. |
| **`.env`** | na raiz `auli_new/` (ver §4). |

---

## 3. Compilar (build)

```bash
cd /home/ubu/Desktop/auli_new/auli

# cmake desta máquina (pip) + compat de policy do cmake 4 — INÓCUO onde já houver cmake de sistema:
export PATH="$HOME/.local/bin:$PATH"
export CMAKE_POLICY_VERSION_MINIMUM=3.5

# (opcional) reaproveita os artefatos já compilados (fastembed/ort/aws-lc) -> build rápido:
export CARGO_TARGET_DIR=/home/ubu/Desktop/auli_new/auli-server/target

cargo build --release --workspace      # ou: cargo build --release --bin auli
```

- Binário: `target/release/auli` (ou `auli-server/target/release/auli` se usar o `CARGO_TARGET_DIR`
  compartilhado acima).
- 1º build sem `CARGO_TARGET_DIR` compartilhado recompila fastembed/ort/aws-lc (alguns minutos);
  depois é incremental (segundos).
- Numa máquina com cmake de sistema, **nenhuma** das três `export` é necessária.
- Testes: `cargo test --workspace`.

> Se faltar cmake: `python3 -m pip install --user cmake` (sem sudo) e garanta `~/.local/bin` no PATH.

---

## 4. Dados necessários para servir

Tudo vive na pasta única **`data/`** na raiz (`AULI_DATA_DIR`, default `../data` a partir de
`auli-engine/`). O `auli server` lê de lá: `registry.toml`, `prompts/` e os packs por entidade.

### 4.1 Pacotes de vetores (`data/<id>/packs/`)
Gerados pelo `scripts/build-packs.sh`, que aponta o `auli update --source` para `data/<id>/raw/`
(onde o scraper grava o contrato `<id>-faqs.json` / `<id>-servicos.json`):

```bash
scripts/build-packs.sh rs        # e: scripts/build-packs.sh sc
```
Produz `data/rs/packs/rs-services.json` (≈627), `rs-faqs.json` (≈1914) e `rs.manifest.json`
(`strategy_version: 2`). `pareceres`/`notas` são autorados (sem scraper) e ainda **não** têm fonte
struct no contrato — ficam **ausentes** até serem modelados; o server tolera packs ausentes (sobe
com a coleção vazia). **Só precisa rodar de novo quando o conteúdo ou a estratégia de embedding mudar.**

### 4.2 Entidades (`data/registry.toml`)
A lista de entidades e o caminho do prompt de cada uma vêm do **registro único**
`data/registry.toml` (não há mais symlink `./entities`). O system prompt é lido de
`data/prompts/<id>.txt`. Adicionar um estado = uma entrada `[[entities]]` no registry + os dados em
`data/<id>/`.

### 4.3 Modelo (`./models`)
Cache do BGE-M3 (`EMBED_CACHE_DIR=./models` → `auli-engine/models`). Baixa do Hugging Face no 1º uso;
depois é reaproveitado (sem rede).

### 4.4 `.env` (raiz `auli_new/`)

Carregado via `dotenv` a partir do CWD para cima — por isso um `.env` na raiz `auli_new/` serve
para o server rodando em `auli-engine/`. Variáveis:

| Variável | Obrigatória? | Uso |
| --- | --- | --- |
| `LLM_API_URL` / `LLM_API_KEY` / `LLM_API_MODEL` | sim | LLM externo (Groq-compat) que redige a resposta |
| `EMBED_CACHE_DIR` | não (def. `./models`) | cache do modelo |
| `EMBED_THREADS` | não (def. `16`) | threads do ONNX Runtime |
| `VECTOR_DB_PATH` | não | pasta padrão dos pacotes |

> Faltando uma variável **obrigatória**, o server dá `panic` no boot com mensagem clara.
>
> O server **não tem auth nem banco**: só precisa das variáveis de LLM + embedding acima. Não há
> `DATABASE_URL`, chaves `JWT_*` nem Postgres no boot.

---

## 5. Subir o servidor + túnel Cloudflare

O jeito recomendado é o script [start_server.sh](start_server.sh) (na raiz `auli_new/`):

```bash
./start_server.sh                       # build incremental + server + túnel
./start_server.sh --no-build            # pula o cargo build (restart rápido) + túnel
./start_server.sh --no-tunnel            # só o servidor local, sem túnel
./start_server.sh --no-build --no-tunnel # restart local puro
```

O que ele faz: exporta o env de cmake desta máquina, reusa `auli-server/target`, entra em `auli-engine/`,
exporta `AULI_DATA_DIR=../data`, derruba uma instância anterior na porta, compila (a menos de
`--no-build`), sobe o **túnel cloudflared em background** e o **server em foreground**. **Ctrl+C**
encerra os dois (um `trap` derruba o cloudflared junto). O túnel precisa ter sido configurado 1× com
`./setup-cloudflared.sh` (ver §10); sem isso, sobe só o servidor local.

Variáveis de ambiente para sobrescrever:
```bash
PORT=8080 ./start_server.sh                       # outra porta
PACKS_DIR=/dados/packs ./start_server.sh          # outra pasta de pacotes
TUNNEL_NAME=outro-tunel ./start_server.sh         # outro túnel cloudflared
./start_server.sh --no-tunnel                     # só o servidor local, sem túnel
```

**Boot saudável** se parecer com:
```
🏛️  Entidades carregadas: [rs, sc]
🔎 Manifesto de 'rs' validado contra a identidade local.
📦 rs-services — 627 registros
📦 rs-faqs — 1914 registros
📦 rs-pareceres — 331 registros
📦 rs-notas — 1 registros
🔎 Manifesto de 'sc' validado contra a identidade local.
📦 sc-services — 208 registros
🧠 Embedder fastembed (BGE-M3) carregado
✅ Server started successfully at 0.0.0.0:3000
```

### Sem o script (comando direto)
```bash
cd /home/ubu/Desktop/auli_new/auli
AULI_DATA_DIR=../data ../auli-server/target/release/auli server --packs-dir ../data
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

| Tipo | Onde | Conteúdo |
| --- | --- | --- |
| **Q&A do RAG** | `auli-engine/logs/<AAAA-MM-DD_HH-MM-SS>.txt` (um arquivo por pergunta) | `Pergunta:` + contexto RAG recuperado + `Resposta:`. Gravado por [rag.rs](auli-engine/crates/auli-cli/src/rag.rs) em `./logs/` **relativo ao CWD** — como o server roda em `auli-engine/`, caem em `auli-engine/logs/`. |
| **cloudflared** | `/tmp/auli-cloudflared.log` | saída do túnel Cloudflare (redirecionada pelo `start_server.sh`). |
| **Console (`tracing`)** | **stdout/stderr** do terminal (não vai a arquivo) | boot, scores, `info/debug/warn`. Controlado por `RUST_LOG`. |

Exemplos:
```bash
ls -lt auli-engine/logs/ | head                 # últimas consultas
tail -f /tmp/auli-cloudflared.log              # acompanhar o túnel
RUST_LOG=auli_cli=debug ./start_server.sh   # ver arrays de score + prompt RAG completo no console
```

> Para gravar também o console em arquivo: `./start_server.sh --no-tunnel 2>&1 | tee auli-engine/logs/console.log`.

---

## 8. Parar / reiniciar

- **Parar:** `Ctrl+C` no terminal do server (encerra server + cloudflared pelo `trap`).
- **Matar à força (porta presa):** `pkill -f "release/auli server"`.
- **Reiniciar rápido (sem recompilar):** `./start_server.sh --no-build`.

---

## 9. Troubleshooting

| Sintoma | Causa / correção |
| --- | --- |
| `Não foi possível ler o registro de entidades` / `Entidades carregadas: []` | `AULI_DATA_DIR` não aponta para a pasta `data/` (com `registry.toml`). Rode via `start_server.sh` (exporta `../data`). |
| `📦 Pacotes carregados de ./vectors` (e nada carregado) | Esqueceu `--packs-dir ../data` — caiu no default `./vectors`. Use o script ou passe a flag. |
| Rebaixando `model_quantized.onnx` toda vez | CWD errado → `./models` vazio. Rode de `auli-engine/` com o modelo em `auli-engine/models` (`EMBED_CACHE_DIR=./models`). |
| Erro de cmake / `aws-lc-sys` no build | Sem cmake no PATH ou cmake 4 reclamando de policy. `export PATH="$HOME/.local/bin:$PATH"` e `export CMAKE_POLICY_VERSION_MINIMUM=3.5` (o `start_server.sh` já faz). |
| `Variável de ambiente obrigatória ausente: ...` | Falta variável de LLM no `.env` (`LLM_API_URL`/`LLM_API_KEY`/`LLM_API_MODEL`). Ver §4.4. |
| `Manifest incompatível ...` no boot | Pacotes gerados com modelo/dim/`strategy_version` diferente do binário. Re-gere com `auli update`. |
| `Permission denied` ao rodar o script | Faltou `chmod +x start_server.sh`, ou usou `sudo` (não use). |
| cloudflared com "connection refused"/"context deadline" no início | Normal: o túnel tenta conectar enquanto o server ainda carrega o modelo; conecta quando o boot termina. |
| `failed to create tunnel ... already exists` | O túnel `auli-api` já existe. O `setup-cloudflared.sh` é idempotente; rode de novo (ele detecta e reaproveita). |
| `An A, AAAA, or CNAME record with that host already exists` | O CNAME do ngrok ainda está em `api.auli.com.br`. O script usa `--overwrite-dns`; se persistir, apague o registro no painel e rode de novo. |
| HTTP 1015 / "rate limited" no cliente | A regra de rate limiting do Cloudflare disparou (§10.2). Ajuste o limite ou o período. |

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

No painel Cloudflare, **reative a zona `auli.com.br`** (estava em *pause*) e confira que
`api.auli.com.br` está **proxied** (nuvem laranja).

### 10.2 Regra de rate limiting (no painel Cloudflare)

**Security -> WAF -> Rate limiting rules -> Create rule**:

| Campo | Valor |
| --- | --- |
| **If incoming requests match** | `Hostname` equals `api.auli.com.br` **AND** `URI Path` equals `/v1/question` |
| **(opcional)** | **AND** `Request Method` equals `POST` |
| **Rate** | ex.: **60** requests per **1 minute** (ajuste à realidade de um órgão; lembre que NAT faz um escritório inteiro sair por **um IP**) |
| **Counting characteristic** | `IP` (com proxy, é o IP real do cliente) |
| **Then** | **Block** por **10s**-**60s**, resposta **429** (corpo custom opcional em pt-BR) |

Defesa em camadas: a regra do Cloudflare é a barreira de borda; o limitador interno do app
(`/v1/question`, por IP) é a segunda linha. Não há mais URL de ngrok pública para contornar.

### 10.3 Verificação

```bash
tail -f /tmp/auli-cloudflared.log                 # túnel: deve logar "Registered tunnel connection"
curl -s https://api.auli.com.br/v1/health         # 200 via Cloudflare
# dispare > limite no /v1/question e espere 429 (vinda do Cloudflare; veja em Security -> Events)
```
