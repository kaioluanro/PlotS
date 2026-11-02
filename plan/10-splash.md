# Tela de abertura

**Estado:** Não iniciado · OK 0 · PARTIAL 0 · MISSING 1 · BUG 0 · N/A 0

## Objetivo

Preparar tudo antes de mostrar a janela principal.

## Plano de implementação

- [ ] Definir a imagem `ui/assets/splash.png` (pendência abaixo)
- [ ] Janelas `splash` e `main` no Tauri, com carregamento em segundo plano
- [ ] Mínimo de 800 ms, máximo de 10 s e aviso do que falhou
- [ ] Fade de 220 ms, com troca direta em movimento reduzido

## Pronto quando

A janela principal só aparece pronta, e uma falha de carregamento vira aviso, não travamento.

## Depende de

`09-janela-e-abas`. **Pendência do usuário:** a imagem já existe ou precisa ser criada?

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A020 | Splash 480×300, carregamento até 10 s, depois a janela principal | MISSING |

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
