# imagens-marca: imagem da tela de abertura e ícone do usuário

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia `plan/10-splash.md`, as regras comuns de `plan/briefs/coordenacao-paralela-2.md` e a norma `E:\PlotS\plan\estilo-ui-ux.md`. Build e saídas em `%LOCALAPPDATA%/plots-build/imagens-marca`, porta CDP 9351. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`.

**Arquivos:** `ui/src/abertura/`, `ui/abertura.html`, `ui/assets/` (novo), `src-tauri/icons/`, a marca em `ui/src/components/TitleBar.tsx`, `src-tauri/tauri.conf.json` só se precisar. Três outros agentes trabalham em calibração, `PlotSim/` e `plots-print`/`plots-cli`, então não toque em mais nada.

## Fontes, do usuário, em `imgs/`

- `imgs/PlotS_ Frente e Verso.png`: 1586 x 992, RGB. Tela de abertura. Traz o nome, a frase e o autor na própria imagem.
- `imgs/Ícone_logo.png`: 1254 x 1254, RGB, **sem transparência**: os cantos fora do quadrado arredondado são pretos.

Não altere nem recorte o desenho; não troque texto. Os dois arquivos de `imgs/` continuam como estão.

## Escopo

1. **Tela de abertura:** a imagem substitui a marca provisória, em 480 x 300 na janela e **960 x 600** no arquivo, para telas de alta resolução, com a mesma proporção. Gere `ui/assets/splash.png` a partir da fonte, com peso razoável (menos de 400 KB), mantendo a nitidez do texto: confira na captura em 1x e 2x. O fundo da janela combina com o fundo da imagem, sem faixa nem clarão ao abrir. O aviso de falha e o toast continuam legíveis por cima, com contraste medido sobre a imagem. A imagem é bloco inteiro com `alt` "PlotS: frente e verso, sempre na ordem. Por Kaio Luan."; em inglês, o `alt` traduz, e a imagem fica como é. Remova `MARCA_PROVISORIA`, e o que só existia por causa dela. Testes atualizados.
2. **Ícone:** gere o `.ico` do programa com 16, 24, 32, 48, 64, 128 e 256 px, com transparência fora do quadrado arredondado: os cantos pretos viram transparentes, sem halo. A borda branca do desenho fica. Confira 16 e 32 px, porque ali o desenho precisa ser legível: se o traço fino sumir, relate, sem redesenhar. Atualize `gerar-icone.mjs`, ou substitua-o por um script que gere tudo das fontes de `imgs/`, sem dependência nova; se isso for impossível só com Node, use o que já existe no projeto e relate. Confira que o instalador e o `tauri build` apontam para o ícone novo, sem executar o instalador.
3. **Marca da barra superior:** troque pelo novo ícone, em tamanho legível, sem fundo preto em volta nos dois temas. Se o desenho, que é branco sobre preto, não funcionar direto no tema claro, mantenha o ícone inteiro, com o quadrado preto. Confira nos dois temas.

## Testes e final

App real por CDP: abertura normal; com a `pdfium.dll` faltando, com o aviso; tema escuro e claro do sistema, por emulação; movimento reduzido; barra superior nos dois temas e em inglês. Leia cada imagem. Build de release sem instalar, conferindo o ícone do exe pelos recursos, se der. CI completo em um único log, incluindo `ui/` e `node --test scripts/avisos.test.mjs` com `PLOTS_PDFIUM_DIR`, autoverificação, `git status` limpo com wip, apague `ui/node_modules` e `ui/dist`. Relatório em `%LOCALAPPDATA%/plots-build/imagens-marca/relatorio.md`, com a mesma mensagem na resposta final, curto: o que mudou, tamanhos dos arquivos, capturas com caminho, contraste e o que não foi verificado. Limite de tempo em todo comando de cena.
