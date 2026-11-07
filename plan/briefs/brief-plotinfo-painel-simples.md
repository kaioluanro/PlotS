# plotinfo-painel: painel simples de tinta e custo

Trabalhe no worktree isolado que o coordenador indicar, criado a partir do `master` que contém o
B2/B3 e este brief. Faça commits wip; nunca faça push nem squash.

## Ambiente

- **Build:** `CARGO_TARGET_DIR=%LOCALAPPDATA%\plots-build\plotinfo-painel\target`,
  `CARGO_INCREMENTAL=0`, `-j 4`. Um comando pesado por vez.
- **PDFium:** `PLOTS_PDFIUM_DIR=%LOCALAPPDATA%\plots-build\pdfium\chromium-8125\bin`.
- **UI:** `npm ci`, `npm test`, `npm run build` em `ui/`.
- **CI:** depois da última edição, rode o comando do `.claude/team.json` uma vez, no foreground,
  com log em `saidas/ci.log`; precisa terminar em `CI_SUCCESS: cargo test --workspace` e
  `CI_EXIT 0`. Rode também `npm test` e `npm run build`, com logs.
- **Falha-antes:** `saidas/falha-antes.log`, com execuções reais.
- **Saídas/capturas:** `saidas/`, prefixo `NN-`; leia cada captura.

## Navegação de código

Antes do primeiro grep/leitura:
`graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.
Confirme cada hit no worktree e leia só as linhas localizadas. Ao terminar, `graphify update .`.

Comece por:
- `ui/src/App.tsx`, `ui/src/components/TitleBar.tsx`, `TitleBar.module.css`;
- `ui/src/components/DocumentArea.tsx`, `ui/src/visualizador/Visualizador.tsx` (página atual,
  barra do visualizador);
- `ui/src/components/OpcoesDaImpressoraDialog.tsx` + CSS (padrão de painel/engrenagem/diálogo);
- `ui/src/components/PainelDeAvisos.tsx`/`AvisosDoTopo.tsx` (estado/lista);
- `ui/src/backend.ts`, `ui/src/i18n.ts`, `ui/src/erros.ts`, `ui/src/theme.css`;
- `src-tauri/src/analise.rs`, `src-tauri/src/plotinfo.rs`, `src-tauri/src/main.rs`;
- `plan/30-plotinfo.md`, `plan/briefs/brief-plotinfo-analise.md`,
  `plan/briefs/brief-plotinfo-motor.md`.

## Segurança de processo

- Mate só os seus PIDs, por número exato. Nunca mate todas as instâncias do PlotS nem processos do
  usuário. Não deixe app rodando.
- Se uma ação for negada, pare e reporte; não peça a outra pessoa para fazer.

## Fontes

- `AGENTS.md`, `.claude/team.json`, `.claude/skills/craft-team/SKILL.md`,
  `.claude/agents/plots-implementer.md`, `.claude/agents/plots-interface.md`,
  `plan/estilo-ui-ux.md`.
- `plan/30-plotinfo.md` e os briefs do B1/B2/B3.
- Os arquivos listados acima.
- Não leia `PlotInfo/`.

## Regras de plano

- Comentários, nomes de teste e mensagens descrevem comportamento com nossas palavras. Nada de
  ids de requisito, sinais de seção, “manual”, “lote” ou “próximo lote”.
- **Self-check, precisa não imprimir nada:**
  `sh E:/PlotS/.claude/team/selfcheck.sh <base-sha> ui/src src-tauri/src`. Rode também nos
  arquivos novos não rastreados.

## Escopo

O painel é uma **calculadora independente**. Ele lê o documento da aba ativa e estima tinta/custo,
mas **não altera** o intervalo, a face Frente/Verso, as cópias nem o envio do PlotS. A interface
precisa dizer isso (“não altera o envio”) e um teste precisa provar que os campos do painel não
mexem no estado da aba.

1. **Ícone e gaveta.**
   - `Calculator` ao lado do sino na `TitleBar`, com `id="botao-plotinfo"`, `aria-label` e `title`
     “Tinta e custo”, estado aberto/fechado.
   - Gaveta de ~320 px à direita do PDF, com borda de 1 px, cabeçalho, engrenagem e fechar.
     Abaixo de 900 px, sobreposição sobre o documento (sem quebrar a prévia).
   - Resumo compacto na barra do visualizador: `N pág · X mL · R$ Y`, clicável para abrir a
     gaveta; só aparece quando há resultado.
2. **Conteúdo da gaveta** (simples, sem mapa de calor/ICC):
   - **ARQUIVO**: nome, páginas, papel, dpi e modo.
   - **PÁGINA N DE M**: página atual do visualizador, uma cópia; uma linha por canal com ponto,
     nome, cobertura % e mL; total e custo. Página em branco = 0,0% real.
   - **PÁGINAS A ESTIMAR**: campo de texto próprio do painel, independente da lateral, com
     `1`, `2,5,7,9`, `5-16`, mistos e vazio = todas; campo de cópias (1–999); total
     `páginas × cópias` explícito; tinta e custo da seleção.
   - **DOCUMENTO**: totais por cor, total, média por página e página mais cara (link para a
     página, opcional).
   - **CARTUCHO**: % consumido por cor e “~N cópias até esvaziar”, com a cor que acaba primeiro.
   - Rodapé: `Estimativa · não altera o envio · perfil · qualidade · modo`.
   - Progresso: “Analisando página 3 de 12” com barra de 2 px, atualizado pelos eventos do B2.
   - Estados: vazio, analisando, erro com motivo, página em branco, documento recusado.
3. **Ligação com a análise.**
   - Ao abrir a gaveta, iniciar a análise do documento da aba ativa; ao fechar/trocar de aba,
     cancelar; ao voltar, usar o cache.
   - `Páginas` e `Cópias` chamam a interpretação/recalculo do backend; o backend é a fonte da
     verdade da seleção. Não duplique a regra de páginas em TypeScript; mostre o erro devolvido.
   - Trocar perfil/qualidade/preço/dpi/modo chama o recálculo; não rasteriza de novo o que já
     está em cache. O modo aparece no rodapé (`CMYK (MuPDF)` ou `Simples`).
   - Eventos: assine `plotinfo://progresso`, `plotinfo://pagina`, `plotinfo://fim`,
     `plotinfo://falha`; desassine ao fechar.
4. **Engrenagem.**
   - Diálogo simples (Radix `Dialog`, foco preso, ESC) com:
     - perfil (selecionar, novo, duplicar, apagar, restaurar padrões);
     - tabela por canal: volume (mL), rendimento (páginas a 5%), preço por mL;
     - moeda do perfil (ISO 4217, padrão BRL);
     - fatores de qualidade Rascunho/Normal/Alta;
     - qualidade ativa, dpi 150/300, modo CMYK/Simples, “cinzas só com preto”, tamanho da imagem
       (A4/DPI).
   - Usa os comandos de `plotinfo.toml` do B1. Validação com erro claro; não salva inválido.
   - O preço por mL é o valor usado no custo. Mostre a ajuda “preço do cartucho ÷ volume”.
5. **CSV.**
   - Botão “Exportar CSV” no rodapé/cabeçalho; usa o comando do B3 e um `save` do plugin de
     diálogo; uma linha por página da seleção + total. Erro de escrita vira aviso.
6. **Visual e idiomas.**
   - Tokens `--ink-cyan`, `--ink-magenta`, `--ink-yellow`, `--ink-key` nos temas `dark` e `gray`;
     ponto/barra fina + nome do canal; amarelo com contorno no tema claro. Nada de estado só por
     cor.
   - Números em `--font-mono`/`tabular-nums`; `Intl.NumberFormat`; mL e moeda do perfil.
   - Todas as strings via `t()` em pt-BR e en; nenhum literal visível.
   - Foco 2 px, alvos 24×24, `aria-live` para “Análise concluída”, `prefers-reduced-motion` e
     `forced-colors`.

## Regras

- **Testes:** total `1×2=2`, `2,5,7,9×1=4`, `5-16×12=144`, vazio com 5; seleção inválida; cópias
  0/1000; página em branco; ida e volta do gear; CSV; independência do envio (não muda
  `aba.intervalo`/`copias`/`face`); estados vazio/analisando/erro; i18n; teclado/foco. Nenhuma
  asserção afrouxada.
- **UI review:** carregue `ui-design-craft`, meça os pares novos e capture, em `gray` e `dark`:
  vazio, resultado com documento, erro, página em branco, gear e gaveta fechada. 1024×768.
  Se o `dark` não puder ser obtido sem alterar configuração permanente, use o WebView2
  (`preferredColorScheme`) e registre.
- **Nunca crashar:** seleção/cópias/arquivo hostis; valide antes de alocar; nada de panic.
- **Escopo proibido:** não mexa em `plots-core`, nas passadas de `plots-pdf`, no backend de
  impressão de `plots-windows`, no PlotSim, no zip, nos perfis/filas do usuário, no intervalo/
  face/cópias do envio. Não mude o layout do visualizador além do resumo.

## Teste de uso

1. Abra um PDF de 10 páginas; clique no ícone; confira as seções e o total de uma cópia.
2. Digite `2,5,7,9` e 1 cópia: total 4 páginas, tinta/custo coerentes; digite `1-10` e 5 cópias:
   total 50.
3. Abra uma imagem: o painel funciona igual.
4. Abra a engrenagem, troque o preço por mL, salve; o custo muda sem reanalisar.
5. Exporte o CSV e confira as linhas.
6. Capture tudo nos dois temas e feche só o PID que você abriu.

## Relatório

- O que o app fazia antes.
- Cada item do escopo: feito, parcial ou pulado.
- Testes e grupos de falha-antes.
- Strings novas, capturas, pares de contraste.
- Linha do CI, caminhos, self-check.
- Retries e o que não foi verificado.