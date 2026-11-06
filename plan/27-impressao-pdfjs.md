# Impressão no Windows: faces corretas e desenho compatível

**Estado:** Em andamento

## Objetivo

Corrigir a folha em branco que sai em impressoras reais e fazer "Frente" e "Verso" imprimirem as faces que a pessoa espera.

## Causas confirmadas

1. **"Frente" enviava a passada par.** O plano manual chama de `passada1` as páginas pares (o verso físico da folha) e de `passada2` as ímpares (a frente). A opção "Frente" mandava `passada1`. Em um documento de 1 página, `passada1` é só a face em branco: a impressora puxa uma folha e devolve em branco, exatamente o relato. Em documento ímpar maior, a "Frente" imprimia as páginas 2, 4... em vez de 1, 3...
2. **Desenho por `StretchDIBits` com DIB BGRA de cima para baixo direto no DC da impressora.** Funciona na fila "Microsoft Print to PDF", mas drivers de fabricante podem ignorar esse formato, sobretudo em 600 dpi. O caminho do pdf.js não entrega um DIB gigante ao driver: rasteriza e deixa o sistema compor. O equivalente nativo é desenhar por um DC de memória com bitmap compatível e `StretchBlt`, que é o caminho GDI clássico e não depende de `RC_STRETCHDIB`.

## Comportamento

- **Frente** = páginas ímpares do documento (1, 3, 5...), na ordem da primeira passada, sem o giro de verso.
- **Verso** = páginas pares (2, 4, 6...) e a face em branco final, na ordem da primeira passada, com o giro do perfil quando houver. É o mesmo `passada1` de hoje.
- **Documento de 1 página:** "Frente" imprime a página 1, sem folha em branco, sem pedir recolocação e sem segundo trabalho. "Verso" não tem o que imprimir e a interface não oferece o envio.
- **Documento de 3 páginas:** "Frente" imprime 1 e 3; "Verso" imprime 2 e a face em branco que fecha a última folha.
- **Frente e verso manual:** as duas passadas continuam como hoje; só a escolha "Frente"/"Verso" isolada muda.
- **Desenho:** cada página é copiada para um DC de memória com bitmap compatível com a impressora e levada ao DC da impressora com `StretchBlt`, em `HALFTONE`. Se a criação do DC/bitmap falhar, cai no `StretchDIBits` atual. `RC_STRETCHDIB` deixa de ser exigência para o caminho de memória; só o fallback depende dele.
- **O que não muda:** papel, uma face por passada, uma cópia, escala 100%, retrato, uma página por folha, a ordem do perfil e o tamanho físico da página.
- **Resolução:** continua a do driver. O painel de opções da impressora (lote 28) permite escolher uma resolução menor quando o driver não aguentar a padrão.

## Testes

- `plots-print`/`plots-core`: uma página tem frente `[1]` e verso vazio; três páginas têm frente `[1,3]` e verso `[2, branco]`; duas páginas têm frente `[1]` e verso `[2]`; a ordem e o giro do perfil continuam valendo.
- `src-tauri`: a simulação de uma página termina em `FrentesProntas` com uma folha, sem cartão de verso; a de três páginas fecha o maço certo.
- `plots-windows`: o envio à fila "Microsoft Print to PDF" continua gerando PDF com conteúdo; conferir com um PDF de 1 página que a saída não é branca.
- Falha anterior registrada para os testes novos de face.
- Uso real: app com CDP, PDF de 1 página, fila "Microsoft Print to PDF"; a saída tem uma página com conteúdo e o cartão não oferece verso.

## Depende de

`08-impressao-windows`, `13-fluxo-e-recolocacao`, `24-impressao-sem-lote`.

## Pronto quando

Um PDF de 1 página sai na frente, sem folha em branco, e um de 3 páginas tem a frente e o verso nos números certos, com o desenho passando por `StretchBlt`.
