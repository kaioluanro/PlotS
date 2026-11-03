# plotsim-janela: a janela do PlotSim (fase 6)

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia `PlotSim/AGENTS.md` (seções "Simulação física e visualização" e "Interface"), `plan/23-plotsim.md`, as regras comuns de `plan/briefs/coordenacao-paralela-2.md` e a norma `E:\PlotS\plan\estilo-ui-ux.md`, com a skill `ui-design-craft` carregada. Build e saídas em `%LOCALAPPDATA%/plots-build/plotsim-janela`, porta CDP 9350. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`.

**Arquivos:** só `PlotSim/app/` (novo, Tauri 2 + React, CSS Modules) e a linha do membro no `Cargo.toml`. O nome do crate começa com `plotsim-`, para o gerador de avisos do PlotS o excluir. Outro agente faz a suíte em `PlotSim/plotsim-suite/` e mexe em `plotsim-core`: não mexa nele; se precisar, relate.

**Reaproveite do PlotS, sem mudar o PlotS:** tokens e CSS da interface, barra de título, diálogos, botões, a inicialização do Tauri, `plotsim-core` para filas e recepção, e o PDFium. Copie só o que não puder compartilhar por importação, e diga qual. **Nunca** o planejador, o simulador nem a conferência do PlotS: a conferência é a do `plotsim-physics`.

## Escopo

Uma janela só, em pt-BR e en, nos dois temas:

- **Lista à esquerda:** impressoras virtuais com estado em ponto e texto ("Ociosa", "Recebendo", "Aguardando verso"), modo e "Nova impressora virtual" (nome e modo). Remover pede confirmação e diz o que apaga.
- **Centro:** a pilha de saída da selecionada, folha por folha, como na especificação: cada página recebida vira folha; na recolocação, o que já foi impresso passa ao verso, espelhado e a 30% de opacidade, e de cabeça para baixo se o gesto girar; a passada 2 cai por cima, com opacidade cheia. Por folha, certa ou errada, com o motivo. Sem marca visual legível, a folha diz isso, sem palpite.
- **Rodapé:** "3 de 3 folhas corretas", "Virar folha", "Recolocar", "Limpar bandeja".
- **Modo manual:** ao "Recolocar", a pessoa escolhe o gesto na tela, lendo o cartão no PlotS. Isso testa se a instrução é clara. Sem o canal do pipe por enquanto: deixe o ponto de entrada pronto.
- **Estados:** vazio, recebendo, erro de fila, sem permissão para criar fila, PDF ilegível, janela mínima. Nunca trava à espera do spooler.
- Avise onde os PDFs recebidos ficam, e ofereça "Limpar histórico". Sem telemetria.

## Limites

Só filas `PlotSim — `; remova as de teste ao fim. Sem elevação: se negado, pare e relate. Sem impressora real. Capturas só por CDP. Sem instalador. Sem dependência npm nova além das do PlotS: mesma versão. Se a leitura da marca visual ainda não existir, use os números pelo que `plotsim-core` oferece e deixe o ponto de encaixe.

## Testes e final

Vitest da lógica da tela. App real por CDP: criar uma fila, enviar pelo `plots imprimir` em uma fila virtual, ver as duas passadas, virar folha, limpar, remover; dois temas, inglês, janela mínima, cores forçadas por emulação. Leia cada imagem. CI completo em um único log, com `ui/` do PlotS e a do app, autoverificação, `git status` limpo com wip, apague `node_modules` e `dist`. Relatório em `%LOCALAPPDATA%/plots-build/plotsim-janela/relatorio.md`, com a mesma mensagem na resposta final: item a item, o que foi reaproveitado, testes com grupo de falha prévia, capturas com caminho, contraste, textos novos, o que restou no Windows e o que não foi verificado. Limite de tempo em todo comando de cena.
