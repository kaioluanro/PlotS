# Perfis salvos

**Estado:** Não iniciado · OK 0 · PARTIAL 0 · MISSING 2 · BUG 0 · N/A 0

## Objetivo

Guardar por fila de impressão o que foi calibrado, para não medir de novo.

## Plano de implementação

- [x] Estrutura `PerfilImpressora` no núcleo
- [ ] Ler e gravar `%APPDATA%/plots/perfis.toml` com `serde`, `toml` e `directories`
- [ ] Chave pelo nome da fila, com apelido
- [ ] Arquivo inválido gera erro claro, sem apagar o que existe; gravação atômica

## Pronto quando

Um perfil salvo é reaberto igual, e arquivo corrompido nunca derruba o programa.

## Depende de

`01-nucleo-ordenacao`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A045 | Config com serde, toml e directories; perfis em `%APPDATA%/plots/perfis.toml` | MISSING |
| PS-A063 | Perfil por fila: chave com o nome da fila e apelido amigável | MISSING |

## Especificação

Cada impressora vira um perfil com quatro parâmetros, todos medidos por um assistente de calibração (seção Fases, etapa 2). Com eles, um único algoritmo gera a ordem de envio para qualquer impressora.

| Parâmetro | O que significa | Jato de tinta (esperado) | Laser (esperado) |
| --- | --- | --- | --- |
| `passada1_inversa` | Enviar a frente da última para a primeira | sim | não |
| `passada2_inversa` | Enviar o verso da última para a primeira | a calibrar | a calibrar |
| `verso_girar_180` | Girar as páginas do verso 180° | a calibrar | a calibrar |
| `gesto_recolocar` | Como devolver o maço: virar ou não, qual borda entra primeiro | texto + figura | texto + figura |

| Peça | Escolha | Por quê |
| --- | --- | --- |
| Config | `serde` + `toml` + `directories` | Perfis em `%APPDATA%/plots/perfis.toml` |
- **Perfil por fila**: chave = nome da fila no sistema, com apelido amigável ("Jato — escritório").
