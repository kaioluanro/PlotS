# PlotS portátil: o perfil viaja ao lado do executável

Pedido do usuário em 2026-11-05: o arquivo de perfis é persistente; na versão portátil ele deve
ficar **na mesma pasta do `PlotS.exe`**, para o perfil acompanhar a pasta quando ela muda de
computador. Hoje ele fica em `%APPDATA%\plots\perfis.toml`.

Leia `E:\PlotS\AGENTS.md`, `.claude/team.json`, `.claude/skills/craft-team/SKILL.md` e
`.claude/agents/plots-implementer.md` antes de agir; siga o método do papel. Não faça push nem
squash. Trabalhe no worktree isolado em que você foi iniciado.

**Graphify:** antes do primeiro grep ou leitura de código, `graphify query "<pergunta>"
--graph E:/PlotS/graphify-out/graph.json`. Ao terminar, `graphify update .` no worktree.

Base: `master` `43e2049` (`1.0.0-alpha.4`). Build em
`%LOCALAPPDATA%\plots-build\perfil-portatil\target`, com `CARGO_TARGET_DIR`,
`CARGO_INCREMENTAL=0`, `-j 4`; `PLOTS_PDFIUM_DIR=%LOCALAPPDATA%\plots-build\pdfium\chromium-8125\bin`;
`npm ci` em `ui/`. CDP na porta **9374**.

**Capturas:** não há mudança visível; não precisa de captura de tela. Não toque nas filas
`PlotSim - IMP1`/`IMP2` nem em processos do usuário, e não altere
`%APPDATA%\plots\perfis.toml` (a migração só lê). Não use o PDF assinado.

## Comportamento

1. **Modo portátil = marcador ao lado do executável.**
   - `plots-perfis/src/armazenamento.rs`: constante pública do marcador,
     `PLOTS-PORTATIL.txt`; a presença de um **arquivo** com esse nome na pasta do executável
     (`std::env::current_exe()`) liga o modo portátil. Um diretório com esse nome não conta.
   - Em modo portátil, `caminho_padrao()` devolve `<pasta do exe>/perfis.toml`. Sem o marcador,
     continua exatamente `%APPDATA%\plots\perfis.toml`.
   - `PLOTS_PERFIS` continua vencendo tudo (os chamadores já tratam a variável; não mexa neles).
   - O app instalado (sem marcador) não muda de comportamento.
2. **Primeira abertura do portátil traz o perfil atual.**
   - `migrar_para_portatil()`: se o marcador existe, o `perfis.toml` local **não** existe e o
     `%APPDATA%\plots\perfis.toml` existe, ler o de `%APPDATA%` com `carregar` (valida o
     formato) e gravar o local com `salvar` (atômico). Nunca sobrescrever um `perfis.toml` que já
     esteja na pasta. Sem origem, não faz nada.
   - Chamar **uma vez** no `setup` de `src-tauri/src/main.rs`, antes de `abertura::iniciar` e de
     qualquer leitura de perfis. Falha não pode derrubar o app: a janela abre; um `perfis.toml`
     inválido em `%APPDATA%` continua sendo reportado pelo caminho de erro que já existe quando a
     lista é lida (não invente um segundo aviso).
   - Nada de copiar o perfil para dentro do repositório, de teste ou do zip.
3. **Leitura e gravação são as mesmas.** `carregar`/`salvar` não mudam; só o caminho resolve
   para a pasta do exe.
4. **Documentação.** `README.md`, seção "Seus dados": dizer que na versão portátil (com
   `PLOTS-PORTATIL.txt` ao lado do `PlotS.exe`) o `perfis.toml` fica nessa pasta e viaja com o
   programa; na primeira abertura ele traz o que estiver em `%APPDATA%\plots\perfis.toml`; o
   histórico continua em `%APPDATA%`. Dizer também que o arquivo pode conter ajustes de driver
   com senha/PIN e não deve ser compartilhado.

## Testes (falha prévia registrada)

- `plots-perfis`: função pura que resolve o caminho a partir de `(pasta do exe, pasta de
  configuração)`. Casos: marcador presente → `exe/perfis.toml`; marcador ausente →
  `config/plots/perfis.toml`; marcador como diretório → configuração.
- Migração pura entre dois caminhos: origem válida → destino criado com o mesmo conteúdo e o
  arquivo de origem intacto; destino já existente → não sobrescreve; origem ausente → nada é
  criado; origem inválida → erro e destino não criado.
- O teste existente `caminho_padrao_termina_em_plots_perfis_toml` continua passando (o binário
  de teste não tem marcador).
- Sem mudança de comportamento com `PLOTS_PERFIS`: os testes existentes devem continuar verdes.
- Registre o log de falha anterior em `saidas/falha-antes.log`. Se a API é nova e não há como
  rodar antes, diga isso no relatório (grupo "API nova").

## Uso real (sem tocar no perfil do usuário)

Com o app de desenvolvimento (`--features` padrão; simuladas), `ui/dist` embutido:
1. Copie `target\debug\plots-app.exe` e a `pdfium.dll` para uma pasta temporária em
   `%LOCALAPPDATA%\plots-build\perfil-portatil\portatil-teste`, e crie nela
   `PLOTS-PORTATIL.txt`.
2. Abra por CDP (porta 9374). Prove:
   - `perfis.toml` apareceu na pasta e o conteúdo é idêntico ao de
     `%APPDATA%\plots\perfis.toml` (compare SHA-256);
   - escolha um modelo/calibre uma impressora **simulada** pela janela; o `perfis.toml` da pasta
     muda e o de `%APPDATA%` continua com o mesmo SHA-256 de antes;
   - feche e abra de novo: o perfil lido é o da pasta (o modelo escolhido continua).
3. Numa segunda cópia **sem** o marcador, com `PLOTS_PERFIS` apontando para um arquivo temporário,
   abra e confirme que nenhum `perfis.toml` é criado ao lado do exe.
4. Leia o `perfis.toml` produzido para confirmar que não foi corrompido. Encerre os PIDs pelo
   número exato e apague a pasta temporária ao fim (ela contém uma cópia do perfil do usuário).

Filas reais só para leitura; nenhum envio. Nada de tela do Windows nem SendKeys.

## Final

- CI em um log único `saidas/ci.log`: `cargo fmt --all --check`,
  `cargo test --workspace --locked -j 4`, em `ui/` `npm ci`, `npm test`, `npm run build`,
  conferência de ganchos, terminando em `CI_SUCCESS: cargo test --workspace` e `CI_EXIT 0`.
- Wip commit com árvore limpa; `node_modules`/`dist` e `target` do lote apagados; nenhum
  `perfis.toml` do usuário no worktree ou no commit.
- Não mexa em `plan/`, no PlotSim nem no zip (o coordenador monta o zip com o marcador).
- Resposta final item a item: causa, falha prévia (caminho), provas de uso com os SHA-256, linha
  do CI e o que não foi verificado.
