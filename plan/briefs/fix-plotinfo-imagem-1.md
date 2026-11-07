# plotinfo-imagem fix round 1

**Worktree:** `E:\PlotS\.claude\worktrees\agent-plotinfo-imagem`, HEAD `6a0fd87` (wip). Trabalhe
direto nele e não crie outro worktree. Leia primeiro `plan/briefs/brief-plotinfo-imagem.md`; as
regras de ambiente, memória, segurança de processo, fontes e self-check continuam valendo.
Termine com um wip commit, só depois do CI verde; nunca faça push nem squash.

Logs: CI `C:\Users\Kaio Luan\AppData\Local\plots-build\plotinfo-imagem\saidas\ci-fix1.log`;
falha-antes `...\saidas\falha-antes-fix1.log`. Não há tela nova; as mudanças de texto são cobertas
por teste.

## Required (code review), in order

1. **Consultar o cache antes de converter.** `src-tauri/src/documento.rs:142-148`: hoje
   `pdf_de_imagem(&bytes, …)` roda sempre e `gravar_derivado` só evita a escrita quando o arquivo
   existe. Isso decodifica e recomprime a imagem em cada `abrir_documento`, `contar_paginas`,
   `ler_documento` e por face na impressão. Calcule o caminho do derivado a partir do hash antes
   de chamar `pdf_de_imagem` e volte cedo quando o arquivo existir e não estiver vazio. Só
   decodifique quando faltar o cache. Reforce
   `documento::testes::derivado_e_reaproveitado_no_cache` para provar que a segunda abertura não
   reescreve o derivado (por exemplo, grave um comentário/sentinela no PDF derivado depois da
   primeira abertura e exija que a segunda o preserve) — o teste atual só compara `modified` e
   passa mesmo com reconversão. Registre a falha-antes no log.
2. **Cobrir a conversão pelo mutex de memória.** `src-tauri/src/documento.rs:146` roda
   `abrir_para_leitura_em` fora de `CARGA_EM_ANDAMENTO`, que só é tomado em `:312`. O comentário
   de `:298-300` deixou de valer para imagem: prévia e contagem podem converter duas imagens
   grandes ao mesmo tempo. Coloque a validação/derivação dentro do mesmo mutex (com uma função
   interna sem lock para evitar deadlock quando `carregar_validado_em` já o segura) e garanta que
   `abrir_para_leitura`/`abrir`, `carregar_validado_em` e `ler_pdf` usem esse caminho. Mantenha o
   tratamento de lock envenenado e não bloqueie a thread da interface (o comando já é async em
   `spawn_blocking`).

## Optional, do it

3. Inclua uma constante de versão do formato do derivado no hash do cache, para que uma mudança
   em `plots-pdf/src/imagem.rs` invalide derivados antigos. Sem renomear o arquivo de forma
   incompatível; só acrescente a versão ao material do hash.
4. `ui/src/i18n.ts` (`cartao.motivo.pdf`) + `ui/src/cartoes.ts`: uma falha de preparação de
   impressão de uma imagem ainda diz “Não foi possível preparar este PDF.”. Troque por um texto
   neutro (“documento”) nos dois idiomas e ajuste o teste que citar a frase.
5. `plots-pdf/src/imagem.rs:251`: o teto de alocação de 512 MiB recusa RGB de 16 bits a 100 MP
   (~600 MB), embora o limite declarado seja 100 MP. Ajuste o teto para caber o limite (por
   exemplo 768 MiB) ou documente a diferença no comentário; o comportamento não pode passar a
   aceitar mais pixels do que o limite declarado.
6. `plots-pdf/src/imagem.rs:53-54`: o comentário de `FormatoNaoSuportado` fala de conteúdo que
   não corresponde à extensão, mas bytes de outro formato caem em `Decodificacao`. Corrija o
   comentário para descrever o comportamento real.

## CI e relatório

Rode `cargo fmt --all -- --check` e o comando de CI do `.claude/team.json` uma vez, no
foreground, depois da última edição, com log no caminho acima; precisa terminar em
`CI_SUCCESS: cargo test --workspace` e `CI_EXIT 0`, com todas as linhas de build no worktree.
Rode também `npm test` em `ui/` se mexer em texto de interface (com `npm ci`), e registre. Rode o
self-check e garanta que ele não imprime nada.

Relatório: cada item feito com arquivo:linha; testes e grupos de falha-antes; strings novas ou
alteradas; linha do CI, conferência de caminhos e self-check; retries; o que não foi verificado.