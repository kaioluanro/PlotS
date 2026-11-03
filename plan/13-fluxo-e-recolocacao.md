# Fluxo de impressão e recolocação

**Estado:** Em andamento · OK 5 · PARTIAL 1 · MISSING 0 · BUG 0 · N/A 0

## Objetivo

Guiar o usuário do primeiro envio ao verso, com um cartão por impressora.

## Plano de implementação

- [x] Estados do job e pausa guiada no terminal
- [x] Cartão EM ANDAMENTO com estado, lote e botão do verso
- [x] Frase do gesto por impressora
- [x] Figura do gesto por impressora (`ca10be1`)
- [x] Na gaveta simulada, dizer ao usuário para virar o maço final antes de ler, no CLI
- [x] Levar a frase do gesto ao cartão da interface; em fila real, o lado vem da calibração
- [x] Botão do verso liberado só com a confirmação do spooler
- [x] "Simular verso" no modo simulação

## Decisões

- **Frase do gesto (`256e8ce`):** segue a ordem física, porque quem lê em ordem e age a cada linha perderia o lote com a frase antiga: esperar a última folha, virar ou não virar, "Depois, sem mudar a ordem das folhas, coloque na entrada com a face impressa para {cima|baixo}.", clicar em "Imprimir verso". Vale para o cartão e para o CLI. Substitui a frase fixa da especificação abaixo.
- **Reimprimir** em fila real pede confirmação. Se o cancelamento falhar, a frente não é reenviada e o cartão manda conferir a fila do Windows.
- **Fechar com trabalho em andamento:** para de acompanhar sem apagar nada da fila.
- **Avisos (`ca10be1`):** papel acabado, atolado e sem resposta, com frase por passada; no verso, o texto proíbe pôr folhas em branco. Só a condição mais urgente na frase principal, e o texto do driver em linha própria. O aviso oferece "Reimprimir lote", e a confirmação manda deixar só folhas em branco na entrada.
- **Rolagem adiada:** uma linha fixa avisa que um cartão pede ação. Ao sumir sob o ponteiro, por qualquer causa, deixa um espaço inerte. Ao trocar de aba, o rodapé da outra aba pode empurrar esse espaço para fora do ponteiro; aceito, porque a própria pessoa provocou a mudança.
- **Pendente:** `src-tauri` usa `plots_print` pelo reexport de `plots-windows`; `leitura_pdf.rs` repete o limite de leitura de `documento.rs`.
- **Não verificado:** quais bits cada driver liga em atolamento e falta de papel (só backend falso e cartões do gancho); reimprimir confirmado em fila real; impressora real; o que o spooler faz com trabalho parcial quando a janela fecha no meio do envio; cancelamento com spooler real (só backend falso); fechar a aba durante a preparação no app (coberto por teste).

## Pronto quando

Fluxo completo sem terminal, com impressoras avançando de forma independente.

## Depende de

`08-impressao-windows`, `11-lateral-impressoras`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A003 | `gesto_recolocar` com texto e figura para o usuário | OK |
| PS-A011 | Mostrar o gesto de recolocação e esperar "Imprimir verso" antes da passada 2 | OK |
| PS-A012 | Cartão com a frase fixa e a figura do gesto | OK |
| PS-A015 | "Imprimir frente" envia para cada impressora a ordem do seu perfil | PARTIAL |
| PS-A016 | Botão do verso libera só depois que o spooler confirma a frente | OK |
| PS-A035 | Cartão EM ANDAMENTO com "Simular verso" | OK |

## Especificação

**Gesto único em todas as impressoras**: o padrão é o da jato de tinta, que você já faz de cor. Primeiro saem as pares; depois o maço volta inteiro, sem reorganizar folha nenhuma, e as ímpares são impressas no verso. Na laser o gesto físico muda só no lugar onde o maço entra (gaveta em vez de traseira); a ordem e a rotação das ímpares são compensadas pelo PlotS. O cartão sempre diz a mesma frase: "Pegue o maço como saiu, não embaralhe, e coloque na entrada com a face impressa para …", completando o lado conforme a impressora.
### Fluxo do usuário

O usuário só decide o documento, as impressoras e o tamanho do lote; o resto é guiado.

1. Abrir o PDF no PlotS (arrastar, "Abrir com" ou imprimir de outro programa na impressora virtual — Fase 5).
2. Ver as páginas no visualizador e escolher o intervalo.
3. Marcar uma ou mais impressoras e o tamanho do lote (padrão: 10 folhas).
4. Clicar em "Imprimir frente". Cada impressora recebe a sua ordem.
5. Seguir o cartão de cada impressora: uma figura mostra como pegar o maço e onde recolocar.
6. Clicar em "Imprimir verso" naquele cartão. Impressoras diferentes avançam de forma independente.
7. Repetir para o próximo lote até o fim do documento.

Regras de segurança no fluxo:

- O botão do verso só libera quando o spooler confirma que a frente terminou.
- Folha de controle opcional: um número discreto no canto do verso para conferir se o maço entrou certo.
- Se o usuário errar, "Reimprimir lote" refaz só aquele lote.
