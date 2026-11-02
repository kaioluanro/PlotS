# Perfis salvos

**Estado:** Em andamento · OK 0 · PARTIAL 2 · MISSING 0 · BUG 0 · N/A 0

## Objetivo

Guardar por fila de impressão o que foi calibrado, para não medir de novo.

## Plano de implementação

- [x] Estrutura `PerfilImpressora` no núcleo
- [x] Ler e gravar `%APPDATA%/plots/perfis.toml` com `serde`, `toml` e `directories`, no crate `plots-perfis`
- [x] Chave pelo nome da fila, com apelido
- [x] Arquivo inválido gera erro claro, sem apagar o que existe; gravação atômica
- [ ] Usar os perfis em `plots calibrar` e `plots imprimir`

## Pronto quando

Um perfil salvo é reaberto igual, e arquivo corrompido nunca derruba o programa.

## Depende de

`01-nucleo-ordenacao`.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A045 | Config com serde, toml e directories; perfis em `%APPDATA%/plots/perfis.toml` | PARTIAL (72030fb) |
| PS-A063 | Perfil por fila: chave com o nome da fila e apelido amigável | PARTIAL (72030fb) |

## Licenças de terceiros

`option-ext` 0.2.0, usado por `directories` e `dirs-sys`, é MPL-2.0 e não permissivo. Usado sem alterações, por decisão do usuário em 2026-11-02. Ao distribuir o binário: incluir o texto da MPL 2.0 nos avisos de terceiros e informar a origem do código-fonte, https://github.com/soc/option-ext.

## Cuidados ao levar para a interface

- Os erros de TOML vêm em inglês e reproduzem a linha do arquivo, que pode conter nome de fila ou apelido. Traduzir antes de mostrar, e não gravar esse detalhe em log nem enviá-lo para fora.
- A mensagem "1 MB" se refere a 1.048.576 bytes.

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
