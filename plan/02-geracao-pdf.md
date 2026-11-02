# Geração das passadas em PDF

**Estado:** Em andamento · OK 4 · PARTIAL 3 · MISSING 0 · BUG 0 · N/A 0

## Objetivo

Transformar o plano de cada lote em PDFs com a ordem e a rotação certas, sem perder conteúdo nem sobrescrever arquivos.

## Plano de implementação

- [x] `plots-pdf` gera as passadas com `lopdf`, com rotação e página em branco
- [x] `plots gerar-pdf`, sem sobrescrever e removendo só o que criou em caso de falha
- [x] Recusa de PDF criptografado e de formulários que exigem regeneração
- [ ] Validar ordem e rotação no Firefox (o modo headless não gerou imagem; aguarda escolha do usuário)
- [ ] Tirar do PDF gerado recursos de páginas de fora do lote e o `/ID` do documento original, com teste
- [ ] Limitar os bytes por fluxo de conteúdo para conter PDF malicioso
- [ ] Exercitar com PDFs reais de formulários e anotações

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
| PS-A072 | Os PDFs gerados abrem no Firefox com ordem e rotação certas | PARTIAL |
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
