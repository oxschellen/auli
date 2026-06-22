# Auli — Operação (compilar, subir, ngrok, logs)

Runbook prático para **compilar, gerar os dados e subir o servidor da Auli** (workspace `auli`,
modo `server`) com o túnel **ngrok**, e para saber **onde ficam os logs**. Para a descrição
técnica do código, ver [auli_code.md](auli_code.md) (§9 cobre o workspace `auli`).

> TL;DR — numa máquina já preparada (build feito, packs gerados, Postgres no ar):
> ```bash
> ./start_server.sh                  # compila (incremental) + sobe server + ngrok
> ./start_server.sh --no-build       # restart rápido, sem recompilar
> ./start_server.sh --no-ngrok       # só o servidor local, sem túnel
> ```

---

## 1. O que sobe

O binário único `auli` tem dois modos:

- **`auli server`** — sobe a API HTTP (axum) em `:3000`, **somente leitura**: carrega os pacotes
  de vetores, valida o manifesto e responde perguntas (RAG). Embeda só a pergunta; nunca escreve.
- **`auli update`** — vetoriza os `portal-*.txt` de uma entidade em **pacotes** (`<id>-<kind>.json`
  + `<id>.manifest.json`). É o **único** que escreve dados.

Embeddings (fastembed/BGE-M3) e busca vetorial rodam **in-process** — não há Ollama, ChromaDB nem
serviço de embedding para subir à parte.

---

## 2. Pré-requisitos

| Item | Detalhe |
| --- | --- |
| **Rust** | toolchain estável (`cargo`, `rustc`). |
| **cmake + compilador C** | exigidos por `aws-lc-sys` (TLS rustls). **Nesta máquina não há cmake de sistema**: foi instalado via `pip install --user cmake` (fica em `~/.local/bin`). Ver §3. |
| **Rede (1º build/run)** | `ort` baixa o ONNX Runtime no build; o modelo **BGE-M3** baixa do Hugging Face para `EMBED_CACHE_DIR` no 1º uso. |
| **PostgreSQL** | o `server` conecta no boot para autenticação (`DATABASE_URL`). Precisa estar no ar. |
| **ngrok** | para o túnel público (`api.auli.com.br`). Opcional (use `--no-ngrok`). |
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

O `auli server` lê três coisas a partir do diretório de trabalho (`auli/`):

### 4.1 Pacotes de vetores (`./packs`)
Gerados pelo `auli update` a partir dos `portal-*.txt` da entidade:

```bash
cd /home/ubu/Desktop/auli_new/auli
EMBED_CACHE_DIR=./models \
  ../auli-server/target/release/auli update \
    --entity rs \
    --source ../auli-server/entities/rs \
    --out ./packs --version 1
```
Produz `packs/rs-services.json` (627), `rs-faqs.json` (1734), `rs-pareceres.json` (331),
`rs-notas.json` (1) e `packs/rs.manifest.json`. **Só precisa rodar de novo quando o conteúdo ou a
estratégia de embedding mudar.**

### 4.2 Entidades (`./entities`)
O server lê `./entities/<id>/` (`entity.json` + `prompt.txt`). No workspace isso é um symlink para
o baseline (o `start_server.sh` cria automaticamente se faltar):
```bash
ln -s ../auli-server/entities entities    # dentro de auli/
```

### 4.3 Modelo (`./models`)
Cache do BGE-M3 (`EMBED_CACHE_DIR=./models` → `auli/models`). Baixa do Hugging Face no 1º uso;
depois é reaproveitado (sem rede).

### 4.4 `.env` (raiz `auli_new/`)
Carregado via `dotenv` a partir do CWD para cima — por isso um `.env` na raiz `auli_new/` serve
para o server rodando em `auli/`. Variáveis:

| Variável | Obrigatória? | Uso |
| --- | --- | --- |
| `LLM_API_URL` / `LLM_API_KEY` / `LLM_API_MODEL` | sim | LLM externo (Groq-compat) que redige a resposta |
| `DATABASE_URL` | sim | Postgres (auth no boot) |
| `JWT_RSA_PRIVATE_KEY` / `JWT_RSA_PUBLIC_KEY` / `JWT_SECRET` | sim | JWT RS256 |
| `EMBED_CACHE_DIR` | não (def. `./models`) | cache do modelo |
| `EMBED_THREADS` | não (def. `16`) | threads do ONNX Runtime |
| `POSTGRES_USER` / `VECTOR_DB_PATH` | não | — |

> Faltando uma variável **obrigatória**, o server dá `panic` no boot com mensagem clara.

---

## 5. Subir o servidor + ngrok

O jeito recomendado é o script [start_server.sh](start_server.sh) (na raiz `auli_new/`):

```bash
./start_server.sh                       # build incremental + server + ngrok
./start_server.sh --no-build            # pula o cargo build (restart rápido) + ngrok
./start_server.sh --no-ngrok            # só o servidor local, sem túnel
./start_server.sh --no-build --no-ngrok # restart local puro
```

O que ele faz: exporta o env de cmake desta máquina, reusa `auli-server/target`, entra em `auli/`,
garante o symlink `entities`, derruba uma instância anterior na porta, compila (a menos de
`--no-build`), sobe o **ngrok em background** e o **server em foreground**. **Ctrl+C** encerra os
dois (um `trap` derruba o ngrok junto).

Variáveis de ambiente para sobrescrever:
```bash
PORT=8080 ./start_server.sh                       # outra porta
PACKS_DIR=/dados/packs ./start_server.sh          # outra pasta de pacotes
NGROK_DOMAIN=meu.dominio.ngrok.app ./start_server.sh
```

**Boot saudável** se parecer com:
```
🏛️  Entidades carregadas: [rs]
🔎 Manifesto de 'rs' validado contra a identidade local.
📦 rs-services — 627 registros
📦 rs-faqs — 1734 registros
🧠 Embedder fastembed (BGE-M3) carregado
✅ Server started successfully at 0.0.0.0:3000
```

### Sem o script (comando direto)
```bash
cd /home/ubu/Desktop/auli_new/auli
../auli-server/target/release/auli server --packs-dir ./packs
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
| **Q&A do RAG** | `auli/logs/<AAAA-MM-DD_HH-MM-SS>.txt` (um arquivo por pergunta) | `Pergunta:` + contexto RAG recuperado + `Resposta:`. Gravado por [rag.rs](auli/crates/auli-cli/src/rag.rs) em `./logs/` **relativo ao CWD** — como o server roda em `auli/`, caem em `auli/logs/`. |
| **ngrok** | `/tmp/auli-ngrok.log` | saída do túnel (redirecionada pelo `start_server.sh`). |
| **Console (`tracing`)** | **stdout/stderr** do terminal (não vai a arquivo) | boot, scores, `info/debug/warn`. Controlado por `RUST_LOG`. |

Exemplos:
```bash
ls -lt auli/logs/ | head                 # últimas consultas
tail -f /tmp/auli-ngrok.log              # acompanhar o túnel
RUST_LOG=auli_cli=debug ./start_server.sh   # ver arrays de score + prompt RAG completo no console
```

> Para gravar também o console em arquivo: `./start_server.sh --no-ngrok 2>&1 | tee auli/logs/console.log`.

---

## 8. Parar / reiniciar

- **Parar:** `Ctrl+C` no terminal do server (encerra server + ngrok pelo `trap`).
- **Matar à força (porta presa):** `pkill -f "release/auli server"`.
- **Reiniciar rápido (sem recompilar):** `./start_server.sh --no-build`.

---

## 9. Troubleshooting

| Sintoma | Causa / correção |
| --- | --- |
| `Não foi possível ler o diretório de entidades './entities'` / `Entidades carregadas: []` | Rodou fora de `auli/`, ou falta o symlink `entities`. Use o `start_server.sh` (cria o link e entra em `auli/`). |
| `📦 Pacotes carregados de ./vectors` (e nada carregado) | Esqueceu `--packs-dir ./packs` — caiu no default `./vectors`. Use o script ou passe a flag. |
| Rebaixando `model_quantized.onnx` toda vez | CWD errado → `./models` vazio. Rode de `auli/` com o modelo em `auli/models` (`EMBED_CACHE_DIR=./models`). |
| Erro de cmake / `aws-lc-sys` no build | Sem cmake no PATH ou cmake 4 reclamando de policy. `export PATH="$HOME/.local/bin:$PATH"` e `export CMAKE_POLICY_VERSION_MINIMUM=3.5` (o `start_server.sh` já faz). |
| `Variável de ambiente obrigatória ausente: ...` | Falta variável no `.env` (LLM/JWT/DATABASE_URL). Ver §4.4. |
| `Falha ao conectar ao PostgreSQL` | Postgres não está no ar ou `DATABASE_URL` errada. |
| `Manifest incompatível ...` no boot | Pacotes gerados com modelo/dim/`strategy_version` diferente do binário. Re-gere com `auli update`. |
| `Permission denied` ao rodar o script | Faltou `chmod +x start_server.sh`, ou usou `sudo` (não use). |
| ngrok com "connection refused" no início | Normal: o túnel tenta conectar enquanto o server ainda carrega o modelo; conecta quando o boot termina. |
