# Edicao Arcano 3: registra o TraceDraw como servidor MCP no aplicativo do
# ChatGPT (OpenAI Codex), que le os servidores de %USERPROFILE%\.codex\config.toml
# (ou de %CODEX_HOME%\config.toml, quando a variavel existe).
# Faz backup e so mexe no bloco [mcp_servers.tracedraw]; os outros ficam como estao.
param([string]$ConfigPath)

$ErrorActionPreference = 'Stop'

function Fechar {
    try { Write-Host ''; Write-Host 'Pressione Enter para fechar.'; [void](Read-Host) } catch { }
}

$exe = @("$env:ProgramFiles\TraceDraw\tracedraw.exe",
         "$env:LOCALAPPDATA\Programs\TraceDraw\tracedraw.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $exe) {
    Write-Host 'TraceDraw instalado nao encontrado. Instale primeiro.' -ForegroundColor Red
    Fechar
    exit 1
}

if (-not $ConfigPath) {
    $homeCodex = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $env:USERPROFILE '.codex' }
    $ConfigPath = Join-Path $homeCodex 'config.toml'
}
$dir = Split-Path $ConfigPath
if (-not (Test-Path $dir)) {
    Write-Host "O aplicativo do ChatGPT nao parece instalado: falta $dir" -ForegroundColor Red
    Write-Host 'Abra o ChatGPT (Codex) uma vez, feche, e rode de novo.'
    Fechar
    exit 1
}

# O comando vai entre aspas duplas: no TOML a barra invertida precisa ser dobrada.
$cmdToml = $exe.Replace('\', '\\')
$bloco = @(
    '[mcp_servers.tracedraw]',
    'enabled = true',
    ('command = "' + $cmdToml + '"'),
    'args = ["--mcp-window"]'
)

$linhas = @()
if (Test-Path $ConfigPath) {
    $bak = "$ConfigPath.antes-do-tracedraw-$(Get-Date -Format yyyyMMdd-HHmmss).bak"
    Copy-Item $ConfigPath $bak
    Write-Host "Backup: $bak"
    $linhas = @(Get-Content $ConfigPath -Encoding UTF8)
}

# Ja existe um bloco [mcp_servers.tracedraw]? Entao troca so ele (e as
# subsecoes dele, tipo [mcp_servers.tracedraw.env]), ate a proxima secao.
$inicio = -1
for ($i = 0; $i -lt $linhas.Count; $i++) {
    if ($linhas[$i] -match '^\[mcp_servers\.tracedraw\]\s*$') { $inicio = $i; break }
}
if ($inicio -ge 0) {
    $fim = $linhas.Count
    for ($j = $inicio + 1; $j -lt $linhas.Count; $j++) {
        if ($linhas[$j] -match '^\[') { $fim = $j; break }
    }
    $novo = @()
    if ($inicio -gt 0) { $novo += $linhas[0..($inicio - 1)] }
    $novo += $bloco
    if ($fim -lt $linhas.Count) { $novo += '' ; $novo += $linhas[$fim..($linhas.Count - 1)] }
    $linhas = $novo
    Write-Host 'Bloco [mcp_servers.tracedraw] atualizado.'
} else {
    if ($linhas.Count -gt 0 -and $linhas[$linhas.Count - 1].Trim() -ne '') { $linhas += '' }
    $linhas += $bloco
    Write-Host 'Bloco [mcp_servers.tracedraw] acrescentado.'
}

$texto = ($linhas -join "`r`n") + "`r`n"
[System.IO.File]::WriteAllText($ConfigPath, $texto, (New-Object System.Text.UTF8Encoding($false)))
Write-Host "Gravado em: $ConfigPath" -ForegroundColor Green
Write-Host ''
Write-Host 'Confira o bloco:'
$bloco | ForEach-Object { Write-Host "  $_" }
Write-Host ''
Write-Host 'Agora: no TraceDraw, Ferramentas > Opcoes > Global > Geral, ligue'
Write-Host '"Deixar agentes de IA deste computador editarem o desenho aberto (MCP)",'
Write-Host 'e feche e abra o aplicativo do ChatGPT (com o TraceDraw ja aberto).'
Fechar
