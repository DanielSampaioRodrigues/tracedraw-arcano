# Estilos de texto

Um **estilo de texto** é um conjunto de formatação com nome que fica ligado
ao texto que o usa: mude o estilo uma vez e todos os parágrafos do desenho
que o usam mudam junto. Abra o painel [[docker.text_styles]] em
[[menu.window > menu.window.dockers]].

Há dois tipos:

- **Estilos de parágrafo** definem o parágrafo inteiro: alinhamento,
  entrelinha e espaço entre parágrafos, recuos, tabulações, hifenização, e a
  fonte, o tamanho e o peso dos caracteres.
- **Estilos de caractere** definem só alguns caracteres: itálico no título
  de uma obra, negrito, versalete. Ficam por cima do estilo de parágrafo.

## Aplicar um estilo {#apply}

Edite um texto (ou selecione objetos de texto) e clique num estilo do painel:

- o estilo de parágrafo vai para os parágrafos que a seleção toca (o do
  cursor, se nada estiver selecionado);
- o estilo de caractere vai para os caracteres selecionados.

O estilo no cursor fica marcado. [[tstyle.none]] tira o vínculo e deixa o
texto com a aparência que tem.

## Criar e mudar estilos {#edit}

- [[tstyle.new]] cria um estilo com a aparência do texto no cursor e já o
  aplica.
- [[tstyle.book_set]] cria um conjunto inicial para livros: texto corrido,
  primeiro parágrafo, título de capítulo, subtítulo, citação, ênfase, negrito
  e versalete. Eles não definem fonte, então o texto mantém a sua.
- A seta ao lado de um estilo abre os ajustes dele: o nome, o estilo em que
  ele **se baseia** (o que ele não define, pega daquele e acompanha as
  mudanças), [[tstyle.redefine]] (o estilo pega a aparência do texto no
  cursor), [[tstyle.clear]] (aplica de novo, tirando o que foi mudado à mão)
  e [[tstyle.delete]].

Quando o estilo muda, o que ele define vence a formatação feita à mão; o que
ele não define fica como você deixou. Cada mudança é um passo de Desfazer, e
os estilos são salvos com o desenho.
