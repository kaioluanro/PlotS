# plotsim-filas: filas virtuais e recepção de trabalhos (fases 0 a 2)

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia `PlotSim/AGENTS.md` (especificação do usuário), `plan/23-plotsim.md` e as regras comuns de `plan/briefs/coordenacao-paralela-2.md`. Build e saídas em `%LOCALAPPDATA%/plots-build/plotsim-filas`. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.

**Arquivos:** só `PlotSim/plotsim-core/` e `PlotSim/scripts/` (novos), e no `Cargo.toml` da raiz só a linha do membro novo. Outro agente cria `PlotSim/plotsim-physics/`. Reaproveite por dependência, sem copiar: `plots-pdf` e `pdfium-render` para ler, `plots-windows` para a leitura do spooler, `plots-perfis` para gravação atômica. Se precisar expor algo de `plots-windows`, relate antes: outro lote mexe nesse crate. Não mexa em `plan/`. A única dependência nova aceita é `notify`, prevista na especificação, e o `Cargo.lock` muda só por ela.

## Limites importantes

- **Mudança no sistema:** criar fila altera o Windows do usuário. O usuário pediu essas filas na especificação, então pode criá-las, só com o prefixo `PlotSim — ` no nome, e porta sob `C:\ProgramData\PlotSim\`. Remova as filas e portas de teste ao terminar, e relate o que restou. Não altere nem remova nenhuma outra fila, porta ou driver. Não instale driver. Nada de impressora real.
- **Administrador:** tente sem elevação. Se o Windows negar, **não contorne**: não use `runas`, tarefa agendada ou qualquer truque de UAC. Pare a parte que depende disso e entregue, em `PlotSim/scripts/`, o script exato para o usuário rodar, dizendo o que ele faz e como desfazer. Continue no que der sem a fila (recepção com pasta e arquivos preparados, testes com backend falso).
- **Sem diálogo:** se aparecer a janela "Salvar como", é a Fase 0 reprovando: registre e tente fechar só o seu processo.

## Escopo

1. **Fase 0, viabilidade:** fila com driver "Microsoft Print To PDF" e porta de arquivo. Três envios seguidos, pelo caminho real, sem diálogo, cada um com arquivo correto. Relate se a porta sobrescreve, se o spooler trava o arquivo, e o que acontece se o arquivo existir. Se reprovar, descreva o Plano B e **pare para o coordenador decidir**: ele depende de Ghostscript, que tem licença AGPL.
2. **Fase 1, filas:** `plotsim-core` cria, lista e remove filas virtuais, com o modo físico guardado em `%APPDATA%/plotsim`, com caminho sobreposto em teste. Comandos PowerShell sem interpolar nome sem validar: nome e caminho são entrada hostil. Operação idempotente; remoção só do que é do PlotSim.
3. **Fase 2, receber:** observar a pasta com `notify`, saber pelo spooler (`EnumJobsW`) quando o trabalho terminou, copiar o PDF para o histórico da fila, e abrir com PDFium: número de páginas, tamanho, rotação e texto de cada página. Sem perder trabalho quando dois vêm seguidos. Página sem texto legível é erro claro. PDF é entrada hostil.

Sem interface. Nada de modelo físico: outro agente faz.

## Testes

Backend falso para PowerShell e spooler; testes reais `#[ignore]`, rodados à mão com log. Prova real de ponta a ponta, se a fila existir: gerar PDF de teste, enviar pelo `plots imprimir` à fila virtual, com perfil temporário (`PLOTS_PERFIS`), e ler as páginas de volta.

## Final

CI do workspace em um único log (`cargo fmt --all --check`, `cargo test --workspace --locked -j 4`), autoverificação, `git status` limpo com commits wip. Relatório em `%LOCALAPPDATA%/plots-build/plotsim-filas/relatorio.md`, com a mesma mensagem na resposta final: resultado da Fase 0 com evidência, o que foi criado e removido no Windows, o que precisa do usuário, testes com grupo de falha prévia, linha do CI. Limite de tempo em todo comando.
