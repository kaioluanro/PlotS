# varias-impressoras: modos espelho e divisão na janela, e grupos salvos

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia primeiro `E:\PlotS\plan\briefs\coordenacao-paralela-2.md`, inclusive o contrato com `intervalo-paginas`. Fontes: `plan/15-multiplas-impressoras.md`, `plan/13-fluxo-e-recolocacao.md`, `plan/06-perfis.md`, no worktree.

## Comportamento

1. **Modo:** com duas ou mais impressoras marcadas, as opções mostram Espelho (cada impressora imprime tudo; padrão) e Divisão (os lotes são repartidos). Com uma só, nada aparece, e o modo não altera o envio. O modo fica na aba.
2. **Espelho** continua como já é hoje.
3. **Divisão:** `distribuir_lotes` já reparte em rodízio. Use-o por impressora, recalculando as passadas com o perfil de cada uma. Cada trabalho avança sozinho.
   - O cartão diz o número do lote **no documento** e quantos faltam nesta impressora, para o usuário saber onde cada maço entra.
   - **Juntar os maços é o que mais pode dar errado:** a confirmação e o fim do trabalho dizem, em frases curtas e na ordem física, qual lote saiu de qual impressora e como empilhar para o documento ficar em ordem, lembrando quando uma impressora precisa virar o maço para ler. Se os modelos marcados pedem gestos finais diferentes, o texto trata cada maço pelo nome da impressora.
   - Mais impressoras do que lotes: as que ficam sem lote não recebem trabalho nem cartão, e a confirmação diz isso antes de enviar.
4. **"Reimprimir"** refaz só aquele lote naquela impressora. Erro em uma impressora não para as outras, e o cartão diz quais lotes ficaram sem imprimir nela.
5. **Grupos salvos:** salvar a seleção atual e o modo com um nome ("Todas do escritório"), aplicar e apagar. Ficam no mesmo arquivo de perfis, por `plots-perfis`, com gravação atômica como a dos perfis. Arquivo antigo sem grupos continua carregando, e arquivo novo não perde perfis. Nome com limite de tamanho, sem repetir. Ao aplicar, impressoras que não existem mais são ignoradas, e a interface diz quais. Comandos em `src-tauri/src/grupos.rs`.

**Sem `tokio`:** cada trabalho já roda na própria thread, e o requisito de avançar em paralelo vale pelo comportamento. Não adicione dependência, e relate se achar algum motivo real para o contrário.

Fora de escopo: consulta do spooler (é de outro lote), calibrar novas filas.

## Testes

- Rust: divisão com perfis diferentes (cada impressora com a sua ordem, mesmas páginas no total, nenhuma página repetida ou perdida); mais impressoras que lotes; erro em uma não afeta a outra; grupos (ida e volta, arquivo antigo, impressora sumida, nome repetido, nome hostil, arquivo corrompido não apaga nada).
- Propriedade com `proptest`, que já é do projeto: toda página do documento sai exatamente uma vez na divisão.
- Vitest: redutor, frases de junção.
- App real por CDP: as duas simuladas juntas com 6 lotes, em espelho e em divisão, levando os trabalhos até o fim, cada um no próprio ritmo. Salve e aplique um grupo com `PLOTS_PERFIS` temporário. Capturas nos dois temas, em inglês e na janela mínima. Leia cada imagem.
