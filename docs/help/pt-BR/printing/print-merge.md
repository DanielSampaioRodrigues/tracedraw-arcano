# Mesclagem de impressão

A mesclagem de impressão preenche um desenho com dados, para fazer muitas
versões dele: crachás, ingressos numerados, certificados, etiquetas com códigos
QR. Cada registro dos seus dados vira uma cópia da página, preenchida.

## Configurar uma mesclagem {#setup}

1. Desenhe a página. Onde vai um valor, digite o nome do campo entre sinais de
   menor e maior em um objeto de texto, como `<Name>`.
2. Escolha [[menu.file > menu.file.print_merge > menu.file.print_merge_create]].
3. Acrescente os dados:
   - **Importar arquivo** lê um arquivo CSV cuja primeira linha nomeia os
     campos;
   - ou **Adicionar coluna** para acrescentar campos à mão, depois acrescente
     registros com **+**.
4. Clique em **Concluir**.

![A caixa de diálogo Mesclagem de impressão](../../img/printing-print-merge.png)

A página atual é copiada uma vez por registro, logo depois dela, e cada cópia é
preenchida. As cópias recebem o nome da página e o número do registro.

## Tipos de campo {#fields}

- **Texto**: substitui `<Field>` em todos os textos da página.
- **Número**: um contador que você não digita: um início, um passo e um número
  de dígitos (início 7, passo 3, dígitos 3 dá 007, 010, 013).
- **Imagem**: a célula tem o caminho de uma imagem. Um texto que contém apenas
  `<Field>` vira a imagem, do maior tamanho que caiba na caixa dele.
- **Código QR**: um texto que contém apenas `<Field>` vira um código QR do
  texto da célula.

Uma célula vazia, ou uma imagem que não carrega, remove esse objeto da página
do registro.

## Editar os registros {#records}

**Configurar colunas** renomeia campos, muda o tipo e as configurações do
contador e remove campos. **Ver um registro** mostra um registro como
formulário; use as setas ou **Ir para o registro** para percorrê-los. Os
botões de marcador inserem um campo no texto que você está editando. **Salvar
fonte de dados** grava os registros como arquivo CSV.

[[menu.file.print_merge_edit]] reabre os registros, e
[[menu.file.print_merge_perform]] executa a mesclagem de novo.
