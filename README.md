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

Os perfis das impressoras ficam em `%APPDATA%\plots\perfis.toml`, um arquivo de texto. Para apagá-los, feche o PlotS e apague esse arquivo (ou a pasta `plots`). Na próxima abertura o programa começa sem perfis.

`%APPDATA%` é a pasta Roaming. Em um computador com perfil móvel (por exemplo, de uma empresa ou escola), o Windows pode copiar essa pasta para o servidor e levá-la a outros computadores, junto com o nome das impressoras gravado nos perfis.

O desinstalador foi configurado para não apagar os perfis.

## Aviso

Nenhuma medida do PlotS foi conferida em impressora real: os testes usaram impressoras virtuais (simuladas e "Microsoft Print to PDF"). Antes de imprimir um lote grande, imprima uma página de teste e confira a posição do papel.

## Compilar

Requer Rust, Node.js e a `pdfium.dll` (baixada por `node scripts/baixar-pdfium.mjs`). Para gerar o instalador, com `PLOTS_PDFIUM_DIR` apontando para a pasta da biblioteca: `node scripts/empacotar.mjs`. Para refazer `THIRD-PARTY-NOTICES`: `node scripts/gerar-avisos.mjs`.
