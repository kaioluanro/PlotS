# Múltiplas impressoras

**Estado:** Em andamento · OK 4 · PARTIAL 3 · MISSING 0 · BUG 0 · N/A 1

## Objetivo

Enviar o mesmo documento a várias impressoras, cada uma com o próprio perfil.

## Plano de implementação

- [x] Modos espelho e divisão no núcleo
- [x] Máquina de estados por job
- [x] Jobs paralelos, uma thread por trabalho, sem `tokio`
- [x] Consulta periódica ao spooler, com aviso de papel acabado ou atolado (`ca10be1`); falta conferir com driver real
- [x] Grupos salvos de impressoras (`62d51a4`)
- [x] Modos na interface (`62d51a4`)

## Decisões

- **Sem `tokio`:** cada trabalho já roda na própria thread; a especificação cita `tokio`, mas o requisito vale pelo comportamento.
- **Divisão com impressora ocupada ou fora da conta:** o botão fica desabilitado com o motivo, em vez de mandar o documento inteiro à livre. Com três ou mais marcadas e uma ocupada, a divisão é refeita entre as livres, e a confirmação mostra quem imprime o quê. Desmarcada a ocupada, o seletor some, e o envio é de uma impressora só.
- **Documento trocado:** total de páginas e lotes são conferidos ao preparar. Um PDF com o mesmo total e outro conteúdo passa: risco baixo, aceito.
- **Rodapé na altura curta:** só cede com a divisão em curso ou quando toda marcada tem cartão em andamento nesta aba. Em 640x420, com lote faltando e nome de 60 caracteres, só um lote da junção aparece sem rolar: o que vem antes é a falta, que importa mais.
- **Pendente, menor:** fio de 1 px sob a faixa do nome presa; poucos pixels de rolagem nos campos do rodapé com erro no nome de grupo; nome de grupo longo cortado sem reticências; hover de botão secundário no tema claro, token comum.
- **Não verificado:** papel e impressora real; divisão em fila real, que só chegou à confirmação; três impressoras até o fim, só com dados de teste; todas ocupadas em outra aba, só teste de unidade; título de Reimprimir em divisão e `todosConferem` na tela; cores forçadas na faixa; leitor de tela real.

## Depois da primeira versão

- **Divisão fora da janela, por decisão do usuário em 2026-11-03 (`24-impressao-sem-lote`):** todas as marcadas imprimem a mesma coisa. Só voltaria com a divisão por velocidade, se ele pedir.
- **Divisão por velocidade:** dar mais lotes à impressora mais rápida, com a velocidade medida no histórico (`22-opcoes-de-envio-e-historico`). Começar só com medidas de impressoras reais. Hoje os lotes são repartidos igualmente, em rodízio, e a confirmação apenas avisa da diferença. A confirmação e a lista de junção precisam mostrar a repartição antes do envio, e cada página continua saindo exatamente uma vez.

## Pronto quando

Duas impressoras avançam de forma independente e entregam o mesmo maço.

## Depende de

`08-impressao-windows`, `13-fluxo-e-recolocacao`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A044 | tokio, um job por impressora em paralelo | N/A |
| PS-A061 | Marcar várias impressoras e enviar a mesma origem, cada uma com o perfil próprio | OK |
| PS-A062 | Descoberta das filas do sistema (EnumPrintersW) | PARTIAL (a9f9185) |
| PS-A064 | Máquina de estados por job: Ocioso, FrenteEnviada, AguardandoRecolocacao, VersoEnviado, Concluído, Erro | PARTIAL |
| PS-A065 | Modos espelho e divisão | OK |
| PS-A066 | Consulta periódica do spooler; avisa papel acabado ou atolado | PARTIAL |
| PS-A067 | Grupos salvos de impressoras ("Todas do escritório") | OK |
| PS-A080 | Várias impressoras, jobs paralelos, espelho e divisão, status do spooler, fila virtual "PlotS" | PARTIAL |

## Especificação

### Múltiplas impressoras

Você marca várias impressoras e o PlotS envia o mesmo documento a todas, cada uma com a ordem do seu próprio perfil — o maço final sai igual em qualquer uma.

- **Descoberta**: o backend lista as filas do sistema; o PlotS mostra só as que têm perfil ou oferece calibrar as novas.
- **Perfil por fila**: chave = nome da fila no sistema, com apelido amigável ("Jato — escritório").
- **Jobs independentes**: cada impressora tem sua máquina de estados (`Ocioso → FrenteEnviada → AguardandoRecolocação → VersoEnviado → Concluído`, com `Erro` em qualquer ponto).
- **Modos de distribuição**: *espelho* (todas imprimem o documento inteiro, ex.: várias cópias) e *divisão* (lotes repartidos entre as impressoras para terminar mais rápido).
- **Status**: consulta periódica ao spooler (IPP `Get-Job-Attributes` no CUPS; `GetJob` no Windows) para liberar o botão do verso e avisar papel acabado ou atolado.
- **Grupos salvos**: um conjunto de impressoras + modo vira um atalho ("Todas do escritório").

| Peça | Escolha | Por quê |
| --- | --- | --- |
| Assíncrono | `tokio` | Um job por impressora em paralelo |
