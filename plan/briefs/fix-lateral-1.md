# lateral-impressoras, correção 1

Responda em português brasileiro. Você é um agente novo: trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-ace083b803c1e4acb` (branch `worktree-agent-ace083b803c1e4acb`, commit `6e96c6f` sobre `47a6969`); não crie outro nem mexa em E:\PlotS fora dele. Faça um commit wip ao terminar; nunca dê push nem squash.

As regras de ambiente, segurança, identificadores do plano e relatório de `E:\PlotS\plan\briefs\brief-lateral-impressoras.md` continuam valendo: leia esse brief primeiro. Use `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/lateral"` e `CARGO_INCREMENTAL=0`. Rode `npm ci` em `ui/`. Os scripts de uso estão em `target/e2e/` (`run.mjs`, `foco.mjs`, `foco-confirmar.mjs`). Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh 47a6969 src-tauri ui`.

## Escopo
1. **Linha de impressora** em duas linhas independentes: `nome | modo` e `estado | face`, com gap de 6 px e nome com cerca de 132 px. "Calibrar…" fica na primeira linha, à direita. Clicar em nome ou estado continua marcando a caixa; clicar no modo não marca. `aria-describedby` aponta os dois textos do modo.
2. **Nome** com até 2 linhas (`line-clamp: 2`, `overflow-wrap: anywhere`, `title`). Registre a exceção à §6.5 e §8.4 em "Decisões" de `plan/14-padrao-visual-e-idiomas.md`: errar a impressora desperdiça papel.
3. **Estado** com até 2 linhas e `title`. Em `atencao` e `problema`, texto `--fg` com peso 500, nos dois temas. Sem texto vermelho na linha.
4. **Avisos de perfil:** acrescente recuperação real a `perfis_invalido`, `perfis_grande`, `perfis_versao` e `perfis_pasta`, em pt-BR e en, com o caminho em mono quando o texto mandar agir sobre o arquivo. Confira em `plots-perfis` o que acontece de verdade: arquivo ausente vira lista vazia, então "removê-lo e calibrar de novo" é verdadeiro. Não prometa o que o código não faz. `invalido` e `grande`: restaurar uma cópia ou remover o arquivo e calibrar de novo. `versao`: atualizar o PlotS. `pasta`: conferir a permissão da pasta e usar Atualizar lista.
5. **Diálogo Calibrar:**
   - Botão secundário "Copiar comando" no rodapé, à esquerda de "Fechar", com retorno "Copiado" em `aria-live`.
   - Troque o marcador por `--saida .` e diga "na pasta atual". Antes, execute `plots-cli calibrar` com `.` em uma pasta temporária sob `target/`, com `--perfis` temporário e entrada que encerra logo, para confirmar que o `.` funciona. Corrija a parte das aspas: mostre o nome de um jeito que não quebre no PowerShell quando ele tiver `"`, `$` ou crase, escapando conforme as regras do PowerShell, com teste.
   - O comando `plots` ainda não está no PATH de nenhuma instalação, porque não há instalador. O coordenador aceita isso como transitório. Acrescente uma frase honesta ao diálogo: "O PlotS ainda não instala este comando: quem compilou o projeto usa o executável `plots-cli`." e o equivalente em inglês.
6. **Tema claro:** `.estado` em `--fg`.
7. **`.dica`** em `--fg-muted`.
8. **`forced-colors`:** `opacity: 1` em `.botao:disabled` e `.campoLote:disabled`.
9. **"Perfil não lido"** em 11 px, peso 500, `--fg-muted`, com `CircleAlert` de 12 px em `--fg-muted`.
10. **Vocabulário:** "impressora(s)" / "printer(s)" em texto visível e `title`, em vez de "fila(s)" / "queue(s)", exceto no aviso sobre as filas do Windows.
11. **Inglês:** troque "Drawer" por "Tray", no modo e no nome simulado.
12. **Texto de PDF protegido:** "{nome} foi recusado: o PDF está protegido por senha. Se você tiver permissão para isso, salve uma cópia sem senha e abra de novo." / "{nome} was refused: the PDF is password protected. If you are allowed to, save a copy without a password and open it again."
13. **Ajustes finos:** `.linha` com `padding-left: 4px`; com lote inválido, o erro ocupa o lugar do resumo; mantenha `Calibrar…` em `2px 8px` e registre a exceção no plano 14.

## Capturas
Refaça todas, nos dois temas, em `target/shots/`, e acrescente: `08-foco-seguinte-light`, `12-ingles-calibrar-light`, `13-bundle-normal-light`; lista vazia; `lendo`; "Contando páginas…"; aviso `filas`; nome e estado em 2 linhas; nome longo com `Calibrar…`; diálogo aberto por teclado com foco; "Copiado"; 125% e 150% por emulação, dizendo que é emulação. Leia cada imagem. Meça todo par novo com `contrast.py`.

## Testes
Vitest para os textos dos avisos nos dois idiomas, escape do nome no comando e erro no lugar do resumo. Registre a falha prévia onde houver base, em `target/fail-before-fix.log`. Repita `run.mjs` e os scripts de foco.

## Relatório
Cada item com o que mudou e seu teste, linha do CI (`cargo fmt --all --check`, `cargo test --workspace --locked`, `npm test`, `npm run build`), autoverificação vazia, contraste, capturas com o que cada uma mostra, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo.
