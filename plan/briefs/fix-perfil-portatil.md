# Correção: apagar os dados no modo portátil

Rodada de correção do lote `agent-perfil-portatil`, pedida pela revisão de código. Só o
`README.md` muda; nenhum código, teste, `plan/` ou zip.

Leia `E:\PlotS\AGENTS.md`, `.claude/skills/craft-team/SKILL.md` e
`.claude/agents/plots-implementer.md`. Worktree existente
`E:\PlotS\.claude\worktrees\agent-perfil-portatil`, HEAD `9c3607c`, sem criar outro.

**Graphify:** antes do primeiro grep ou leitura de código, `graphify query "<pergunta>"
--graph E:/PlotS/graphify-out/graph.json`. Ao terminar, `graphify update .`.

## O que corrigir

Em `README.md`, seção **Seus dados**:

1. A primeira frase hoje afirma sem condição que os perfis ficam em
   `%APPDATA%\plots\perfis.toml`. Troque por: "No programa instalado, os perfis das impressoras e
   os ajustes que você salvou nas preferências do driver ficam em `%APPDATA%\plots\perfis.toml`,
   um arquivo de texto."
2. A receita de apagar tudo hoje diz só para apagar a pasta `plots`, o que não vale no portátil:
   o `perfis.toml` ao lado do `PlotS.exe` continua lá (com possível PIN/senha) e, se só ele for
   apagado, a migração traz a cópia de `%APPDATA%` de volta na abertura seguinte.
   - Para o instalado: "Para apagar tudo no programa instalado, feche o PlotS e apague a pasta
     `plots`."
   - Para o portátil: "No portátil, apague também o `perfis.toml` que está na pasta do
     `PlotS.exe` **e** a cópia em `%APPDATA%\plots` (ou a pasta inteira): apagar só uma das duas
     não basta, porque o arquivo local continua ou a cópia de `%APPDATA%` volta na próxima
     abertura."
   - Mantenha o resto da frase (o que a próxima abertura faz, histórico e a opção
     **Guardar histórico**), ajustando "na próxima abertura" para "na próxima abertura, no
     instalado".
3. Não mude o parágrafo do modo portátil já existente, salvo o que for necessário para o texto
   acima concordar.

O texto deve continuar em português, no tom do README, sem marca de terceiro e sem prometer
comportamento que o código não tem.

## Final

- `git status` limpo, um wip commit com o `README.md`.
- `node_modules` e `dist` do worktree apagados.
- Não rode CI completo (mudança só de documentação); diga no relatório que o CI anterior continua
  válido para o código e que esta rodada não o invalida.
- Resposta final com o diff do README e o que não foi verificado.
