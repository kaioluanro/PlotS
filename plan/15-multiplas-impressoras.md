# Múltiplas impressoras

**Estado:** Em andamento · OK 0 · PARTIAL 2 · MISSING 6 · BUG 0 · N/A 0

## Objetivo

Enviar o mesmo documento a várias impressoras, cada uma com o próprio perfil.

## Plano de implementação

- [x] Modos espelho e divisão no núcleo
- [x] Máquina de estados por job
- [ ] Jobs paralelos com `tokio`
- [ ] Consulta periódica ao spooler, com aviso de papel acabado ou atolado
- [ ] Grupos salvos de impressoras
- [ ] Modos na interface

## Pronto quando

Duas impressoras avançam de forma independente e entregam o mesmo maço.

## Depende de

`08-impressao-windows`, `13-fluxo-e-recolocacao`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A044 | tokio, um job por impressora em paralelo | MISSING |
| PS-A061 | Marcar várias impressoras e enviar a mesma origem, cada uma com o perfil próprio | MISSING |
| PS-A062 | Descoberta das filas do sistema (EnumPrintersW) | MISSING |
| PS-A064 | Máquina de estados por job: Ocioso, FrenteEnviada, AguardandoRecolocacao, VersoEnviado, Concluído, Erro | PARTIAL |
| PS-A065 | Modos espelho e divisão | PARTIAL |
| PS-A066 | Consulta periódica do spooler; avisa papel acabado ou atolado | MISSING |
| PS-A067 | Grupos salvos de impressoras ("Todas do escritório") | MISSING |
| PS-A080 | Várias impressoras, jobs paralelos, espelho e divisão, status do spooler, fila virtual "PlotS" | MISSING |

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
