# Fluxo de impressão e recolocação

**Estado:** Em andamento · OK 0 · PARTIAL 2 · MISSING 4 · BUG 0 · N/A 0

## Objetivo

Guiar o usuário do primeiro envio ao verso, com um cartão por impressora.

## Plano de implementação

- [x] Estados do job e pausa guiada no terminal
- [ ] Cartão EM ANDAMENTO com estado, lote e botão do verso
- [ ] Frase fixa do gesto e figura por impressora
- [ ] Botão do verso liberado só com a confirmação do spooler
- [ ] "Simular verso" no modo simulação

## Pronto quando

Fluxo completo sem terminal, com impressoras avançando de forma independente.

## Depende de

`08-impressao-windows`, `11-lateral-impressoras`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A003 | `gesto_recolocar` com texto e figura para o usuário | MISSING |
| PS-A011 | Mostrar o gesto de recolocação e esperar "Imprimir verso" antes da passada 2 | PARTIAL |
| PS-A012 | Cartão com a frase fixa e a figura do gesto | MISSING |
| PS-A015 | "Imprimir frente" envia para cada impressora a ordem do seu perfil | MISSING |
| PS-A016 | Botão do verso libera só depois que o spooler confirma a frente | PARTIAL |
| PS-A035 | Cartão EM ANDAMENTO com "Simular verso" | MISSING |

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
