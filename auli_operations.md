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

- Binários em `auli-server/target/release/`: **`auli`** (server/update) e um scraper por entidade —
  **`auli-scraper-rs`** (headless Chrome; FAQs + serviços), **`auli-scraper-sc`** (API JSON Next.js),
  **`auli-scraper-sp`** (REST SharePoint, JSON) e **`auli-scraper-pr`** (HTML Drupal server-side). Só
  o `-rs` puxa headless Chrome.
- 1º build recompila fastembed/ort/aws-lc (alguns minutos); depois é incremental (segundos). Os
  scrapers **não** dependem de fastembed/ort — compilam leves.
- Numa máquina com cmake de sistema, **nenhuma** das `export` é necessária.
- Testes: `cargo test --workspace`.

> Se faltar cmake: `python3 -m pip install --user cmake` (sem sudo) e garanta `~/.local/bin` no PATH.

---

## 4. Dados necessários para servir

Tudo vive na pasta única **`data/`** na raiz (`AULI_DATA_DIR`, default `../data` a partir de
`auli-server/`). O `auli server` lê de lá: `registry.toml`, `prompts/` e os packs por entidade.

### 4.1 Pacotes de vetores (`data/<id>/packs/`)

Pipeline em **três passos** (a coleta virou binários próprios na fase 2; tudo roda de `auli-server/`):

1. **Raspar** (rede; headless Chrome só no RS) → grava o snapshot `data/<id>/<id>-snapshot.json` (v2):
   `auli-scraper-rs [faqs|servicos|all]` (RS), `auli-scraper-sc servicos` (SC),
   `auli-scraper-sp servicos` (SP) e `auli-scraper-pr servicos` (PR). `--usecache` reusa o cache de
   páginas (offline, sem rede).
2. **Derivar** (offline) → o contrato `<id>-faqs.json`/`<id>-servicos.json` + prints + index +
   per-público (e a árvore `faqs-tree.json` p/ a UI, no RS) em `data/<id>/raw/`:
   `./target/release/auli-collections <id>`.
3. **Vetorizar** → `scripts/build-packs.sh <id>` (aponta o `auli update --source` para `raw/`).

```bash
cd auli-server
./target/release/auli-scraper-rs all && ./target/release/auli-collections rs && cd .. && scripts/build-packs.sh rs
# SC: (cd auli-server && ./target/release/auli-scraper-sc servicos && ./target/release/auli-collections sc) && scripts/build-packs.sh sc
# SP: (cd auli-server && ./target/release/auli-scraper-sp servicos && ./target/release/auli-collections sp) && scripts/build-packs.sh sp
# PR: (cd auli-server && ./target/release/auli-scraper-pr servicos && ./target/release/auli-collections pr) && scripts/build-packs.sh pr
```

Produz, por entidade, `data/<id>/packs/<id>-servicos.json` + `<id>.manifest.json` (`strategy_version: 2`,
kind `servicos`) — e `<id>-faqs.json` onde houver FAQs (hoje só RS). Contagens atuais: **rs** serviços
586 + FAQs 1937, **sc** 208, **sp** 537, **pr** 141. `pareceres`/`notas` são autorados (sem scraper) e
ainda **não** têm fonte struct no contrato — ficam **ausentes** até serem modelados; o server tolera
packs ausentes (sobe com a coleção vazia). **Só precisa rodar de novo quando o conteúdo ou a estratégia
de embedding mudar.**

> Regenerar **sem re-raspar** (ex.: após bump de `STRATEGY_VERSION`): com o snapshot já em disco,
> `auli-collections <id>` re-deriva os contratos offline e `build-packs.sh` regera os packs
> (substitui o antigo subcomando `rebuild`, removido).

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

**Boot saudável** se parecer com (4 entidades ativas):

```text
🏛️  Entidades carregadas: [pr, rs, sc, sp]
🔎 Manifesto de 'rs' validado contra a identidade local.
📦 rs-servicos — 586 registros
📦 rs-faqs — 1937 registros
🔎 Manifesto de 'sp' validado contra a identidade local.
📦 sp-servicos — 537 registros
🔎 Manifesto de 'pr' validado contra a identidade local.
📦 pr-servicos — 141 registros
🔎 Manifesto de 'sc' validado contra a identidade local.
📦 sc-servicos — 208 registros
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
