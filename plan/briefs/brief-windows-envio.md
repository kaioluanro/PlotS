# windows-envio: enviar as passadas ao spooler do Windows

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master` (base 762e40e). Faça commits wip; nunca dê push nem squash. Não altere nada em E:\PlotS fora do seu worktree.

## Regra de segurança mais importante
**A única fila que você pode usar para enviar é a "Microsoft Print to PDF", sempre com arquivo de saída informado.** Nunca envie a nenhuma outra fila, física ou de rede. Em cada teste que envia, confira antes que o nome é exatamente esse e, se a fila não existir, pule o teste. Não use diálogos nativos. Se o envio abrir um diálogo de salvar, encerre só o seu próprio PID, pare e relate; não envie teclas. Depois de cada teste, confira que não sobrou trabalho na fila.

## Ambiente
- Use em todo comando cargo `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/windows-envio"` e `CARGO_INCREMENTAL=0`: o disco E: está quase cheio. Logs ficam em `target/` do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- Se o sistema matar o build por falta de memória, repita uma vez; se morrer de novo, apague a pasta `debug/incremental` do target e tente mais uma vez. Só então relate.
- Depois da última edição, rode uma vez `cargo fmt --all --check` e `cargo test --workspace --locked`, gravando em `target/ci.log`, seguido de `CI_EXIT <código>`. Nunca faça wip depois de um CI vermelho. **O CI não pode depender da DLL nem enviar nada:** testes que precisam da DLL ou do envio ficam com `#[ignore]` e são rodados à mão no teste de uso.
- Log de falha prévia: `target/fail-before.log`, com execuções reais.
- Rede com `CARGO_NET_OFFLINE=false`, só para `pdfium-render` e para o pacote abaixo. **Se o sistema negar um download, pare e relate; não tente contornar.**
- Limpeza conforme "Limpeza constante do disco" em AGENTS.md. Saídas temporárias ficam dentro de `target/` e são apagadas ao fim. Nunca toque o `%APPDATA%` real: use `--perfis` temporário.

## Navegação
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; depois leia só as linhas localizadas e confirme no worktree. Não rode `graphify update`.
- `plots-print/src/job.rs`: `trait Backend`, `Job`, `Passada`, `EstadoEnvio`
- `plots-windows/src/sistema.rs`: estilo do `unsafe` e leitura de buffers
- `plots-cli/src/imprimir.rs`, `plots-cli/src/lib.rs`: `MSG_SEM_SIMULAR`, `FRASE_NAO_ENVIA`, `ENVIO_REAL_LIBERADO`
- `plots-pdf/src/lib.rs`: `PdfOrigem::carregar`, `gerar_passada`
- `plots-core/src/lib.rs`: formatos de papel

## Fontes
`plan/08-impressao-windows.md`, `plan/05-tamanhos-de-papel.md`, `plan/07-cli-impressao.md`, `plan/pdfium-Pesquisa.md` e `plan/licencas-terceiros.md`, no worktree, e a documentação pública da Microsoft das funções usadas. Não abra `E:\PlotS\plan\estilo-ui-ux.md` nem `E:\PlotS\codex-session-*.md`.

## Regras sobre identificadores do plano
Código, comentários e testes descrevem comportamento com palavras próprias: sem IDs de requisito, nome de lote, "auditoria" ou "brief". Nada de marca de impressora ou fila fixa no código de produção: o nome "Microsoft Print to PDF" só pode aparecer em testes.
Autoverificação, não pode imprimir nada de relevante: `sh E:/PlotS/.claude/team/selfcheck.sh 762e40e plots-windows plots-cli plots-print scripts`

## Escopo
Primeiro registre no relatório o que existe hoje.

1. **Script versionado `scripts/baixar-pdfium.mjs`,** só com a biblioteca padrão do Node. Baixa só a URL da tag `chromium/8125`, nunca `latest`, com o arquivo `pdfium-win-x64.tgz`. Confere 3 904 987 bytes e o sha256 `9a964261760cff52570aabf038ab346d3a5034dd8b30d0d5fe8861235d356b8b` **antes** de extrair; se divergir, apaga e aborta. Extrai só `bin/pdfium.dll`, `LICENSE`, `licenses/` e `args.gn` para `%LOCALAPPDATA%/plots-build/pdfium/chromium-8125/`. Falha se `args.gn` não tiver `pdf_enable_v8 = false` e `pdf_enable_xfa = false`. Imprime o sha256 da DLL.
2. **Leitura das licenças.** Rode o script e leia **cada** texto de `licenses/`. Liste no relatório arquivo por arquivo: componente e licença. Compare com `args.gn` e aponte o que pode ter entrado sem texto, como libc++. **Se aparecer copyleft, pare e relate antes de integrar.**
3. **Dependência:** `pdfium-render = { version = "=0.9.5", default-features = false, features = ["pdfium_8125", "thread_safe"] }`, em `plots-windows`, só para alvo Windows. Se precisar de outra feature, justifique. Carregue só por caminho explícito: ao lado do executável ou na pasta indicada por `PLOTS_PDFIUM_DIR`. Nunca use `bind_to_system_library` nem `Pdfium::default`. Sem a DLL, erro claro que aponta o script.
4. **Barreira de PDF protegido.** Todo PDF passa por `PdfOrigem::carregar` antes do PDFium, com teste. O código nunca passa senha nem ignora permissões.
5. **Rasterização:** uma página por vez, na resolução informada por `GetDeviceCaps` da fila, com `use_print_quality` e fundo branco, respeitando `/Rotate` e a rotação de 180° do plano. Pixels em aritmética checada, com limite claro de memória: erro explicativo em vez de travar. Página em branco do plano sai com o ponto discreto já usado nas passadas. Lógica de tamanho em função pura e testada.
6. **`DEVMODE`:** `DocumentPropertiesW` em dois passos, nunca montado do zero. Alterar só `dmDuplex` simplex, `dmCollate` falso e `dmPaperSize` conforme o papel do lote, marcando `dmFields`; papel fora da tabela usa as medidas ou dá erro claro. Tabela de formatos para `DMPAPER_*` em função pura e testada. Bandeja fica como a do driver neste lote.
7. **Envio GDI:** `CreateDCW`, `StartDocW`, por página `StartPage`, `StretchDIBits`, `EndPage`, depois `EndDoc`, e `AbortDoc` em falha. Confira `RC_STRETCHDIB`. Nome do documento sem caminho pessoal. Todo o trabalho fora de qualquer thread de interface. Parâmetro opcional de arquivo de saída, ligado a `lpszOutput`.
8. **Backend e estado:** implemente `trait Backend` para Windows em `plots-windows`, com `OpenPrinterW` e `GetJobW` nível 2. Traduza o status do trabalho em função pura testada. Trabalho que sumiu da fila depois de enviado é tratado como terminado, e isso fica documentado como limite. Erro, papel e offline viram estado de erro do `Job`. `cancelar` usa `SetJobW`.
9. **`plots imprimir` sem `--simular`:** envia de verdade, com o mesmo fluxo por lote. Exige o perfil salvo da fila e **uma confirmação explícita** antes de cada envio ("Enviar a frente do lote 1 para FILA? (s/n)"); fim da entrada cancela. O verso só é oferecido depois de o `Job` confirmar a frente, e o texto pede: `Espere a última folha sair antes de recolocar o maço.` A saída nunca chama de "impresso" o que só foi enviado. Opção de teste `--arquivo-saida PASTA`, que grava a passada em arquivo pela fila, sem papel. Atualize `USO_*`, `FRASE_NAO_ENVIA`, `MSG_SEM_SIMULAR` e `ENVIO_REAL_LIBERADO`, com os testes coerentes. `plots calibrar` continua sem enviar neste lote.

## Regras
- **`unsafe`:** mínimo, cada bloco com comentário de garantia, handles sempre liberados por um tipo com `Drop`, mesmo em erro.
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes; PDF e respostas do sistema são hostis.
- **Testes:** funções puras no CI. Fluxo de `imprimir` real com um backend falso. `#[ignore]`: rasterização com a DLL e envio à fila de PDF.
- **Textos:** CLI em pt-BR. Liste os textos novos.

## Teste de uso (binário real)
Com o script já rodado, `PLOTS_PDFIUM_DIR` apontando para a pasta, `--perfis` temporário e `--arquivo-saida` dentro de `target/`:
1. Use um PDF de teste gerado pelo próprio projeto: `gerar-calibracao`.
2. Envie frente e verso à fila de PDF e confirme que os arquivos foram criados sem diálogo. Se `lpszOutput` não evitar o diálogo, pare e relate.
3. Renderize os PDFs de saída para PNG com PyMuPDF (`python -I`) e leia as imagens: confira a ordem das páginas, a seta para cima e as rotações. Relate o que vê em cada uma.
4. Teste A4 e uma página paisagem.
5. Confirme que não sobrou trabalho na fila.

## Relatório
O que existia antes; cada item com testes e grupo de falha prévia; o sha256 da DLL; a lista de licenças arquivo por arquivo; o que o `lpszOutput` fez; o que `GetJobW` mostrou até o fim do trabalho; linha do CI; autoverificação; textos novos; limpeza; e o que não foi verificado, especialmente o que só uma impressora real confirma. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
