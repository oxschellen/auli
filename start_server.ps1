# start_server.ps1 — compila e sobe o servidor da Auli (workspace `auli-server`, modo `server`) em :3000.
# Equivalente Windows/PowerShell do start_server.sh. Rode a partir de qualquer lugar:
#   powershell -ExecutionPolicy Bypass -File .\start_server.ps1
#
# Sobe também o túnel do Cloudflare (cloudflared) se estiver no PATH e configurado
# (%USERPROFILE%\.cloudflared\config.yml). Configure 1x com `cloudflared tunnel login` + criação + DNS.
#
# Flags:  -NoBuild    pula o `cargo build` e sobe o binário já compilado (restart rápido).
#         -NoTunnel   sobe só o servidor local, sem o túnel Cloudflare.
# Variáveis opcionais (env): PORT (3000), BIND (127.0.0.1 = só localhost; use 0.0.0.0 p/ LAN),
#         AULI_DATA_DIR (../data),
#         TUNNEL_NAME (auli-api), CARGO_TARGET_DIR (reuso do build).

[CmdletBinding()]
param(
    [switch]$NoBuild,
    [switch]$NoTunnel
)

$ErrorActionPreference = 'Stop'

# Raiz = pasta deste script (.../auli). Workspace Cargo = auli-server/.
$Root = $PSScriptRoot
$Ws   = Join-Path $Root 'auli-server'

# cmake 4 reclama de policy antiga (aws-lc-sys). Inócuo com cmake de sistema mais novo.
if (-not $env:CMAKE_POLICY_VERSION_MINIMUM) { $env:CMAKE_POLICY_VERSION_MINIMUM = '3.5' }

# Reaproveita artefatos já compilados (fastembed/ort/aws-lc) -> build incremental rápido.
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $Ws 'target' }

# Raiz data/ (registry.toml + prompts/ + <id>/packs/). O server roda em auli-server/, então é ../data.
if (-not $env:AULI_DATA_DIR) { $env:AULI_DATA_DIR = '../data' }

# Cache do modelo BGE-M3 (ONNX) e logs de Q&A — caminhos ABSOLUTOS na raiz do repo (CWD-independentes).
if (-not $env:EMBED_CACHE_DIR) { $env:EMBED_CACHE_DIR = Join-Path $Root 'models' }
if (-not $env:AULI_LOG_DIR)    { $env:AULI_LOG_DIR    = Join-Path $Root 'logs' }

$Port       = if ($env:PORT) { $env:PORT } else { '3000' }
$Bind       = if ($env:BIND) { $env:BIND } else { '127.0.0.1' }
$TunnelName = if ($env:TUNNEL_NAME) { $env:TUNNEL_NAME } else { 'auli-api' }
$Bin        = Join-Path $env:CARGO_TARGET_DIR 'release\auli.exe'

Set-Location $Ws

# Derruba uma instância anterior, se houver, para liberar a porta.
Get-CimInstance Win32_Process -Filter "Name = 'auli.exe'" |
    Where-Object { $_.CommandLine -match 'server' } |
    ForEach-Object {
        Write-Host "[*] Encerrando instancia anterior (PID $($_.ProcessId))."
        Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
    }

if ($NoBuild) {
    if (-not (Test-Path $Bin)) {
        Write-Error "--NoBuild, mas o binário não existe em $Bin. Rode uma vez sem a flag."
        exit 1
    }
    Write-Host "[*] Pulando build (-NoBuild)."
} else {
    Write-Host "[*] Compilando (release)..."
    cargo build --release --bin auli
    if ($LASTEXITCODE -ne 0) { Write-Error "cargo build falhou."; exit 1 }
}

# Túnel Cloudflare (cloudflared) em background; morre junto com o script.
$Tunnel = $null
if (-not $NoTunnel) {
    $cf = Get-Command cloudflared -ErrorAction SilentlyContinue
    $cfCfg = Join-Path $env:USERPROFILE '.cloudflared\config.yml'
    if ($cf -and (Test-Path $cfCfg)) {
        $log = Join-Path $env:TEMP 'auli-cloudflared.log'
        Write-Host "[*] cloudflared tunnel run $TunnelName (log: $log)"
        $Tunnel = Start-Process -FilePath $cf.Source -ArgumentList @('tunnel','run',$TunnelName) `
            -RedirectStandardOutput $log -RedirectStandardError "$log.err" -PassThru -NoNewWindow
    } else {
        Write-Host "[!] Tunel Cloudflare nao configurado - subindo so o servidor local (use -NoTunnel para silenciar)."
    }
}

Write-Host "[*] Subindo 'auli server' em ${Bind}:${Port} (packs: $env:AULI_DATA_DIR). Ctrl+C para parar."
try {
    & $Bin server --port $Port --bind $Bind
} finally {
    if ($Tunnel -and -not $Tunnel.HasExited) {
        Write-Host "[*] Encerrando cloudflared (PID $($Tunnel.Id))."
        Stop-Process -Id $Tunnel.Id -Force -ErrorAction SilentlyContinue
    }
}
