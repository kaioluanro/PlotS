# calibracao-na-janela: calibrar cada impressora pelo PlotS, sem terminal e sem ajuda

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia `plan/04-calibracao.md`, `plan/19-modelos-de-impressora.md`, `plan/23-plotsim.md` e as regras comuns de `plan/briefs/coordenacao-paralela-2.md`. Build em `%LOCALAPPDATA%/plots-build/calibracao-na-janela`, porta CDP 9349. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`.

**Pedido do usuário:** ele vai ter a impressora na mão, e eu não. Com 2 folhas, ele precisa deixar cada impressora certa sozinho, informando o que viu.

**Outros agentes ao mesmo tempo:** `laser-verso-girado` mexe em `ModeloImpressora::Laser`, na mecânica do simulador e na frase da laser, e entra antes; filas do PlotSim mexem em `PlotSim/plotsim-core/`, `plots-windows/src/trabalho.rs` e `scripts/`. Ponha o seu trabalho em arquivos novos (`src-tauri/src/calibracao.rs`, `ui/src/calibracao*.ts`, `ui/src/components/CalibracaoDialog.*`), com uma linha nos compartilhados. Um rebase virá depois.

## Hoje

`CalibrarDialog` só mostra um comando para o terminal; o CLI gera um PDF, não envia nada, faz duas perguntas (verso certo, verso invertido) e grava o perfil por `ajustar_perfil_por_calibracao`.

## Comportamento

O "Calibrar…" da linha de cada fila real abre uma calibração guiada na própria janela, em passos curtos, que substitui o diálogo do terminal:

1. **Preparar:** coloque 2 folhas em branco. Parte do modelo escolhido na linha, ou do jato, se não houver.
2. **Imprimir a frente pelo PlotS**, em uma fila real, com confirmação: envia as páginas do teste de 2 folhas, com número grande e seta. Reaproveite o envio existente. Sem depender de `Pedido` de documento do usuário: o PDF é gerado em pasta temporária e apagado ao fim.
3. **Recolocar:** mostra a frase e a figura do gesto daquele perfil; "Imprimir verso".
4. **Dizer o que saiu**, com perguntas de resposta única, escritas para quem tem a folha na mão, com desenho quando ajudar:
   - imprimiu no lado em branco, ou por cima do que já estava impresso?
   - no verso da folha com o 1 está o 2?
   - virando a folha como página de livro, as duas setas apontam para o mesmo lado?
   - com as folhas como saíram, para ler em ordem, o 1 fica por cima sem virar o maço? E precisa virar?
5. **Ajustar e conferir:** o PlotS calcula o perfil e mostra em palavras o que mudou. Oferece imprimir de novo com o perfil novo. Só encerra quando as respostas dizem que está certo, ou quando a pessoa desiste; desistir não grava nada. No máximo 3 rodadas; depois disso, diz que esta impressora não se encaixou e o que escolher.
6. **Gravar:** perfil por fila, com gravação atômica e a trava `GRAVACAO_DE_PERFIS`. A linha mostra "Calibrada". O perfil guarda também como recolocar (lado na entrada, virada, borda) e se o driver inverte, vindos das respostas: "imprimiu por cima" muda o lado; maço invertido muda a ordem ou liga a compensação de driver. Calibrada, a fila não usa mais o modelo pronto, e escolher um modelo pede confirmação, como já é hoje.

**Mesmo fluxo nas simuladas**, sem passar por fila, e a fila de PDF serve de teste, abrindo os arquivos. Automático: a calibração confere a virada pela borda longa e a ordem, numa rodada só, sem gesto.

## Prova de que resolve

Esse é o critério principal. Teste Rust com `plotsim-physics` como julgador, em `[dev-dependencies]` ou em teste no `PlotSim/plotsim-physics/tests/`: para **cada** mecânica física do modelo (entrada, saída, face, curva ou reta, driver que inverte), partindo de **qualquer** perfil inicial, responda às perguntas com o que o modelo físico produziu e aplique a calibração. Depois de no máximo 3 rodadas, o maço final tem de sair correto. Se houver mecânica que não converge, ou perguntas que não distinguem dois casos, relate: é o resultado mais importante deste lote. O que o modelo não conseguir distinguir, a pessoa tem de ver numa pergunta.

Não reaproveite a lógica de conferência do PlotS no julgamento.

## Limites

Nada a impressora real em teste: só simuladas e "Microsoft Print to PDF". Nunca toque em `%APPDATA%` real. A janela do driver não abre aqui. O CLI continua funcionando: só ajuste se o núcleo mudar. Frases em pt-BR e en, prefixo `calibracao.*`; nada de marca. A janela mínima de 640x420 tem de funcionar, com botões sempre visíveis.

## Testes e final

Vitest do fluxo e das respostas. App real por CDP: simuladas, jato e laser, com perfil errado de propósito, até dar certo; dois temas, inglês, janela mínima, cores forçadas por emulação. Leia cada imagem. CI completo em um único log, incluindo `ui/`, autoverificação, `git status` limpo com wip, apague `ui/node_modules` e `ui/dist`. Relatório em `%LOCALAPPDATA%/plots-build/calibracao-na-janela/relatorio.md`, com a mesma mensagem na resposta final: item a item, tabela de convergência, casos que não convergem, testes com grupo de falha prévia, capturas, textos novos e o que não foi verificado. Limite de tempo em todo comando de cena.
