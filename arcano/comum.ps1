# Funcoes comuns aos scripts da Edicao Arcano 3.
$ErrorActionPreference = 'Continue'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
$Upstream = 'https://github.com/igtoth/tracedraw.git'
$ForkRs = Join-Path $Root 'apps\tracedraw\src\fork.rs'

function Passo($t) { Write-Host ''; Write-Host "==> $t" -ForegroundColor Green }
function Aviso($t) { Write-Host "    $t" -ForegroundColor Yellow }
function Falha($t) { Write-Host ''; Write-Host "XX  $t" -ForegroundColor Red; exit 1 }
function Tem($cmd) { [bool](Get-Command $cmd -ErrorAction SilentlyContinue) }

function Repo-Da-Edicao {
    # owner/name do repositorio da edicao, lido de fork.rs
    $m = Select-String -Path $ForkRs -Pattern '^\s*"([^"/]+/[^"]+)"\s*$' | Select-Object -First 1
    if (-not $m) { Falha 'Nao achei o repositorio da edicao em fork.rs' }
    return $m.Matches[0].Groups[1].Value
}

function Iscc {
    foreach ($p in @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
                     "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
                     "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe")) {
        if (Test-Path $p) { return $p }
    }
    return $null
}

function Versao-Base {
    $m = Select-String -Path (Join-Path $Root 'Cargo.toml') -Pattern '^version = "(.+)"' | Select-Object -First 1
    return $m.Matches[0].Groups[1].Value
}
