# Edicao Arcano 3: registra o TraceDraw como servidor MCP no aplicativo
# Claude. Acha o arquivo de configuracao nos dois formatos de instalacao:
#   - instalador classico: %APPDATA%\Claude\claude_desktop_config.json
#   - pacote da Microsoft Store (MSIX, o do WindowsApps):
#     %LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude\claude_desktop_config.json
# O pacote da Store le so o segundo, e ignora o primeiro: por isso os dois
# sao procurados. Faz backup e so acrescenta a entrada "tracedraw"; os
# outros servidores ficam como estao.
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

# Onde o Claude guarda o configuracao nesta maquina.
$classico = Join-Path $env:APPDATA 'Claude\claude_desktop_config.json'
$achados = @()
if (Test-Path $classico) { $achados += $classico }
$pacotes = Join-Path $env:LOCALAPPDATA 'Packages'
if (Test-Path $pacotes) {
    Get-ChildItem $pacotes -Directory -Filter 'Claude_*' -ErrorAction SilentlyContinue | ForEach-Object {
        $p = Join-Path $_.FullName 'LocalCache\Roaming\Claude\claude_desktop_config.json'
        if ((Test-Path $p) -and -not ($achados -contains $p)) { $achados += $p }
    }
}
if ($achados.Count -eq 0) {
    # Claude nunca aberto nesta maquina: cria no lugar classico.
    New-Item -ItemType Directory -Force (Split-Path $classico) | Out-Null
    $achados += $classico
    Write-Host "Nenhum configuracao encontrado. Vai criar: $classico" -ForegroundColor Yellow
}

foreach ($cfg in $achados) {
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
    Write-Host "Registrado em: $cfg" -ForegroundColor Green
}

Write-Host ''
Write-Host "Pronto: TraceDraw registrado ($exe --mcp-window)." -ForegroundColor Green
Write-Host 'Agora: no TraceDraw, Ferramentas > Opcoes > Global > Geral, ligue'
Write-Host '"Deixar agentes de IA deste computador editarem o desenho aberto (MCP)",'
Write-Host 'e feche e abra o aplicativo Claude (com o TraceDraw ja aberto).'
Fechar
