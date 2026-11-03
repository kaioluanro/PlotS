# seletor-modelos, correção 1

Responda em português brasileiro. Você é um agente novo: trabalhe no worktree existente `E:\PlotS\.claude\worktrees\agent-a68b705f874ca4a0e` (branch `worktree-agent-a68b705f874ca4a0e`, commit `95f3594` sobre `60ef119`); não crie outro nem mexa em E:\PlotS fora dele. Faça um commit wip ao terminar, depois do CI verde; nunca dê push nem squash.

As regras de ambiente e segurança de `E:\PlotS\plan\briefs\brief-seletor-modelos.md` continuam valendo: leia esse brief primeiro. Use `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/seletor"` e `CARGO_INCREMENTAL=0`. Scripts de uso em `target/e2e/`. Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh 60ef119 plots-core plots-perfis plots-print plots-cli src-tauri ui`.

**Capturas só por CDP (`Page.captureScreenshot`).** Apague `target/e2e/tela.ps1` e a função `fotoDaTela` de `target/e2e/cdp.mjs`: nunca capture a tela do Windows, porque isso já pegou a área de trabalho do usuário por engano.

## A. Dados
1. **Um perfil ruim não pode trancar o arquivo inteiro.** Em `plots-perfis/src/formato.rs`, na leitura, um `modelo` desconhecido ou com valores divergentes vira entrada personalizada (`modelo: None`, valores gravados), e `borda_virada` em perfil que não vira é tratada como não virar. O arquivo abre, e as outras filas ficam intactas. A recusa fica só ao gravar, em `codificar`. Troque `modelo_com_valores_de_outro_recusa_o_arquivo`, `modelo_invalido_recusa_o_arquivo_sem_alterar_nada` e `borda_da_virada_em_perfil_que_nao_vira_recusa_o_arquivo` por testes do comportamento novo, e acrescente um teste de que `definir_modelo` conserta uma entrada incoerente. Registre a falha prévia.
2. **`plots filas`:** a sugestão passa a incluir `plots perfil "NOME" --modelo jato|laser`, e a listagem mostra o modelo, quando houver.
3. **Teste novo:** `imprimir --simular --ordem impares` com perfil de cada modelo, com "3 de 3 folhas corretas".
4. **`ui/src/calibrar.test.ts`:** troque `Jato G3110` por `Jato Sala 3`, na entrada e no comando esperado.

## B. Interface
5. **Confirmar antes de substituir perfil personalizado.** Só quando o estado for "personalizado" e a escolha for um modelo, abra o diálogo existente (role dialog, foco preso, Esc) antes de gravar.
   - Título: "Substituir o perfil personalizado de {nome}?" / "Replace the custom profile of {nome}?"
   - Texto: "Os valores salvos para esta impressora serão trocados pelos do modelo {modelo}. Para recuperá-los, será preciso calibrar de novo." / "The values saved for this printer will be replaced by those of the {modelo} model. To get them back, you will need to calibrate again."
   - Botões à direita, gap 8 px: "Cancelar" / "Cancel" (secundário, foco inicial) e "Substituir perfil" / "Replace profile" (variante perigo).
   - Cancelar ou Esc: nada é gravado, o seletor continua em personalizado e o foco volta a ele. Modelo para modelo e "Sem perfil" para modelo continuam sem confirmação.
6. **Posição do seletor independente do modelo.** Largura fixa de 104 px em todos os valores, sem reticências: meça nos dois idiomas e ajuste se o rótulo mais largo não couber. Encurte "Perfil personalizado" para "Personalizado" e "Custom profile" para "Custom"; o `title` mantém a palavra perfil. Só nas linhas com seletor, troque o `flex-basis: 88px` do nome por `flex: 1 1 auto`: o seletor fica à direita se o nome couber em 1 linha ao lado dele e desce, alinhado em x=38, se não couber. Alternar Jato, Laser e Sem perfil não muda altura nem posição, e "Office inkjet — second floor, room 12" aparece inteiro em 2 linhas.
7. **Falha ao gravar junto da impressora.** Para `modelo_nao_gravado`, mostre o aviso logo abaixo da linha afetada, dentro do item da lista, com o mesmo texto, ícone, borda e caminho, mantendo `role="alert"`. Role para deixar linha e aviso visíveis (`scrollIntoView`, `block: "nearest"`) e mantenha o foco no seletor. Avisos gerais continuam no topo.
8. **Divisor antes de "Calibrar…":** `hr` de 1 px em `--border`, margem de 4 px em cima e embaixo, `CanvasText` em cores forçadas. Confira também na lista do sistema.
9. **Um único item ativo na lista:** o ponteiro sobre uma opção move o foco para ela.
10. **Vocabulário e planos.** Troque "modo" por "perfil" nos textos visíveis de `calibrar.*`, `aviso.perfis_*` e `modo.desconhecido.titulo`. Atualize `plan/11-lateral-impressoras.md` (tabela de modos, decisões) para Jato de tinta e Laser, e `plan/14-padrao-visual-e-idiomas.md` (seletor, 104 px, regra de descida, exceção de 24 px e raio de 4 px).
11. **Entrada da lista:** opacidade em 120 ms com `--ease-out-fluid`, sem deslocamento.
12. **Lista do sistema como reserva:** com `@supports` desligado, se as setas gravarem a cada passo ou abrirem "Calibrar…", corrija: as setas abrem a lista.
13. **Indicador do valor marcado na lista aberta:** visível além do peso. Se não aparecer, use o check Lucide de 12 px em `currentColor`.

## Capturas
Em `target/shots/`, com prefixo `fix-`, nos dois temas, salvo indicação: diálogo de confirmação (também em en); seletor nos três estados, na mesma linha; nome longo em 2 linhas; falha na janela mínima com a lista rolada até a última fila (pt-BR e en); lista aberta com divisor e com modelo marcado; hover e foco juntos; lista do sistema com setas; lista aberta na última linha visível da janela mínima; gravação lenta. Leia cada imagem. Meça todo par novo com `contrast.py`.

## Relatório
Cada item com o que mudou e seu teste, grupo de falha prévia, linha do CI (`cargo fmt --all --check`, `cargo test --workspace --locked`, `npm test`, `npm run build`), autoverificação, contraste, capturas, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo.
