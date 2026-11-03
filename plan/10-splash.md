# Tela de abertura

**Estado:** Concluído · OK 1 · PARTIAL 0 · MISSING 0 · BUG 0 · N/A 0

## Objetivo

Preparar tudo antes de mostrar a janela principal.

## Plano de implementação

- [x] Imagem `ui/assets/splash.png` (`d4f3325`), gerada de `imgs/` por `src-tauri/icons/gerar-imagens.mjs`
- [x] Janelas `abertura` e `main` no Tauri, com carregamento em segundo plano
- [x] Mínimo de 800 ms, máximo de 10 s e aviso do que falhou
- [x] Fade de 220 ms, com troca direta em movimento reduzido

## Pronto quando

A janela principal só aparece pronta, e uma falha de carregamento vira aviso, não travamento.

## Depende de

`09-janela-e-abas`. A imagem e o ícone foram entregues pelo usuário em `imgs/` em 2026-11-03.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A020 | Splash 480×300, carregamento até 10 s, depois a janela principal | OK |

## Decisões e limitações

- O aviso flutuante mostra só o limite de 10 s e a falta da `pdfium.dll`. Falhas de documento, de filas e de perfis ficam no banner e na barra lateral, com a causa exata.
- A `pdfium.dll` é só procurada, nos mesmos lugares que a impressão usa, sem carregar. A verificação é feita no início, para o aviso sair mesmo quando o limite estoura.
- **Limitação conhecida:** em 640 × 420, com os dois avisos juntos, o toast cobre 23 px da parte de baixo de "Abrir PDF". O botão continua acessível e o toast pode ser dispensado.
- **Em aberto:** em 3 de cerca de 25 execuções a janela principal levou de 9 a 10 s, com a máquina carregada. Não reproduziu em 18 execuções com 6 apps em paralelo (1,3 a 2,6 s). O rastro de depuração diz qual condição faltou, se voltar a ocorrer. Causa não encontrada.
- Não verificado: build de release, abertura com PDF real por "Abrir com" e a duração do fade medida visualmente.

## Especificação

#### Tela de abertura (splash)

Ao abrir, o PlotS mostra primeiro uma janela pequena com uma imagem, enquanto prepara tudo; só depois a janela principal aparece, já pronta.

- **Janela**: 480 × 300 px, centralizada, sem bordas nem barra de título, fora da barra de tarefas.
- **Conteúdo**: a imagem do PlotS no centro, sobre `--bg`; embaixo, barra de progresso de 2 px (`--accent` sobre `--border`) e uma linha de estado em mono 11 px ("Carregando impressoras…").
- **Imagem**: arquivo `ui/assets/splash.png` em 960 × 600 px (o dobro, para telas de alta resolução). PNG é permitido aqui pela norma de estilo, que só aceita PNG no logotipo de carregamento.
- **O que carrega enquanto isso**: perfis em `%APPDATA%/plots/perfis.toml`, lista de impressoras (`EnumPrintersW`), a `pdfium.dll` e um PDF passado por "Abrir com", se houver.
- **Tempo**: fica pelo menos 800 ms (para não piscar) e no máximo 10 s; passado o limite, abre a janela principal mesmo assim e mostra um aviso do que falhou.
- **Movimento**: a janela principal entra com fade de 220 ms; com `prefers-reduced-motion`, troca direta.

No Tauri 2 são duas janelas no `tauri.conf.json`: `splash` (visível, `decorations: false`) e `main` (`visible: false`). O `setup` do Rust roda o carregamento em segundo plano; quando termina e o frontend avisa que montou, um comando fecha `splash` e mostra `main`.

Em aberto: você já tem a imagem do splash ou ela precisa ser criada?
