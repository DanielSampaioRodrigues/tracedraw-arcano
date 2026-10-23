# Edicao Arcano 3 do TraceDraw: preparacao desta maquina (uma vez so).
# Instala o que falta (Inno Setup, GitHub CLI), liga o repositorio original
# e cria a copia publica da editora no GitHub, de onde as outras maquinas
# baixam as versoes.
. (Join-Path $PSScriptRoot 'comum.ps1')

Passo 'Conferindo as ferramentas'
foreach ($c in 'git','cargo') { if (-not (Tem $c)) { Falha "Falta '$c' nesta maquina. Instale e rode de novo." } }
if (-not (Iscc)) {
    Aviso 'Instalando o Inno Setup (gera o instalador)...'
    winget install --id JRSoftware.InnoSetup -e --accept-source-agreements --accept-package-agreements
}
if (-not (Tem 'gh')) {
    Aviso 'Instalando o GitHub CLI...'
    winget install --id GitHub.cli -e --accept-source-agreements --accept-package-agreements
    $env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
}

Passo 'Entrando no GitHub (abre o navegador)'
gh auth status 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) { gh auth login --web --git-protocol https }
$login = (gh api user --jq .login).Trim()
if (-not $login) { Falha 'Nao consegui ler o seu usuario do GitHub.' }
Aviso "Usuario do GitHub: $login"

# O repositorio da edicao fica na sua conta: ajusta fork.rs se preciso.
$repo = Repo-Da-Edicao
$want = "$login/tracedraw-arcano"
if ($repo -ne $want) {
    (Get-Content $ForkRs -Raw -Encoding UTF8).Replace("""$repo""", """$want""") | Set-Content $ForkRs -Encoding UTF8 -NoNewline
    git add $ForkRs
    git commit -m "Arcano: the edition's repository is $want" | Out-Null
    $repo = $want
}

Passo 'Ligando o projeto original (upstream)'
if ((git remote) -contains 'origin') {
    if ((git remote get-url origin) -like '*igtoth/tracedraw*') { git remote rename origin upstream }
}
if (-not ((git remote) -contains 'upstream')) { git remote add upstream $Upstream }
git fetch upstream

Passo "Criando a copia publica da editora: github.com/$repo"
gh repo view $repo 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    gh repo create $repo --public --description 'TraceDraw, edicao Editora Arcano 3: portugues revisado, IA com modelos atuais, MCP e atualizacoes'
}
if (-not ((git remote) -contains 'origin')) { git remote add origin "https://github.com/$repo.git" }
git push -u origin arcano

Passo 'Pronto. Para atualizar e publicar: arcano\update.ps1 (ou Ajuda > Atualizacoes no TraceDraw).'
