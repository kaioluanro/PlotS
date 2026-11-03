# laser-verso-girado: corrigir os versos de cabeça para baixo na laser

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia `plan/19-modelos-de-impressora.md`, `plan/23-plotsim.md` e as regras comuns de `plan/briefs/coordenacao-paralela-2.md`. Build em `%LOCALAPPDATA%/plots-build/laser-verso-girado`, porta CDP 9348. Antes do primeiro grep ou leitura de código, `graphify query "<pergunta>" --graph E:/PlotS/graphify-out/graph.json`. Não mexa em `plan/`. Outros dois agentes trabalham ao mesmo tempo: opções de envio, em `.deps-fetch/retomada-opcoes` (entra antes de você), e filas do PlotSim, em `PlotSim/plotsim-core/`, `plots-windows/src/trabalho.rs` e `scripts/`. Não toque nesses arquivos.

## Fatos, confirmados pelo usuário em 2026-11-03

Laser: a folha entra pela gaveta, sai pela parte de cima da máquina com a face impressa para baixo, e a borda que entra primeiro é a primeira a sair. A geometria do `plotsim-physics` e a revisão à mão dizem: recolocando o maço como saiu, sem girar, entra primeiro a outra borda; com o verso sem giro, todo verso sai de cabeça para baixo. A hipótese restante é o verso ser lido como página de livro, a convenção do PlotS.

## Decisão

**Opção A: o PlotS gira o verso 180° na laser, e a pessoa recoloca como saiu.** O gesto não muda. Nenhuma ordem de página muda: frente 6, 4, 2; verso 1, 3, 5, agora girado.

## Escopo

1. `ModeloImpressora::Laser` com `verso_girar_180` verdadeiro, em `plots-core`. Inclua o perfil embutido e tudo o que derive dele. Confira perfil salvo: fila com modelo laser gravado tem de valer o novo perfil. **Perfil personalizado, calibrado, não muda sozinho.**
2. **Corrija o simulador do PlotS**, que aprovava o erro: em `plots-print`, a mecânica da saída para baixo tem de fazer a curva em U, com a inversão da aresta que lidera. Mantenha a tabela de mecânicas coerente. Atualize os testes que afirmavam o contrário, explicando em cada um por que mudou, sem enfraquecer nada. As 16 mecânicas contra os 24 perfis precisam fechar na mesma regra do modelo físico: virada de livro pede verso sem giro; bloco de notas pede verso girado; não virar pede verso girado.
3. **Frase e figura:** a frase da laser passa a dizer que não se gira: "Não vire nem gire o maço." / "Do not flip or turn the stack.", no cartão e no CLI, porque girar sem perceber agora estragaria o verso. Na figura, o X de "não virar" continua; a sequência do cartão mostra o verso como girado, como no jato. Confira o texto de calibração e o que mais descreva a laser.
4. **Divisão e espelho** com jato e laser: as frases de junção continuam certas. Confira.
5. **PlotSim:** em `PlotSim/plotsim-physics/tests/`, remova o `#[ignore]` de `a_laser_ao_pe_da_letra_fecha_em_toda_a_matriz`, e inverta os testes que documentavam o defeito (`a_laser_ao_pe_da_letra_acerta_a_ordem_e_erra_so_a_orientacao_do_verso`, `quadro_de_cada_modelo_por_forma_de_recolocar`): agora a laser literal fecha. Atualize a tradução da borda no adaptador, sem mudar o modelo físico: **`plotsim-physics/src/` não muda**; se parecer necessário, pare e relate.
6. **Fila de PDF:** envie na laser com a fila "Microsoft Print to PDF" um documento de 6 páginas e abra os PDFs de saída: verso 1, 3, 5 em ordem, girado 180°, frente 6, 4, 2 sem giro. Cena por CDP nas simuladas, jato e laser, dois temas.

## Testes e final

Falha prévia registrada onde couber. CI completo em um único log, incluindo `ui/`, autoverificação, `git status` limpo com wip, apague `ui/node_modules` e `ui/dist`. Relatório em `%LOCALAPPDATA%/plots-build/laser-verso-girado/relatorio.md`, com a mesma mensagem na resposta final: item a item, cada teste antigo alterado com o motivo, resultado da matriz depois, o que acontece com perfis salvos, capturas e o que não foi verificado. Limite de tempo em todo comando.
