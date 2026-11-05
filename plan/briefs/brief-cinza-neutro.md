# PlotS: alerta no diálogo do verso e cinza do InDesign

Pedido do usuário em 2026-11-04, depois de testar o PlotSim corrigido:

1. O diálogo "Imprimir os versos?" já existe e já pergunta o que precisa. Falta **um ícone de
   alerta amarelo** no começo do título, antes do texto. Nada mais muda nesse diálogo.
2. A interface do PlotS, hoje branca no tema claro, passa a usar o **cinza do InDesign**. Só o
   PlotS; o PlotSim não muda. O tema escuro atual continua; o que hoje é o tema claro vira o
   cinza. O documento PDF continua branco, como a página no InDesign.

Leia `E:\PlotS\AGENTS.md`, `.claude/team.json`, `.claude/skills/craft-team/SKILL.md` e
`.claude/agents/plots-implementer.md` antes de agir; siga o método do papel. Não faça push nem
squash. Trabalhe no worktree isolado em que você foi iniciado.

**Graphify:** antes do primeiro grep ou leitura de código, `graphify query "<pergunta>"
--graph E:/PlotS/graphify-out/graph.json`. Ao terminar, `graphify update .` no worktree.

Base: `master` `6c6474d`. Build em `%LOCALAPPDATA%\plots-build\cinza-indesign\target`, com
`CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, `-j 4`. `PLOTS_PDFIUM_DIR=%LOCALAPPDATA%\plots-build\pdfium\chromium-8125\bin`.
Rode `npm ci` em `ui/`. CDP na porta **9373** (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9373`).

**Capturas:** só por CDP, prefixo `cz-`, em `%LOCALAPPDATA%\plots-build\cinza-indesign\capturas`.
Nunca capture a tela do Windows nem use SendKeys. Não use o PDF assinado do usuário. Não toque
nas filas `PlotSim - IMP1` / `IMP2` nem em processos do usuário: use só as impressoras
simuladas do build de desenvolvimento. Filas temporárias, se precisar, só `PlotSim — Uso *`,
criadas e removidas pela janela.

## 1. Cinza do InDesign (decisão da autoridade de interface)

Substitua **integralmente** o bloco `[data-theme='light']` de `ui/src/theme.css` por
`[data-theme='gray']` com exatamente estes valores (medição em `contrast.py` anexa ao relatório):

```css
[data-theme='gray'] {
  color-scheme: dark;

  --bg: #333333;
  --bg-sunken: #2b2b2b;
  --bg-elevated: #3f3f3f;
  --panel: #4f4f4f;
  --panel-hover: #595959;
  --border: #454545;
  --border-control: rgba(255, 255, 255, 0.45);
  --border-control-hover: rgba(255, 255, 255, 0.58);
  --border-strong: rgba(255, 255, 255, 0.24);
  --overlay-scrim: rgba(12, 12, 12, 0.6);
  --overlay-scrim-clean: rgba(12, 12, 12, 0.72);
  --overlay-scrim-nested: rgba(12, 12, 12, 0.66);

  --fg: #f2f2f2;
  --fg-muted: #dedede;
  --fg-faint: #b4b4b4;
  --text-primary: #f2f2f2;
  --text-secondary: #dedede;
  --text-tertiary: #b4b4b4;
  --text-quaternary: #8f8f8f;

  --accent: #f2f2f2;
  --accent-strong: #ffffff;
  --accent-on: #1c1c1c;
  --accent-soft: rgba(255, 255, 255, 0.06);
  --accent-faint: rgba(255, 255, 255, 0.06);
  --accent-bg-soft: rgba(255, 255, 255, 0.08);
  --accent-border: rgba(255, 255, 255, 0.24);
  --accent-border-soft: rgba(255, 255, 255, 0.24);
  --accent-ring: rgba(255, 255, 255, 0.1);

  --surface-modal: #3f3f3f;
  --surface-card-default: #4a4a4a;
  --surface-card-default-strong: #555555;
  --surface-card-selected: rgba(255, 255, 255, 0.07);
  --border-subtle: #3d3d3d;
  --border-accent: rgba(255, 255, 255, 0.24);

  --status-working: #34d399;
  --status-working-soft: rgba(52, 211, 153, 0.16);
  --status-waiting: #f59e0b;
  --status-waiting-soft: rgba(245, 158, 11, 0.16);
  --status-stopped: #b4b4b4;
  --status-stopped-soft: rgba(180, 180, 180, 0.14);
  --status-disabled: #8f8f8f;
  --status-disabled-soft: rgba(143, 143, 143, 0.14);
  --status-offline: #fb8b8b;
  --status-offline-soft: rgba(251, 139, 139, 0.16);
  --status-offline-text: #fb8b8b;
  --status-offline-hover: #fecaca;

  --icon-warning: #f59e0b;
  --focus-ring: #38bdf8;
  --shape-tabs-lane-bg: #2e2e2e;
  --shape-tabs-lane-border: rgba(255, 255, 255, 0.07);

  --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.4);
  --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.5);
  --shadow-lg: 0 12px 32px rgba(0, 0, 0, 0.6);
}
```

- `--window-close-*`, `--radius-*`, `--anim-*`, `--ease-*` e as fontes continuam no `:root`;
  não redeclare.
- Declare `--icon-warning: #f59e0b` também no bloco escuro (`:root, [data-theme='dark']`), para
  o token existir nos dois temas. O `#101114` do escuro não muda.
- `color-scheme` do cinza é `dark` (controles nativos, popup do `select`, barras de rolagem).
- Não mexa em `Visualizador.module.css`: a página do PDF continua branca sobre o pasteboard.

### Nome do tema

`ui/src/theme.ts`: `Tema` passa a ser `"dark" | "gray"` e a preferência clara do sistema vira
`gray`. `[data-theme='light']` não pode sobrar em CSS nenhum.

### Regras do branco que saem

- `ui/src/components/BlocoGrupos.module.css`: **remover** as duas regras
  `:global([data-theme='light'])` do hover (linhas ~56-58 e a linha do `.opcaoAtiva:hover`).
- `ui/src/components/Sidebar.module.css`: **remover** as três regras
  `:global([data-theme='light'])` do `.linha .estado` (linhas ~493-498).
- `ui/src/components/AvisosDoTopo.module.css` (~53-56): **manter a regra** do ícone de "ativo"
  em `--fg-muted` e **reescrever o comentário**, que dizia que o verde não passa sobre o claro
  — no cinza ele passaria, mas o banner não deve carregar um segundo verde.
- Remova o comentário e os valores do "tema claro" escurecidos para o branco
  (`--status-working #059669`, `--status-waiting #c2610a`, `--status-offline #dc2626` etc.):
  o cinza usa os valores da tabela.

## 2. Ícone de alerta no título do verso

- `ui/src/confirmacao.ts`: em `ConteudoConfirmacao`, acrescente
  `/** Pede atenção no título, antes do texto: o passo seguinte gasta papel. */ atencao?: boolean;`
- `ui/src/face-do-envio.ts`: `conteudoDoVerso` devolve `atencao: true` (uma linha, junto de
  `titulo`/`texto`). Nenhum outro diálogo define `atencao`.
- `ui/src/components/ConfirmDialog.tsx`: no `Dialog.Title`, quando `visivel.atencao === true`,
  renderize `TriangleAlert` do `lucide-react` com `size={16}`, `aria-hidden="true"`, antes de um
  `<span>` com o texto traduzido. O nome acessível do título continua sendo só o texto.
  Se a versão 1.66 do `lucide-react` não exportar `TriangleAlert`, use `CircleAlert` no mesmo
  tamanho e token e registre no relatório.
- Para poder testar sem DOM (o projeto usa `renderToStaticMarkup`, não Testing Library),
  exporte um componente pequeno `TituloDoDialogo({ conteudo })` com o ícone e o `<span>`, e use
  esse componente dentro do `Dialog.Title`. Nada de duplicar o ícone fora dele.
- `ConfirmDialog.module.css`: `.titulo` com `display: flex`, `align-items: flex-start`, `gap:
  8px`, `line-height: 20px`; `.iconeDoTitulo` com `flex-shrink: 0`, `margin-top: 2px`, `color:
  var(--icon-warning)`; `.textoDoTitulo` com `min-width: 0`. Em duas linhas, o ícone fica na
  primeira e o texto quebra na coluna dele.
- O aviso de cópias diferentes no corpo continua `CircleAlert` 14 px em `--fg`; não mude.

## 3. Testes (com falha prévia registrada)

- Novo `ui/src/theme.test.ts`: com `matchMedia` stubado, preferência clara → `data-theme="gray"`
  e escura → `"dark"`. Falha antes: hoje devolve `"light"`.
- `ui/src/face-do-envio.test.ts` (ou `copias.test.tsx`): `conteudoDoVerso(...).atencao === true`;
  um diálogo comum (ex.: `conteudoDoEnvio`) **não** define `atencao`.
- Teste de render com `renderToStaticMarkup` (o projeto não tem Testing Library) sobre
  `TituloDoDialogo`: com `atencao: true` existe um `svg[aria-hidden="true"]` no título; sem a
  flag, não existe.
- Teste de fonte lendo `theme.css`: existe `[data-theme='gray']`, **não** existe
  `[data-theme='light']`, e `--icon-warning` aparece nos dois temas.
- `npm test` e `npm run build` na UI; `cargo fmt --all --check` e
  `cargo test --workspace --locked -j 4`.

## 4. Uso real e capturas

Abra o app de desenvolvimento por CDP (porta 9373), com impressoras simuladas. Troque o tema
**pelo sistema emulado** (`Emulation.setEmulatedMedia`, `prefers-color-scheme`), nunca escrevendo
`data-theme` à mão. Faça os cliques de verdade.

Mínimo obrigatório (as demais são desejáveis):

| # | Tema | Cena | Janela | Idioma |
|---|---|---|---|---|
| 1 | gray | app com PDF aberto, impressoras em estados diferentes, abas, titlebar | 1280×800 | pt-BR |
| 2 | gray | a mesma cena | 1280×800 | en |
| 3 | gray | a mesma cena | 640×420 | pt-BR |
| 4 | gray | diálogo "Imprimir os versos?" com o triângulo | 1280×800 | pt-BR |
| 5 | gray | o mesmo diálogo | 1280×800 | en |
| 6 | gray | o mesmo diálogo com zoom 150 %, título em 2 linhas | 1280×800 | pt-BR |
| 7 | gray | linha da lateral em hover, selecionada e desabilitada | 1280×800 | pt-BR |
| 8 | gray | foco visível (Tab) na lateral e no campo de intervalo | 1280×800 | pt-BR |
| 9 | gray | aviso de topo (espera e problema) e erro de linha | 1280×800 | pt-BR |
| 10 | gray | editor de PDF: página branca, barra, miniaturas, carregando e erro | 1280×800 | pt-BR |
| 11 | gray | cartões com os quatro pontos de estado | 1280×800 | pt-BR |
| 12 | gray | Painel de avisos, Histórico e Calibração abertos | 1280×800 | pt-BR |
| 13 | gray | seletor de modelo aberto (popup nativo) | 1280×800 | pt-BR |
| 14 | dark | cenas 1, 3 e 4 (regressão do escuro) | 1280×800 / 640×420 | pt-BR |
| 15 | gray + dark | `forced-colors: active` | 1280×800 | pt-BR |

Leia cada imagem. Sem as capturas 3, 4, 5, 6, 7, 8, 10 e 14 a revisão de interface reprova.
Rode `contrast.py` para os pares da tabela da autoridade e anexe o log; nenhum par novo sem
medição.

## 5. Final

- CI em um log único `saidas/ci.log`, na ordem: `cargo fmt --all --check`,
  `cargo test --workspace --locked -j 4`, em `ui/` `npm ci`, `npm test`, `npm run build`,
  conferência de que `dist` não contém gancho de verificação, terminando em
  `CI_SUCCESS: cargo test --workspace` e `CI_EXIT 0`.
- Wip commit com árvore limpa; `node_modules` e `dist` apagados; `target` do lote apagado
  depois do CI; filas temporárias removidas; seus PIDs encerrados pelo número exato.
- Não mexa em `plan/` nem no PlotSim.
- Resposta final item a item: causa, falha prévia (caminho do log), capturas lidas, tabela de
  contraste, linha do CI e o que não foi verificado.
