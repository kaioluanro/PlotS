# Impressão no Windows

**Estado:** Em andamento · OK 1 · PARTIAL 7 · MISSING 0 · BUG 0 · N/A 0

## Objetivo

Enviar cada passada direto ao spooler, na ordem do PlotS, sem diálogo de impressão.

## Plano de implementação

- [x] `trait Backend` e máquina de estados com o verso bloqueado até a frente terminar
- [x] Listar filas e estado com `EnumPrintersW` e `GetDefaultPrinterW`, comando `plots filas` (a9f9185)
- [x] Rasterizar com `pdfium-render`, uma página por vez, na resolução da impressora (126ac26)
- [x] `DEVMODE` forçado e conferido (uma face, uma cópia, 100%, retrato, uma página por folha, papel); desenhar com GDI (126ac26)
- [ ] Escolher a bandeja
- [x] Acompanhar o trabalho com `GetJobW`, sem apagar nada sem perguntar (126ac26)
- [x] Testar sem gastar papel na fila "Microsoft Print to PDF"
- [ ] Confirmar em impressora real

## Decisões

- **Fim da frente:** a confirmação do spooler só garante a entrega, não a saída do papel. O botão do verso libera com ela, e o cartão pede para esperar a última folha. O momento real se mede em impressora real.
- **PDF protegido:** todo PDF passa pela barreira de `PdfOrigem::carregar` antes do PDFium.

## A observar no primeiro teste com papel

- Se o driver aceita os valores forçados, em especial uma página por folha: um driver que não confirme faz o envio ser recusado, e a regra pode precisar de ajuste.
- Quais bits de `GetJobW` chegam em falta de papel e atolamento. Se vierem só como erro genérico, responder "não" encerra o comando, e recomeçar reenvia a frente.
- O que o driver guarda só em dados privados, como ordem invertida, que o `DEVMODE` público não mostra.
- Se o ponto discreto evita o descarte da folha em branco.
- Volume no spool: A4 a 600 dpi dá cerca de 139 MB por página; o pico de memória é de cerca de 2 GiB; o PDFium não tem limite de tempo.

## Pronto quando

Duas passadas saem na ordem enviada, e o verso só libera quando o spooler confirma o fim da frente.

## Depende de

`02-geracao-pdf`, `05-tamanhos-de-papel`. A confirmação depende de impressora real.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A001 | Enviar as passadas à impressora real pelo spooler, sem driver manual | PARTIAL (126ac26) |
| PS-A040 | Camada de backends: trait com duas implementações (Windows e Simulado) | OK (126ac26) |
| PS-A042 | pdfium-render para rasterizar (impressoras que não aceitam PDF) | PARTIAL (126ac26) |
| PS-A048 | Impressão só pelo PlotS, sem diálogo, envio direto ao spooler | PARTIAL (126ac26) |
| PS-A049 | Rasterizar na resolução da impressora (GetDeviceCaps); DEVMODE com simplex, sem agrupar, bandeja manual e tamanho | PARTIAL (126ac26) |
| PS-A050 | Acompanhar o trabalho no spooler (GetJob ou notificação) | PARTIAL (126ac26) |
| PS-A051 | Detectar a opção "inverter ordem" do driver e pedir para desligar | MISSING |
| PS-A076 | Backend CUPS ou Windows | PARTIAL (126ac26) |

## Especificação

### Como o PlotS imprime no Windows

Quem imprime é sempre o PlotS: ele monta cada passada já na ordem certa e manda direto para o spooler do Windows. O diálogo de impressão do Firefox e do pdf.js nunca entra no caminho, então nada reorganiza as páginas depois do PlotS.

O pdf.js tem um único papel: mostrar o documento e as miniaturas dentro da janela do PlotS.

Caminho de cada passada:

1. O Planejador decide quais páginas entram, em que ordem e quais giram 180°.
2. O `pdfium-render` desenha cada página na resolução da impressora (lida com `GetDeviceCaps`, ex.: 600 dpi), uma página por vez para poupar memória.
3. O PlotS abre a impressora (`OpenPrinterW`) e prepara um `DEVMODE` com: uma face só (`dmDuplex` simplex), sem agrupar cópias (`dmCollate`), a bandeja do perfil (`dmDefaultSource`, ex.: alimentação manual traseira) e o tamanho do papel.
4. Desenha as páginas com GDI (`StartDocW`, `StartPage`, `EndPage`, `EndDoc`): cada página vira uma folha, na ordem enviada.
5. Acompanha o trabalho no spooler (`GetJobW` ou `FindFirstPrinterChangeNotification`) e libera "Imprimir verso" quando a frente termina.

As duas impressoras estão ligadas por USB no mesmo PC; para o Windows são filas locais, listadas com `EnumPrintersW`.

O único ajuste que o PlotS não controla é a opção "inverter ordem" que alguns drivers têm no painel próprio. A calibração detecta se ela está ligada e o PlotS pede para desligar.


| Peça | Escolha | Por quê |
| --- | --- | --- |
| Rasterizar | `pdfium-render` | Para impressoras/sistemas que não aceitam PDF |
| Windows | crate `windows` (winspool + GDI) | Listar impressoras, enviar páginas rasterizadas |
