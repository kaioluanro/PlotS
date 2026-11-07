# plotinfo-imagem fix round 1 — exigências da revisão de interface

Acrescentado durante o fix round, depois que o agente já estava trabalhando. Vale junto com
`plan/briefs/fix-plotinfo-imagem-1.md`.

## Required (interface), in order

1. **`erro.imagem_invalida`**: acrescentar a recuperação “Salve uma nova cópia pelo programa de
   origem ou reduza a imagem e abra de novo.” / “Save a new copy from the original program or
   reduce the image and open it again.”

2. **Trocar “PDF” por “documento/arquivo”** nos dois idiomas, exatamente nestas chaves:

| Chave | pt-BR | en |
|---|---|---|
| `barra.abrir` | Abrir arquivo | Open file |
| `lateral.semDocumento` | Abra um documento para marcar impressoras. | Open a document to select printers. |
| `folhas.recusado` | Documento recusado | Document refused |
| `visualizador.carregando` | Abrindo o documento. | Opening the document. |
| `visualizador.erro.texto` | Não foi possível mostrar este documento... | Could not show this document... |
| `cartao.botao.semDocumento` | Abra um documento para imprimir. | Open a document to print. |
| `cartao.botao.recusado` | Este documento foi recusado... | This document was refused... |
| `intervalo.fechado.semDocumento` | Abra um documento para escolher as páginas. | Open a document to choose the pages. |
| `intervalo.fechado.recusado` | Este documento foi recusado... | This document was refused... |

Manter “PDF” em `erro.pdf_invalido`, `erro.criptografado` e no aviso de assinatura digital: ali o
PDF é a condição real.

3. **Capturas novas depois das correções**, em `gray` e `dark`, a 1024×768:

- (a) estado vazio sem documento, mostrando título/texto/botão e a lateral “Abra um documento…”;
- (b) banner `nao_suportado` com um arquivo de nome 60+ caracteres (use a linha de comando como
  entrada real; é o mesmo caminho do arrasto), conferindo quebra de linha e “Dispensar” 24×24;
- (c) banner `imagem_invalida` com um PNG corrompido, conferindo a frase de recuperação e a quebra.

Opcional: a variante `recusa` do `EmptyState` (280 px, o caso mais apertado) e a tela de imagem
aberta em `dark`.

Se o tema `dark` não puder ser obtido sem alterar configuração permanente do usuário, registre a
limitação e peça decisão ao coordenador; não altere configuração permanente.

O seletor nativo não é capturável; o teste de `filtroDoSeletor` é a evidência aceita.

## Copy opcional, faça

- `cartao.motivo.pdf` → “documento” (ou justificar o termo técnico).
- `documento.vazioTexto` → “um PDF ou uma imagem”; “os arquivos” no plural.
- `erro.imagem_invalida` menciona também o limite de quadros do TIFF, não só 100 MP.
- `erro.grande` para imagem >1 GiB → “reduza a imagem”, não “divida o documento”.
- `abertura.lenta` se ainda estiver visível.
- Recortar a outra janela de ~13 px na borda de `01-previa.png`.
- Capturar a imagem aberta também em `dark`, se possível.

## Regras

- Atualize os testes de i18n/UI que citarem as frases antigas; o `en` é tipado por `Chave`.
- Nenhuma cor, espaçamento ou layout novo.
- Rode `npm test` em `ui/` e o CI completo; self-check vazio.
- As capturas vão para `saidas/` com prefixo `fix1-` e devem ser lidas pelo agente.