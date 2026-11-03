# cartoes-impressao: imprimir pela janela, com um cartão por impressora

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado, baseado em `master`. Faça commits wip; nunca dê push nem squash. Não altere nada em E:\PlotS fora do seu worktree.

## Regra de segurança mais importante
**O teste de envio pela janela só pode usar as impressoras simuladas e a fila "Microsoft Print to PDF", com arquivo de saída.** Nunca envie a nenhuma outra fila. Em teste automatizado, use só as simuladas ou um backend falso. Para a fila de PDF, o app aceita `PLOTS_ARQUIVO_SAIDA=<pasta>`, variável que você cria neste lote e que só tem efeito se estiver definida. Sem ela, fila cuja porta pede arquivo é recusada com mensagem clara, como no CLI, e nunca abre diálogo nativo. Confira depois de cada teste que não sobrou trabalho na fila.

## Ambiente
- Use em todo comando cargo `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/cartoes"` e `CARGO_INCREMENTAL=0`: o disco E: está quase cheio. Logs e capturas ficam em `target/` do worktree. No mínimo 4 jobs. Um comando pesado por vez.
- `PLOTS_PDFIUM_DIR=C:/Users/Kaio Luan/AppData/Local/plots-build/pdfium/chromium-8125` para o envio à fila de PDF. Não baixe nada de novo.
- Se o sistema matar o build por falta de memória, repita uma vez; se morrer de novo, apague a pasta `debug/incremental` do target e tente mais uma vez. Só então relate.
- Em `ui/`, rode `npm ci`. Sem dependências novas. Se o sistema negar algo, pare e relate; não contorne.
- CI depois da última edição: `cargo fmt --all --check`, `cargo test --workspace --locked`, e em `ui/` `npm test` e `npm run build`. Grave em `target/ci.log`, seguido de `CI_EXIT <código>`. Nunca faça wip depois de um CI vermelho. O CI não depende da DLL nem envia nada.
- Log de falha prévia: `target/fail-before.log`, com execuções reais.
- **Capturas só por CDP (`Page.captureScreenshot`)**, nunca a tela do Windows. Use `PLOTS_PERFIS` temporário e nunca o `%APPDATA%` real. Encerre só seus próprios PIDs; não deixe app rodando. Não use diálogos nativos.
- Limpeza conforme "Limpeza constante do disco" em AGENTS.md.

## Navegação
Antes do primeiro grep ou leitura de código, use `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`; o grafo pode não cobrir a interface. Não rode `graphify update`.
- `plots-cli/src/enviar.rs` e `plots-cli/src/imprimir.rs`: o fluxo atual, a separar do terminal
- `plots-print/src/job.rs`: `Job`, `EstadoJob`, `Backend`; `plots-windows/src/backend.rs`: `BackendWindows`
- `plots-core/src/lib.rs`: `ModeloImpressora`, `plano_lotes_por_papel`
- `src-tauri/src/main.rs`, `src-tauri/src/impressoras.rs`
- `ui/src/App.tsx`, `ui/src/tabs.ts`, `ui/src/components/Sidebar.tsx`, `ui/src/components/ConfirmDialog.tsx`, `ui/src/i18n.ts`

## Fontes
`plan/13-fluxo-e-recolocacao.md`, `plan/11-lateral-impressoras.md`, `plan/14-padrao-visual-e-idiomas.md`, `plan/08-impressao-windows.md`, `plan/05-tamanhos-de-papel.md`, `plan/19-modelos-de-impressora.md` e a tabela de componentes de `plan/03-simulacao.md`, no worktree, e a norma em `E:\PlotS\plan\estilo-ui-ux.md`. Antes de decidir detalhes visuais, carregue a skill `ui-design-craft`. Não abra `E:\PlotS\codex-session-*.md`.

## Regras sobre identificadores do plano
Código, comentários e testes descrevem comportamento com palavras próprias: sem IDs de requisito, nome de lote, "auditoria" ou "brief". Sem marca ou modelo de impressora no produto e nos testes, salvo o nome "Microsoft Print to PDF" em teste real marcado.
Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh <base> plots-print plots-cli plots-windows plots-core src-tauri ui`, com `<base>` igual ao commit de `master` em que você começou.

## Escopo
Primeiro registre no relatório o que existe hoje.

1. **Fluxo reutilizável.** Separe do terminal, em uma biblioteca, a condução de um trabalho por impressora: lotes, frente, espera, recolocação, verso, avisos e cancelamento. O CLI e o app usam o mesmo código, e os testes do CLI continuam passando. Sem entrada ou saída de terminal dentro dela.
2. **Comandos e eventos no Tauri.** Iniciar a frente, enviar o verso, reimprimir o lote, cancelar e responder a um aviso. Um trabalho por impressora, fora da thread da interface, com o estado enviado à interface por evento. Impressoras diferentes avançam de forma independente.
3. **Botão "Imprimir frente".**
   - Conta só impressoras que podem receber: marcadas, com modelo ou perfil, sem estado de problema, e com PDF aceito. Sem nenhuma, fica desabilitado com o motivo visível.
   - Confirmação antes do envio real, em diálogo, listando as impressoras, as páginas e o papel. Impressora simulada não precisa de confirmação.
4. **Cartão EM ANDAMENTO**, conforme o plano: card de ação, padding `12px 14px`, raio 8, com ponto de 8 px e texto de estado (Imprimindo, Aguardando recolocação, Concluído, Erro), em `aria-live="polite"`. Mostra lote atual e total, e papel. Em aguardando recolocação: `Espere a última folha sair antes de recolocar o maço.` e a frase do gesto, com o lado e a segunda linha, vinda do mesmo código do CLI; com perfil personalizado, sem afirmar lado. Na saída final da laser, avisa para virar o maço antes de ler. O botão secundário "Imprimir verso" só libera depois da confirmação do `Job`. Troca de papel entre lotes é avisada. Sem figura neste lote: registre a figura como pendente no plano.
5. **Simulação na janela.** Com impressora simulada, os botões dizem "Simular frente" e "Simular verso", e o cartão traz um selo SIMULAÇÃO e o resultado `N de M folhas corretas (na simulação)`. Nada é enviado.
6. **Avisos e erros no cartão.** Offline, sem papel e intervenção do usuário mostram o motivo e oferecem "Cancelar trabalho" e "Continuar esperando". Cancelar pede confirmação. Erro mostra o motivo e "Reimprimir lote". O texto nunca diz "impresso": usa os termos do CLI, que dizem que o trabalho terminou ou saiu da fila.
7. **Fechar com impressão em andamento.** Ligue `impressaoEmAndamento` da aba ao estado real, para as confirmações de fechar aba e janela já existentes passarem a valer, e confira se o texto da consequência é verdadeiro. Se não for, corrija. Ao confirmar, os trabalhos em andamento param de ser acompanhados, sem apagar nada da fila.
8. **Por documento.** Cada aba guarda os próprios cartões. Trocar de aba troca a lista, e os trabalhos continuam.

Fora de escopo: visualizador de PDF, intervalo de páginas, modos espelho e divisão, grupos salvos, histórico, figura do gesto, tela de abertura.

## Regras
- **Testes:** Rust para o fluxo com backend falso, em cada caminho: frente, verso, aviso, cancelamento, reimpressão, duas impressoras independentes. Vitest para o estado do botão e a contagem, cartões por aba e textos em dois idiomas.
- **Nunca travar:** sem unwrap/expect/indexação sem checagem fora de testes; thread interrompida vira erro no cartão.
- **Textos:** via `t()` em pt-BR e en. O CLI continua em pt-BR.
- **Contraste:** meça cada par novo com `contrast.py` nos dois temas.

## Capturas e teste de uso
App real, com mouse e teclado por CDP, nos dois temas, em `target/shots/`: botão desabilitado com motivo; contagem excluindo impressora inválida; confirmação; duas simuladas (jato e laser) lado a lado em cada estado, com a frase do gesto e o resultado; troca de papel; aviso e erro, com backend falso ou porta de teste; duas abas; janela no mínimo; inglês; cores forçadas. Fluxo completo com as duas simuladas e 6 páginas, conferindo a sequência do plano 19. Depois, um envio à fila de PDF com `PLOTS_ARQUIVO_SAIDA` e modelo gravado: confira que a frente e o verso foram criados, sem diálogo, e renderize com PyMuPDF (`python -I`), lendo as imagens. Confirme que não sobrou trabalho na fila. Leia cada captura.

## Relatório
O que existia antes, cada item com testes e grupo de falha prévia, linha do CI, conferência de caminhos, autoverificação, contraste, capturas, textos novos, limpeza e o que não foi verificado, especialmente o que só uma impressora real confirma. Termine com `git status` limpo. Informe o caminho do worktree e o nome do branch.
