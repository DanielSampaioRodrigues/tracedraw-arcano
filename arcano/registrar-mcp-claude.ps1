# Edicao Arcano 3: registra o TraceDraw como servidor MCP no aplicativo
# Claude (claude_desktop_config.json). Faz backup e so acrescenta a
# entrada "tracedraw"; os outros servidores ficam como estao.
$ErrorActionPreference = 'Stop'
$exe = @("$env:ProgramFiles\TraceDraw\tracedraw.exe",
         "$env:LOCALAPPDATA\Programs\TraceDraw\tracedraw.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $exe) { Write-Host 'TraceDraw instalado nao encontrado. Instale primeiro.' -ForegroundColor Red; pause; exit 1 }

$dir = Join-Path $env:APPDATA 'Claude'
$cfg = Join-Path $dir 'claude_desktop_config.json'
New-Item -ItemType Directory -Force $dir | Out-Null
if (Test-Path $cfg) {
    $bak = "$cfg.antes-do-tracedraw-$(Get-Date -Format yyyyMMdd-HHmmss).bak"
    Copy-Item $cfg $bak
    Write-Host "Backup: $bak"
    $json = Get-Content $cfg -Raw -Encoding UTF8 | ConvertFrom-Json
} else {
    $json = [pscustomobject]@{}
}
if (-not ($json.PSObject.Properties.Name -contains 'mcpServers')) {
    $json | Add-Member -NotePropertyName mcpServers -NotePropertyValue ([pscustomobject]@{})
}
$entry = [pscustomobject]@{ command = $exe; args = @('--mcp-window') }
if ($json.mcpServers.PSObject.Properties.Name -contains 'tracedraw') {
    $json.mcpServers.tracedraw = $entry
} else {
    $json.mcpServers | Add-Member -NotePropertyName tracedraw -NotePropertyValue $entry
}
$text = $json | ConvertTo-Json -Depth 32
[System.IO.File]::WriteAllText($cfg, $text, (New-Object System.Text.UTF8Encoding($false)))
Write-Host ''
Write-Host "Pronto: TraceDraw registrado ($exe --mcp-window)." -ForegroundColor Green
Write-Host 'Agora: no TraceDraw, Ferramentas > Opcoes > Global > Geral, ligue'
Write-Host '"Deixar agentes de IA deste computador editarem o desenho aberto (MCP)",'
Write-Host 'e feche e abra o aplicativo Claude.'
pause
