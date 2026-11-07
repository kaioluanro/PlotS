# Auditoria do PlotS contra AGENTS.md

Data: 2026-11-02. A especificação foi dividida depois em `plan/`, então as linhas de `AGENTS.md` citadas abaixo são da versão anterior. Escopo: especificação em `AGENTS.md` (seções de Fases, Modo simulação, Perfis, Papel, Múltiplas impressoras, Windows e Interface). Nenhum código de produto foi alterado. Nenhum commit. `Cargo.toml`, `Cargo.lock`, `AGENTS.md`, `CLAUDE.md` e `estilo-ui-ux.md` mantêm os mesmos hashes (md5 conferido antes e depois). **Atenção:** o `.gitignore` foi reescrito às 13:56:41, durante a auditoria, por outro processo ou usuário (este agente não o editou). O conteúdo novo acrescenta `/.claude/`, `/.codex/`, `/estilo-ui-ux.md`, `/plan/`, `/AUDIT-*.md` e `/STATUS-MANUAIS.md`, e por isso o `git status` mostra só `AGENTS.md` e `CLAUDE.md`. Não foi revertido; decisão do usuário.

Contagem: **OK 75 · PARTIAL 28 · MISSING 2 · BUG 0 · N/A 4** (109 requisitos)

Reauditoria de 2026-11-04, sobre o `master` `b0c7302`. Catorze linhas mudaram de status e seis tiveram a evidência atualizada. As contagens foram recalculadas a partir da tabela: o total anterior (38/33/20/2) não batia com as linhas, que já somavam 52/32/6/3. Não rodei o app nem fiz teste em papel nesta reauditoria; as mudanças vêm de código, testes e commits. Mudanças de status:
- PS-A014: OK para N/A (lote saiu da janela, `plan/24`).
- PS-A017, PS-A060: MISSING para OK (`1ad3307`).
- PS-A022, PS-A023, PS-A026, PS-A030, PS-A034, PS-A045, PS-A062, PS-A077, PS-A079: PARTIAL para OK.
- PS-A051: MISSING para PARTIAL (a opção é marcada pela pessoa, sem detecção no driver).
- PS-A078: MISSING para PARTIAL (exige impressora real e papel, pela regra da auditoria).

Segunda reauditoria, em 2026-11-04 sobre o `7b16a4f`: acrescentei PS-A094 a PS-A100 para o que entrou na alpha.2 (avisos no topo e painel, confirmação do verso com o trabalho enviado, envio simultâneo e registro de impressão, papel misto mantendo o lado, janela mínima). Seis são OK e um é PARTIAL (os três controles de "Mais opções" não cabem juntos em 640×420).

Terceira reauditoria, em 2026-11-05 sobre o `c248cf8`: acrescentei PS-A101 e PS-A102 para o que entrou na alpha.3 no PlotS (tema cinza neutro no lugar do branco e alerta no título da confirmação do verso). Os dois são OK; o CI, as capturas e as três revisões estão em `%LOCALAPPDATA%\plots-build\cinza-indesign`.

Quarta reauditoria, em 2026-11-05 sobre o `97643ec`: acrescentei PS-A103 para o perfil na versão portátil (ao lado do executável, com migração única de `%APPDATA%` e o instalado sem marcador intacto). É OK; o CI e o uso real estão em `%LOCALAPPDATA%\plots-build\perfil-portatil`.

Quinta reauditoria, em 2026-11-06 sobre o `559a9c8`: acrescentei PS-A104 a PS-A106 para a correção da folha em branco (frente = ímpares, verso = pares, 1 página sem verso), o desenho GDI por bitmap compatível + `StretchBlt` e a chave de boca com as opções do driver. PS-A104 é OK; PS-A105 e PS-A106 são PARTIAL pelas provas em impressora/driver de fabricante que faltam.

Sexta reauditoria, em 2026-11-06 sobre `6a972c6`: acrescentei PS-A107 para a cor das imagens impressas. O pdfium-render entregava RGBA por padrão e o DIB do GDI lia como BGRA; laranja e amarelo saíam azuis. É OK, com teste de bytes e prova na fila de PDF.

Sétima reauditoria, em 2026-11-07 sobre `d6ae384`: acrescentei PS-A108 para a abertura e a impressão de imagens. PNG, JPEG, BMP, GIF, WebP e TIFF multipágina viram um PDF derivado em cache e passam pelo mesmo caminho de prévia e impressão do PDF; o PDF existente não muda. É OK com a fila de PDF; falta a impressora real e os demais formatos enviados à fila.

Oitava reauditoria, em 2026-11-07 sobre `be877c1`: acrescentei PS-A109 para o motor de tinta e os perfis. A seleção de páginas do painel, a cobertura CMYK, o custo em mL/R$, os perfis com preço por mL e o `plotinfo.toml` estavam prontos e testados; o painel e a análise com mutool ainda faltavam.

Nona reauditoria, em 2026-11-07 sobre `0f66aa7`: PS-A109 passa de PARTIAL para OK. A análise de tinta entrou em `8a9866f` (PDFium, cache, eventos e cancelamento), o mutool/CSV em `eab8d30` (processo separado, CMYK, fallback com aviso, CSV atômico), o painel em `15bc485` e o ajuste final em `0f66aa7` (aviso traduzível e lotes de até 256 MiB/32 páginas). Continuam pendentes a impressora real e a prova de tinta em papel; o código e as provas de PDF/PDFium/mutool estão fechados.

Verificação executada:
- `cargo test --workspace --locked`: 51 testes passam (9 de integração do CLI, 19 em plots-core, 12 em plots-pdf, 11 em plots-print). Tempo de 30 s.
- CLI real via `cargo run -q -p plots-cli --locked -- ...`, sem envio a nenhuma impressora. Saídas e PDFs de teste ficaram em `E:\PlotS\target\audit\run-135346` (apagados ao final).
- Verificação dos PDFs gerados por `gerar-pdf` com pypdf (texto da página, MediaBox, Rotate). Não houve verificação visual no Firefox nem no pdf.js.
- Capturas de tela: nenhuma. Não existe aplicativo gráfico (`ui/` e `src-tauri/` não existem), então não há tela para capturar.

## Estado da integração de `gerar-pdf` (sessão anterior)

Está funcional e consistente no núcleo, com 2 testes de integração e saída coerente. Pendências encontradas:
- Texto de uso desatualizado: `plots-cli/src/main.rs:8` mostra só `plots simular ...` e é impresso em todo erro, inclusive erros de `gerar-pdf` (PS-A091).
- Mensagens "PDF não é aceito" continuam no ramo genérico de `plots-cli/src/lib.rs:73-80` e no ramo de `simular` (lib.rs:93-95). Com `imprimir doc.pdf` a resposta fica enganosa (PS-A090).
- O comando `gerar-pdf` não tem teste para `--help`, para opção repetida, para PDF criptografado nem para a mistura de orientação (lacunas de cobertura).
- Os nomes das passadas são `lote-0001-passada-1.pdf`, e o AGENTS fala em `passada1.pdf` e `passada2.pdf` temporários (PS-A010).
- A sessão anterior parou com a integração "em andamento, ainda sem validação final". Esse estado foi confirmado: a integração funciona, mas as mensagens e a documentação de uso não foram fechadas.

## Requisitos

Legenda: OK = atende; PARTIAL = atende parte; MISSING = não existe ainda; BUG = comportamento errado; N/A = sem efeito observável ou processo.

| ID | Seção | Requisito | Status | Evidência |
| --- | --- | --- | --- | --- |
| PS-A001 | Visão geral | Enviar as passadas à impressora real pelo spooler, sem driver manual | PARTIAL (126ac26) | `plots imprimir` envia ao spooler (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A002 | Perfis | Perfil com os 4 parâmetros: `passada1_inversa`, `passada2_inversa`, `verso_girar_180`, `gesto_recolocar` | OK | `plots-core/src/lib.rs:59-66` (`PerfilImpressora`). |
| PS-A003 | Perfis | `gesto_recolocar` com texto e figura para o usuário | OK (`ca10be1`) | Frase e figura vêm de lado e virada; oito combinações conferidas com o plano 19, sem papel. |
| PS-A004 | Perfis | Jato (traseira) inverte a passada 1; laser (gaveta) não | OK | `plots-cli/src/lib.rs:27-33`. Execução: traseira "Passada 1: 4, 2"; gaveta "Passada 1: 2, 4". |
| PS-A005 | Perfis | `passada2_inversa` e `verso_girar_180` ficam "a calibrar", sem presunção | PARTIAL | `plots-cli/src/lib.rs:30`: a traseira já pré-define `passada2_inversa = true`. Não há calibração que confirme isso. |
| PS-A006 | Algoritmo | Total ímpar recebe página em branco no fim, que entra na passada das pares | OK | `plots-core/src/lib.rs:147-148` e `185-191`; teste `uma_folha_tem_frente_par_vazia_e_verso_impar` (lib.rs:512). Execução: 5 páginas, lote 2 traseira, "Passada 1: branca" no lote 2. |
| PS-A007 | Algoritmo | Passada 1 = pares, passada 2 = ímpares, com ordem configurável | OK (cf22592) | `OrdemPassadas` tem os dois valores (`plots-core/src/lib.rs:88-94`), mas CLI e `gerar-pdf` fixam `ParesPrimeiro` (`plots-cli/src/lib.rs:142` e `265`). |
| PS-A008 | Algoritmo | Aplicar inversão e giro de 180 graus (`/Rotate 180` no verso) | OK | `plots-core/src/lib.rs:197-207` e `449-459`; `plots-pdf/src/lib.rs:188-191`. Verificado: traseira com 5 páginas gera passada 1 "BRANCA, P4, P2" e passada 2 "P5, P3, P1", todos com rot=0. |
| PS-A009 | Algoritmo | Função pura `plano_lote(paginas, perfil) -> (Vec<Pagina>, Vec<Pagina>)`, com testes | PARTIAL | A função não existe com esse nome (grep sem resultado). Equivalentes: `plano_documento` (`plots-core/src/lib.rs:135`, retorna `Vec<Lote>`) e `plano_lotes_por_papel` (lib.rs:357). Testes unitários existem. |
| PS-A010 | Algoritmo | Gerar `passada1.pdf` e `passada2.pdf` temporários com lopdf | PARTIAL | Gerados como `lote-NNNN-passada-N.pdf` (`plots-cli/src/lib.rs:278-284`), persistentes, não temporários. Nome diverge da especificação. |
| PS-A011 | Algoritmo | Mostrar o gesto de recolocação e esperar "Imprimir verso" antes da passada 2 | OK (`256e8ce`) | Cartão mostra o gesto e espera "Imprimir verso"; conferido no app com as duas simuladas. |
| PS-A012 | Gesto único | Cartão com a frase fixa e a figura do gesto | OK (`ca10be1`) | Cartão com a frase física e a figura; substitui a frase fixa da especificação. |
| PS-A013 | Fluxo | Abrir PDF (arrastar, "Abrir com") e escolher o intervalo de páginas | OK (`ccb1ebc`) | Trecho contínuo por aba até o plano, com os números originais; soltar um PDF abre a aba. O gesto nativo de arrastar não foi feito no teste. |
| PS-A014 | Fluxo | Marcar impressoras e tamanho do lote (padrão 10 folhas) | N/A | Campo de lote saiu da janela por decisão do usuário (`plan/24-impressao-sem-lote.md`, `18bb5cc`). Marcar impressoras segue OK em PS-A024 e PS-A061. Lote continua no CLI e no núcleo. |
| PS-A015 | Fluxo | "Imprimir frente" envia para cada impressora a ordem do seu perfil | PARTIAL (`256e8ce`) | Envio pela janela conferido nas simuladas e na fila de PDF; sem impressora real. |
| PS-A016 | Fluxo | Botão do verso libera só depois que o spooler confirma a frente | OK (`256e8ce`) | Verso só libera com a confirmação do `Job`; o texto avisa que isso não garante que o papel saiu. |
| PS-A017 | Fluxo | Folha de controle opcional no canto do verso | OK (1ad3307) | Opção por aba "Folha de controle" (`ui/src/components/BlocoAjuste.tsx:71-92`); marca no canto do papel escolhido em `plots-pdf/src/opcoes.rs`, teste `a_marca_sai_no_canto_do_papel_escolhido_e_nao_dentro_da_escala` (opcoes.rs:588). Fica de fora do frente e verso automático (`controle.semMarca`). |
| PS-A018 | Fluxo | "Reimprimir lote" refaz só aquele lote | OK (`256e8ce`) | "Reimprimir lote" no cartão, com confirmação em fila real; falha ao cancelar não reenvia. |
| PS-A019 | Interface | Janela sem menus; barra superior única (ícone, botão da lateral, abas, "+", controles) | OK (353f54d) | Barra sem menus, medidas conferidas no app. |
| PS-A020 | Interface | Splash 480×300, carregamento até 10 s, depois a janela principal | OK (`d4f3325`) | Imagem definitiva do usuário, janela, tempos e avisos no app real. |
| PS-A021 | Interface | Barra lateral de impressoras com caixa de seleção e seletor de modo (Traseira, Gaveta, "Calibrar…") | OK (826b726) | Lista real com caixa, estado e seletor de modelo: Jato de tinta, Laser e Calibrar. |
| PS-A022 | Interface | Abas por documento com estado próprio; confirmação ao fechar com impressão em andamento | OK (353f54d, ccb1ebc, f6e3433) | A aba guarda impressoras marcadas, intervalo, cópias, ajuste de papel e folha de controle (`ui/src/tabs.ts:52-73`). Fechar com impressão em andamento pede confirmação (`tabs.ts:55, 80, 116`, `aguardandoConfirmacao`). Lote saiu da aba por decisão (plan 24). |
| PS-A023 | Interface | Atalhos Ctrl+O, Ctrl+T, Ctrl+W, Ctrl+Tab, Ctrl+B; arrastar PDF abre aba | OK (353f54d, ccb1ebc) | Atalhos em `ui/src/atalhos.ts:11-30`, com `ui/src/atalhos.test.ts`. Arrastar PDF: evento `drop` em `ui/src/App.tsx:313-330`. Ctrl+W, Ctrl+Tab e Ctrl+B foram testados no app; o gesto nativo de arrastar não teve teste real. |
| PS-A024 | Interface | Seleção de impressoras por documento (decidido no AGENTS) | OK (f6e3433) | Cada aba guarda as próprias impressoras; aba nova começa sem marcas. |
| PS-A025 | Interface | Visualizador pdf.js com miniaturas, página e zoom | OK (`b29796e`) | Páginas, miniaturas, página e zoom conferidos no app real com PDFs de teste. |
| PS-A026 | Interface | Tokens da norma de estilo, tema escuro, foco visível, ESC fecha diálogos, sem emoji, `aria-live` para o estado | OK (f6e3433, 256e8ce) | Tokens em `ui/src/theme.css`; tema por `data-theme` (`ui/src/theme.ts`); foco visível nos CSS Modules; `aria-live` em Sidebar, CartaoImpressao e CalibracaoDialog; sem emoji em `ui/src` (busca fora de testes sem resultado). ESC nos diálogos Radix é padrão da biblioteca; não foi testado no app. |
| PS-A027 | Interface | Textos em pt-BR e en via `t()` | OK (353f54d) | `t()` em pt-BR e en nesta tela. |
| PS-A028 | Modo simulação | Rodar o fluxo inteiro sem impressora (`simular`) | OK | `plots-cli/src/lib.rs:51-223`. Testes `plots-cli/tests/cli.rs:8-43` e `94-149`. Execução verificada. |
| PS-A029 | Modo simulação | Chave "Simular impressão" por documento; botão vira "Simular frente"; selo SIMULAÇÃO | PARTIAL (`256e8ce`) | Botões "Simular frente"/"Simular versos" e selo SIMULAÇÃO existem (`ui/src/i18n.ts:182-209`). O rótulo "Simular impressão" troca sozinho conforme as impressoras marcadas (`ui/src/cartoes.ts:328, 939`). Não há chave por documento. |
| PS-A030 | Modo simulação | Duas impressoras virtuais "Simulada — Traseira" e "Simulada — Gaveta", com outras combinações possíveis | OK (166adee, 3085baf) | A janela lista as duas sempre (`src-tauri/src/impressoras.rs:23-24, 326-336`), mais "Simulada — Automática" (0a8f451). A CLI reconhece os nomes (`plots-cli/src/lib.rs:89-118`). Outras combinações: o PlotSim cria filas reais com qualquer modelo (`PlotSim/plotsim-core/src/filas.rs:144`, `criar(curto, modo)`). Dentro do PlotS, as simuladas têm modelo fixo. |
| PS-A031 | Modo simulação | `SimBackend` implementa o mesmo trait dos backends reais | OK | Trait `Backend` em `plots-print/src/job.rs:19-25`; impl em `job.rs:180-208`. |
| PS-A032 | Modo simulação | Modelo físico: folha `{frente, verso, girada}`, pilha de saída, gesto como operação | OK | `plots-print/src/lib.rs:8-24` (`Folha`), `66-72` (`Recolocacao`), `190-215` (`recolocar`). |
| PS-A033 | Modo simulação | "Simular erro": recolocar sem virar, girado, ou com uma folha a menos | OK (cf22592) | O núcleo suporta (`Recolocacao`, `retirar_folhas`, `inverter_ordem`, `girar_180`, teste `falta_de_folha_permite_cancelar_e_reimprimir_lote`, lib.rs:593). O CLI sempre usa `virar_maco: true` (`plots-cli/src/lib.rs:187-190`) e não expõe a opção. |
| PS-A034 | Modo simulação | Vista por lote com passada 1, pilha, passada 2 e folhas finais marcadas certo/errado ("3 de 3 folhas corretas") | OK (7c47c9e) | PlotSim marca cada folha como certa ou errada (`PlotSim/app/ui/src/components/Folha.tsx`, `folha.certa`/`folha.errada`) e mostra o resumo "N de M folhas corretas" (`PlotSim/app/ui/src/i18n.ts:87-90`). A pilha de saída aparece na janela (`i18n.ts:36`). No PlotS, o cartão mostra "N de M folhas corretas (na simulação)" (`ui/src/i18n.ts:257-258`). |
| PS-A035 | Modo simulação | Cartão EM ANDAMENTO com "Simular verso" | OK (`256e8ce`) | Cartão EM ANDAMENTO com "Simular verso" e resultado da simulação. |
| PS-A036 | Modo simulação | Exportar `passada1.pdf` e `passada2.pdf` para abrir no Firefox | PARTIAL | `gerar-pdf` gera os PDFs (`plots-cli/src/lib.rs:225-310`). Nomes diferentes. Abertura no Firefox não verificada. |
| PS-A037 | Modo simulação | Exportar PDF de folhas finais (frente e verso lado a lado) | OK (6baa24a) | `gerar-pdf --folhas-finais`, conferido com PyMuPDF; sem conferência no Firefox. |
| PS-A038 | Modo simulação | Modelo com `proptest`: 8 perfis, 1 a 200 páginas, lotes de 1 a 50 folhas, maço final 1\|2, 3\|4 | OK (6baa24a) | proptest com tabela fixa de 8 perfis; só traseira e gaveta correspondem às impressoras simuladas, e o modelo da gaveta será revisto. |
| PS-A039 | Arquitetura | App Tauri 2 com webview e pdf.js | OK (`b29796e`) | App Tauri 2 com webview e pdf.js em worker, sem afrouxar a CSP. |
| PS-A040 | Arquitetura | Camada de backends: trait com duas implementações (Windows e Simulado) | OK (126ac26) | `BackendWindows` e `SimBackend` implementam o mesmo trait. |
| PS-A041 | Arquitetura | lopdf para reordenar, extrair, inserir página em branco e `/Rotate` | OK | `plots-pdf/src/lib.rs:112-244`. Testes `reabre_ordem_heranca_rotacao_e_branco` (lib.rs:630) e outros. Verificado pelos PDFs gerados. |
| PS-A042 | Arquitetura | pdfium-render para rasterizar (impressoras que não aceitam PDF) | PARTIAL (126ac26) | `pdfium-render` rasteriza na resolução da fila (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A043 | Arquitetura | Linux e macOS com IPP/CUPS ou `lp` | MISSING | Fase futura. AGENTS decide que o CUPS fica para depois do backend Windows. |
| PS-A044 | Arquitetura | tokio, um job por impressora em paralelo | N/A (`62d51a4`) | Cada trabalho roda na própria thread; o paralelismo vale pelo comportamento, sem tokio. |
| PS-A045 | Arquitetura | Config com serde, toml e directories; perfis em `%APPDATA%/plots/perfis.toml` | OK (72030fb, 1ad3307) | `plots-perfis/Cargo.toml:8-12` (serde, toml, directories, thiserror). Caminho em `plots-perfis/src/armazenamento.rs:27-30` (`config_dir()/plots/perfis.toml`, que no Windows é %APPDATA%). Usado pelo CLI (`plots-cli/src/filas.rs`, `calibrar.rs`) e pela janela (`src-tauri/src/impressoras.rs`). |
| PS-A046 | Arquitetura | Erros com thiserror e logs com tracing | PARTIAL | `thiserror` só em `plots-perfis` (erro.rs, grupos.rs, historico.rs). Nos demais crates, erros escritos à mão (`plots-core/src/lib.rs:107-128`). Nenhum uso de `tracing` no código. |
| PS-A047 | Arquitetura | Frontend React + TypeScript, CSS Modules, lucide-react, Radix | OK (353f54d) | React, TypeScript, CSS Modules, lucide e Radix. |
| PS-A048 | Windows | Impressão só pelo PlotS, sem diálogo, envio direto ao spooler | PARTIAL (126ac26) | Envio direto, sem diálogo (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A049 | Windows | Rasterizar na resolução da impressora (GetDeviceCaps); DEVMODE com simplex, sem agrupar, bandeja manual e tamanho | PARTIAL (126ac26) | Resolução por `GetDeviceCaps`; `DEVMODE` forçado e conferido; bandeja fica a do driver (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A050 | Windows | Acompanhar o trabalho no spooler (GetJob ou notificação) | PARTIAL (126ac26) | `GetJobW` com avisos; nenhum estado real observado (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A051 | Windows | Detectar a opção "inverter ordem" do driver e pedir para desligar | PARTIAL (1ad3307) | Não detecta: a opção é marcada pela pessoa na linha da impressora (`src-tauri/src/inversao.rs:1-5, 43-80`), com aviso que manda desligar no driver (`ui/src/i18n.ts:446`). Falta ler a configuração do driver. |
| PS-A052 | Windows | Fila virtual "PlotS" que recebe o PDF de qualquer programa | MISSING | Fase 5. |
| PS-A053 | Papel | Ler o tamanho de cada página do PDF | OK | `plots-pdf/src/lib.rs:71-108` (`dimensoes_paginas`). Verificado com `rotacionada.pdf`. |
| PS-A054 | Papel | Reconhecer A4, A5, Carta, Ofício, Legal, com tolerância de 2 mm | OK | `plots-core/src/lib.rs:297-328`. Testes `reconhece_formatos_orientacao_e_personalizado_com_tolerancia` (lib.rs:624) e `valida_par_frente_verso...` (lib.rs:655). Verificado: `tolerancia-par.pdf` (598×844) aceito. |
| PS-A055 | Papel | Formato "Outro" com `DMPAPER_USER` | PARTIAL (126ac26) | Medidas personalizadas pelo `DEVMODE`, com conferência (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A056 | Papel | Documento com tamanhos misturados dividido em lotes por tamanho | OK | `plots-core/src/lib.rs:357-462`. Verificado: `mistura-papel.pdf` gera lote 1 A4 e lote 2 A5 (`BRANCA` com 420×595). |
| PS-A057 | Papel | Cartão avisa a troca de papel | OK (`256e8ce`) | Cartão e CLI avisam a troca de papel. |
| PS-A058 | Papel | Frente e verso de tamanhos diferentes são recusados antes de imprimir | OK | `plots-core/src/lib.rs:385-397`. Verificado: `personalizado-par.pdf` (595×842 + 200×300) sai com "as páginas 1 e 2 têm tamanhos incompatíveis para a mesma folha", exit 2. Teste `rejeita_paginas_de_tamanhos_incompativeis_na_mesma_folha` (`plots-cli/tests/cli.rs:260-281`). |
| PS-A059 | Papel | Frente e verso da mesma folha com mesma orientação (retrato e paisagem) | OK (882ba68) | Ver BUG-1. Orientação diferente é aceita sem aviso. Decisão do usuário: bloquear ou só avisar. |
| PS-A060 | Papel | Opção "Ajustar ao papel" (imprimir Carta em A4) | OK (1ad3307) | Opção "Ajustar ao papel" por aba (`ui/src/components/BlocoAjuste.tsx:27-54`); escala proporcional e centralizada em `plots-pdf/src/opcoes.rs` (teste `carta_em_a4_fica_centralizada_na_caixa_do_a4`, opcoes.rs:498). |
| PS-A061 | Múltiplas | Marcar várias impressoras e enviar a mesma origem, cada uma com o perfil próprio | OK (`62d51a4`) | Várias impressoras, cada uma com o próprio perfil; conferido nas simuladas, fila real só até a confirmação. |
| PS-A062 | Múltiplas | Descoberta das filas do sistema (EnumPrintersW) | OK (a9f9185, 826b726) | `plots-windows/src/sistema.rs:1, 76` (EnumPrintersW). A janela lista as filas por `listar_filas()` (`src-tauri/src/impressoras.rs:206`). |
| PS-A063 | Múltiplas | Perfil por fila: chave com o nome da fila e apelido amigável | PARTIAL | Chave pelo nome exato da fila e campo `apelido` em `plots-perfis/src/perfis.rs:12`, gravado e listado (`src-tauri/src/impressoras.rs:893-909`). Falta a tela de apelido: `ui/src` não tem nenhuma edição de apelido. |
| PS-A064 | Múltiplas | Máquina de estados por job: Ocioso, FrenteEnviada, AguardandoRecolocacao, VersoEnviado, Concluído, Erro | PARTIAL | `plots-print/src/job.rs:45-53`: `EstadoJob` tem `Pronto` (no lugar de Ocioso), `FrenteEnviada`, `AguardandoRecolocacao`, `VersoEnviado`, `DuasFacesEnviadas` (extra), `Concluido` e `Erro`. Nomes divergem. |
| PS-A065 | Múltiplas | Modos espelho e divisão | OK (`62d51a4`, `18bb5cc`) | Na janela, todas as marcadas imprimem o mesmo, por decisão do usuário (`plan/24-impressao-sem-lote.md`). A divisão segue no núcleo, em `plots-core/src/distribuicao.rs`, testada pelo PlotSim. |
| PS-A066 | Múltiplas | Consulta periódica do spooler; avisa papel acabado ou atolado | PARTIAL (`ca10be1`) | Consulta distingue papel acabado, atolado e sem resposta; só com backend falso, sem driver real. |
| PS-A067 | Múltiplas | Grupos salvos de impressoras ("Todas do escritório") | OK (`62d51a4`) | Grupos salvos no arquivo de perfis, com grupo inválido ignorado e preservado. |
| PS-A068 | Fase 1 | Testes unitários: lotes pares, ímpares, 1 folha e as 8 combinações de perfil | OK | `plots-core/src/lib.rs:512-586`. `cargo test` passa. |
| PS-A069 | Fase 1 | Crate `plots-core` com lógica pura, sem E/S | OK | `plots-core/src/` (sem dependências externas em `plots-core/Cargo.toml`). |
| PS-A070 | Fase 2 | PDF de teste de calibração com números grandes e setas | OK (cf22592) | Não há geração de texto em `src/`. `.qa-pdf/make.py` é um script de QA com PyMuPDF, fora do repositório, sem integração. |
| PS-A071 | Fase 2 | Calibração: 2 folhas com páginas 1 a 4, duas perguntas e ajuste do perfil | PARTIAL | Já existem: PDF de calibração (`plots-pdf/src/calibracao.rs`), perguntas e ajuste na janela (`ui/src/components/CalibracaoDialog.tsx`, `src-tauri/src/calibracao.rs`, `c410c8c`) e no CLI (`plots calibrar`); ajuste em `plots-core/src/ajuste.rs` (`1ad3307`). Falta a conferência em 2 folhas de papel. |
| PS-A072 | Fase 2 | Os PDFs gerados abrem no Firefox com ordem e rotação certas | OK | O usuário abriu no Firefox as passadas e as folhas finais de um documento de 4 páginas: ordem e rotação certas (2026-11-02). |
| PS-A073 | Fase 2 | Backend Simulado com impressoras virtuais | OK | `SimBackend` (`plots-print/src/job.rs:159`) e `plots simular`. |
| PS-A074 | Fase 3 | `plots imprimir doc.pdf --impressora X --lote 10` | PARTIAL (126ac26) | `plots imprimir` envia com confirmação (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A075 | Fase 3 | `plots calibrar X` | PARTIAL | `plots calibrar` faz as perguntas e grava o perfil (92772e5); falta enviar e confirmar em impressora real. |
| PS-A076 | Fase 3 | Backend CUPS ou Windows | PARTIAL (126ac26) | Backend Windows pronto (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A077 | Fase 3 | Critério: simulação correta nas duas impressoras virtuais | OK (108bede, a34a54e) | Suíte rápida do PlotSim: 42 de 44, 0 falhas, com a matriz das duas impressoras (`plan/23-plotsim.md`, "Resultado da suíte rápida"; registro em `STATUS-MANUAIS.md`, lote `plotsim-suite`). Ressalva: o Ofício 216 × 330 mm não tem conferência de ponta a ponta. |
| PS-A078 | Fase 3 | Critério: 3 lotes seguidos sem erro numa impressora real | PARTIAL | Falta a prova em papel: o fluxo de lotes existe (`plots-cli/src/enviar.rs`; janela com envio por trecho, `18bb5cc`), mas não há impressora real nem papel nesta máquina. Depende de decisão do usuário e de teste em papel. |
| PS-A079 | Fase 4 | Interface Tauri + pdf.js: visualizador, painel de impressoras, cartões com figura, fluxo sem terminal | OK (b29796e, 826b726, ca10be1) | Visualizador pdf.js (b29796e); painel de impressoras (`ui/src/components/Sidebar.tsx`, 826b726); cartões com a figura do gesto (`ui/src/components/FiguraGesto.tsx`, ca10be1); fluxo na janela sem terminal (`ui/src/App.tsx`). |
| PS-A080 | Fase 5 | Várias impressoras, jobs paralelos, espelho e divisão, status do spooler, fila virtual "PlotS" | PARTIAL (`62d51a4`) | Falta a fila virtual; envio paralelo, modos e status já existem. |
| PS-A081 | Fase 6 | Instalador (tauri bundle), reimprimir lote, folha de controle, histórico | PARTIAL (1a1f6d7, 1ad3307) | Folha de controle (1ad3307) e histórico local (`plots-perfis/src/historico.rs`, `src-tauri/src/historico.rs`) existem. Falta executar o instalador NSIS em máquina ou conta de teste (depende do usuário). |
| PS-A082 | Estrutura | Workspace Cargo com plots-core, plots-print, plots-cli, src-tauri e ui | OK (353f54d) | Workspace com `src-tauri` e `ui`. |
| PS-A083 | Limpeza do disco | Um único `target/` (até 2 GB); artefatos de verificação removidos após o registro | PARTIAL | `target/` tem 444 MB. `.qa-pdf/` mantém `entrada.pdf`, `saida.pdf`, `saida-1..3.png` da sessão anterior, já registrados. Os arquivos da auditoria foram apagados no fim. |
| PS-A084 | Segurança dos dados | Não sobrescrever arquivos existentes; em falha, remover só o que foi criado | OK | `plots-cli/src/lib.rs:286-302` (`create_new` e rollback). Verificado: segunda execução na mesma pasta falha e mantém os PDFs; pasta com `lote-0001-passada-2.pdf` pré-existente mantém o conteúdo "ORIGINAL". |
| PS-A085 | Segurança dos dados | PDF criptografado e inválido são recusados com mensagem | OK | `plots-pdf/src/lib.rs:42-46` e teste `rejeita_pdf_criptografado` (lib.rs:712). Verificado: "PDF criptografado não é suportado"; arquivo inválido: "erro ao processar PDF". |
| PS-A086 | Modo simulação | Limite de 100 000 páginas e lotes grandes sem estouro | OK | `plots-core/src/lib.rs:8`, `144-152`. Verificado: `--paginas 100001` recusado; `--lote` com `usize::MAX` recusado com "o documento é grande demais para planejar"; 100 000 páginas em 1,4 s. |
| PS-A087 | Modo simulação | Página em branco com ponto discreto para o driver não descartar a folha | OK | `plots-pdf/src/lib.rs:150-184` (quadrado cinza 0,98 de 0,1 pt). Só o código foi conferido; o efeito no driver não foi verificado. |
| PS-A088 | Perfis | "Pares primeiro" é o padrão | OK | `plots-core/src/lib.rs:91-92` (`#[default]`). |
| PS-A089 | Fase 1 | Teste de calibração ajusta as respostas relativas ao perfil testado | OK | `plots-core/src/lib.rs:78-86`, teste 469-493 (combinações exaustivas). |
| PS-A090 | Texto de erro | Mensagens do CLI coerentes com as funções atuais | OK (882ba68) | Ver BUG-2. |
| PS-A091 | Texto de uso | Texto de uso lista todos os comandos e aparece só no erro de uso | OK (882ba68) | Ver BUG-3. |
| PS-A092 | Processo | Uso obrigatório da craft-team e do Graphify | N/A | Processo do desenvolvimento (`AGENTS.md:3-7`, `294-307`). Sem efeito no produto. |
| PS-A093 | Modo simulação | "Limite: a simulação prova a lógica; só a impressora real confirma o perfil" | N/A | Explicação de limite, sem requisito observável. |
| PS-A094 | Avisos | Cartões de andamento saem da lista de impressoras; cada mudança vira aviso no topo e no painel ao lado do histórico | OK (`7b16a4f`) | `ui/src/components/AvisosDoTopo.tsx`, `PainelDeAvisos.tsx`, `ui/src/avisos.ts`; lista sem a seção de cartões (`Sidebar.tsx`). Capturas `cap6/` e `cap7/`; dois temas, inglês, 640×420. |
| PS-A095 | Avisos | Aviso do topo não cobre a barra de título, some em 6 s (erro fica), no máximo 3, pausa em hover e foco, e não cobre a lateral em 640×420 | OK (`7b16a4f`) | `AvisosDoTopo.module.css` dentro da área do documento; medições em `saidas/r7-medicoes.json` e `640x420-clicaveis.txt`. Em 640×420 o estado do aviso pode truncar, anotado em `plan/25`. |
| PS-A096 | Fluxo | O verso que espera na fila sempre pede confirmação, com gesto, páginas, papel e cópias do trabalho enviado | OK (`7b16a4f`) | `ui/src/face-do-envio.ts:186-193`, `ui/src/App.tsx:520-541`; teste `verso-plano.test.ts` com falha prévia em `saidas/falha-a1b.log`. |
| PS-A097 | Fluxo | Trabalho anterior em erro avisa, na confirmação do verso, que pode continuar na fila do Windows | OK (`7b16a4f`) | `face.envio.erroAnterior` em `conteudoDoVerso`; teste com falha prévia em `saidas/falha-a2.log`. |
| PS-A098 | Impressão no Windows | Envio simultâneo não perde trabalho no driver que falha com dois trabalhos: uma fila por vez nesse driver, e erro detectado no registro vira Reimprimir | OK (`7b16a4f`) | `plots-windows/src/vez.rs` (semáforo nomeado) e `registro.rs` (evento 372 por fila e documento); 10 rodadas de 2 filas sem perda e sem evento novo. |
| PS-A099 | Tamanhos de papel | Papel misto mantém cada página no lado do documento; só o par de tamanhos diferentes se separa, com verso em branco, avisado na análise | OK (`7b16a4f`) | `plots-core/src/lib.rs` (`emparelhar`, `pares_separados_do_intervalo`); aviso em `i18n.ts`; teste `papel_misto_mantem_o_lado_e_so_o_par_diferente_se_separa`. |
| PS-A100 | Interface | Em 640×420 e 700×500 nada fica inalcançável com "Mais opções"; os três controles cabem juntos | PARTIAL (`7b16a4f`) | "Mais opções" abre e rola, e o botão de envio fica visível. Em 640×420 a área rolável tem 84 px e os três controles somam 114 px: os três só aparecem juntos em 640×480. Registrado em `plan/25`. |
| PS-A101 | Interface | O tema claro do PlotS usa o cinza neutro dos painéis, com a mesma hierarquia de superfícies, o tema escuro intacto e a página do PDF branca | OK (`c248cf8`) | `ui/src/theme.css` (`[data-theme='gray']`, `--icon-warning` nos dois temas), `ui/src/theme.ts` (`Tema = "dark" \| "gray"`); `theme.test.ts` e `theme-estilos.test.ts` com falha prévia `saidas/falha-antes.log` (8 falhas). Capturas `cz-01/02/03/07/08/10/14`; contraste em `saidas/contraste.log`; as três revisões aprovaram. |
| PS-A102 | Interface | A confirmação "Imprimir os versos?" leva um triângulo de alerta amarelo antes do título, só nela, com o nome acessível só pelo texto | OK (`c248cf8`) | `ui/src/confirmacao.ts` (`atencao`), `ui/src/face-do-envio.ts` (`conteudoDoVerso`), `ui/src/components/ConfirmDialog.tsx/.module.css`; `titulo-do-dialogo.test.tsx` e `face-do-envio.test.ts`; capturas `cz-04/05/06/14c`. |
| PS-A103 | Portabilidade | Com `PLOTS-PORTATIL.txt` ao lado do executável, o `perfis.toml` fica na pasta do programa e a primeira abertura traz o de `%APPDATA%` sem sobrescrever; sem o marcador, o instalado continua em `%APPDATA%` | OK (`97643ec`) | `plots-perfis/src/armazenamento.rs` (`caminho_para`, `migrar_para_portatil`), `src-tauri/src/main.rs`; testes em `plots-perfis/src/testes.rs`; CI em `%LOCALAPPDATA%\plots-build\perfil-portatil\saidas\ci.log`; uso real com SHA-256 do perfil local e do `%APPDATA%` no relatório do lote. |
| PS-A104 | Impressão | "Frente" manda as páginas ímpares e "Verso" as pares; documento de 1 página termina sem verso e sem recolocação; 3 páginas fecham nas duas faces | OK (`559a9c8`) | `plots-core/src/lib.rs` (`frentes_do_lote`, `versos_do_lote`), `plots-print/src/job.rs` (`ErroJob::SemVerso`), `src-tauri/src/trabalhos.rs`, `ui/src/tabs.ts`/`face-do-envio.ts`/`BlocoFace.tsx`/`cartoes.ts`; testes de 1, 2 e 3 páginas; capturas `20`–`29`; PDF real de 1 página em `saidas/uso-final`. |
| PS-A105 | Impressão | O desenho GDI usa bitmap compatível em DC de memória e `StretchBlt`, com `SetDIBits` antes de `SelectObject` e queda para `StretchDIBits` | PARTIAL (`559a9c8`) | `plots-windows/src/spooler.rs`; `saidas/envio-real-3.log` (1 página, 418 bytes, 1 desenho pela memória); falta driver de fabricante a 600 dpi e forçar o fallback. |
| PS-A106 | Interface | Chave de boca ao lado da engrenagem; painel com bandeja, resolução e cor declaradas pelo driver; aplica no ajuste salvo; "Padrão" repõe o padrão; campos do envio fora do painel | PARTIAL (`559a9c8`) | `plots-windows/src/opcoes.rs`/`preferencias_driver.rs`, `src-tauri/src/preferencias.rs`, `ui/src/components/OpcoesDaImpressoraDialog.tsx`/`Sidebar.tsx`; CI, capturas nativas 640×420; falta driver real com bandeja/zero opções e envio usando o `DEVMODE` salvo. |
| PS-A107 | Impressão | Imagem colorida sai com as cores do PDF: laranja e amarelo não viram azul | OK (`6a972c6`) | `plots-windows/src/backend.rs` (`.set_reverse_byte_order(false)`); testes `as_faixas_de_cor_saem_com_os_bytes_bgra_certos` e `a_pagina_colorida_chega_ao_pdf_da_fila_com_as_cores_certas`; falha anterior e `envio-cor.log` em `%LOCALAPPDATA%\plots-build\cor-imagens\saidas`; CI em `integracao-alpha6`. |
| PS-A108 | Interface | O PlotS abre imagens PNG, JPEG, BMP, GIF, WebP e TIFF multipágina, mostra a prévia e imprime pelo mesmo caminho do PDF | OK (`d6ae384`) | `plots-pdf/src/imagem.rs` (PDF derivado com Flate RGB, alfa sobre branco, A4/DPI, TIFF multipágina), `src-tauri/src/documento.rs` (tipo, cache atômico, mutex, versão do hash), `plots-pdf/src/imagem.rs`/`src-tauri/src/documento.rs` testes; `saidas/envio-imagem.log` (laranja 118405, amarelo 118405, trocados 0), `fix1-a..d` nos dois temas; CI em `plotinfo-imagem`. Falta impressora real e os demais formatos na fila. |
| PS-A109 | Interface | Estimativa de tinta e custo: seleção de páginas do painel, cobertura CMYK, mL, R$, cartucho e perfis com preço por mL | OK (`0f66aa7`) | `plots-info/src/*` (54 testes: seleção, BGRA/PAM, perfis, custo, A3 = 2× A4), `src-tauri/src/analise.rs` (job, cache, eventos, cancelamento, recálculo e CSV), `src-tauri/src/mutool.rs` (processo separado, lotes de 256 MiB/32 páginas, timeout/kill e PAM), `src-tauri/src/csv_tinta.rs`, `src-tauri/src/plotinfo.rs` (21 testes, `plotinfo.toml`), UI `PlotInfoGaveta`/`PlotInfoEngrenagem`/`analise-tinta.ts` (69 arquivos/1.162 testes, tsc limpo). Provas: PDF DeviceCMYK C 10/20/30 exato a 72 dpi (`real-mutool-lote.log`), job completo `modoUsado = cmyk` + CSV (`real-cmyk-lote.log`), fallback e cancelamento nos testes; capturas 15–24 e capturas finais da interface em `alpha7-final`. Falta impressora real e prova de tinta em papel. |

## Bugs (com passos de reprodução)

### BUG-1 (PS-A059): frente retrato com verso paisagem é aceita sem aviso

- Estado em 2026-11-04: **corrigido** (`882ba68`). `pares_com_orientacao_mista` (`plots-core/src/lib.rs:508`) acusa a folha com retrato e paisagem, com teste (`lib.rs:1765`). O texto abaixo é o registro de 2026-11-02.

- Causa: `plots-core/src/lib.rs:386-387` compara só `formato` (que ignora orientação). `lib.rs:389-390` compara as dimensões já normalizadas para retrato, então A4 retrato e A4 paisagem parecem iguais. A checagem de papel por lote (`lib.rs:402-412`) compara a orientação só com a primeira folha do lote, nunca com o verso.
- Repro: criar um PDF de 2 páginas, página 1 com 595×842 e página 2 com 842×595.
- Comando: `cargo run -q -p plots-cli --locked -- gerar-pdf <arquivo> --lote 10 --impressora gaveta --saida <pasta_vazia>`
- Resultado observado: exit 0, "Gerados 2 PDFs", sem aviso. `lote-0001-passada-1.pdf` tem a página 2 em 842×595, e `lote-0001-passada-2.pdf` tem a página 1 em 595×842. Mesmo caso com `/Rotate 90` na página 2.
- Esperado (AGENTS, Papel): "Frente e verso ... nunca ficam com tamanhos diferentes; se o PDF tiver isso, o PlotS avisa antes de imprimir." Decisão do usuário: orientação diferente conta como tamanho diferente? Recomendação: sim, e recusar com a mesma mensagem de `ParIncompativel`.

### BUG-2 (PS-A090): mensagem enganosa para comandos que não existem

- Estado em 2026-11-04: **corrigido**. A frase "PDF não é aceito" saiu do caminho de `imprimir`; cada comando tem o próprio uso (`plots-cli/src/lib.rs:26-44`, `USO_*`). O texto abaixo é o registro de 2026-11-02.

- Repro: `cargo run -q -p plots-cli --locked -- imprimir doc.pdf --impressora "Microsoft Print to PDF" --lote 10`
- Resultado: "Erro: PDF não é aceito: esta versão do CLI só executa simulações numéricas." (exit 2). O CLI aceita PDF em `gerar-pdf`, e `imprimir` ainda não existe.
- Esperado: dizer que `imprimir` ainda não existe e indicar `gerar-pdf` ou `simular`.
- Causa: `plots-cli/src/lib.rs:73-80` (ramo genérico) e `93-95` (ramo de `simular`). O teste `rejeita_pdf_com_mensagem_explicita` (`plots-cli/tests/cli.rs:46-55`) fixa a frase antiga.
- Também: `plots calibrar X` responde "comando ausente ou desconhecido", sem dizer que o comando é previsto.

### BUG-3 (PS-A091): texto de uso incompleto e sempre o mesmo

- Estado em 2026-11-04: **corrigido**. O texto de uso sai por comando (`plots-cli/src/lib.rs:26-44`, `USO_*`) e não é mais o de `simular` em todo erro. O texto abaixo é o registro de 2026-11-02.

- Repro 1: `cargo run -q -p plots-cli --locked --` (sem argumentos) mostra "Uso: plots simular ..." sem mencionar `gerar-pdf`.
- Repro 2: `cargo run -q -p plots-cli --locked -- gerar-pdf` mostra "Erro: informe o arquivo PDF" e depois "Uso: plots simular --paginas N ...", que é o uso de outro comando.
- Repro 3: qualquer erro de PDF (`gerar-pdf falso.pdf ...`) também imprime o uso de `simular`.
- Causa: `plots-cli/src/main.rs:8` tem texto fixo. O `--help` (`plots-cli/src/lib.rs:67`) está correto. Os dois textos divergem.
- Cosmético no mesmo lote: comentários com caracteres corrompidos em `plots-cli/src/lib.rs:57` ("sa?da") e `plots-print/src/job.rs:23` ("n?o"). Há também um bloco de doc-comment partido em `plots-print/src/job.rs:147-158`.

Nenhuma perda de dados foi encontrada. Colisão de arquivos, falha de escrita, PDF criptografado e PDF inválido se comportam como deveriam (PS-A084 e PS-A085).

## Frases possivelmente desatualizadas

Em `AGENTS.md`:
1. Linha 7: "O projeto ainda não possui workspace Cargo nem CI." O workspace existe (`Cargo.toml`, 4 membros). Não há pipeline de CI (`.github/` não existe), mas `.claude/team.json` já traz `ci_command` e `ci_success_line` preenchidos, e o comando passou na execução desta auditoria. A frase "os valores atuais são pendências" também está defasada.
2. Linha 248: "as Fases 1 a 4 são feitas e validadas no modo simulação". Só a Fase 1 está pronta. A Fase 2 está parcial (sem PDF de calibração nem folhas finais). As Fases 3 e 4 não começaram. A frase soa como estado presente, mas é plano.
3. Linha 261: "workspace Cargo com plots-core, plots-print, plots-cli e src-tauri + ui/". Falta `plots-pdf`, que existe e é essencial para `gerar-pdf`.
4. Linha 48: o exemplo "frente enviada como 9, 7, 5, 3, 1" contradiz as linhas 42, 54 e 279 ("pares primeiro"). Com o mesmo lote de 5 folhas, o CLI produz frente 10, 8, 6, 4, 2 e verso 9, 7, 5, 3, 1. O exemplo troca frente e verso. Decisão do usuário: corrigir o exemplo.
5. Linha 167 (proptest): a especificação pede `proptest`. Não há a dependência. Não é frase desatualizada, mas o texto descreve algo que não existe.

Em `CLAUDE.md` (fora do escopo, mas afeta a mesma pergunta): "O comando e o resultado de CI ainda precisam ser definidos quando o workspace Cargo existir." Já estão definidos em `.claude/team.json`.

Em `.claude/team.json`: `ci_command` usa sintaxe PowerShell e roda `cargo test --workspace` sem `--locked`. Não altera o resultado, mas permite que o CI altere o `Cargo.lock`.

## Lotes propostos (ordem de prioridade, reauditoria de 2026-11-04)

Não há BUG aberto. Nenhuma perda de dados foi encontrada nos registros. Os itens marcados como **decisão do usuário** dependem dele.

### Lote 1: bugs e perda de dados
- Nenhum aberto. BUG-1, BUG-2 e BUG-3 estão corrigidos (seção Bugs).

### Lote 2: fidelidade (sem papel)
- PS-A005: a traseira ainda pré-define `passada2_inversa = true` (`plots-cli/src/lib.rs:30`), e o requisito pede "a calibrar". **Decisão do usuário:** manter o modelo como ponto de partida ou exigir calibração antes do uso.
- PS-A010 e PS-A036: os PDFs se chamam `lote-NNNN-passada-N.pdf` (`plots-cli/src/geracao.rs:105`), e a especificação diz `passada1.pdf` e `passada2.pdf`. **Decisão do usuário:** aceitar o nome atual ou renomear.
- PS-A009: `plano_lote` não existe; a função equivalente é `plano_documento` (`plots-core/src/lib.rs:223`). **Decisão do usuário:** aceitar o nome ou atualizar a especificação.
- PS-A029: a chave "Simular impressão" por documento não existe; o rótulo muda sozinho conforme as impressoras marcadas. **Decisão do usuário:** manter ou criar a chave.
- PS-A064: os estados divergem (`Pronto` no lugar de Ocioso, `DuasFacesEnviadas` extra). **Decisão do usuário:** ajustar o código ou a especificação.
- PS-A063: falta a tela de apelido; o campo já existe e aparece na lista. **Decisão do usuário:** entra na primeira versão?
- PS-A046: `tracing` não é usado; `thiserror` só existe em `plots-perfis`. Adotar `tracing` exige dependência nova, o que precisa de aprovação.
- PS-A051: a detecção automática da opção do driver não existe; hoje a pessoa marca a opção, com aviso. Validar exige impressora real. **Decisão do usuário:** aceitar a marcação manual para a primeira versão.
- PS-A065: a divisão saiu da janela (`plan/24`), e a linha segue OK por causa do espelho. **Decisão do usuário:** marcar a divisão como N/A na janela ou manter.
- PS-A083: `.qa-pdf/` ainda guarda arquivos da sessão anterior. **Decisão do usuário:** apagar ou manter.
- `AGENTS.md`: as frases da seção "Frases possivelmente desatualizadas" não foram revisadas nesta reauditoria. **Decisão do usuário:** quem edita a especificação.

### Lote 3: depende de papel ou impressora real (decisão do usuário)
- PS-A001, PS-A015, PS-A042, PS-A048, PS-A049, PS-A050, PS-A055, PS-A074, PS-A075, PS-A076 e PS-A078: envio e papel. Falta uma rodada em impressora real, ou nas filas virtuais do PlotSim (`plots imprimir --teste`), com confirmação em papel.
- PS-A071 e PS-A075: calibração em 2 folhas de papel.
- PS-A081: executar o instalador em máquina ou conta de teste.
- PS-A077, ressalva: o Ofício 216 × 330 mm não tem conferência de ponta a ponta na suíte do PlotSim.

### Lote 4: fora da primeira versão
- PS-A052 (fila virtual "PlotS", plano 16, não iniciado) e PS-A080, que depende dela. Pelo plano 00, a impressora virtual vem depois da primeira versão. **Decisão do usuário:** confirmar.
- PS-A043 (Linux e macOS, plano 18). Fora da primeira versão, como já registrado em `STATUS-MANUAIS.md`.

## Não verificado

- Abertura e ordem visual das passadas no Firefox ou no pdf.js. Foi verificado só com pypdf (texto e caixas de página).
- Efeito real da página em branco com ponto no driver (PS-A087).
- Qualquer envio a impressora, física ou virtual. Não foi feito, conforme a instrução. A fila "Microsoft Print to PDF" não foi usada.
- Nesta reauditoria, o app não foi rodado e não houve capturas de tela. O estado da janela vem de código, testes e registros de commit.
- Comportamento com PDFs reais grandes (o teste usou um PDF de 300 páginas gerado para o caso, em 1,5 s).
- `plots-cli` lendo PDFs com formulários e anotações do mundo real: o código tem tratamento e testes, mas não foi exercitado com PDFs externos.
- Compatibilidade do `.claude/team.json` com `cargo test --workspace` pelo PowerShell (rodado só via bash, com `--locked`).

## Limpeza

- Pasta de auditoria: `E:\PlotS\target\audit\run-135346` (com PDFs de teste, saídas, scripts e logs). Apagada ao final. Não houve `cargo clean`. `E:\PlotS\target\` foi mantido, conforme a instrução do usuário.
- Arquivos de sessões anteriores que não foram tocados: `E:\PlotS\.qa-pdf\` (entrada.pdf, saida.pdf, saida-1..3.png, make.py, src) e `codex-session-*.md`. Sugestão de limpeza, sem apagar: decisão do usuário.
