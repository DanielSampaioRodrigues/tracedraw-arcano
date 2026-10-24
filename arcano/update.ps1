# Edicao Arcano 3 do TraceDraw: traz as novidades do projeto original,
# junta com as nossas mudancas, testa, compila, gera o instalador,
# publica para as outras maquinas e instala nesta.
#   -NaoPublicar   so compila e instala aqui
#   -SemTestes     pula os testes (mais rapido)
#   -SoEsteCodigo  nao busca novidades: compila o codigo como esta
param([switch]$NaoPublicar, [switch]$SemTestes, [switch]$SoEsteCodigo)
. (Join-Path $PSScriptRoot 'comum.ps1')

Passo 'Conferindo o codigo'
if (git status --porcelain --untracked-files=no) { Falha 'Ha mudancas sem commit no codigo. Salve (commit) ou descarte antes de atualizar.' }
if ((git rev-parse --abbrev-ref HEAD) -ne 'arcano') { git checkout arcano }
$antes = (git rev-parse HEAD).Trim()

$novas = 0
if (-not $SoEsteCodigo) {
Passo 'Buscando novidades do projeto original'
if (-not ((git remote) -contains 'upstream')) { git remote add upstream $Upstream }
git fetch upstream
$novas = [int](git rev-list --count HEAD..upstream/main)
if ($novas -eq 0) { Aviso 'Nenhuma novidade no projeto original.' } else { Aviso "$novas novidade(s) para juntar." }
if ($novas -gt 0) {
    git merge upstream/main --no-edit
    if ($LASTEXITCODE -ne 0) {
        git merge --abort
        Falha 'Alguma novidade bateu de frente com uma mudanca nossa. Nada foi alterado; peca ao Claude para resolver o conflito.'
    }
}
}

if (-not $NaoPublicar -and (Tem 'gh')) {
    # Nada novo desde a ultima versao publicada: para por aqui.
    $tagAgora = "v$(Versao-Base)+arcano.$([int](git rev-list --count HEAD))"
    gh release view $tagAgora --repo (Repo-Da-Edicao) 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) { Passo "Ja esta tudo atualizado ($tagAgora publicada)."; exit 0 }
}

if (-not $SemTestes) {
    Passo 'Testando as bibliotecas'
    cargo test --workspace --exclude tracedraw
    if ($LASTEXITCODE -ne 0) { git reset --hard $antes; Falha 'Os testes falharam. A juncao foi desfeita; o programa instalado continua como estava.' }
}

Passo 'Compilando a versao final (alguns minutos)'
$n = [int](git rev-list --count HEAD)
$base = Versao-Base
$env:ARCANO_BUILD = "$n"
cargo build --release -p tracedraw -p tracedraw-cli
if ($LASTEXITCODE -ne 0) { git reset --hard $antes; Falha 'A compilacao falhou. A juncao foi desfeita; o programa instalado continua como estava.' }

Passo 'Gerando o instalador'
$iscc = Iscc
if (-not $iscc) {
    Aviso 'Instalando o Inno Setup (gera o instalador)...'
    winget install --id JRSoftware.InnoSetup -e --accept-source-agreements --accept-package-agreements
    $iscc = Iscc
}
if (-not $iscc) { Falha 'Inno Setup nao encontrado. Instale em jrsoftware.org/isinfo.php e rode de novo.' }
$ver = "$base-arcano.$n"
& $iscc "/DVersion=$ver" "/DSrcDir=$Root\target\release" (Join-Path $Root 'packaging\windows\tracedraw.iss') | Out-Host
$setup = Join-Path $Root "dist\tracedraw-$ver-windows-x86_64-setup.exe"
if (-not (Test-Path $setup)) { Falha 'O instalador nao foi gerado.' }
$pasta = Join-Path $Root 'arcano\instalador'
New-Item -ItemType Directory -Force $pasta | Out-Null
Get-ChildItem $pasta -Filter '*.exe' | Remove-Item -ErrorAction SilentlyContinue
Copy-Item $setup $pasta -Force
Copy-Item (Join-Path $Root 'arcano\LEIA-ME-NOELIA.txt') $pasta -Force -ErrorAction SilentlyContinue
# O que liga uma IA ao TraceDraw vai junto com o instalador.
foreach ($f in 'LIGAR-IA-AO-TRACEDRAW.txt', 'REGISTRAR-MCP-CLAUDE.bat', 'registrar-mcp-claude.ps1') {
    Copy-Item (Join-Path $Root "arcano\$f") $pasta -Force -ErrorAction SilentlyContinue
}

if (-not $NaoPublicar) {
    Passo 'Publicando para as outras maquinas'
    $repo = Repo-Da-Edicao
    git push origin arcano
    $tag = "v$base+arcano.$n"
    gh release create $tag $setup --repo $repo --target arcano --title "TraceDraw $base (Arcano $n)" --notes "Edicao Editora Arcano 3. $novas novidade(s) do projeto original nesta versao."
    if ($LASTEXITCODE -ne 0) { Aviso 'Nao consegui publicar esta versao; as outras maquinas ainda nao a veem.' }
}

Passo 'Instalando nesta maquina'
Start-Process $setup -ArgumentList '/SILENT','/SP-','/CLOSEAPPLICATIONS','/NORESTART' -Wait
Passo "Pronto: TraceDraw $base+arcano.$n instalado. O instalador tambem esta em arcano\instalador."
