# preferencias-driver: rebase e correções das revisões

Responda em português brasileiro. Trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-ac938077b8ec2422e`, branch `worktree-agent-ac938077b8ec2422e`, wip `af00d30`. Não crie outro worktree. Leia `E:\PlotS\plan\20-preferencias-do-driver.md`, `coordenacao-paralela-2.md`, `brief-preferencias-driver.md` e o relatório em `%LOCALAPPDATA%/plots-build/preferencias-driver/relatorio.md`. Build e saídas: `%LOCALAPPDATA%/plots-build/preferencias-driver`. `ui/node_modules` foi apagado: rode `npm ci`. Antes do primeiro grep ou leitura de código, rode `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`.

**Exceções ao mapa de arquivos:** `src-tauri/src/trabalhos.rs` e `ui/src/cartoes.ts`, `CartaoImpressao.*` (só a linha do item 13), `src-tauri/src/main.rs` (encerramento, item 1), `ui/src/confirmacao.ts` (item 12).

## Rebase

Rebase sobre o `master` atual, com o gesto e spooler, o instalador, o intervalo de páginas e as várias impressoras. Só uma trava de gravação de `perfis.toml`: adote a que `varias-impressoras` criou, e remova `GRAVACAO` de `preferencias.rs`. Teste de regressão: uma thread grava modelo, outra grava ajuste, N vezes; no fim os dois valores estão no arquivo.

## Revisão de código (obrigatório)

1. **Janela do driver em thread própria.**
   - Troque `spawn_blocking` por `std::thread::Builder` com nome. Nela: `CoInitializeEx(None, COINIT_APARTMENTTHREADED)`, `DocumentPropertiesW`, `CoUninitialize`, com guarda por `Drop`. Devolva o resultado por canal. Thread que não inicia, ou canal que cai, vira `nao_abriu`.
   - Antes de abrir, desabilite a janela principal; reabilite em `Drop`, em todo caminho de saída.
   - Em `CloseRequested` e no encerramento, se houver janela aberta, localize-a com `EnumThreadWindows` na thread do diálogo e envie `WM_CLOSE`. Espere a thread terminar, com prazo curto. Não chame `destroy()` antes disso.
   - Se `dono` não for válida (`IsWindow`), passe `None`.
   - Janela já aberta (`ja_aberta`): traga-a para a frente.
2. **Prova real, sem captura e sem teclas simuladas:** teste `#[ignore]` que abre a janela de "Microsoft Print to PDF" pelo caminho real, com a thread acima, confirma que ela apareceu com `EnumThreadWindows`, envia `WM_CLOSE`, e confirma que nada foi gravado e que a thread terminou. Limite de tempo em tudo. Registre o título que a janela mostra. Se o driver abrir algo que não fecha assim, encerre seu processo por PID e relate.
3. **Duas faces em dados privados não são detectáveis: decisão do coordenador.** Não tente adivinhar dados privados. Mantenha as conferências do campo público; se `dmDuplex` salvo já vier como duas faces, a linha do ajuste avisa que o PlotS imprime uma face por vez no frente e verso manual. Teste com driver falso, com privados que "ligam duas faces" e público `SIMPLEX`, deixando escrito no teste que esse caso passa e é um limite conhecido.
4. **Saneamento do arquivo:** `dmFormName` copiado do padrão; `dmFields` limitado à máscara do que o padrão devolveu. Teste com `dmFormName` de 32 `u16` sem zero final.
5. **Descartar o ajuste só por erro do driver** (`Resposta`, `SimplexNaoAceito`, papel), nunca por `Sistema` de `CreateDCW` transitório. Teto por ajuste de 32 KiB. Remova `pub` sem uso de `ajuste_padrao_da_fila` e `driver_aceita_ajuste`.

## Revisão de interface (obrigatório, nesta ordem)

6. **Estado "janela aberta" na linha**, do clique até a janela fechar: engrenagem com fundo `--panel-hover`, `aria-busy` e foco mantido. Abaixo da linha, nota neutra com a geometria de `.avisoNaLinha`: padding 4px 8px, borda 1px `--border`, fundo `--bg-sunken`, texto `--fg` 11px, ícone `Info` 12px, `role="status"`.
   - pt: "Vale só no PlotS, a partir do próximo envio. O PlotS continua decidindo papel, cópias, escala, orientação, páginas por folha e faces."
   - en: "Applies only in PlotS, from the next job. PlotS still decides paper, copies, scale, orientation, pages per sheet and sides."
7. **Textos:** `preferencias.titulo`: "Preferências do driver, só no PlotS" / "Driver preferences, PlotS only". `preferencias.rotulo`: "Preferências do driver de {nome}, só no PlotS" / "Driver preferences for {nome}, PlotS only". `preferencias.salvo`: "Ajuste só no PlotS" / "PlotS-only settings". O texto de `.ajusteSalvoTexto` também por `aria-describedby` em "Usar padrão", sem depender só do `title`. `preferencias.salvo.titulo`: "O PlotS usa este ajuste do driver nesta impressora e não altera o padrão do Windows." / "PlotS uses these driver settings for this printer and does not change the Windows default."
8. **"Usar padrão" pede confirmação**, no padrão de diálogo existente. Título "Voltar {nome} ao padrão?" / "Return {nome} to default?". Corpo: "O ajuste salvo no PlotS será apagado. Para tê-lo de novo, refaça na janela do driver." / "The settings saved in PlotS will be deleted. To get them back, redo them in the driver window." Botões "Manter ajuste" / "Keep settings" (foco inicial) e "Usar padrão" / "Use default". Esc fecha e devolve o foco a "Usar padrão". Confirmar leva o foco à engrenagem.
9. **Linha de fila real com uma só anatomia:** primeira fileira, nome e engrenagem à direita; segunda, seletor de 108 px, sempre; depois, estado, entrada, ajuste. `.textos` com a largura inteira, e "Ajuste só no PlotS" + "Usar padrão" em uma linha, em pt e en. Ordem de DOM e de Tab: caixa, engrenagem, seletor, "Usar padrão". Mantenha `flex-wrap` como reserva. Simulada não muda.
10. **Avisos de ajuste:** botão X de 12 px em caixa de 24×24, com `aria-label` "Dispensar aviso de {nome}" / "Dismiss warning for {nome}", na linha e no topo. O foco vai à engrenagem da linha; sem linha, ao botão de atualizar. A dispensa vale até o fim da sessão: `avisosDepoisDaLeitura` não recria o aviso de "corrompido" dispensado. `fila_sumiu` sai sozinho quando a fila deixa a lista, e nunca sobe ao topo pedindo "Atualize a lista". Ajuste de fila que não está na lista aparece no topo com "Usar padrão", para poder ser apagado (`filas_com_ajuste()`).
11. **Foco de "Usar padrão":** `column-gap` de `.ajusteSalvo` de 2 para 6 px e `margin-top` de 2 px; outline de 2 px e offset 0. O anel não toca texto vizinho, nos dois temas e idiomas.
12. **Diálogo de envio real**, para fila com ajuste, depois de "Papel": "Driver: ajuste só no PlotS. Papel e faces seguem o PlotS." / "Driver: PlotS-only settings. Paper and sides follow PlotS."
13. **Recusa no cartão:** linha sem caixa, `CircleAlert` 12 px em `--status-offline-text`, texto `--fg` 11 px. "Enviado com o padrão da impressora: o driver não aceitou o ajuste salvo." / "Sent with the printer default: the driver did not accept the saved settings." O aviso da linha continua. Estenda o `scrollIntoView({ block: "nearest" })` a `avisoDoAjuste`, exceto com cartão da mesma impressora na tela, sem tirar o foco.
14. **Acabamento:** `:active` com `scale(.97)` em `.engrenagem` e `.voltarAoPadrao`; `box-shadow` na transição de `--anim-fast`; `.voltarAoPadrao` e `.ajusteSalvoTexto` em 11/16; `.iconeAviso { color: CanvasText }` nas cores forçadas.
15. **Capturas** nos dois temas e idiomas, lidas uma a uma: envio concluído no padrão (a 04 antiga é igual à 03); "janela aberta" e `ja_aberta`, com a engrenagem de uma fila fictícia ou pelo gancho, sem capturar a janela nativa; recusa no cartão, inclusive na janela mínima; confirmação de "Usar padrão"; dispensa de aviso; `fila_sumiu` depois de atualizar; cores forçadas com "Ajuste só no PlotS" e "Usar padrão", em repouso, hover e foco.

## Final

CI completo em um único log, autoverificação com a base do rebase, novo wip, `git status` limpo, apague `ui/node_modules` e `ui/dist`. Relatório em `%LOCALAPPDATA%/plots-build/preferencias-driver/relatorio-correcoes.md`, com a mesma mensagem na resposta final: item a item, testes com grupo de falha prévia, linha do CI, capturas com caminho, textos novos, o que não foi verificado. Não espere processo em segundo plano sem limite de tempo. Use pasta própria para scripts: o scratchpad é compartilhado entre os agentes.
