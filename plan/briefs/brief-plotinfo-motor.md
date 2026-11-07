# plotinfo-motor: seleção, cobertura, custo e perfis de tinta

Trabalhe no worktree isolado que o coordenador indicar, criado a partir do `master` que contém
este brief. Faça commits wip; nunca faça push e nunca faça squash.

## Ambiente

- **Build:** `CARGO_TARGET_DIR=%LOCALAPPDATA%\plots-build\plotinfo-motor\target`,
  `CARGO_INCREMENTAL=0`, `-j 4`. Um comando pesado por vez.
- **CI:** depois da última edição, rode o comando de `.claude/team.json` uma vez, no foreground,
  com log em `saidas/ci.log`; o log precisa terminar em
  `CI_SUCCESS: cargo test --workspace` e `CI_EXIT 0`.
- **Falha-antes:** `saidas/falha-antes.log`, com execuções reais e a saída. Classifique cada teste
  novo em: falha-antes registrada, API nova sem execução na base, ou guarda.
- **Saídas:** `saidas/`. Não há tela nova; não há captura de UI neste lote.

## Navegação de código

Antes do primeiro grep ou leitura de código:
`graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.
Confirme cada hit no worktree e leia só as linhas localizadas. Ao terminar, `graphify update .`.

Comece por:
- `Cargo.toml` — membros do workspace.
- `plots-core/src/lib.rs` — `DimensoesPagina`, `LIMITE_PAGINAS`, `LIMITE_COPIAS`.
- `plots-perfis/src/armazenamento.rs` — `caminho_padrao`, escrita atômica e limite de arquivo.
- `plots-perfis/src/lib.rs` — o que é público.
- `src-tauri/src/main.rs` — registro de comandos.
- `src-tauri/src/preferencias.rs` — modelo de comando async com `spawn_blocking` e erros.
- `src-tauri/Cargo.toml`.

## Segurança de processo

- Mate só os seus PIDs, por número exato. Nunca mate todas as instâncias do PlotS nem processos do
  usuário. Não deixe app rodando.
- Se uma ação for negada, pare e reporte; não peça a outra pessoa para fazer.

## Fontes (só estas)

- `AGENTS.md`, `.claude/team.json`, `.claude/skills/craft-team/SKILL.md`,
  `.claude/agents/plots-implementer.md`.
- `plan/30-plotinfo.md` e este brief.
- Os arquivos listados acima.
- Não leia `PlotInfo/`; o plano 30 traz o recorte aprovado.

## Regras de plano

- Comentários, nomes de teste e mensagens descrevem comportamento com nossas palavras. Nada de
  ids de requisito, sinais de seção, “manual”, “lote” ou “próximo lote”.
- **Self-check, precisa não imprimir nada:**
  `sh .claude/team/selfcheck.sh <base-sha> plots-info/src src-tauri/src`. Rode também nos arquivos
  novos não rastreados.

## Escopo

Primeiro registre no relatório o que o app faz hoje: não existe estimativa de tinta, perfil de
tinta nem seleção de páginas do painel.

1. **Crate `plots-info`**, sem I/O e sem pdfium/mutool:
   - `selecao.rs`: interpreta o texto do painel.
     - vazio = documento inteiro;
     - `1`, `2,5,7,9`, `5-16`, `1,3,5-9,12`; vírgulas separam, faixa inclusiva;
     - espaços em volta ignorados; texto máximo de 4096 bytes; no máximo 10.000 números
       distintos;
     - normaliza em ordem crescente e remove repetidas;
     - erros próprios para item vazio, item não numérico, página fora de `1..total`, faixa
       invertida e faixa fora do documento; nada de panic;
     - `quantidade()` e os números selecionados.
   - `cobertura.rs`:
     - `cobertura_de_bgra(pixels, largura, altura, passo, cinzas_so_preto)` em percentual por
       canal C/M/Y/K;
     - fórmula simples `K = 1 - max(R,G,B)`, `C/M/Y` como no plano;
     - `cinzas_so_preto` zera C/M/Y quando R=G=B;
     - `cobertura_de_pam(bytes)` lê PAM `P7`, `DEPTH 4`, `MAXVAL 255`, valida
       `WIDTH`/`HEIGHT`/tamanho dos dados; recusa profundidade ou `MAXVAL` diferente sem panic;
     - arredondamento/limites que não estourem.
   - `perfil.rs`:
     - `PerfilDeTinta` com `id`, `nome`, `moeda`, `volumes_ml[4]`, `rendimentos_paginas[4]`,
       `precos_ml[4]`, `fatores_qualidade[3]`;
     - validação: id/nome não vazios, moeda ISO de 3 letras maiúsculas, volumes/rendimentos
       positivos, preços não negativos, fatores positivos, limites superiores razoáveis;
     - dois perfis padrão (cartucho e tanque) com números coerentes;
     - busca por id.
   - `configuracao.rs`: perfil ativo, qualidade (Rascunho/Normal/Alta), dpi (150/300), modo
     (CMYK/Simples), cinzas só com preto, tamanho da imagem (A4/DPI); validação.
   - `custo.rs`:
     - `mL_c = cobertura_c_percentual × (largura_mm × altura_mm / 62370) × (V_c / (5 × P_c)) × f`;
     - `custo = mL × preco_ml`;
     - página, seleção × cópias, documento × 1 cópia;
     - cartucho: percentual consumido e cópias até esvaziar (a cor que acaba primeiro);
     - página mais cara pela soma dos canais;
     - cópias de 1 a 999;
     - nada de divisão por zero; perfil inválido é recusado.
   - Reexporte os tipos em `lib.rs`.
   - Dependências: `plots-core`, `serde` derive e `thiserror`, nas versões do workspace.
2. **Persistência em `src-tauri/src/plotinfo.rs`**:
   - arquivo `plotinfo.toml` em `plots_perfis::caminho_padrao()?.with_file_name("plotinfo.toml")`,
     para o portátil continuar ao lado do `PlotS.exe`;
   - esquema versionado `versao = 1`, `[configuracao]` e `[[perfis]]` como no plano 30;
   - leitura: ausente ou corrompido cai nos perfis padrão e devolve um aviso, sem panic; arquivo
     acima de 1 MiB é recusado;
   - gravação atômica (temporário + rename), sem deixar temporário para trás;
   - valida o perfil antes de gravar; id repetido é recusado ou substituído de forma explícita;
   - comandos Tauri, em `spawn_blocking`:
     `perfis_de_tinta`, `salvar_perfil_de_tinta`, `apagar_perfil_de_tinta`,
     `configuracao_de_tinta`, `definir_configuracao_de_tinta`;
   - registre os comandos em `main.rs`;
   - `src-tauri/Cargo.toml` ganha `plots-info` e `toml = "=1.1.6"`.
3. **Sem UI neste lote.** Nada de painel, ícone, gaveta, textos de interface ou CSS. As strings
   de erro da interface ficam para o lote do painel; aqui os erros são códigos/tipos Rust.

## Regras

- **Testes:**
  - seleção: `1`, `2,5,7,9`, `5-16`, `1,3,5-9,12`, vazio, repetidas, fora do total, faixa
    invertida, item vazio, item não numérico, texto enorme;
  - cobertura: branco 0%; preto K 100%; quadrado preto em 25% = K 25%; ciano puro C 100%;
    cinza 50% com e sem “cinzas só com preto”; PAM sintético com `[102,0,0,0]` = C 40%;
    PAM de profundidade errada recusado; BGRA com passo maior que a largura;
  - custo: 5% de cobertura em A4 = V/P mL; A3 = 2× A4; qualidade 0,6/1,0/1,3; preço por mL;
    cartucho e cópias até esvaziar;
  - perfis: validação e padrões;
  - persistência: arquivo ausente, round-trip, corrompido, grande demais, gravação atômica e
    nenhum temporário sobrando;
  - guarda: `cargo test --workspace` continua passando.
- **Nunca crashar:** texto e bytes são hostis; use `checked_*`, limites e validação antes de
  alocar. Nada de índice direto sem checar.
- **Escopo proibido:** não mexa em `plots-core`, `plots-pdf`, `plots-windows`, no PlotSim, no
  painel, no zip, nos perfis do usuário ou nas filas dele. Não implemente mutool nem leitura de
  PDF neste lote.

## Teste de uso

Não há tela nova. A prova é a suíte: rode `cargo test -p plots-info`, os testes de persistência e
o CI completo. Registre no relatório os casos exatos e os números obtidos.

## Relatório

- O que o app fazia antes.
- Cada item do escopo: feito, parcial ou pulado, com o porquê.
- Testes, cada um com seu grupo de falha-antes.
- Arquivos alterados e dependências novas.
- Linha do CI, conferência de caminhos e self-check vazio.
- Retries e o que não foi verificado.