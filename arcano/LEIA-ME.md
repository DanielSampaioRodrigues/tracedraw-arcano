# TraceDraw: Edição Editora Arcano 3

Esta é a versão do TraceDraw usada na Editora Arcano 3. Ela é o projeto
original ([igtoth/tracedraw](https://github.com/igtoth/tracedraw), licença
MIT ou Apache 2.0) com algumas mudanças por cima:

- **Português do Brasil revisado**: todas as linhas de tradução automática
  foram conferidas, e a terminologia foi unificada (Mistura, Sombreamento,
  Aderir, Painel, Moldura de recorte...).
- **Estúdio de IA** (detalhes abaixo):
  - cinco empresas de imagem, cada uma com a própria chave: Gemini, OpenAI, Recraft, Magnific e Stability;
  - cinco de texto: Claude, GPT, Gemini, Sabiá e DeepSeek;
  - os modelos atuais, com tamanho de 1K a 4K, proporção e preço estimado;
  - a página **Editar imagem**: aumentar qualidade, expandir, remover e trocar fundo, reiluminar e vetorizar.
- **MCP** para o Claude e outras IAs trabalharem no desenho aberto, agora
  com a ferramenta `render_page`, que permite à IA *ver* a página.
- **Atualizações que preservam as mudanças da editora** (*Ajuda ›
  Atualizações*).
- Ícone verde e tela de abertura com a marca da editora.

Tudo o que é só da edição fica em `apps/tracedraw/src/fork.rs`, em
`apps/tracedraw/src/studio/`, em `arcano/` e em `assets/arcano/`. Assim, as novidades do projeto original
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

## IA: o estúdio

Cada empresa tem a própria chave, guardada separadamente: trocar de serviço nunca pede chave de novo.
As chaves ficam em *Ferramentas › Opções › IA › Chaves*, cada uma com os botões "Obter chave" e "Testar".

**Imagem** (no painel *Gerar com IA*, linha "Serviço"):
- **Google Gemini:** Nano Banana 2.1 e Pro.
- **OpenAI:** GPT Image 2.5 Sunburst e Flare. Gera no tamanho exato, em qualquer proporção, até 3840 px.
- **Recraft:** gera **vetor SVG editável** e vetoriza imagens.
- **Magnific (Freepik):** com uma chave só, dá acesso ao Mystic, ao Nano Banana, ao GPT Image, ao Seedream e ao Ideogram, que é ótimo com texto.
- **Stability AI:** só aceita pedidos em inglês. O programa traduz os pedidos pelo serviço de texto.

Para cada modelo você escolhe:
- o **tamanho** (1K, 2K ou 4K);
- a **proporção**;
- as opções do modelo, como qualidade e quanto a imagem muda num remix.

O preço aparece embaixo do modelo.

**Editar imagem** (quinto botão do painel), sempre sobre o bitmap selecionado:
- **Aumentar qualidade:** Magnific criativo e de precisão, Recraft nítido, criativo e realçar, Stability rápido 4×, conservador e criativo.
- **Expandir:** com "Expandir até a proporção", que calcula os lados sozinho.
- **Remover fundo**, **Trocar fundo** e **Reiluminar**.
- **Vetorizar** (Recraft).

O painel mostra o tamanho em pixels e os dpi da imagem no tamanho em que ela está na página. Cada resultado é um passo de desfazer.

**Texto** (desenhos escritos pela IA), em *Opções › IA › Serviço*:
- Anthropic (Claude);
- OpenAI (GPT-6 Luna, Sol e Astra);
- Gemini (3.8 Flash e 4 Argon);
- Maritaca (Sabiá 4);
- DeepSeek (Flash e V4 Pro).

Todos mostram o custo estimado por pedido.

Os modelos, tamanhos e preços vêm de um catálogo (`assets/arcano/ai-catalog.json`). Quando sair modelo novo, basta atualizar esse arquivo no GitHub da editora. Os programas instalados recebem a lista nova sozinhos, uma vez por dia, sem recompilar.

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
| `run_script` | roda um script no desenho aberto, no modelo do TraceDraw ou no das macros; tudo o que um clique faz, o script faz, e cada script vira um passo de desfazer; se der erro, nada fica pela metade |
| `api_reference` | o manual dos dois modelos de script |
| `document_info` | descreve páginas, camadas e objetos, a seleção, a parte da página que aparece na tela e as abas abertas |
| `get_selection` / `select` | lê o que você selecionou e seleciona objetos |
| `render_page` | mostra a página como imagem |
| `render_area` | mostra de perto a seleção, objetos, um retângulo ou o que está na tela |
| `font_metrics` / `list_fonts` | medidas reais da fonte e lista das fontes instaladas |
| `windows` / `use_window` / `switch_document` | lista as janelas e as abas abertas e escolhe uma |
| `undo` / `redo` | desfaz e refaz |

A IA trabalha na **janela em que você mexeu por último**, e toda resposta
diz em qual janela e desenho ela está. Com duas janelas abertas, ela pode
listar as duas (`windows`) e escolher uma (`use_window`).

Aplicativos que só aceitam servidores MCP remotos (pela internet) não
conseguem usar este servidor local.
