# Retomada dos dois lotes interrompidos

Os lotes `frente-e-verso-automatico` e `opcoes-de-envio` avançaram em outra sessão, que não tinha os agentes de revisão. Nada foi integrado, e nenhuma afirmação daquela sessão vale como prova: confira.

| Lote | Cópia (clone do repositório, não worktree) | Último commit | Build | Porta CDP | Brief original |
|---|---|---|---|---|---|
| automático | `E:\PlotS\.deps-fetch\retomada` | `70caa27` | `%LOCALAPPDATA%/plots-build/retomada-automatico` | 9346 | `brief-frente-e-verso-automatico.md` |
| opções | `E:\PlotS\.deps-fetch\retomada-opcoes` | `d240cb2` | `%LOCALAPPDATA%/plots-build/opcoes-de-envio` | 9347 | `brief-opcoes-de-envio.md` e tempo de impressão abaixo |

Base comum: `4b466f7`. O `master` atual é `0d9afdc`, que só acrescenta registros em `plan/`. Regras comuns em `coordenacao-paralela-2.md`. Planos: `plan/21-frente-e-verso-automatico.md` e `plan/22-opcoes-de-envio-e-historico.md`.

**Mapa entre os dois.** O automático entra primeiro. Ele é dono de `ModeloImpressora`, do seletor de modelo, de `dmDuplex` e do cartão sem gesto. O lote de opções põe cada opção em módulo próprio, com uma linha nos arquivos compartilhados (`Sidebar.tsx`, `App.tsx`, `confirmacao.ts`, `tabs.ts`, `trabalhos.rs`, `i18n.ts` com seus prefixos) e será rebaseado sobre o automático: não reformate nada. Só um build pesado por vez, por agente; se o sistema matar um build por memória, espere 2 minutos.

## Automático: fechar o lote

1. Compare o estado da cópia com cada decisão do brief e do plano 21, e relate o que falta; complete o que faltar, inclusive o acabamento da engrenagem. A sessão anterior deixou por fazer ao menos: o teste de uso do aplicativo e o CI final.
2. Teste de uso real por CDP, como no brief: simulada automática com A4, sem clique no meio; documento misto; junto de uma simulada manual em espelho e em divisão; dois temas, inglês, janela mínima, cores forçadas por emulação. Leia as imagens.
3. CI completo em um único log (`saidas/ci.log`), depois da última edição.

## Opções: terminar o lote

Faltam, segundo a sessão anterior, a interface e parte dos requisitos: confira item a item no brief. Inclui:

- **Tempo de impressão** por impressora no histórico: só o tempo em que a impressora trabalhou, do envio da passada à confirmação da fila, sem a espera da pessoa na recolocação, nem aviso de papel, atolamento ou fila parada. Cancelado ou com erro não entra na média. Velocidade em folhas por minuto, só com pelo menos 10 folhas medidas; sem dados, nada de número ou aviso.
- **Aviso de compensação** na confirmação, com duas ou mais impressoras e dados de todas, com diferença de 25% ou mais: "A Traseira imprime cerca de 2x mais rápido que a Gaveta. Na divisão, ela termina antes: deixe mais papel nela." Frases próprias para espelho e divisão. Só aviso: **não mude a repartição dos lotes**.
- **Decisão sobre "Imprimir de novo" no histórico: fica de fora.** O histórico guarda só o nome do arquivo, sem o caminho, por privacidade, e por isso não dá para achá-lo de novo com segurança.

Testes como no brief. Simuladas são instantâneas: relógio falso em Rust, tempo fingido nas capturas, dito no relatório. CI completo em um único log.

## Para os dois

Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Scripts em pasta própria, nunca no E: fora da cópia. Não toque em `E:\PlotS\` fora da sua cópia, nem em `plan/` e nem no remoto `origin` (sem push). Não abra `codex-session-*.md`. Encerre só seus PIDs. Limite de tempo em todo comando de cena; não espere processo em segundo plano sem prazo. Teclado só por CDP; janela em primeiro plano pode receber teclas do usuário, então repita a cena se aparecer caractere estranho. Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh 4b466f7 <pastas>` (falso positivo em texto de licença não conta). Termine com `git status` limpo e commits wip; apague `ui/node_modules` e `ui/dist` ao fim. Relatório em `%LOCALAPPDATA%/plots-build/<pasta do lote>/relatorio.md`, com a mesma mensagem na resposta final: o que já estava feito e o que você fez, item a item, testes com grupo de falha prévia, linha do CI, capturas com caminho, textos novos, decisões e o que não foi verificado.
