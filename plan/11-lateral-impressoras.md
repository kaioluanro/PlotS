# Barra lateral de impressoras

**Estado:** Concluído · OK 2 · PARTIAL 0 · MISSING 0 · BUG 0 · N/A 1

## Objetivo

Escolher por documento quais impressoras recebem o arquivo e com qual perfil cada uma imprime.

## Plano de implementação

- [x] Listar filas e impressoras simuladas, com caixa de seleção e modo somente leitura (f6e3433)
- [x] Seletor de modelo interativo de 24 px em cada fila real, reavaliando a linha
- [x] Seleção por aba; aba nova começa sem nenhuma marcada
- [x] Campo de lote com resumo e botão "Imprimir frente" com a contagem, desabilitado
- [ ] Ligar "Imprimir frente" ao envio real. Impressora sem perfil, offline ou com PDF recusado não pode contar em "Imprimir frente (N)"
- [ ] Resumo do lote considerar a divisão por papel
- [x] "Calibrar…" como opção do seletor, com o comando para copiar
- [ ] Quando houver instalador, voltar o comando de `.\plots-cli` para `plots` e retirar a frase de instalação

## A reavaliar com o conteúdo real

Lateral de 240 px e altura mínima de 420 px, com a linha de impressora e "Imprimir frente" sempre visível; o que Ctrl+P faz; menu de contexto no campo de lote.

## Decisões da reavaliação com o conteúdo real

- **Largura de 240 px:** a linha não comporta nome, caixa e "Traseira ↑ face para cima" em uma só altura. Menor mudança adotada, com a largura de 240 px mantida: duas fileiras independentes, `nome | seletor` e `estado | face`, com gap de 6 px. O nome tem até 2 linhas e `title`, e o estado também. O seletor tem largura fixa de 108 px: o nome fica ao lado dele quando cabe em 1 linha; se não couber, o seletor desce, alinhado em x=38. Trocar de modelo não muda altura nem posição.
- **Altura mínima de 420 px:** confirmada com o conteúdo real. Rodapé com lote, resumo, botão e explicação fixo; só a lista rola. A lista fica com 205 px e cabem cerca de 4 impressoras de nome curto.
- **Ctrl+P:** por ora não faz nada, com o bloqueio mantido. Deve ser reavaliado junto com o envio real.
- **Menu de contexto:** liberado só no campo de lote, por um atributo explícito.
- **Perfil:** cada fila real tem um seletor com "Jato de tinta", "Laser" e, depois de um divisor, "Calibrar…". O valor vem do campo `modelo` do perfil, sem deduzir: perfil sem modelo mostra "Personalizado", fila sem perfil mostra "Sem perfil", e nenhum dos dois aparece na lista para ser escolhido. Simulada mostra o modelo fixo. Com o arquivo de perfis ilegível, a fila mostra "Perfil não lido" em vez de fingir que não tem perfil.
- **Entrada ruim:** um `modelo` desconhecido ou com valores divergentes abre como perfil personalizado, com os valores gravados, e as outras filas ficam intactas. `borda_virada` em perfil que não vira é tratada como não virar. Valor inválido em `borda_virada` vale como virada pela lateral, ou não virar quando o perfil não vira, e em `borda_primeiro` vale como borda curta; nos dois casos a entrada abre sem modelo. Escolher um modelo conserta a entrada. A recusa fica só ao gravar. Campo obrigatório ausente, tipo errado nos outros campos e TOML malformado ainda recusam o arquivo.
- **Substituir perfil personalizado:** escolher um modelo sobre um perfil personalizado pede confirmação, porque os valores só voltam calibrando de novo. Cancelar ou Esc não grava nada, e o foco volta ao seletor. Modelo para modelo e "Sem perfil" para modelo não pedem confirmação.
- **Falha ao gravar:** o aviso fica dentro do item da lista, logo abaixo da linha afetada; a lista rola para mostrar linha e aviso, e o foco fica no seletor. Se a linha sumir da lista, o aviso vai para o topo, junto com os avisos gerais.
- **Teclado:** com a lista fechada, Enter, as setas, Home, End, PageUp e PageDown abrem a lista em vez de trocar o valor. Na lista do sistema, as teclas de andar gravariam a cada passo e abririam "Calibrar…". Na lista própria, Esc não grava; na lista do sistema, que é só a reserva, o navegador recebe as teclas e o Esc depois de andar nas opções entrega a opção em que parou, sem que a página veja a tecla; sobre um perfil personalizado, isso abre a confirmação, e nada é gravado sem ela. O ponteiro sobre uma opção move o foco para ela, para que só um item fique ativo.
- **Resumo do lote:** conta folhas de duas páginas, como `plano_documento`. Ainda ignora a divisão por tamanho de papel de `plano_lotes_por_papel`, então um PDF com papéis misturados pode resultar em mais lotes que o resumo.

**Exceção à altura mínima (`256e8ce`):** em até 560 px de altura, com cartão na tela, a linha de lote e o resumo somem, e o rodapé inteiro some quando todas as marcadas estão ocupadas, para o gesto e o botão caberem. Com impressora livre marcada, o rodapé fica.

## Pronto quando

Trocar de aba troca a seleção, e o modelo escolhido define o perfil inteiro da impressora.

## Depende de

`06-perfis`, `09-janela-e-abas`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A014 | Marcar impressoras e tamanho do lote (padrão 10 folhas) | OK (f6e3433) |
| PS-A021 | Barra lateral de impressoras com caixa de seleção e seletor de modelo (Jato de tinta, Laser, "Calibrar…") | OK (826b726) |
| PS-A024 | Seleção de impressoras por documento (decidido no AGENTS) | OK (f6e3433) |

## Especificação

**Barra lateral de impressoras**: lista as impressoras instaladas na máquina. A caixa de seleção define quem recebe o arquivo; junto do nome fica o seletor do modelo daquela impressora.

| Modelo | Entrada do papel | Saída | Frente enviada | Verso enviado | Gesto |
| --- | --- | --- | --- | --- | --- |
| Jato de tinta | Atrás, em pé | Na frente, face impressa para cima | Da última para a primeira | Da última para a primeira, girado 180° | Vira pela borda de cima |
| Laser | Embaixo | Em cima, face impressa para baixo | Da última para a primeira | Da primeira para a última, sem giro | Não vira |

O modelo escolhido define o perfil inteiro, conforme `19-modelos-de-impressora`, e é um ponto de partida: deve ser conferido com 2 ou 3 folhas. Calibrar grava um perfil sem modelo, que aparece como "Personalizado". Impressora sem perfil aparece como "Sem perfil", no mesmo seletor.

Abaixo da lista ficam o tamanho do lote e o botão "Imprimir frente", que mostra quantas impressoras vão receber. Durante o lote, a seção EM ANDAMENTO mostra um cartão por impressora com o estado, o lote atual e o botão "Imprimir verso".

**Decidido:** a seleção de impressoras é de cada documento. Uma aba nova começa sem nenhuma marcada; trocar de aba troca a lista marcada na lateral.

