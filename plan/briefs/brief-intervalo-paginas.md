# intervalo-paginas: escolher de qual a qual página imprimir

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia primeiro `E:\PlotS\plan\briefs\coordenacao-paralela-2.md`, inclusive o contrato com `varias-impressoras`. Fontes: `plan/12-visualizador.md`, `plan/13-fluxo-e-recolocacao.md`, `plan/05-tamanhos-de-papel.md`, no worktree.

## Comportamento

1. O padrão é o documento inteiro. O usuário pode escolher um único trecho contínuo, de uma página a outra, nas opções de impressão da lateral. Cada aba guarda o seu.
2. **Backend decide:** `IntervaloPaginas` em `plots-core`, validado contra o total (`1 <= início <= fim <= total`, números hostis recusados sem estouro). O plano do trecho **mantém os números originais do documento**: o trecho 3–8 envia as páginas 3 a 8 de verdade, e a sequência do cartão usa esses números, na ordem de cada modelo. Trecho ímpar recebe o verso em branco como o documento inteiro recebe. Papéis diferentes dentro do trecho seguem a regra de lotes por papel, considerando só o trecho.
3. **A confirmação** e o `resumo_do_envio` contam só o trecho: páginas, folhas, lotes, papéis.
4. **Validação na interface** sem travar a digitação: campo vazio ou fora do total explica o problema, e o botão de imprimir explica por que está desabilitado. Enquanto as páginas não foram contadas, ou o PDF foi recusado, os campos ficam desabilitados.
5. **Trabalho em andamento não muda:** alterar o intervalo só vale no próximo envio. "Reimprimir" refaz o mesmo lote já planejado.
6. **Arrastar:** confira se soltar um PDF na janela abre a aba. Se não, implemente com o evento de arrastar do Tauri, sem afrouxar a CSP e sem permissão nova que não seja necessária. O arrasto nativo do sistema não dá para simular por CDP: teste a lógica e relate o gesto real como não verificado.

Fora de escopo: vários trechos separados (`1,3,5-7`), marcar o trecho nas miniaturas, CLI.

## Testes

- Rust, em `plots-core` e no pedido: trecho par e ímpar nas duas ordens, primeira e última página, uma página só, o documento inteiro igual ao sem intervalo, fim antes do início, zero, acima do total, valor enorme, papéis mistos, números originais preservados.
- Vitest: redutor da aba e validação dos campos.
- App real por CDP, com impressoras simuladas: 12 páginas de teste, trecho 3–8, em jato e laser. A sequência do cartão e as páginas enviadas têm os números certos. Capturas nos dois temas, com erro de validação, em inglês e na janela mínima. Leia cada imagem. Confira também a fila de PDF com um trecho, abrindo o arquivo de saída para contar páginas.
