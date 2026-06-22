# sync-to-wsl.ps1 — empurra o código do Windows para a cópia de build no WSL.
#
# Por que existe: o caminho deste repo tem acento + espaços ("Área de Trabalho") e
# fica sob o OneDrive. Passar esse caminho como ARGUMENTO para wsl.exe quebra
# (erros tipo "de: command not found"). Então a lógica de verdade vive em
# scripts/auli-sync.sh, onde o caminho é um literal UTF-8. Este launcher só:
#   1. copia o auli-sync.sh para o HOME do WSL (via UNC \\wsl.localhost — file op
#      nativa do PowerShell, que lida bem com o acento), forçando fim de linha LF;
#   2. roda `wsl bash /home/ubu/auli-sync.sh` (sem nenhum argumento acentuado).
#
# Uso:
#   .\scripts\sync-to-wsl.ps1            # sincroniza
#   .\scripts\sync-to-wsl.ps1 -DryRun    # só mostra o que mudaria
[CmdletBinding()]
param(
    [switch]$DryRun,
    [string]$Distro = 'Ubuntu-24.04'
)

$ErrorActionPreference = 'Stop'

$localScript = Join-Path $PSScriptRoot 'auli-sync.sh'
if (-not (Test-Path $localScript)) {
    throw "Não encontrei $localScript"
}

$wslHome = "\\wsl.localhost\$Distro\home\ubu"
if (-not (Test-Path $wslHome)) {
    throw "WSL home inacessível: $wslHome  (a distro '$Distro' está rodando?)"
}

# Copia com LF garantido (lê texto, troca CRLF->LF, grava UTF-8 sem BOM).
$content = [System.IO.File]::ReadAllText($localScript) -replace "`r`n", "`n"
$target  = Join-Path $wslHome 'auli-sync.sh'
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($target, $content, $utf8NoBom)
Write-Host ">> auli-sync.sh atualizado em $target" -ForegroundColor DarkGray

# Roda no WSL. Nenhum argumento acentuado é passado para wsl.exe.
$arg = if ($DryRun) { '--dry-run' } else { '' }
Write-Host ">> Rodando sync no WSL ($Distro)..." -ForegroundColor Cyan
if ($DryRun) {
    & wsl.exe -d $Distro bash /home/ubu/auli-sync.sh --dry-run
} else {
    & wsl.exe -d $Distro bash /home/ubu/auli-sync.sh
}
if ($LASTEXITCODE -ne 0) {
    throw "sync falhou (exit $LASTEXITCODE)"
}
Write-Host ">> OK." -ForegroundColor Green
