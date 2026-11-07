# PlotS

O PlotS ajuda a imprimir frente e verso à mão: ele separa as páginas em passadas (frente, depois verso), guarda como cada impressora recoloca o papel e envia cada passada à fila escolhida. Funciona no Windows e fala português do Brasil e inglês.

## Instalar

O instalador foi gerado, mas ainda não foi testado em uma instalação real.

1. Baixe o instalador `PlotS_..._x64-setup.exe` da página de versões do projeto no GitHub.
2. Execute-o. Ele foi configurado para instalar só para o seu usuário, sem pedir administrador, em `%LOCALAPPDATA%\PlotS`.
3. O instalador não é assinado, então o Windows pode mostrar o aviso do SmartScreen. Escolha "Mais informações" e "Executar assim mesmo".

Se o WebView2 Runtime faltar, o instalador o baixa da Microsoft; ele segue os termos da Microsoft.

O instalador foi configurado para registrar o PlotS no "Abrir com" dos arquivos PDF, sem mudar o leitor padrão: o programa que você já usa deve continuar sendo o padrão.

## Impressão

A caixinha **Uma cópia por vez**, logo acima da Quantidade, vem marcada por padrão. Com ela, as frentes saem em um trabalho por cópia: o PlotS só manda a próxima depois que a impressora confirma a anterior. No verso, as cópias continuam saindo juntas, num segundo pedido. Desmarcada, todas as cópias da frente voltam a sair num maço único. Em impressora que imprime as duas faces sozinha, marcada mantém uma cópia por trabalho e desmarcada junta todas, preservando o branco entre cópias de total ímpar.

## Licença

O PlotS é software livre sob a licença MIT (arquivo `LICENSE`). Os componentes de terceiros mantêm as próprias licenças, reunidas em `THIRD-PARTY-NOTICES`, que também acompanha o programa instalado.

O pacote portátil inclui o `mutool.exe` separado, do MuPDF 1.28.5, sob a GNU Affero General Public License 3.0 ou posterior. Ele não é linkado ao PlotS: é chamado como processo separado para separar as páginas em CMYK. O código do PlotS continua MIT. O binário ocupa 44,3 MB e o portátil cresce na mesma faixa. O texto integral da AGPL, as licenças do `thirdparty/` e o endereço da fonte correspondente ficam em `mutool/` (`AGPL-3.0.txt` e `MUPDF-SOURCE.txt`). MuPDF e Artifex são marcas da Artifex Software, Inc.; o PlotS não tem vínculo, patrocínio nem endosso da Artifex.

O `mutool.exe` oficial usa o runtime Microsoft Visual C++ 2015-2022 (x64). Sem esse runtime, o conversor não inicia e o PlotS cai no modo simples com aviso; o modo simples não depende dele.

## Tinta e custo (estimativa)

O ícone de calculadora ao lado do sino abre um painel de **estimativa** de tinta e custo. Ele tem seleção de páginas e cópias próprias, independentes do intervalo, da frente/verso e das cópias da lateral: mudar o painel não altera o envio. O painel mostra a cobertura por canal, o volume em mL, o custo pela moeda do perfil, o cartucho que acaba primeiro, o total do documento e a página mais cara, e exporta um CSV com uma linha por página e uma de total.

Os números são uma estimativa pela área e pela cobertura, não uma medição do consumo real da impressora; não há mapa de calor nem perfil ICC. No modo **CMYK**, o `mutool.exe` separa as páginas e a cobertura vem do PAM; no modo **simples**, o PDFium rasteriza e a fórmula por cor calcula o CMYK. O modo simples funciona sem o `mutool`. Os perfis de tinta guardam volume, rendimento e preço por mL, editáveis na engrenagem e salvos em `plotinfo.toml`.

## Seus dados

No programa instalado, os perfis das impressoras e os ajustes do driver — os que você salvou na engrenagem ou nas opções da impressora — ficam em `%APPDATA%\plots\perfis.toml`, um arquivo de texto. Os ajustes ficam em Base64, que não é criptografia, e o PlotS não interpreta o que o driver de cada fabricante guarda neles. Se o driver da sua impressora usa senha, PIN ou código de usuário, esses dados podem estar no arquivo, então evite compartilhá-lo. Para apagar o ajuste de uma impressora, use **Usar padrão** na linha dela (também no painel de opções). Para apagar tudo no programa instalado, feche o PlotS e apague a pasta `plots`. No portátil, apague também o `perfis.toml` que está na pasta do `PlotS.exe` **e** a cópia em `%APPDATA%\plots` (ou a pasta inteira): apagar só uma das duas não basta, porque o arquivo local continua ou a cópia de `%APPDATA%` volta na próxima abertura. Na próxima abertura, no instalado, o programa começa sem perfis, sem ajustes e sem histórico, e com **Guardar histórico** ligado de novo: se não quiser registrar os envios, desligue a opção outra vez.

Na versão portátil, com um arquivo `PLOTS-PORTATIL.txt` ao lado do `PlotS.exe`, o `perfis.toml` fica nessa mesma pasta e viaja com o programa quando a pasta muda de computador. Na primeira abertura, ele traz o que estiver em `%APPDATA%\plots\perfis.toml`, sem apagar o original; depois disso, o arquivo da pasta é o que vale. O histórico continua em `%APPDATA%\plots\historico.toml`. O arquivo de perfis pode conter ajustes do driver com senha, PIN ou código de usuário, então não o compartilhe nem o envie junto de uma cópia do programa. Sem o marcador, o programa instalado continua usando `%APPDATA%\plots\perfis.toml`.

O histórico de impressão fica em `%APPDATA%\plots\historico.toml`, também em texto sem criptografia, e guarda até 500 envios feitos pela janela: o nome do arquivo impresso (sem a pasta), a data, as impressoras, a face (frentes ou versos), as páginas, o trecho, os lotes, como o envio terminou, e o tempo, as folhas e a rota (manual ou automática) de cada impressora. O nome de um arquivo ou de uma impressora pode revelar algo pessoal ou de trabalho, e quem usa a mesma conta do Windows consegue ler essa lista. O PlotS não envia o histórico pela internet. Para não registrar novos envios, desligue **Guardar histórico** no próprio histórico. Para apagá-lo, abra o histórico na barra de título e use **Apagar histórico**. Um envio que começou com a opção ligada entra na lista quando terminar, mesmo que você desligue a opção ou apague a lista antes disso.

Quando um trabalho sai da fila, o PlotS procura no registro de impressão do Windows, quando consegue lê-lo, se houve falha naquela fila. O conteúdo do evento é usado só nessa comparação: não é gravado, mostrado nem enviado. Se o registro não puder ser lido, o trabalho segue como concluído.

`%APPDATA%` é a pasta Roaming. Em um computador com perfil móvel (por exemplo, de uma empresa ou escola), o Windows pode copiar essa pasta para o servidor e levá-la a outros computadores, junto com o nome das impressoras, os ajustes do driver gravados no arquivo e o histórico, com os nomes dos arquivos impressos.

O desinstalador foi configurado para não apagar os perfis, os ajustes nem o histórico.

## Aviso

Nenhuma medida do PlotS foi conferida em impressora real: os testes usaram impressoras virtuais (simuladas e "Microsoft Print to PDF"). Antes de imprimir um lote grande, imprima uma página de teste e confira a posição do papel.

## Compilar

Requer Rust, Node.js e a `pdfium.dll` (baixada por `node scripts/baixar-pdfium.mjs`). Para gerar o instalador, com `PLOTS_PDFIUM_DIR` apontando para a pasta da biblioteca: `node scripts/empacotar.mjs`. Para refazer `THIRD-PARTY-NOTICES`: `node scripts/gerar-avisos.mjs`.
