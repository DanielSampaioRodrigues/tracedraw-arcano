# TraceDraw: Edição Editora Arcano 3

Esta é a versão do TraceDraw usada na Editora Arcano 3. Ela é o projeto
original ([igtoth/tracedraw](https://github.com/igtoth/tracedraw), licença
MIT ou Apache 2.0) com algumas mudanças por cima:

- **Português do Brasil revisado**: todas as linhas de tradução automática
  foram conferidas, e a terminologia foi unificada (Mistura, Sombreamento,
  Aderir, Painel, Moldura de recorte...).
- **IA com os modelos atuais** (*Ferramentas › Opções › IA*):
  - Gemini Nano Banana 2.1 e Gemini 3 Pro Image;
  - GPT Image 2.5 Flare e Sunburst;
  - Maritaca AI (Sabiá 4), que só faz desenhos, sem imagens.
  - Cada modelo mostra o preço estimado por imagem ou por pedido.
  - Imagens em 1K, 2K ou 4K: 4K tem cerca de 35 cm a 300 dpi.
- **MCP** para o Claude e outras IAs trabalharem no desenho aberto, agora
  com a ferramenta `render_page`, que permite à IA *ver* a página.
- **Atualizações que preservam as mudanças da editora** (*Ajuda ›
  Atualizações*).
- Ícone verde e tela de abertura com a marca da editora.

Tudo o que é só da edição fica em `apps/tracedraw/src/fork.rs`, em
`arcano/` e em `assets/arcano/`. Assim, as novidades do projeto original
entram quase sempre sem conflito.

## Máquina que compila (Dan)

Primeira vez, no PowerShell, dentro da pasta do código:

```powershell
powershell -ExecutionPolicy Bypass -File arcano\setup.ps1
```

O script:
1. instala o Inno Setup e o GitHub CLI, se faltarem;
2. abre o navegador para você entrar no GitHub;
3. liga o projeto original como `upstream`;
4. cria a cópia pública `seu-usuario/tracedraw-arcano`.

Para atualizar, use *Ajuda › Atualizações › Trazer as novidades do
projeto e recompilar*, ou rode:

```powershell
powershell -ExecutionPolicy Bypass -File arcano\update.ps1
```

O script:
1. busca as novidades do projeto original e junta com as nossas mudanças;
2. testa e compila;
3. gera o instalador em `arcano\instalador\`;
4. publica esse instalador no GitHub da editora;
5. instala nesta máquina.

Se alguma novidade entrar em conflito com uma mudança nossa, ele para sem
alterar nada.

Opções do script:
- `-NaoPublicar`: só compila e instala aqui.
- `-SemTestes`: mais rápido.

## Outras máquinas (Noélia)

1. Rode o instalador (`tracedraw-...-setup.exe`) uma vez.
2. Depois disso, use *Ajuda › Atualizações › Instalar*. O TraceDraw baixa
   a versão nova do GitHub da editora, confere o arquivo, fecha
   (perguntando sobre desenhos não salvos), instala e abre de novo.

Essas máquinas não precisam de Rust, Git nem código-fonte.

## IA: chaves

Em *Ferramentas › Opções › IA › Serviço*:
1. escolha a predefinição (Google Gemini, OpenAI ou Maritaca AI);
2. cole a chave;
3. marque "Lembrar a chave neste computador".

O TraceDraw usa **um** serviço de cada vez. Com a Maritaca, a geração de
imagens fica desligada, porque o Sabiá só gera texto. Ao trocar de
serviço, cole a chave do serviço novo.

Onde conseguir as chaves:
- Gemini: Google AI Studio.
- OpenAI: platform.openai.com.
- Maritaca: plataforma.maritaca.ai.

## MCP: ligar o Claude e outras IAs

1. No TraceDraw: *Ferramentas › Opções › Global › Geral* e ligue
   **"Deixar agentes de IA deste computador editarem o desenho aberto
   (MCP)"**. O TraceDraw passa a atender só neste computador
   (`127.0.0.1`), com uma senha aleatória que muda a cada vez.
2. Registre o TraceDraw no programa de IA. O caminho abaixo é o da
   instalação padrão; numa instalação só para o usuário, ele fica em
   `%LOCALAPPDATA%\Programs\TraceDraw\tracedraw.exe`.

**Claude (aplicativo de desktop)**: arquivo
`%APPDATA%\Claude\claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "tracedraw": {
      "command": "C:\\Program Files\\TraceDraw\\tracedraw.exe",
      "args": ["--mcp-window"]
    }
  }
}
```

**Claude Code**:

```powershell
claude mcp add tracedraw -- "C:\Program Files\TraceDraw\tracedraw.exe" --mcp-window
```

**Cursor** (`%USERPROFILE%\.cursor\mcp.json`), **Windsurf**, **LM Studio**
(`mcp.json`) e **Gemini CLI** (`settings.json`): o mesmo bloco
`"mcpServers"` do Claude.

**VS Code** (`.vscode\mcp.json` ou nas configurações do usuário):

```json
{
  "servers": {
    "tracedraw": {
      "type": "stdio",
      "command": "C:\\Program Files\\TraceDraw\\tracedraw.exe",
      "args": ["--mcp-window"]
    }
  }
}
```

Para trabalhar **sem janela**, em lote (converter, gerar arquivos), use
`"args": ["--mcp"]` no lugar de `--mcp-window`.

Ferramentas disponíveis:

| Ferramenta | O que faz |
| --- | --- |
| `open` | abre `.cdr`, SVG, PDF, AI, EPS e outros formatos |
| `new_document` | cria um documento novo |
| `save` | salva em `.tdraw` ou `.cdr`, na versão 7 a 27 |
| `export` | exporta em PDF, PNG, SVG e outros formatos |
| `run_script` | roda um script com o modelo de objetos completo; tudo o que um clique faz, o script faz, e cada script vira um passo de desfazer |
| `document_info` | descreve páginas, camadas e objetos |
| `render_page` | mostra a página como imagem (novo) |
| `undo` / `redo` | desfaz e refaz |

Aplicativos que só aceitam servidores MCP remotos (pela internet) não
conseguem usar este servidor local.
