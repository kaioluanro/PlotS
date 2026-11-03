# calibracao-na-janela: terceira rodada de correções

Responda em português brasileiro. Trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-a3ae2fc018eebc568`, commit `fbce7e1`. Não crie outro worktree. Build, saídas e cenas em `%LOCALAPPDATA%/plots-build/calibracao-na-janela`, porta CDP 9349. As cenas já prontas estão em `cenas/` (`fila2.mjs`, `recusa.mjs` e as demais). Rode `npm ci`, porque `ui/node_modules` foi apagado. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`. Regras comuns: `plan/briefs/coordenacao-paralela-2.md`. Limite de tempo em todo comando; não espere processo em segundo plano sem prazo.

Os textos abaixo são definitivos. Não os reescreva.

## 1. Botões travados durante chamada em andamento

`ui/src/components/CalibracaoDialog.tsx`: `criarSaidas.gravar`, `comecarRodada`, `imprimirFrente`. Hoje os botões não travam durante a chamada, e isso causa:

- A: na tela "certo", Gravar e logo Desistir: a gravação conclui, a tela disse que não gravaria, `aoFechar(false)` não relê a lista.
- B: Desistir com `enviarRodada` em andamento, passo ainda "confirmar": `temPassadaNaFila` dá falso, nada é cancelado, e o trabalho entra na fila depois de a janela fechar.
- C: clique duplo em Imprimir: o segundo recebe `ocupada`, e o alerta pode aparecer sobre o passo "frente".

Correção: estado `enviando` ou `gravando`, com ref para valer no mesmo clique, que desabilita os botões principais e também Desistir e Esc enquanto durar a chamada. Se a gravação concluir, `aoFechar` recebe `true`. Teste de regressão de cada cenário, com falha prévia registrada em `saidas/fail-before.log`.

## 2. Tela "certo"

Ordem: título; "Grave para usar este perfil nesta impressora. Confira depois com um documento curto." (o inglês já existente, se equivalente); 16 px de espaço; um grupo com recuo de 12 px. O grupo abre com "Depois de gravar, ao imprimir o verso:" / "After saving, when printing the back:", em 13 px peso 600. Abaixo, em lista, as frases do gesto e "Para ler…". Na simulada: "Com este perfil, ao imprimir o verso:" / "With this profile, when printing the back:", com "nada é gravado" no lugar de "Grave…". Isso substitui `certo.daquiEmDiante`.

**Conferir respostas, sem imprimir de novo:** na mesma tela, um resumo curto das respostas dadas nesta rodada e o botão "Corrigir uma resposta" / "Change an answer", que volta à última pergunta e mantém as respostas anteriores (`estado.respostas`). As folhas já estão na mão: nada é reimpresso nem conta rodada nova. Se a resposta corrigida levar a outro resultado, o fluxo segue como em qualquer rodada.

## 3. Aviso de driver trocado

`ui/src/components/BlocoAjuste.tsx`. Use a classe `.avisoOpcao` existente, com `CircleAlert` de 14 px e 4 px acima. Ícone e texto em `--fg`, nunca em `#d97706`. Texto: "Esta opção mudou depois da calibração. Se a ordem sair errada, calibre de novo." / "This option changed after calibration. If the order comes out wrong, calibrate again."

O aviso sobrevive a desmarcar e remarcar a impressora: compare com o valor do momento da calibração, não com `useState`. Se o registro da calibração em `plots-perfis/src/calibracoes.rs` não guardar esse valor, acrescente campo opcional: arquivo antigo continua abrindo e, sem o valor, não há aviso. Teste de arquivo antigo.

## 4. Confirmação de desistir

Ao abrir, o foco vai para "Voltar". Esc continua voltando. Texto: em peso 600, só "Sair sem gravar?" / "Leave without saving?"; em peso normal, "O perfil da impressora fica como estava. Para calibrar depois, será preciso imprimir o teste de novo." / "The printer's profile stays as it was. To calibrate later, you will need to print the test again."

## 5. Perguntas

`calibracao.pergunta.leitura`: "Sem mudar a ordem, olhe o número de cima. Depois vire o maço inteiro e olhe de novo. Quando o 1 aparece por cima?" / "Without changing the order, look at the top number. Then flip the whole stack and look again. When is the 1 on top?"

Em `lado`, `leitura` e `setas`, separe instrução (peso normal) e pergunta (600), com 4 px entre elas, em duas chaves de texto. Só a pergunta em negrito.

## 6. Recusa

Esconda `.progresso` quando `naoServe`. Confirme no código que `filaNaoServe` só ocorre em fila de arquivo, e só então use: "Esta impressora salva em arquivo, sem papel. Escolha uma impressora de papel." / "This printer saves to a file, without paper. Choose a paper printer." Se ocorrer em outro caso, mantenha o texto atual e relate. Atualize a exceção em `textos-lateral.test.ts`: se a palavra "fila" saiu, ela não é mais necessária.

## 7. Miúdos

- `aoContinuar`: troque `lista[lista.indexOf(p)+1]` por uma guarda para `indexOf` igual a -1.
- Falha do julgamento ganha frase própria, em vez de `gravarFalhou`: "Não foi possível conferir as respostas. Tente de novo." / "Could not check the answers. Try again."
- Em `confirmar`, tire "Cada rodada gasta 2 folhas"; o total fica só no preparo.
- Laser: reescreva sem "Depois" repetido e sem "Depois" após a negação. Não mude o gesto.

## Final

Não mude o modelo físico nem as regras de convergência. Capturas novas, por CDP, em `saidas/shots`, nos dois temas, em inglês, na janela mínima e em cores forçadas por emulação: tela "certo" real e simulada, "Corrigir uma resposta", desistir com o foco e a volta para "certo", driver trocado (com desmarcar e remarcar), perguntas separadas, recusa, botões travados durante a chamada, e os estados `ocupada`, `falha_envio`, `falha_perfil`, `falha_ao_gravar` e `verso_falhou`, com rodapé, forçados pelo gancho de verificação (0 ocorrências de `__plotsE2e` no `dist`). Leia cada imagem. Sem impressora real; só simuladas e "Microsoft Print to PDF". Testes nunca tocam `%APPDATA%`.

CI completo em um único log `saidas/ci.log`, incluindo `ui/` e `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR`, terminando com `CI_EXIT <código>`. Autoverificação com a base `90becf7`, `git status` limpo com novo wip, apague `ui/node_modules` e `ui/dist`. Relatório em `%LOCALAPPDATA%/plots-build/calibracao-na-janela/relatorio-rodada3.md`, curto, com a mesma mensagem na resposta final, mesmo que falte algo: item a item, testes com grupo de falha prévia, linha do CI, capturas com caminho e o que não foi verificado.
