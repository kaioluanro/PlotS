# PlotInfo no PlotS: imagens e custo de tinta

**Estado:** Em andamento

## Objetivo

Integrar o motor do PlotInfo ao PlotS: um painel que estima a tinta e o custo do documento
aberto, com seleção de páginas e cópias próprias do painel, perfis de tinta e suporte a imagens
no PlotS.

O painel é uma **calculadora independente**. Ele não altera a seleção de páginas, a face
Frente/Verso, as cópias nem o envio do PlotS. A impressão continua exatamente como está hoje.

## Decisões do usuário (2026-11-07)

- O PlotInfo funciona quase independente do PlotS; a seleção de páginas e as cópias pedidas são
  só para o painel.
- O PlotS passa a abrir e imprimir imagens, além de PDF.
- Painel em gaveta à direita do PDF, com um resumo compacto na barra do visualizador.
- Engrenagem no canto superior da gaveta.
- Perfil global de tinta, com preço por mL editável por canal.
- `mutool.exe` incluído para CMYK nativo de PDFs; fallback para PDFium + fórmula simples.
- Sem mapa de calor e sem os modos C/M/Y/K nesta versão.
- Seleção do painel aceita `1`, `2,5,7,9`, `5-16` e mistos; total = páginas × cópias.
- O PlotS não muda em `plots-core`, `plots-pdf` (passadas), `plots-windows` (impressão), no
  intervalo da lateral, na face ou nas cópias do envio.

## Escopo

### 1. Imagens no PlotS

Formatos: PNG, JPEG, BMP, GIF, WebP e TIFF (inclusive multipágina). A imagem vira um **PDF
derivado**, para que prévia, contagem de páginas, frente/verso, marca e impressão usem o mesmo
caminho do PDF sem duplicação.

- Página A4 por padrão, com a imagem ajustada preservando a proporção e o branco em volta;
  orientação retrato ou paisagem conforme a imagem.
- Opção “tamanho físico pelo DPI” para PNG, JPEG, BMP e TIFF; GIF e WebP sem DPI caem em A4.
- Transparência composta sobre branco.
- TIFF multipágina vira um PDF com uma página por quadro.
- Limites: arquivo de até 1 GiB e 100 megapixels por quadro; acima disso, recusa clara.
- PDF derivado em `%LOCALAPPDATA%\plots\cache\imagens\<hash>.pdf`, escrita atômica e limpeza
  best-effort de arquivos com mais de 30 dias.
- O arquivo original nunca é alterado; o nome, o caminho e o histórico continuam os do original.
- Seletor, arrasto e linha de comando aceitam as extensões novas.

### 2. Motor de tinta

Crate novo `plots-info`, sem I/O:

- seleção de páginas do painel;
- conversão RGB/BGRA → CMYK simples, com “cinzas só com preto”;
- leitura de PAM CMYK gerado pelo `mutool`;
- cobertura por canal, mL, custo, cartucho e totais.

Perfis em `plotinfo.toml`, ao lado do `perfis.toml` (o modo portátil continua valendo).

### 3. Análise

- `mutool draw -q -c cmyk -r <dpi> -o page-%04d.pam <pdf> <páginas>`, em lotes e em paralelo
  (núcleos − 1), com timeout e cancelamento.
- Fallback: PDFium rasteriza em RGB e a fórmula simples calcula o CMYK. A gaveta avisa
  “Modo simples (sem mutool)” e o programa continua funcionando.
- A página atual e as páginas escolhidas são analisadas primeiro; o restante do documento vem
  depois, para os totais do documento.
- Cache em memória por hash do conteúdo + dpi + modo; trocar perfil, qualidade, preço por mL ou
  cópias não rasteriza de novo.

### 4. Painel

- `Calculator` ao lado do sino de avisos, na barra de título.
- Gaveta de ~320 px à direita do PDF, com cabeçalho, engrenagem, fechar e rolagem.
- Resumo na barra do visualizador: `4 pág · 0,34 mL · R$ 1,12`; clique abre a gaveta.
- Seções:
  - **ARQUIVO** — nome, páginas, papel, dpi e modo de cor.
  - **PÁGINA N DE M** — a página atual do visualizador, uma cópia.
  - **PÁGINAS A ESTIMAR** — campo de páginas, campo de cópias, total, tinta e custo.
  - **DOCUMENTO** — documento inteiro, uma cópia: totais por cor, total, média e página mais cara.
  - **CARTUCHO** — consumo da seleção × cópias e “~N cópias até esvaziar”.
- Rodapé: `Estimativa · não altera o envio · perfil · qualidade · modo`.
- Estados: vazio, analisando com progresso, erro com motivo, página em branco com 0,0% real.
- Acessibilidade: foco visível, alvos de 24 px, `aria-live`, nada de estado só por cor.
- i18n em pt-BR e en, números com `Intl.NumberFormat`.
- Exportar CSV com uma linha por página da seleção e uma linha de total.

### 5. Entrega

CI, capturas, AUDIT, STATUS, CHANGELOG, versão `1.0.0-alpha.7`, portátil com `mutool.exe` e as
licenças, SHA-256 e limpeza.

## Seleção de páginas do painel

Campo de texto próprio do painel, independente do intervalo da lateral:

| Texto | Seleção |
|---|---|
| vazio | documento inteiro |
| `1` | página 1 |
| `2,5,7,9` | páginas 2, 5, 7 e 9 |
| `5-16` | páginas 5 a 16 |
| `1,3,5-9,12` | lista + faixa |

Regras:

- vírgulas separam itens; `a-b` é faixa inclusiva;
- espaços em volta são ignorados;
- normaliza em ordem crescente e remove repetidas;
- página fora de `1..total` é erro; faixa invertida é erro; item não numérico é erro;
- campo vazio vale o documento inteiro;
- o erro não apaga o último resultado válido: mantém os números e mostra o motivo.

`Total de páginas = quantidade de páginas selecionadas × cópias`.

Exemplos:

- `1` × 2 = 2 páginas;
- `2,5,7,9` × 1 = 4 páginas;
- `5-16` × 12 = 144 páginas;
- vazio (10 páginas) × 5 = 50 páginas.

O painel não separa a seleção em frente/verso: ele estima a tinta de todas as páginas
selecionadas, na ordem em que aparecem. A separação física é assunto do envio do PlotS.

## Cálculo

Para cada página, em CMYK:

```text
K = 1 - max(R,G,B)
C = (1 - R - K) / (1 - K)
M = (1 - G - K) / (1 - K)
Y = (1 - B - K) / (1 - K)
```

Com “cinzas só com preto”, `R = G = B` vira só K.

- `cobertura_c` = média do canal na página × 100.
- `area_ratio` = (largura_mm × altura_mm) / 62370 (área do A4 em mm²).
- `mL_c = cobertura_c × area_ratio × (V_c / (5 × P_c)) × f_qualidade`.
- `custo_c = mL_c × preco_ml_c`.
- `total_pagina = soma dos quatro canais`.
- `seleção = soma das páginas selecionadas × cópias`.
- `documento = soma de todas as páginas × 1 cópia`.
- `cartucho_c = mL_c_da_seleção / V_c × 100`.
- `cópias até esvaziar = floor(V_c / mL_c_por_cópia)`, limitado pela cor que acaba primeiro.

`f_qualidade`: Rascunho 0,6 · Normal 1,0 · Alta 1,3, editáveis no perfil.

No modo `mutool`, o PAM já vem em CMYK; a cobertura é a média dos bytes do canal / 255 × 100.
O MuPDF faz a separação; “cinzas só com preto” vale no modo simples e é aplicada quando o PAM
traz CMY iguais.

## Perfis e preço por mL

`plotinfo.toml` (esquema versionado):

```toml
versao = 1

[configuracao]
perfil = "tanque-generico"
qualidade = "normal"
dpi = 150
modo = "cmyk"
cinzas_so_preto = true
tamanho_da_imagem = "a4"

[[perfis]]
id = "tanque-generico"
nome = "Tanque de tinta genérico"
moeda = "BRL"
volumes_ml = [65.0, 65.0, 65.0, 65.0]
rendimentos_paginas = [6500, 6500, 6500, 4500]
precos_ml = [0.77, 0.77, 0.77, 1.11]
fatores_qualidade = [0.6, 1.0, 1.3]
```

- O preço usado no custo é `preco_ml` por canal.
- A engrenagem mostra “preço do cartucho ÷ volume = preço por mL” como ajuda; o valor salvo é o
  preço por mL.
- Volume e rendimento ficam no perfil para a seção CARTUCHO.
- Perfis genéricos inclusos: cartucho de jato de tinta e tanque de tinta.
- Arquivo corrompido não impede o uso: cai nos perfis padrão e avisa.

## mutool e licenças

- Versão **1.28.5**, a última com binário Windows oficial; a 1.28.6 saiu só em código-fonte.
- Binário Windows: `mupdf-1.28.5-windows.zip`, 94.292.330 bytes, SHA-256
  `13476a53e84f5527983158acb6442c7853b3cdc022f3b24880780dcfba1fcc99`.
- Fonte correspondente: `mupdf-1.28.5-source.tar.gz`, 68.919.385 bytes, SHA-256
  `98a5c10cda20c3992cdf76ff6b2a1149c32bd79cc796d3f703230b1185b7e934`.
- Só o `mutool.exe` entra no portátil, ao lado do `PlotS.exe`.
- Processo separado, nunca linkado ao Rust. O PlotS continua MIT.
- O zip leva `AGPL-3.0.txt` e `MUPDF-SOURCE.txt` com versão, URLs, SHA-256 e oferta escrita do
  código-fonte; `THIRD-PARTY-NOTICES` ganha a entrada do MuPDF.
- O binário não é commitado no repositório: o empacotador baixa, confere o SHA-256 e copia.
- Revisão jurídica antes de publicação continua recomendada.
- O zip cresce de ~10,4 MB para a faixa de 40–50 MB; o tamanho real é medido no lote de entrega.

## Lotes

| Lote | Entrega | Critério de pronto |
|---|---|---|
| **A — imagem** | PDF derivado, `Documento` com tipo, cache, seletor/arrasto/linha de comando, prévia e impressão | PNG colorido abre e imprime com as cores certas; TIFF de 3 páginas conta e imprime 3; PDF existente não muda |
| **B — motor e análise** | `plots-info`, perfis, seleção, mutool + fallback, progresso, cancelamento, cache, CSV | testes exatos de cobertura e custo; DeviceCMYK 40% exato; cancelar e fallback funcionam |
| **C — painel** | ícone, gaveta, campos de páginas/cópias, engrenagem, resumo, i18n, acessibilidade | 1×2=2, 2,5,7,9×1=4, 5-16×12=144; trocar perfil/cópias não rasteriza; capturas nos dois temas |
| **D — entrega** | CI, AUDIT/STATUS/CHANGELOG, alpha.7 portátil com mutool e licenças | CI verde, zip verificado, sem `perfis.toml`/`plotinfo.toml`, limpeza feita |

## Testes

### Imagens

- PNG, JPEG, BMP, GIF, WebP e TIFF de 2 páginas abrem, contam e geram PDF derivado.
- TIFF multipágina vira PDF com uma página por quadro.
- PNG com alfa composto sobre branco.
- 100 megapixels e 1 GiB são recusados sem travar.
- Impressão real na fila “Microsoft Print to PDF”: imagem com faixas coloridas, reabrir o PDF
  gerado e conferir laranja/amarelo/azul sem troca de canais.
- PDF existente continua com o mesmo comportamento; nenhuma passada muda.

### Seleção do painel

- `1`, `2,5,7,9`, `5-16`, `1,3,5-9,12`, vazio, repetidas, fora do total, faixa invertida e item
  não numérico.

### Cobertura e custo

- Página branca = 0%; preta = K 100%; quadrado preto em 25% = K 25%; ciano puro = C 100%.
- A3 = 2× A4.
- PDF DeviceCMYK com retângulo `C 40 M 0 Y 0 K 0` na página toda = C 40% exato pelo mutool.
- Preço por mL, qualidade e volume mudam o custo na proporção certa.
- Cartucho: fração consumida e cópias até esvaziar.

### Análise e interface

- Progresso por página, cancelamento e retomada pelo cache.
- Fallback sem `mutool.exe` produz números e mostra “Modo simples”.
- Painel: vazio, analisando, erro, página em branco, troca de aba, seleção inválida.
- Campos independentes do envio: mudar páginas/cópias no painel não altera a lateral.
- Teclado, foco, `aria-live`, contraste nos temas escuro e cinza.
- Capturas dos estados e dos dois temas.

## Riscos

- **AGPL do mutool**: obrigações de licença e oferta de fonte; revisão jurídica antes de publicar.
- **Precisão**: a estimativa nunca é o consumo real; o rótulo “Estimativa” fica sempre visível.
- **Imagens grandes**: PDF derivado e cache podem crescer; limites e limpeza de 30 dias.
- **DPR/DPI de imagem**: A4 é o padrão; DPI nativo é opcional e pode surpreender.
- **Tempo de análise**: 150 dpi é o padrão; 300 dpi dobra o custo; progresso e cancelamento.
- **Painel independente**: a interface precisa dizer “não altera o envio” para não confundir com
  a seleção da lateral.
- **Impressora real**: a prova de imagem/cores continua sendo a fila de PDF; hardware fica
  pendente.

## Depende de

`08-impressao-windows`, `12-visualizador`, `27-impressao-pdfjs`, `29-cor-das-imagens`.

## Pronto quando

Um PDF e uma imagem abrem no PlotS; a imagem imprime com as cores certas; o painel estima tinta
e custo para uma seleção como `2,5,7,9` com cópias, sem alterar o envio; os totais por página e
por documento batem com os testes; o portátil alpha.7 inclui o `mutool.exe` e as licenças.