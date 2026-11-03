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

- [ ] Abrir a janela do driver sem travar a interface
- [ ] Guardar o ajuste por fila, com limite de tamanho
- [ ] Partir do ajuste salvo no envio, e validar com o driver
- [ ] Engrenagem na lista de impressoras

## Pronto quando

Um ajuste feito na janela do driver chega ao próximo envio, e cancelar não muda nada.

## Depende de

`08-impressao-windows`, `11-lateral-impressoras`, `06-perfis`.

## Fora de escopo

Limpeza de cabeça e alinhamento, que são do utilitário de cada fabricante. O problema de listras não é tratado aqui, por decisão do usuário.
