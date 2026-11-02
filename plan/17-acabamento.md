# Acabamento e instalador

**Estado:** Em andamento · OK 0 · PARTIAL 1 · MISSING 2 · BUG 0 · N/A 0

## Objetivo

Deixar o programa pronto para outra pessoa usar.

## Plano de implementação

- [x] Reiniciar lote no controlador de job
- [ ] "Reimprimir lote" e histórico na interface
- [ ] Folha de controle opcional
- [ ] Instalador com `tauri bundle`
- [ ] Avisos de terceiros e busca do nome no INPI e USPTO

## Pronto quando

Instalado em máquina limpa e usado por outra pessoa sem ajuda.

## Depende de

Todas as funcionalidades anteriores.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A017 | Folha de controle opcional no canto do verso | MISSING |
| PS-A018 | "Reimprimir lote" refaz só aquele lote | PARTIAL |
| PS-A081 | Instalador (tauri bundle), reimprimir lote, folha de controle, histórico | MISSING |

## Especificação

- Se o usuário errar, "Reimprimir lote" refaz só aquele lote.
| Fase | Entrega | Pronto quando |
| --- | --- | --- |
| 6. Acabamento | Instalador (`tauri bundle`), reimprimir lote, folha de controle, histórico | Instalado em máquina limpa e usado por outra pessoa sem ajuda |
