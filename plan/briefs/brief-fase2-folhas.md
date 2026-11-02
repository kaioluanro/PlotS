# fase2-folhas: PDF de folhas finais, validação no Firefox e tabela explícita de mecânicas

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master`
(`git merge --ff-only master` primeiro; base a261772). Faça commits wip; nunca dê push nem squash.
Não altere nada em E:\PlotS fora do seu worktree.

## Ambiente
- **Build:** deixe CARGO_TARGET_DIR sem valor, para usar `target/` dentro do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- **Antes do primeiro build:** rode `touch plots-cli/src/lib.rs` e confira que as linhas de compilação apontam para o worktree.
- **Pouca memória:** se o sistema matar o build, repita o comando uma vez; se morrer de novo, apague `target/debug/incremental` e tente mais uma vez. Só então relate.
- **CI:** depois da última edição, rode uma vez em primeiro plano `cargo fmt --all --check` e depois `cargo test --workspace --locked`, gravando as saídas em `target/ci.log`, seguidas de `CI_EXIT <código>`. O coordenador repete o CI; não escreva `CI_SUCCESS` à mão. Nunca faça wip depois de um CI vermelho.
- **Log de falha prévia:** `target/fail-before.log`, com execuções reais. Classifique cada teste novo em: falha prévia registrada, API nova sem execução na base, ou guarda.
- **Limpeza** (seção "Limpeza constante do disco" em E:\PlotS\AGENTS.md): apague PDFs, imagens, perfil do Firefox, `node_modules` e pastas temporárias depois de examiná-los, e liste no relatório o que removeu. Mantenha `target/` do worktree: o coordenador o remove junto com o worktree.
- **Sem novas dependências no Cargo:** Cargo.toml e Cargo.lock não mudam. Rede só para baixar `pdfjs-dist` em pasta temporária dentro de `target/`, se precisar dele no item 2. Nada de package.json no repositório.

## Navegação no código
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; depois leia só as linhas localizadas e confirme no worktree. Não rode `graphify update`.
- `plots-pdf/src/lib.rs`: `PdfOrigem`, `gerar_passada` (cópia de páginas, referências excluídas, recursos, Rotate, recusas)
- `plots-cli/src/lib.rs`: `gerar_pdfs`, `USO_GERAR_PDF`, gravação com remoção do que foi criado em caso de falha
- `plots-print/src/lib.rs`: `Simulador`, `conferir_folhas`, `combinacoes_coerentes`, proptest e laço `lotes_e_perfis_coerentes...`
- `plots-print/src/job.rs`: `Job`, `SimBackend`

## Segurança de processos
Encerre só os seus próprios PIDs, por número exato; nunca um padrão que pegue todos os Firefox, porque o usuário pode estar com o dele aberto. Use perfil temporário próprio. Se uma ação for negada, pare e relate; não peça a outro que a faça. Não envie nada a nenhuma impressora, nem à "Microsoft Print to PDF".

## Fontes (somente estas)
E:\PlotS\plan\03-simulacao.md, E:\PlotS\plan\02-geracao-pdf.md e a seção de fases de E:\PlotS\plan\00-visao-geral.md. Não abra E:\PlotS\plan\estilo-ui-ux.md nem E:\PlotS\codex-session-*.md.

## Regras sobre identificadores do plano
Comentários, testes e mensagens descrevem comportamento com palavras próprias: nada de IDs de requisito, nome de lote, "auditoria" ou "brief".
**Autoverificação, não pode imprimir nada:** `sh E:/PlotS/.claude/team/selfcheck.sh a261772 plots-cli plots-core plots-print plots-pdf`

## Escopo
Primeiro registre no relatório o que existe hoje para cada item.

1. **PDF de folhas finais.** `gerar-pdf ... --folhas-finais` grava também `lote-NNNN-folhas-finais.pdf`, um por lote. Cada página do arquivo é uma folha, de cima para baixo no maço: frente à esquerda e verso à direita, em tamanho real. A largura é a soma das duas e a altura é a maior das duas. O que aparece vem do resultado do simulador para aquela impressora virtual e ordem, com a recolocação correta: não reconstrua a partir do número das páginas. Face sem impressão fica branca. Verso de cabeça para baixo, se o simulador disser que está, aparece girado 180°. Em `plots-pdf`, inclua cada página original como Form XObject, respeitando recursos, caixa e `/Rotate`; reaproveite as recusas existentes (PDF criptografado e formulários que exigem regeneração) e não leve conteúdo de páginas excluídas. Anotações não precisam aparecer: diga isso na ajuda da opção. Sem a opção, nada muda. Os arquivos seguem as mesmas regras dos atuais: nunca sobrescrever, e remover só o que foi criado em caso de falha.
2. **Validação no Firefox.** O critério da fase é abrir no Firefox com a ordem e a rotação certas. Gere com o binário real, a partir de um PDF de teste, uma passada 1, uma passada 2 com rotação de 180° (perfil de teste ou função de `plots-pdf`, sem mudar as impressoras virtuais), um `calibracao.pdf` e um PDF de folhas finais. Tente `firefox --headless --profile <tmp> --screenshot <png> file:///...` com tempo limite. Leia cada imagem e relate o que vê. Se o Firefox em modo headless não renderizar o PDF, use `pdfjs-dist` (o mesmo leitor do Firefox) via node para renderizar ou extrair página, tamanho, rotação e texto, e diga exatamente o que foi e o que não foi validado. Não declare validação que não aconteceu. Isso não vira teste do CI.
3. **Tabela explícita de mecânicas.** Hoje `combinacoes_coerentes` procura a primeira mecânica e gesto que acertam um lote-piloto, então a propriedade vale só o que essa escolha vale. Troque a busca por uma tabela fixa, escrita no teste, que para cada um dos 8 perfis diz a mecânica e o gesto, com um comentário curto explicando fisicamente por que aquele par combina. Inclua as mecânicas de `Mecanica::traseira()` e `Mecanica::gaveta()` com seus perfis na mesma propriedade. Se a coerência física de alguma linha não puder ser justificada, relate em vez de esconder.
4. **Laço exaustivo.** O teste `lotes_e_perfis_coerentes...` pula em silêncio mecânicas sem gesto coerente. Faça-o afirmar quantas combinações foram puladas e quantas verificadas, para que uma mudança no modelo não reduza a cobertura sem ninguém notar. Sem aumentar o tempo da suíte.
5. **Ajuda.** `USO_GERAR_PDF` e `--help` cobrem `--folhas-finais`.

## Regras
- **Testes:** folhas finais com `lopdf`: número de páginas, MediaBox de cada folha com tamanhos mistos, referência às duas faces, face em branco no total ímpar, página original com `/Rotate`, nada de conteúdo de página excluída, sem a opção nenhum arquivo extra, e falha sem deixar arquivo parcial. Tudo gerado no próprio teste.
- **Falha prévia:** registre a falha antes de implementar, quando houver base para isso.
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes em código novo; aritmética de dimensões checada contra valores hostis; trabalho limitado por `LIMITE_PAGINAS`.
- **Textos:** CLI em pt-BR. Liste todos os textos novos. Sem i18n nesta etapa.

## Teste de uso (binário real)
Com `cargo run -p plots-cli --locked -- ...`, em pasta temporária dentro de `target/`: rode a opção nova nas duas impressoras virtuais, com total par e ímpar, e nas duas ordens. Renderize as folhas finais para PNG (Firefox, pdf.js ou PyMuPDF com `python -I`; não instale PyMuPDF) e leia as imagens. Cole no relatório comando, saída e código de saída.

## Relatório
O que existia antes; cada item com testes e grupo de falha prévia; a tabela de mecânicas; os números do laço exaustivo; o que o Firefox ou o pdf.js mostraram em cada arquivo; linha do CI, conferência de caminhos e autoverificação vazia; textos novos; o que foi removido na limpeza; o que não foi verificado. Termine com tudo em commits wip e `git status` limpo. Informe o caminho do worktree e o nome do branch.
