# Impressão de uma cópia por vez

**Estado:** Concluído no código; falta impressora real

## Objetivo

Permitir que as frentes de um envio saiam em um trabalho por cópia, esperando a fila confirmar
cada trabalho antes de mandar o próximo. O verso continua saindo inteiro, num segundo pedido.

## Comportamento

- Uma caixinha **“Uma cópia por vez”** fica logo acima do campo **Quantidade**, marcada por
  padrão em cada aba. A escolha vale só para aquela aba; não é preferência global.
- **Frente e verso manual:** com a caixinha marcada, cada cópia da frente vira um trabalho
  separado. O próximo só é enviado depois que o spooler confirma o anterior (o mesmo
  `JOB_STATUS_PRINTED`/`COMPLETO` que o PlotS já usa). Os versos de todas as cópias saem juntos,
  como hoje, quando a pessoa escolhe **Verso**.
- **Simplex:** cada cópia inteira (todas as páginas do trecho) é um trabalho; a próxima só sai
  depois da confirmação.
- **Impressora automática:** marcada, uma cópia por trabalho (comportamento atual); desmarcada,
  todas as cópias num trabalho só.
- **Uma cópia:** a opção não muda nada.
- **Troca de papel:** a sequência para na troca, mostra o papel e retoma depois da confirmação.
- **Erro, aviso, cancelamento ou fechamento da aba:** a sequência para no ponto em que está.
  “Reimprimir” refaz a cópia atual e, quando ela conclui, a sequência continua.

## Decisões

- A separação vale para as **frentes** (`Alcance::SoFrentes`). O verso (`Alcance::SoVersos`) é
  sempre planejado junto, como o usuário pediu.
- O avanço é automático, feito pela mesma thread do trabalho; não há clique sintético nem um
  novo comando de “imprimir de novo”.
- O backend recebe a opção por argumento. O padrão do backend continua o comportamento antigo
  (juntar), para não mudar CLI, PlotSim e testes antigos; a interface manda ligado por padrão.
- O cartão mostra “Cópia N de M” também no fluxo manual separado, reaproveitando o que já existe
  para o automático.
- O resumo e a confirmação continuam contando páginas, folhas e maço físicos; só ganham a nota
  de que as frentes saem uma cópia por vez.

## Plano de implementação

- [x] Núcleo: plano de cópias separadas e plano automático junto, com testes.
- [x] Backend: `Pedido.uma_copia_por_vez`, planejamento por modo, avanço automático entre cópias,
      campo `porCopia` no cartão.
- [x] Comando `iniciar_impressao` recebendo a opção.
- [x] Interface: estado na aba, caixinha acima da quantidade, aviso na confirmação, “Cópia N de M”.
- [x] Textos pt-BR/en e testes de interface.
- [x] Planos, STATUS e auditoria atualizados.

## Resultado

Concluído no código em 2026-11-07.

- `plots-core`: `plano_copias_do_intervalo_separadas` e
  `plano_automatico_copias_do_intervalo_juntas`, com o `juntar_lotes` compartilhado.
- `src-tauri/src/trabalhos.rs`: `Pedido.uma_copia_por_vez`, planejamento separado só nas frentes,
  avanço automático no `Passo::LoteConcluido` e `Cartao.por_copia`.
- `src-tauri/src/comandos_trabalho.rs`: `iniciar_impressao` recebe `umaCopiaPorVez`.
- Interface: `CampoSequencia` acima da quantidade, estado por aba, dica que muda no verso e nota
  na confirmação; o cartão mostra “Cópia N de M” também no fluxo manual separado.
- Provas: 3 cópias de 2 páginas geram 3 frentes, e o teste registra
  `enviar, consultar, consultar, enviar...` — a próxima só sai depois do `Concluido`.
- Gate: `cargo test --workspace --locked` verde, `tsc` limpo, 1.173 testes de UI e build da
  interface concluído.

## Pronto quando

- Com a caixinha marcada, um documento de 3 páginas e 3 cópias envia três trabalhos de frente,
  um de cada vez, e o segundo só começa depois de o primeiro ser confirmado.
- O verso desse envio sai num trabalho só com as 3 cópias.
- O automático marcado envia uma cópia por trabalho; desmarcado, um trabalho com todas.
- Uma cópia, a caixinha desmarcada e o fluxo de verso atual continuam iguais.
- `cargo test --workspace --locked`, `tsc --noEmit`, testes da interface e build verdes.

## Depende de

`13-fluxo-e-recolocacao`, `21-frente-e-verso-automatico`, `24-impressao-sem-lote`.

## Riscos e limites

- O spooler confirma a entrega/impressão; se um driver remover o trabalho da fila sem marcar
  “impresso”, o PlotS trata como concluído e pode mandar a próxima cópia. É o limite que já
  existe hoje.
- Muitas cópias viram muitos trabalhos; o tempo total depende do spooler e do driver.
- No manual, a opção não aumenta as viradas do maço: só as frentes são separadas.