# Visão geral, arquitetura e fases

**Estado:** Em andamento · OK 1 · PARTIAL 2 · MISSING 0 · BUG 0 · N/A 1

## Objetivo

Reunir o que vale para o programa inteiro: propósito, stack, ordem das fases, riscos e decisões. Os detalhes de cada funcionalidade ficam nos outros arquivos.

## Plano de implementação

- [x] Workspace Cargo com `plots-core`, `plots-print`, `plots-pdf` e `plots-cli`
- [x] Comando de CI definido em `.claude/team.json`
- [ ] Adotar `tracing` para os logs de impressão, junto com o backend Windows
- [ ] Criar `src-tauri/` e `ui/` no início da interface
- [ ] Definir a licença do PlotS e gerar os avisos de terceiros antes de distribuir

## Pronto quando

Todos os planos de funcionalidade concluídos e o critério de cada fase atendido.

## Depende de

Nenhuma.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A046 | Erros com thiserror e logs com tracing | PARTIAL |
| PS-A082 | Workspace Cargo com plots-core, plots-print, plots-cli, src-tauri e ui | OK (353f54d) |
| PS-A083 | Um único `target/` (até 2 GB); artefatos de verificação removidos após o registro | PARTIAL |
| PS-A092 | Uso obrigatório da craft-team e do Graphify | N/A |

## Especificação

Nov 2, 2026 · @Kaio

### Visão geral

O **PlotS** faz qualquer impressora entregar o mesmo resultado físico no frente e verso manual: você imprime a frente, recoloca o maço seguindo uma instrução fixa e imprime o verso, sem pensar em qual impressora está usando.

Hoje há duas impressoras com comportamentos opostos:

| Impressora | Entrada do papel | Saída | Ordem natural da pilha |
| --- | --- | --- | --- |
| Jato de tinta | Traseira (vertical) | Frente, face para cima | Invertida — precisa imprimir da última para a primeira |
| Laser (toner) | Gaveta inferior | Topo, face para baixo | Correta — imprime da primeira para a última |

A solução não é mexer no driver de cada impressora. O PlotS calcula, por impressora, **em que ordem e com que rotação** enviar as páginas, a partir de um perfil medido uma única vez. O resultado: pilha final sempre de 1 a N, e um gesto de recolocação mostrado na tela para cada impressora.

Escopo da primeira versão: abrir um PDF, escolher uma ou mais impressoras, imprimir em lotes (ex.: 10 folhas), guiar a recolocação e imprimir o verso.

### Arquitetura e stack

App desktop em **Tauri 2**: backend em Rust e interface numa webview, onde roda o **pdf.js** — o mesmo leitor de PDF do Firefox. O pdf.js só exibe; toda a decisão de ordem fica no Rust.

&#91;embedded content: arquitetura · 3 camadas\]

A camada de backends é um `trait` com duas implementações, para o núcleo não saber em que sistema está.

| Peça | Escolha | Por quê |
| --- | --- | --- |
| Janela e IPC | `tauri` 2 | Rust nativo, webview leve, instalador pequeno |
| Visualização | `pdfjs-dist` no frontend | Leitor do Firefox, miniaturas prontas |
| Manipular PDF | `lopdf` | Reordenar, extrair, inserir página em branco, `/Rotate` |
| Rasterizar | `pdfium-render` | Para impressoras/sistemas que não aceitam PDF |
| Linux/macOS | `ipp` (IPP no CUPS) ou `lp` | Listar filas, enviar PDF, ler status do job |
| Windows | crate `windows` (winspool + GDI) | Listar impressoras, enviar páginas rasterizadas |
| Assíncrono | `tokio` | Um job por impressora em paralelo |
| Config | `serde` + `toml` + `directories` | Perfis em `%APPDATA%/plots/perfis.toml` |
| Erros e log | `thiserror`, `tracing` | Diagnóstico de falhas de impressão |

Frontend: React + TypeScript com CSS Modules e os tokens da norma de estilo (`theme.css`), ícones `lucide-react` e Radix para diálogos e menus. Nenhuma regra de negócio fica no frontend. No Windows, vale avaliar o SumatraPDF por linha de comando para imprimir PDF sem rasterizar.

### Fases de implementação

São seis fases; ao fim da Fase 3 o problema principal já está resolvido pela linha de comando, antes de qualquer interface. Como ainda não há impressora, as Fases 1 a 4 são feitas e validadas no modo simulação; só a calibração real fica para depois.

| Fase | Entrega | Pronto quando |
| --- | --- | --- |
| 1. Núcleo de ordenação | Crate `plots-core`: `plano_lote`, perfis, lotes, página em branco para total ímpar | Testes unitários cobrem lotes pares, ímpares, 1 folha e as 8 combinações de perfil |
| 2. PDF e calibração | Gerar `passada1.pdf`/`passada2.pdf` com `lopdf`; PDF de teste com números grandes e setas; backend Simulado com impressoras virtuais | Os PDFs gerados abrem no Firefox com a ordem e rotação certas |
| 3. Impressão por CLI | `plots imprimir doc.pdf --impressora X --lote 10` e `plots calibrar X`, com backend CUPS ou Windows | Simulação correta nas duas impressoras virtuais; com impressora real (quando houver), 3 lotes seguidos sem erro |
| 4. Interface Tauri + pdf.js | Visualizador, painel de impressoras, cartões de recolocação com figura | Fluxo completo sem terminal |
| 5. Várias impressoras e impressora virtual | Jobs paralelos, modos espelho/divisão, status do spooler; fila virtual "PlotS" que recebe o PDF de qualquer programa | Imprimir pelo Firefox na fila "PlotS" abre o app com o documento |
| 6. Acabamento | Instalador (`tauri bundle`), reimprimir lote, folha de controle, histórico | Instalado em máquina limpa e usado por outra pessoa sem ajuda |

Calibração (Fase 2) em detalhe: o PlotS imprime 2 folhas com páginas "1" a "4" e setas; o usuário recoloca do jeito que já faz; depois responde duas perguntas — "o verso da folha 1 é o 2?" e "o 2 está de cabeça para baixo?". As respostas preenchem `passada2_inversa` e `verso_girar_180`.

Estrutura do repositório: workspace Cargo com `plots-core` (lógica pura), `plots-print` (backends), `plots-cli` e `src-tauri` + `ui/` para o app.

### Riscos e perguntas em aberto

O maior risco técnico é a impressão no Windows; o maior risco de uso é o driver alterar a ordem por conta própria.

| Risco | Efeito | Mitigação |
| --- | --- | --- |
| Driver com "ordem inversa" ou "pular páginas em branco" ativo | Ordem errada; folha sem verso fica fora de sincronia | Calibração detecta; página "em branco" leva um ponto quase invisível |
| Windows não aceita PDF direto na maioria dos drivers | Precisa rasterizar | `pdfium-render` a 300 dpi, ou SumatraPDF por linha de comando |
| Impressora virtual é trabalhosa (backend CUPS; porta de impressora no Windows) | Atrasa a integração com o Firefox | Fica na Fase 5; antes disso, abrir o PDF no app |
| Papel puxado em dobro ou atolado no verso | Lote inteiro desalinhado | "Reimprimir lote" e lotes pequenos (10 folhas) |

Decisões já tomadas:

- Sistema: Windows. O backend Windows vem primeiro; CUPS fica para depois.
- O PlotS imprime sozinho; o pdf.js só exibe. A impressora virtual é uma porta de entrada opcional (Fase 5).
- Impressoras: as duas por USB no mesmo PC.
- Padrão: o gesto da jato de tinta, pares primeiro e ímpares no verso, repetido na laser.
- Papel: todos os formatos padrão (A4, A5, Carta, Ofício, Legal e outros).
- Seleção de impressoras: por documento.
