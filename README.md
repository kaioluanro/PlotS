# PlotS

O PlotS ajuda a imprimir frente e verso à mão: ele separa as páginas em passadas (frente, depois verso), guarda como cada impressora recoloca o papel e envia cada passada à fila escolhida. Funciona no Windows e fala português do Brasil e inglês.

## Instalar

O instalador foi gerado, mas ainda não foi testado em uma instalação real.

1. Baixe o instalador `PlotS_..._x64-setup.exe` da página de versões do projeto no GitHub.
2. Execute-o. Ele foi configurado para instalar só para o seu usuário, sem pedir administrador, em `%LOCALAPPDATA%\PlotS`.
3. O instalador não é assinado, então o Windows pode mostrar o aviso do SmartScreen. Escolha "Mais informações" e "Executar assim mesmo".

Se o WebView2 Runtime faltar, o instalador o baixa da Microsoft; ele segue os termos da Microsoft.

O instalador foi configurado para registrar o PlotS no "Abrir com" dos arquivos PDF, sem mudar o leitor padrão: o programa que você já usa deve continuar sendo o padrão.

## Licença

O PlotS é software livre sob a licença MIT (arquivo `LICENSE`). Os componentes de terceiros mantêm as próprias licenças, reunidas em `THIRD-PARTY-NOTICES`, que também acompanha o programa instalado.

## Seus dados

Os perfis das impressoras e os ajustes que você salvou nas preferências do driver ficam em `%APPDATA%\plots\perfis.toml`, um arquivo de texto. Os ajustes ficam em Base64, que não é criptografia, e o PlotS não interpreta o que o driver de cada fabricante guarda neles. Se o driver da sua impressora usa senha, PIN ou código de usuário, esses dados podem estar no arquivo, então evite compartilhá-lo. Para apagar o ajuste de uma impressora, use **Usar padrão** na linha dela. Para apagar tudo, feche o PlotS e apague a pasta `plots`. Na próxima abertura o programa começa sem perfis, sem ajustes e sem histórico, e com **Guardar histórico** ligado de novo: se não quiser registrar os envios, desligue a opção outra vez.

O histórico de impressão fica em `%APPDATA%\plots\historico.toml`, também em texto sem criptografia, e guarda até 500 envios feitos pela janela: o nome do arquivo impresso (sem a pasta), a data, as impressoras, as páginas, o trecho, os lotes, como o envio terminou, e o tempo, as folhas e a rota (manual ou automática) de cada impressora. O nome de um arquivo ou de uma impressora pode revelar algo pessoal ou de trabalho, e quem usa a mesma conta do Windows consegue ler essa lista. O PlotS não envia o histórico pela internet. Para não registrar novos envios, desligue **Guardar histórico** no próprio histórico. Para apagá-lo, abra o histórico na barra de título e use **Apagar histórico**. Um envio que começou com a opção ligada entra na lista quando terminar, mesmo que você desligue a opção ou apague a lista antes disso.

`%APPDATA%` é a pasta Roaming. Em um computador com perfil móvel (por exemplo, de uma empresa ou escola), o Windows pode copiar essa pasta para o servidor e levá-la a outros computadores, junto com o nome das impressoras, os ajustes do driver gravados no arquivo e o histórico, com os nomes dos arquivos impressos.

O desinstalador foi configurado para não apagar os perfis, os ajustes nem o histórico.

## Aviso

Nenhuma medida do PlotS foi conferida em impressora real: os testes usaram impressoras virtuais (simuladas e "Microsoft Print to PDF"). Antes de imprimir um lote grande, imprima uma página de teste e confira a posição do papel.

## Compilar

Requer Rust, Node.js e a `pdfium.dll` (baixada por `node scripts/baixar-pdfium.mjs`). Para gerar o instalador, com `PLOTS_PDFIUM_DIR` apontando para a pasta da biblioteca: `node scripts/empacotar.mjs`. Para refazer `THIRD-PARTY-NOTICES`: `node scripts/gerar-avisos.mjs`.
