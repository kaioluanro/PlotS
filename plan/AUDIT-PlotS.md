# Auditoria do PlotS contra AGENTS.md

Data: 2026-11-02. A especificação foi dividida depois em `plan/`, então as linhas de `AGENTS.md` citadas abaixo são da versão anterior. Escopo: especificação em `AGENTS.md` (seções de Fases, Modo simulação, Perfis, Papel, Múltiplas impressoras, Windows e Interface). Nenhum código de produto foi alterado. Nenhum commit. `Cargo.toml`, `Cargo.lock`, `AGENTS.md`, `CLAUDE.md` e `estilo-ui-ux.md` mantêm os mesmos hashes (md5 conferido antes e depois). **Atenção:** o `.gitignore` foi reescrito às 13:56:41, durante a auditoria, por outro processo ou usuário (este agente não o editou). O conteúdo novo acrescenta `/.claude/`, `/.codex/`, `/estilo-ui-ux.md`, `/plan/`, `/AUDIT-*.md` e `/STATUS-MANUAIS.md`, e por isso o `git status` mostra só `AGENTS.md` e `CLAUDE.md`. Não foi revertido; decisão do usuário.

Contagem: **OK 38 · PARTIAL 33 · MISSING 20 · BUG 0 · N/A 2** (93 requisitos)

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
| PS-A003 | Perfis | `gesto_recolocar` com texto e figura para o usuário | PARTIAL (`256e8ce`) | Texto do gesto no cartão e no CLI, pelo mesmo código. Falta a figura. |
| PS-A004 | Perfis | Jato (traseira) inverte a passada 1; laser (gaveta) não | OK | `plots-cli/src/lib.rs:27-33`. Execução: traseira "Passada 1: 4, 2"; gaveta "Passada 1: 2, 4". |
| PS-A005 | Perfis | `passada2_inversa` e `verso_girar_180` ficam "a calibrar", sem presunção | PARTIAL | `plots-cli/src/lib.rs:30`: a traseira já pré-define `passada2_inversa = true`. Não há calibração que confirme isso. |
| PS-A006 | Algoritmo | Total ímpar recebe página em branco no fim, que entra na passada das pares | OK | `plots-core/src/lib.rs:147-148` e `185-191`; teste `uma_folha_tem_frente_par_vazia_e_verso_impar` (lib.rs:512). Execução: 5 páginas, lote 2 traseira, "Passada 1: branca" no lote 2. |
| PS-A007 | Algoritmo | Passada 1 = pares, passada 2 = ímpares, com ordem configurável | OK (cf22592) | `OrdemPassadas` tem os dois valores (`plots-core/src/lib.rs:88-94`), mas CLI e `gerar-pdf` fixam `ParesPrimeiro` (`plots-cli/src/lib.rs:142` e `265`). |
| PS-A008 | Algoritmo | Aplicar inversão e giro de 180 graus (`/Rotate 180` no verso) | OK | `plots-core/src/lib.rs:197-207` e `449-459`; `plots-pdf/src/lib.rs:188-191`. Verificado: traseira com 5 páginas gera passada 1 "BRANCA, P4, P2" e passada 2 "P5, P3, P1", todos com rot=0. |
| PS-A009 | Algoritmo | Função pura `plano_lote(paginas, perfil) -> (Vec<Pagina>, Vec<Pagina>)`, com testes | PARTIAL | A função não existe com esse nome (grep sem resultado). Equivalentes: `plano_documento` (`plots-core/src/lib.rs:135`, retorna `Vec<Lote>`) e `plano_lotes_por_papel` (lib.rs:357). Testes unitários existem. |
| PS-A010 | Algoritmo | Gerar `passada1.pdf` e `passada2.pdf` temporários com lopdf | PARTIAL | Gerados como `lote-NNNN-passada-N.pdf` (`plots-cli/src/lib.rs:278-284`), persistentes, não temporários. Nome diverge da especificação. |
| PS-A011 | Algoritmo | Mostrar o gesto de recolocação e esperar "Imprimir verso" antes da passada 2 | OK (`256e8ce`) | Cartão mostra o gesto e espera "Imprimir verso"; conferido no app com as duas simuladas. |
| PS-A012 | Gesto único | Cartão com a frase fixa e a figura do gesto | PARTIAL (`256e8ce`) | Cartão com a frase do gesto. Falta a figura. |
| PS-A013 | Fluxo | Abrir PDF (arrastar, "Abrir com") e escolher o intervalo de páginas | OK (`ccb1ebc`) | Trecho contínuo por aba até o plano, com os números originais; soltar um PDF abre a aba. O gesto nativo de arrastar não foi feito no teste. |
| PS-A014 | Fluxo | Marcar impressoras e tamanho do lote (padrão 10 folhas) | OK (f6e3433) | Caixas por aba e campo de lote com padrão 10, de 1 a 50. |
| PS-A015 | Fluxo | "Imprimir frente" envia para cada impressora a ordem do seu perfil | PARTIAL (`256e8ce`) | Envio pela janela conferido nas simuladas e na fila de PDF; sem impressora real. |
| PS-A016 | Fluxo | Botão do verso libera só depois que o spooler confirma a frente | OK (`256e8ce`) | Verso só libera com a confirmação do `Job`; o texto avisa que isso não garante que o papel saiu. |
| PS-A017 | Fluxo | Folha de controle opcional no canto do verso | MISSING | Sem ocorrência no código. |
| PS-A018 | Fluxo | "Reimprimir lote" refaz só aquele lote | OK (`256e8ce`) | "Reimprimir lote" no cartão, com confirmação em fila real; falha ao cancelar não reenvia. |
| PS-A019 | Interface | Janela sem menus; barra superior única (ícone, botão da lateral, abas, "+", controles) | OK (353f54d) | Barra sem menus, medidas conferidas no app. |
| PS-A020 | Interface | Splash 480×300, carregamento até 10 s, depois a janela principal | PARTIAL | `bd50a56`: janela, tempos e avisos no app real. Falta a imagem definitiva de 960×600. |
| PS-A021 | Interface | Barra lateral de impressoras com caixa de seleção e seletor de modo (Traseira, Gaveta, "Calibrar…") | OK (826b726) | Lista real com caixa, estado e seletor de modelo: Jato de tinta, Laser e Calibrar. |
| PS-A022 | Interface | Abas por documento com estado próprio; confirmação ao fechar com impressão em andamento | PARTIAL (353f54d) | A aba guarda só a impressão em andamento; faltam impressoras, intervalo e lote. |
| PS-A023 | Interface | Atalhos Ctrl+O, Ctrl+T, Ctrl+W, Ctrl+Tab, Ctrl+B; arrastar PDF abre aba | PARTIAL (353f54d) | Ctrl+W, Ctrl+Tab e Ctrl+B testados no app; seletor e arrasto nativos sem teste real. |
| PS-A024 | Interface | Seleção de impressoras por documento (decidido no AGENTS) | OK (f6e3433) | Cada aba guarda as próprias impressoras; aba nova começa sem marcas. |
| PS-A025 | Interface | Visualizador pdf.js com miniaturas, página e zoom | OK (`b29796e`) | Páginas, miniaturas, página e zoom conferidos no app real com PDFs de teste. |
| PS-A026 | Interface | Tokens da norma de estilo, tema escuro, foco visível, ESC fecha diálogos, sem emoji, `aria-live` para o estado | PARTIAL (f6e3433) | Estado com ponto e texto e `aria-live` na lateral; falta o cartão de impressão. |
| PS-A027 | Interface | Textos em pt-BR e en via `t()` | OK (353f54d) | `t()` em pt-BR e en nesta tela. |
| PS-A028 | Modo simulação | Rodar o fluxo inteiro sem impressora (`simular`) | OK | `plots-cli/src/lib.rs:51-223`. Testes `plots-cli/tests/cli.rs:8-43` e `94-149`. Execução verificada. |
| PS-A029 | Modo simulação | Chave "Simular impressão" por documento; botão vira "Simular frente"; selo SIMULAÇÃO | PARTIAL (`256e8ce`) | Botão "Simular frente" e selo SIMULAÇÃO valem por impressora simulada. Não há chave por documento. |
| PS-A030 | Modo simulação | Duas impressoras virtuais "Simulada — Traseira" e "Simulada — Gaveta", com outras combinações possíveis | PARTIAL | Nomes e perfis em `plots-cli/src/lib.rs:27-47`, só acessíveis via `--impressora`. Não aparecem como filas e não há como criar outra combinação. |
| PS-A031 | Modo simulação | `SimBackend` implementa o mesmo trait dos backends reais | OK | Trait `Backend` em `plots-print/src/job.rs:19-25`; impl em `job.rs:180-208`. |
| PS-A032 | Modo simulação | Modelo físico: folha `{frente, verso, girada}`, pilha de saída, gesto como operação | OK | `plots-print/src/lib.rs:8-24` (`Folha`), `66-72` (`Recolocacao`), `190-215` (`recolocar`). |
| PS-A033 | Modo simulação | "Simular erro": recolocar sem virar, girado, ou com uma folha a menos | OK (cf22592) | O núcleo suporta (`Recolocacao`, `retirar_folhas`, `inverter_ordem`, `girar_180`, teste `falta_de_folha_permite_cancelar_e_reimprimir_lote`, lib.rs:593). O CLI sempre usa `virar_maco: true` (`plots-cli/src/lib.rs:187-190`) e não expõe a opção. |
| PS-A034 | Modo simulação | Vista por lote com passada 1, pilha, passada 2 e folhas finais marcadas certo/errado ("3 de 3 folhas corretas") | PARTIAL | CLI com certa/errada por folha e "N de M folhas corretas" (cf22592); falta a vista na interface. O CLI imprime as listas (`plots-cli/src/lib.rs:155-220`). Não há marcação certo/errado nem contagem. |
| PS-A035 | Modo simulação | Cartão EM ANDAMENTO com "Simular verso" | OK (`256e8ce`) | Cartão EM ANDAMENTO com "Simular verso" e resultado da simulação. |
| PS-A036 | Modo simulação | Exportar `passada1.pdf` e `passada2.pdf` para abrir no Firefox | PARTIAL | `gerar-pdf` gera os PDFs (`plots-cli/src/lib.rs:225-310`). Nomes diferentes. Abertura no Firefox não verificada. |
| PS-A037 | Modo simulação | Exportar PDF de folhas finais (frente e verso lado a lado) | OK (6baa24a) | `gerar-pdf --folhas-finais`, conferido com PyMuPDF; sem conferência no Firefox. |
| PS-A038 | Modo simulação | Modelo com `proptest`: 8 perfis, 1 a 200 páginas, lotes de 1 a 50 folhas, maço final 1\|2, 3\|4 | OK (6baa24a) | proptest com tabela fixa de 8 perfis; só traseira e gaveta correspondem às impressoras simuladas, e o modelo da gaveta será revisto. |
| PS-A039 | Arquitetura | App Tauri 2 com webview e pdf.js | OK (`b29796e`) | App Tauri 2 com webview e pdf.js em worker, sem afrouxar a CSP. |
| PS-A040 | Arquitetura | Camada de backends: trait com duas implementações (Windows e Simulado) | OK (126ac26) | `BackendWindows` e `SimBackend` implementam o mesmo trait. |
| PS-A041 | Arquitetura | lopdf para reordenar, extrair, inserir página em branco e `/Rotate` | OK | `plots-pdf/src/lib.rs:112-244`. Testes `reabre_ordem_heranca_rotacao_e_branco` (lib.rs:630) e outros. Verificado pelos PDFs gerados. |
| PS-A042 | Arquitetura | pdfium-render para rasterizar (impressoras que não aceitam PDF) | PARTIAL (126ac26) | `pdfium-render` rasteriza na resolução da fila (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A043 | Arquitetura | Linux e macOS com IPP/CUPS ou `lp` | MISSING | Fase futura. AGENTS decide que o CUPS fica para depois do backend Windows. |
| PS-A044 | Arquitetura | tokio, um job por impressora em paralelo | MISSING | Fase 5. |
| PS-A045 | Arquitetura | Config com serde, toml e directories; perfis em `%APPDATA%/plots/perfis.toml` | PARTIAL | Biblioteca `plots-perfis` (72030fb); falta usar no CLI e na interface. |
| PS-A046 | Arquitetura | Erros com thiserror e logs com tracing | PARTIAL | Erros escritos à mão (`plots-core/src/lib.rs:107-128`, `plots-pdf/src/lib.rs:8-33`). Sem thiserror nem tracing. |
| PS-A047 | Arquitetura | Frontend React + TypeScript, CSS Modules, lucide-react, Radix | OK (353f54d) | React, TypeScript, CSS Modules, lucide e Radix. |
| PS-A048 | Windows | Impressão só pelo PlotS, sem diálogo, envio direto ao spooler | PARTIAL (126ac26) | Envio direto, sem diálogo (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A049 | Windows | Rasterizar na resolução da impressora (GetDeviceCaps); DEVMODE com simplex, sem agrupar, bandeja manual e tamanho | PARTIAL (126ac26) | Resolução por `GetDeviceCaps`; `DEVMODE` forçado e conferido; bandeja fica a do driver (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A050 | Windows | Acompanhar o trabalho no spooler (GetJob ou notificação) | PARTIAL (126ac26) | `GetJobW` com avisos; nenhum estado real observado (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A051 | Windows | Detectar a opção "inverter ordem" do driver e pedir para desligar | MISSING | Fase 3 (calibração). |
| PS-A052 | Windows | Fila virtual "PlotS" que recebe o PDF de qualquer programa | MISSING | Fase 5. |
| PS-A053 | Papel | Ler o tamanho de cada página do PDF | OK | `plots-pdf/src/lib.rs:71-108` (`dimensoes_paginas`). Verificado com `rotacionada.pdf`. |
| PS-A054 | Papel | Reconhecer A4, A5, Carta, Ofício, Legal, com tolerância de 2 mm | OK | `plots-core/src/lib.rs:297-328`. Testes `reconhece_formatos_orientacao_e_personalizado_com_tolerancia` (lib.rs:624) e `valida_par_frente_verso...` (lib.rs:655). Verificado: `tolerancia-par.pdf` (598×844) aceito. |
| PS-A055 | Papel | Formato "Outro" com `DMPAPER_USER` | PARTIAL (126ac26) | Medidas personalizadas pelo `DEVMODE`, com conferência (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A056 | Papel | Documento com tamanhos misturados dividido em lotes por tamanho | OK | `plots-core/src/lib.rs:357-462`. Verificado: `mistura-papel.pdf` gera lote 1 A4 e lote 2 A5 (`BRANCA` com 420×595). |
| PS-A057 | Papel | Cartão avisa a troca de papel | OK (`256e8ce`) | Cartão e CLI avisam a troca de papel. |
| PS-A058 | Papel | Frente e verso de tamanhos diferentes são recusados antes de imprimir | OK | `plots-core/src/lib.rs:385-397`. Verificado: `personalizado-par.pdf` (595×842 + 200×300) sai com "as páginas 1 e 2 têm tamanhos incompatíveis para a mesma folha", exit 2. Teste `rejeita_paginas_de_tamanhos_incompativeis_na_mesma_folha` (`plots-cli/tests/cli.rs:260-281`). |
| PS-A059 | Papel | Frente e verso da mesma folha com mesma orientação (retrato e paisagem) | OK (882ba68) | Ver BUG-1. Orientação diferente é aceita sem aviso. Decisão do usuário: bloquear ou só avisar. |
| PS-A060 | Papel | Opção "Ajustar ao papel" (imprimir Carta em A4) | MISSING | Não existe no código. |
| PS-A061 | Múltiplas | Marcar várias impressoras e enviar a mesma origem, cada uma com o perfil próprio | MISSING | CLI aceita uma impressora virtual por comando. |
| PS-A062 | Múltiplas | Descoberta das filas do sistema (EnumPrintersW) | PARTIAL (a9f9185) | `plots filas` lista as filas e o estado com `EnumPrintersW`; falta usar na interface. |
| PS-A063 | Múltiplas | Perfil por fila: chave com o nome da fila e apelido amigável | PARTIAL | Chave pelo nome exato da fila e apelido em `plots-perfis` (72030fb); falta a tela de apelido. |
| PS-A064 | Múltiplas | Máquina de estados por job: Ocioso, FrenteEnviada, AguardandoRecolocacao, VersoEnviado, Concluído, Erro | PARTIAL | `plots-print/src/job.rs:28-35` usa `Pronto` no lugar de `Ocioso`. Demais estados existem. |
| PS-A065 | Múltiplas | Modos espelho e divisão | PARTIAL | `plots-core/src/distribuicao.rs:52-80`, testes em lib 86-124. Não é usado por CLI nem UI. |
| PS-A066 | Múltiplas | Consulta periódica do spooler; avisa papel acabado ou atolado | MISSING | Fase 5. |
| PS-A067 | Múltiplas | Grupos salvos de impressoras ("Todas do escritório") | MISSING | Fase 5. |
| PS-A068 | Fase 1 | Testes unitários: lotes pares, ímpares, 1 folha e as 8 combinações de perfil | OK | `plots-core/src/lib.rs:512-586`. `cargo test` passa. |
| PS-A069 | Fase 1 | Crate `plots-core` com lógica pura, sem E/S | OK | `plots-core/src/` (sem dependências externas em `plots-core/Cargo.toml`). |
| PS-A070 | Fase 2 | PDF de teste de calibração com números grandes e setas | OK (cf22592) | Não há geração de texto em `src/`. `.qa-pdf/make.py` é um script de QA com PyMuPDF, fora do repositório, sem integração. |
| PS-A071 | Fase 2 | Calibração: 2 folhas com páginas 1 a 4, duas perguntas e ajuste do perfil | PARTIAL | `ajustar_perfil_por_calibracao` (`plots-core/src/lib.rs:78-86`, teste 469-493). Falta o PDF, as perguntas e o comando. |
| PS-A072 | Fase 2 | Os PDFs gerados abrem no Firefox com ordem e rotação certas | OK | O usuário abriu no Firefox as passadas e as folhas finais de um documento de 4 páginas: ordem e rotação certas (2026-11-02). |
| PS-A073 | Fase 2 | Backend Simulado com impressoras virtuais | OK | `SimBackend` (`plots-print/src/job.rs:159`) e `plots simular`. |
| PS-A074 | Fase 3 | `plots imprimir doc.pdf --impressora X --lote 10` | PARTIAL (126ac26) | `plots imprimir` envia com confirmação (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A075 | Fase 3 | `plots calibrar X` | PARTIAL | `plots calibrar` faz as perguntas e grava o perfil (92772e5); falta enviar e confirmar em impressora real. |
| PS-A076 | Fase 3 | Backend CUPS ou Windows | PARTIAL (126ac26) | Backend Windows pronto (sem impressora real; conferido só na fila de PDF do Windows). |
| PS-A077 | Fase 3 | Critério: simulação correta nas duas impressoras virtuais | PARTIAL | `plots simular` está correto (PS-A028). O critério completo depende de `imprimir` e `calibrar`. |
| PS-A078 | Fase 3 | Critério: 3 lotes seguidos sem erro numa impressora real | MISSING | Não verificável nesta máquina: não há impressora real e nada foi enviado a impressoras. Depende de decisão do usuário. |
| PS-A079 | Fase 4 | Interface Tauri + pdf.js: visualizador, painel de impressoras, cartões com figura, fluxo sem terminal | PARTIAL (353f54d) | Só a janela; faltam visualizador, impressoras e cartões. |
| PS-A080 | Fase 5 | Várias impressoras, jobs paralelos, espelho e divisão, status do spooler, fila virtual "PlotS" | MISSING | Bloco. Só o núcleo de distribuição existe (PS-A065). |
| PS-A081 | Fase 6 | Instalador (tauri bundle), reimprimir lote, folha de controle, histórico | MISSING | Bloco. |
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

## Bugs (com passos de reprodução)

### BUG-1 (PS-A059): frente retrato com verso paisagem é aceita sem aviso

- Causa: `plots-core/src/lib.rs:386-387` compara só `formato` (que ignora orientação). `lib.rs:389-390` compara as dimensões já normalizadas para retrato, então A4 retrato e A4 paisagem parecem iguais. A checagem de papel por lote (`lib.rs:402-412`) compara a orientação só com a primeira folha do lote, nunca com o verso.
- Repro: criar um PDF de 2 páginas, página 1 com 595×842 e página 2 com 842×595.
- Comando: `cargo run -q -p plots-cli --locked -- gerar-pdf <arquivo> --lote 10 --impressora gaveta --saida <pasta_vazia>`
- Resultado observado: exit 0, "Gerados 2 PDFs", sem aviso. `lote-0001-passada-1.pdf` tem a página 2 em 842×595, e `lote-0001-passada-2.pdf` tem a página 1 em 595×842. Mesmo caso com `/Rotate 90` na página 2.
- Esperado (AGENTS, Papel): "Frente e verso ... nunca ficam com tamanhos diferentes; se o PDF tiver isso, o PlotS avisa antes de imprimir." Decisão do usuário: orientação diferente conta como tamanho diferente? Recomendação: sim, e recusar com a mesma mensagem de `ParIncompativel`.

### BUG-2 (PS-A090): mensagem enganosa para comandos que não existem

- Repro: `cargo run -q -p plots-cli --locked -- imprimir doc.pdf --impressora "Microsoft Print to PDF" --lote 10`
- Resultado: "Erro: PDF não é aceito: esta versão do CLI só executa simulações numéricas." (exit 2). O CLI aceita PDF em `gerar-pdf`, e `imprimir` ainda não existe.
- Esperado: dizer que `imprimir` ainda não existe e indicar `gerar-pdf` ou `simular`.
- Causa: `plots-cli/src/lib.rs:73-80` (ramo genérico) e `93-95` (ramo de `simular`). O teste `rejeita_pdf_com_mensagem_explicita` (`plots-cli/tests/cli.rs:46-55`) fixa a frase antiga.
- Também: `plots calibrar X` responde "comando ausente ou desconhecido", sem dizer que o comando é previsto.

### BUG-3 (PS-A091): texto de uso incompleto e sempre o mesmo

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

## Lotes propostos (ordem de prioridade)

### Lote 1: bugs e textos (sem perda de dados encontrada)
- Corrigir BUG-1: checar orientação na mesma folha, com teste em `plots-core` e teste de integração no CLI. Decisão do usuário: bloquear ou apenas avisar.
- Corrigir BUG-2 e BUG-3: texto de uso único (`main.rs`), mensagem de comando inexistente, uso por tipo de erro, comentários sem caracteres corrompidos, doc-comment em `job.rs`.
- Adicionar testes que faltam: `--help` com `gerar-pdf`, opção repetida, PDF criptografado pelo CLI, orientação mista.
- Atualizar as frases de `AGENTS.md` listadas acima. Decisão do usuário: quem edita a especificação.

### Lote 2: conclusão da Fase 2
- Fechar a integração de `gerar-pdf` e registrar a validação (PS-A072): abrir as passadas em Firefox ou pdf.js e registrar a ordem.
- Gerar o PDF de calibração com números grandes e setas (PS-A070). Decisão do usuário: layout, fonte e tamanho do papel de teste.
- Exportar PDF de folhas finais lado a lado (PS-A037). Decisão do usuário: layout.
- Expor `passada1.pdf` e `passada2.pdf` com os nomes da especificação, ou aceitar `lote-NNNN-...`. Decisão do usuário.
- Opção `--ordem` no CLI (PS-A007) e `--simular-erro` (PS-A033). Decisão do usuário: se a opção entra agora.
- Cobrir as 8 combinações com `proptest` (PS-A038). Exige aprovação para adicionar dependência, porque `Cargo.toml` não pode ser alterado sem pedido.
- Vista de simulação com certo/errado (PS-A034).

### Lote 3: Fase 3 (CLI de impressão e calibração)
- `plots calibrar X` usando `ajustar_perfil_por_calibracao` e o PDF de calibração (PS-A074 a PS-A075).
- `plots imprimir` com o backend Simulado primeiro, depois Windows (PS-A048 a PS-A050). Decisão do usuário: o backend Windows exige testes com a fila "Microsoft Print to PDF" ou com impressora real.
- Detecção da opção "inverter ordem" do driver (PS-A051). Depende de impressora real.
- Perfis em `%APPDATA%/plots/perfis.toml` com serde, toml e directories (PS-A045). Exige aprovação para adicionar dependências.
- Ajustar ao papel e mapeamento DMPAPER (PS-A055, PS-A060). Decisão do usuário: quais formatos entram.
- Validação de 3 lotes em impressora real (PS-A078). Decisão do usuário e depende de impressora real.

### Lote 4: Fases 4 a 6 e interface (ainda não existem)
- Scaffold Tauri 2 com `src-tauri/` e `ui/`, pdf.js, tokens da norma de estilo e pt-BR/en (PS-A019 a PS-A027, PS-A039, PS-A047). Decisão do usuário: prioridade e ordem das telas.
- Multi-impressora, jobs paralelos, espelho e divisão na interface, fila virtual "PlotS" (PS-A061 a PS-A067, PS-A052, PS-A080). Decisão do usuário.
- Instalador, folha de controle, histórico e reimprimir lote (PS-A017, PS-A081). Decisão do usuário.

## Não verificado

- Abertura e ordem visual das passadas no Firefox ou no pdf.js. Foi verificado só com pypdf (texto e caixas de página).
- Efeito real da página em branco com ponto no driver (PS-A087).
- Qualquer envio a impressora, física ou virtual. Não foi feito, conforme a instrução. A fila "Microsoft Print to PDF" não foi usada.
- Interface gráfica, capturas de tela e teclas. Não existe app gráfico.
- Comportamento com PDFs reais grandes (o teste usou um PDF de 300 páginas gerado para o caso, em 1,5 s).
- `plots-cli` lendo PDFs com formulários e anotações do mundo real: o código tem tratamento e testes, mas não foi exercitado com PDFs externos.
- Compatibilidade do `.claude/team.json` com `cargo test --workspace` pelo PowerShell (rodado só via bash, com `--locked`).

## Limpeza

- Pasta de auditoria: `E:\PlotS\target\audit\run-135346` (com PDFs de teste, saídas, scripts e logs). Apagada ao final. Não houve `cargo clean`. `E:\PlotS\target\` foi mantido, conforme a instrução do usuário.
- Arquivos de sessões anteriores que não foram tocados: `E:\PlotS\.qa-pdf\` (entrada.pdf, saida.pdf, saida-1..3.png, make.py, src) e `codex-session-*.md`. Sugestão de limpeza, sem apagar: decisão do usuário.
