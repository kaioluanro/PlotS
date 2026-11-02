# fase2-calibracao: PDF de calibração, opções de simulação e cobertura com proptest

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master`
(`git merge --ff-only master` primeiro; base 965ed00). Faça commits wip; nunca dê push nem squash.
Não altere nada em E:\PlotS fora do seu worktree.

## Ambiente
- **Build:** deixe CARGO_TARGET_DIR sem valor, para usar `target/` dentro do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- **Antes do primeiro build:** rode `touch plots-cli/src/lib.rs` e confira que as linhas de compilação apontam para o worktree.
- **Pouca memória:** se o sistema matar o build, repita o comando uma vez; se morrer de novo, apague `target/debug/incremental` e tente mais uma vez. Só então relate.
- **Rede:** permitida só para baixar `proptest` e suas dependências, com `CARGO_NET_OFFLINE=false`.
- **CI:** depois da última edição, rode uma vez em primeiro plano `cargo fmt --all --check` e depois `cargo test --workspace --locked`, gravando as saídas em `target/ci.log`, seguidas de `CI_EXIT <código>`. O coordenador repete o CI ao receber o lote; não escreva `CI_SUCCESS` à mão. Nunca faça wip depois de um CI vermelho.
- **Tempo de teste:** a suíte atual leva cerca de 30 s. O novo teste com proptest não pode somar mais de 30 s em build de debug.
- **Log de falha prévia:** `target/fail-before.log`, com execuções reais. Classifique cada teste novo em: falha prévia registrada, API nova sem execução na base, ou guarda.
- **Limpeza** (seção "Limpeza constante do disco" em E:\PlotS\AGENTS.md): apague PDFs, imagens e pastas temporários do teste de uso depois de examiná-los e liste no relatório o que removeu. Mantenha `target/` do worktree: o coordenador o remove junto com o worktree.
- **Dependência:** só `proptest`, como dev-dependency de `plots-print`. Commite o Cargo.lock atualizado. Nenhuma outra.

## Navegação no código
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; depois leia só as linhas localizadas e confirme no worktree. Não rode `graphify update`.
- `plots-cli/src/lib.rs`: `executar_com_io`, `gerar_pdfs`, `ImpressoraVirtual`, `USO_*`
- `plots-print/src/lib.rs`: `Simulador`, `Recolocacao`, `Mecanica`, `executar_lote`, laços que cobrem as combinações de perfil
- `plots-print/src/job.rs`: `Job`, `SimBackend`
- `plots-pdf/src/lib.rs`: como as páginas e os PDFs são gravados
- `plots-core/src/lib.rs`: `OrdemPassadas`, `plano_documento`, `Papel`, formatos

## Segurança de processos
Encerre só os seus próprios PIDs, por número exato. Se uma ação for negada, pare e relate; não peça a outro que a faça. Não envie nada a nenhuma impressora, nem à "Microsoft Print to PDF".

## Fontes (somente estas)
E:\PlotS\AGENTS.md, seções "Perfis de impressora e lógica de ordenação", "Modo simulação" e "Fases de implementação" (calibração). Não abra E:\PlotS\estilo-ui-ux.md nem E:\PlotS\codex-session-*.md.

## Regras sobre identificadores do plano
Comentários, testes e mensagens descrevem comportamento com palavras próprias: nada de IDs de requisito, nome de lote, "auditoria" ou "brief".
**Autoverificação, não pode imprimir nada:** `sh E:/PlotS/.claude/team/selfcheck.sh 965ed00 plots-cli plots-core plots-print plots-pdf`

## Escopo
Primeiro registre no relatório o que existe hoje para cada item.

1. **PDF de calibração.** Em `plots-pdf`, função pura que devolve os bytes de um PDF de 4 páginas, numeradas 1 a 4, no papel pedido. Em cada página: o número grande no centro, com altura de cerca de 40% da página, e uma seta para cima acima dele, para revelar a rotação. Desenhe tudo com caminhos vetoriais, sem fonte. Novo comando: `plots gerar-calibracao --saida DIRETORIO [--papel a4|a5|carta|oficio|legal]`, com A4 como padrão. Grava `calibracao.pdf`, nunca sobrescreve (erro claro se já existir) e diz na saída que nada foi enviado à impressora.
2. **`--ordem pares|impares`** em `simular` e `gerar-pdf`, com `pares` como padrão. Valor inválido ou repetido dá erro claro. Com `impares` em `simular`, as folhas finais continuam corretas nas duas impressoras virtuais; se o perfil atual não garantir isso, ajuste a escolha de perfil da impressora virtual para aquela ordem e cubra com teste. Não faça uma correção que só esconda o erro.
3. **`--simular-erro sem-virar|girado|folha-a-menos`** em `simular`. Aplica o gesto errado na recolocação usando o modelo físico existente, sem nova lógica de resultado, e relata o resultado real em vez de abortar sem explicação. Falta de folha aparece como erro de lote seguido da reimpressão correta, usando o caminho de cancelar e reiniciar que já existe.
4. **Certo ou errado por folha.** A saída de `simular` diz, por folha, `certa` ou `errada`, como texto, e por lote `N de M folhas corretas`. Folha certa: frente e verso formam o par esperado e o verso não está de cabeça para baixo. O julgamento vem do estado do simulador. Código de saída 0 em simulação concluída, mesmo com folhas erradas, pois mostrar o erro é o objetivo.
5. **Propriedade com proptest.** Em `plots-print`: para os 8 perfis, 1 a 200 páginas e lotes de 1 a 50 folhas, combinando cada perfil com uma mecânica física coerente, o maço final é 1|2, 3|4... sem folha de cabeça para baixo. Mantenha claro no teste como os perfis se ligam às mecânicas e não descarte casos em silêncio: se algum perfil não tiver mecânica coerente, relate. Falhas persistidas de proptest não são commitadas; desligue a persistência ou mantenha fora do git.
6. **Uso e ajuda.** `USO_*` e `--help` cobrem as opções novas e o novo comando. Quem usa a ordem `impares` ou um gesto errado vê isso indicado na saída.

## Regras
- **Testes:** testes de CLI com entrada e saída para os itens 1 a 4 e 6, verificando o PDF de calibração com `lopdf` (4 páginas, MediaBox do papel pedido, conteúdo diferente entre páginas, não sobrescrever) e cada gesto errado com o resultado exato esperado.
- **Falha prévia:** registre a falha antes de implementar, quando houver base para isso.
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes em código novo; trabalho limitado por `LIMITE_PAGINAS`.
- **Textos:** CLI em pt-BR. Liste todos os textos novos. Sem i18n nesta etapa.

## Teste de uso (binário real)
Com `cargo run -p plots-cli --locked -- ...`, em uma pasta temporária dentro de `target/`: rode cada comando e opção novos e cole no relatório comando, saída e código de saída. Renderize as 4 páginas de `calibracao.pdf` para PNG com PyMuPDF (`python -I`) e leia as imagens com sua ferramenta de leitura, relatando o que vê em cada uma. Se o PyMuPDF não estiver instalado, diga isso no relatório e não o instale.

## Relatório
O que existia antes; cada item com testes e grupo de falha prévia; linha do CI, conferência de caminhos e autoverificação vazia; textos novos; quanto tempo o teste com proptest leva; o que foi removido na limpeza; o que não foi verificado. Termine com tudo em commits wip e `git status` limpo. Informe o caminho do worktree e o nome do branch.
