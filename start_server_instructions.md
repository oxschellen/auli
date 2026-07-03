# Starting the Auli Server

How to start the Auli API server (`auli server`, read-only RAG on port `3000`) on **Linux** and on
**Windows (localhost only)**. For the full operations runbook (data pipeline, cloudflared tunnel,
logs), see [auli_operations.md](auli_operations.md).

Both launchers do the same thing: set the environment (data dir, model cache, log dir), free the
port, optionally build, then run the server. The only real difference is the shell.

---

## What starts

- **`auli server`** — HTTP API (axum), read-only. Loads the vector packs and answers questions (RAG).
- Embeddings (BGE-M3) and vector search run **in-process** — no Ollama / ChromaDB / separate service.
- Healthy boot ends with: `✅ Server started successfully at <bind>:3000`.

---

## Linux — `start_server.sh`

Run from the repo root (`auli/`). Do **not** use `sudo`.

```bash
./start_server.sh                        # build (incremental) + server + Cloudflare tunnel
./start_server.sh --no-build             # fast restart, skip cargo build
./start_server.sh --no-tunnel            # local server only, no tunnel
./start_server.sh --no-build --no-tunnel # pure local restart
```

Optional environment overrides:

```bash
PORT=8080 ./start_server.sh              # different port
BIND=127.0.0.1 ./start_server.sh         # localhost only (default is 0.0.0.0)
AULI_DATA_DIR=/dados ./start_server.sh   # different data/ root
```

If the script is not executable: `chmod +x start_server.sh`.

---

## Windows — `start_server.ps1` (localhost only)

Runs the server bound to **`127.0.0.1`** by default — reachable only from this machine (not the LAN,
not the internet). The Cloudflare tunnel is **not** configured on Windows, so always pass `-NoTunnel`.

Open a **PowerShell** terminal (VS Code: `` Ctrl+` ``), then:

```powershell
cd "c:\Users\carlo\OneDrive\Área de Trabalho\auli"
.\start_server.ps1 -NoBuild -NoTunnel     # fast restart (binary already built)
```

If PowerShell blocks the script with an execution-policy error, use this form (bypasses the policy
for this one run only):

```powershell
powershell -ExecutionPolicy Bypass -File .\start_server.ps1 -NoBuild -NoTunnel
```

Flags and overrides:

```powershell
.\start_server.ps1 -NoTunnel              # compile first (cargo build), then run — use after code changes
$env:PORT='8080'; .\start_server.ps1 -NoBuild -NoTunnel   # different port
$env:BIND='0.0.0.0'; .\start_server.ps1 -NoBuild -NoTunnel # deliberately expose to the LAN
```

| Flag        | Effect                                                        |
| ----------- | ------------------------------------------------------------ |
| `-NoBuild`  | Skip `cargo build`; run the already-compiled `auli.exe`.     |
| `-NoTunnel` | Local server only (required on Windows — no tunnel set up).  |

> The terminal stays occupied by the server while it runs — that is expected. Run it in a terminal
> you own (not a detached/background one), so **Ctrl+C** can stop it.

---

## Verify it is up

From a **second** terminal:

```bash
# Linux / Git Bash
curl -s localhost:3000/v1/health          # -> OK
```

```powershell
# Windows PowerShell
curl.exe http://localhost:3000/v1/health  # -> OK
```

Full question smoke test (put the accented body in a UTF-8 file — an inline `-d` gets mangled by the
shell encoding and returns `invalid unicode code point`):

```bash
curl -s -X POST localhost:3000/v1/question -H 'Content-Type: application/json' \
  --data-binary @question.json
# question.json:  {"entity":"rs","question":"Como obtenho certidão negativa de débitos?"}
```

---

## Stop the server

- **In its own terminal:** press **Ctrl+C** (on Linux this also stops the cloudflared tunnel).
- **Force-stop by port (stuck instance):**

```bash
# Linux
pkill -f "release/auli server"
```

```powershell
# Windows
Get-CimInstance Win32_Process -Filter "Name='auli.exe'" | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
```

---

## Prerequisites (first run)

- **Rust toolchain** (`cargo`, `rustc`) — on Windows, the MSVC toolchain.
- **cmake + a C compiler** — required by `aws-lc-sys` when building. Both launchers set
  `CMAKE_POLICY_VERSION_MINIMUM=3.5` for cmake 4 compatibility.
- **`.env`** in the repo root with the LLM keys: `LLM_API_URL`, `LLM_API_KEY`, `LLM_API_MODEL`
  (missing a required one panics at boot with a clear message).
- **Model cache** (`models/`) — BGE-M3 downloads from Hugging Face on the first use, then is reused
  offline. Both launchers point `EMBED_CACHE_DIR` at the repo's `models/` (absolute path).

If `auli.exe` (Windows) / `auli` (Linux) is already built in `auli-server/target/release/`, you can
use `-NoBuild` / `--no-build` and skip compiling entirely.
