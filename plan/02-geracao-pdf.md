# Geração das passadas em PDF

**Estado:** Em andamento · OK 5 · PARTIAL 2 · MISSING 0 · BUG 0 · N/A 0

## Objetivo

Transformar o plano de cada lote em PDFs com a ordem e a rotação certas, sem perder conteúdo nem sobrescrever arquivos.

## Plano de implementação

- [x] `plots-pdf` gera as passadas com `lopdf`, com rotação e página em branco
- [x] `plots gerar-pdf`, sem sobrescrever e removendo só o que criou em caso de falha
- [x] Recusa de PDF criptografado e de formulários que exigem regeneração
- [x] Validar ordem e rotação no Firefox: conferido pelo usuário em 2026-11-02
- [x] Aceitar PDFs comuns: assinatura aplicada vira carimbo com a mesma aparência, com aviso de que a cópia não contém a assinatura; 64 de 64 PDFs reais aceitos, com imagens iguais no PyMuPDF (25557c9)
- [x] Tirar `/ID` e `SigFlags` do PDF gerado
- [x] Tirar do PDF gerado recursos de páginas de fora do lote, com teste; na dúvida, mantém tudo
- [x] Limitar os bytes por fluxo de conteúdo para conter PDF malicioso
- [x] Medir o papel pela caixa que o leitor mostra (MediaBox ∩ CropBox)
- [ ] Conferir PDFs reais em leitor real e em impressora; fios e estrutura marcada só têm fixture sintética

## Limites escolhidos

- 64 MiB por fluxo descomprimido, 256 MiB por saída nas folhas finais e por página na análise das passadas, 16 níveis de formulários (acima disso, recusa com motivo).
- Na análise de recursos: 65.536 nomes por varredura, 1 GiB e 50 milhões de visitas por saída. Passou disso, só deixa de retirar recursos, sem recusar. Esses tetos só foram exercitados com valores injetados em teste.
- Resultado nos 64 PDFs reais a 30 dpi: 64 aceitos, nenhum divergente. Nenhum deles tem `/Resources` compartilhado, então a retirada de recursos só foi exercitada por fixtures sintéticas.
- **Riscos aceitos:** Flate truncado devolve só a parte lida, sem sinal, e os nomes depois do corte não são vistos. Fluxo de objetos acima do limite falha como "objeto não encontrado", sem citar o limite. `CropBox` fora da `MediaBox` recusa já na medição.

## Pronto quando

Os PDFs gerados abrem no Firefox com a ordem e a rotação certas.

## Depende de

`01-nucleo-ordenacao`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A010 | Gerar `passada1.pdf` e `passada2.pdf` temporários com lopdf | PARTIAL |
| PS-A036 | Exportar `passada1.pdf` e `passada2.pdf` para abrir no Firefox | PARTIAL |
| PS-A041 | lopdf para reordenar, extrair, inserir página em branco e `/Rotate` | OK |
| PS-A072 | Os PDFs gerados abrem no Firefox com ordem e rotação certas | OK |
| PS-A084 | Não sobrescrever arquivos existentes; em falha, remover só o que foi criado | OK |
| PS-A085 | PDF criptografado e inválido são recusados com mensagem | OK |
| PS-A087 | Página em branco com ponto discreto para o driver não descartar a folha | OK |

## Especificação

4. Gerar dois PDFs temporários (`passada1.pdf`, `passada2.pdf`) e enviar o primeiro.
| Peça | Escolha | Por quê |
| --- | --- | --- |
| Manipular PDF | `lopdf` | Reordenar, extrair, inserir página em branco, `/Rotate` |
| Fase | Entrega | Pronto quando |
| --- | --- | --- |
| 2. PDF e calibração | Gerar `passada1.pdf`/`passada2.pdf` com `lopdf`; PDF de teste com números grandes e setas; backend Simulado com impressoras virtuais | Os PDFs gerados abrem no Firefox com a ordem e rotação certas |
