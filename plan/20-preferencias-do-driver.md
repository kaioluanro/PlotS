# Preferências do driver

**Estado:** Em andamento

## Objetivo

Deixar a pessoa ajustar tudo o que o driver de cada impressora oferece (tipo de papel, qualidade, alta velocidade), sem o PlotS precisar conhecer cada fabricante.

## Comportamento

- Cada impressora real da lista tem uma engrenagem ao lado. O clique abre a janela de preferências do próprio driver.
- Impressoras simuladas não têm engrenagem: não têm driver.
- O ajuste vale **só no PlotS**, por fila. Fica no arquivo de perfis e não muda o padrão do Windows para outros programas.
- Cancelar a janela não muda nada.
- O próximo envio parte do que foi salvo. Trabalho em andamento não muda.
- O que o PlotS decide continua valendo sobre o ajuste: papel do lote, uma cópia, escala real, retrato, uma página por folha e uma face só no frente e verso manual. Sem isso, um driver ajustado para duas faces estragaria o lote.
- Ajuste salvo que o driver não aceita mais, por exemplo depois de uma atualização, é descartado com aviso, e o envio usa o padrão da fila.

## Plano de implementação

- [x] Abrir a janela do driver sem travar a interface (`ec46630`)
- [x] Guardar o ajuste por fila, com limite de tamanho
- [x] Partir do ajuste salvo no envio, e validar com o driver
- [x] Engrenagem na lista de impressoras
- [ ] Conferência da pessoa na janela real: mudar uma opção e clicar em OK, abrir e cancelar, "Usar padrão", imprimir com o ajuste. Registrar se o PlotS volta à frente e o foco volta à engrenagem, por OK e por Cancelar. Se o PlotS ficar atrás de outro programa, o lote volta à revisão

## Decisões e limitações

- **Sem cifra, por decisão do usuário:** os ajustes ficam em Base64, como o Windows e outros programas costumam guardar esse dado. O README avisa para não compartilhar o arquivo se o driver usar PIN ou senha. Reavaliar se o PlotS for usado em empresas.
- **Limite conhecido:** um driver que guarde duas faces só nos dados privados e informe uma face no campo público passa pelas conferências. Duas faces no campo público aparece como aviso na linha.
- **CLI:** continua enviando com o padrão da fila, sem ler o ajuste salvo; ele também grava o arquivo de perfis sem a trava do app, por ser outro processo.
- **Pendente, menor:** ver `21-frente-e-verso-automatico`, que leva os textos e o bloco de fila fora da lista. Fechar o app com um driver que ignora `WM_CLOSE` pode somar mais de 10 s.
- **Conferido:** a janela real de "Microsoft Print to PDF" abre em STA, fecha por `WM_CLOSE`, sem gravar nada, e o app sai em 108 ms com ela aberta. Bytes reais do driver lidos, gravados, relidos e aceitos, e envio a arquivo.
- **Não verificado:** clicar em OK na janela real; foco ao fechar; clique físico na janela principal desabilitada; Alt+F4 e fim da sessão com a janela aberta; drivers de fabricante; diálogo de "Usar padrão" junto com o pedido de fechar a janela.

## Pronto quando

Um ajuste feito na janela do driver chega ao próximo envio, e cancelar não muda nada.

## Depende de

`08-impressao-windows`, `11-lateral-impressoras`, `06-perfis`.

## Fora de escopo

Limpeza de cabeça e alinhamento, que são do utilitário de cada fabricante. O problema de listras não é tratado aqui, por decisão do usuário.
