# Histórico de versões

O PlotS segue o [versionamento semântico](https://semver.org/lang/pt-BR/). As versões alfa ainda não têm teste em papel.

## 1.0.0-alpha.4

Gerada em 2026-11-05 (`97643ec`).

- PlotS portátil: com `PLOTS-PORTATIL.txt` ao lado do `PlotS.exe`, o `perfis.toml` fica nessa pasta e viaja com o programa; na primeira abertura ele traz o perfil de `%APPDATA%` sem apagar o original. O programa instalado continua em `%APPDATA%`.
- PlotS: entregue em portátil `.zip` com a pasta fixa `PlotS\`, sem instalador.

## 1.0.0-alpha.3

Gerada em 2026-11-05 (`c248cf8`).

- PlotSim: a lista de impressoras distingue "Aguardando verso" de "Verso recebido"; o rodapé e cada folha acompanham, e uma linha avisa que o gesto é aplicado em Recolocar.
- PlotSim: "Virar folha" passa a "Ver o outro lado", com o aviso de que só muda a vista, e o rótulo alterna para "Ver a frente" quando a folha está virada. Virar não renomeia mais as faces: a frente vista contra a luz continua sendo a frente.
- PlotS: o tema claro deixa de ser branco e passa a usar o cinza neutro dos painéis, com a mesma hierarquia de superfícies; o tema escuro continua e a página do PDF continua branca.
- PlotS: a confirmação "Imprimir os versos?" leva um triângulo de alerta amarelo antes do título, só nela, para o passo que gasta papel.
- PlotS: entregue em portátil `.zip` (10,36 MB), sem instalador; o zip leva o programa, a `pdfium.dll`, a licença e os avisos de terceiros.

## 1.0.0-alpha.2

Integrada em 2026-11-04 (`7b16a4f`): uso pela janela com várias impressoras, avisos no topo no lugar dos cartões e instalador sem as impressoras simuladas.

- Trabalho que sai da fila com falha registrada pelo Windows para aquela fila (evento 372 do registro de impressão) vira erro com Reimprimir, em qualquer impressora. Com o driver "Microsoft Print To PDF", as filas enviam uma de cada vez.
- O verso que espera na fila passa pela confirmação com o gesto, as páginas, o papel e as cópias do trabalho que já foi enviado, nunca os números que estiverem na aba; o botão do próximo trecho diz "Continuar frentes" e a dica da troca de papel permanece no rodapé.
- O aviso de trabalho anterior em erro também aparece na confirmação do verso.
- O registro do Windows só marca falha quando o nome do documento do evento é o do trabalho (se o Windows o oculta, valem a fila e o tempo).
- Papel misto mantém cada página no lado do documento: só o par de tamanhos diferentes se separa, com verso em branco para cada uma.
- Trabalho em erro não trava mais o botão de imprimir; o foco volta ao botão depois de confirmar pelo teclado.
- Painel de avisos: Cancelar e Reimprimir só na entrada mais recente do trabalho; avisos do topo com o documento e a fila; aviso fixo vencido sai do topo. A faixa não cobre a lateral em 640×420 nem esconde o nome da impressora.
- Cancelar que chega depois do fim diz "Nada para cancelar": nada foi cancelado, e o estado do trabalho não é inventado.
- Janela pequena: "Mais opções" abre o ajuste ao papel, a folha de controle e os grupos.
- O PlotSim confere por folha o que não começa da página 1 (trecho, cópias, papel misto), e uma frente nova abre um lote novo.
## 1.0.0-alpha

Primeira versão numerada, gerada em 2026-11-04.

- Impressão frente e verso manual em qualquer impressora, com os modelos jato de tinta e laser, e automática nas que imprimem as duas faces.
- Calibração de cada impressora na janela, com 2 folhas e figuras.
- Escolha de Frente ou Verso no rodapé, com quantidade de cópias e sem lotes na janela.
- Verso recusado se o arquivo mudou depois das frentes, na mesma aba.
- Folha de controle, ajuste ao papel, histórico e preferências do driver.
- PlotSim, ferramenta de teste com impressoras virtuais, fora do instalador.

**Problemas conhecidos:**

- A impressão pela janela com impressoras virtuais pode perder os versos ([relato do usuário](plan/25-avisos-no-topo.md), em investigação; corrigido na 1.0.0-alpha.2).
- Sem teste em papel, em impressora real nem execução do instalador.
- O instalador ainda mostra as impressoras simuladas.
