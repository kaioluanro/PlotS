# preferencias-driver: engrenagem que abre a janela de preferências do driver

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia primeiro `E:\PlotS\plan\20-preferencias-do-driver.md`, que define o comportamento, e `E:\PlotS\plan\briefs\coordenacao-paralela-2.md`, que traz as regras comuns. Base: `0871fbe`, com o intervalo de páginas já no `master`. Lote `preferencias-driver`, porta CDP 9345, build em `%LOCALAPPDATA%/plots-build/preferencias-driver`. Na autoverificação, use a base `0871fbe`.

## Arquivos

Pode mexer em: `plots-windows/` (só o que for novo em `spooler.rs` e `backend.rs`; `gesto-e-spooler` também mexe neles, então mantenha as mudanças pequenas e em módulo novo, como `preferencias.rs`, quando der), `plots-perfis/` (`varias-impressoras` acrescenta grupos ali: use campo próprio e não reformate nada), `src-tauri/src/preferencias.rs` (novo), `src-tauri/src/main.rs` (só registro de comando), `ui/src/components/Sidebar.*` (só a linha da impressora) e `ui/src/preferencias*.ts` (novo). Prefixo de i18n: `preferencias.*`.

Não mexa em: `plots-core/`, `plots-print/`, `plots-cli/`, `CartaoImpressao.*`, `cartoes*.ts`, `tabs.ts`, `plan/`.

## Escopo

1. **Abrir a janela:** comando que chama `DocumentPropertiesW` com prompt (`DM_IN_PROMPT | DM_IN_BUFFER | DM_OUT_BUFFER`), a janela principal como dona, partindo do ajuste salvo, se houver, e fora da thread da interface, para ela não travar. Uma janela por vez, por fila: outro clique enquanto ela está aberta não abre outra. `IDCANCEL` não grava nada. Fila que sumiu vira mensagem clara.
2. **Guardar:** o `DEVMODE` devolvido, por nome da fila, em `plots-perfis`, em formato de texto sem dependência nova, com teto de tamanho e sem perder perfis nem grupos. Um arquivo antigo continua carregando. Dado corrompido é ignorado com aviso, sem apagar o resto. Confira `dmSize`, `dmDriverExtra` e o tamanho real antes de usar: o arquivo é entrada hostil.
3. **Usar no envio:** em `Impressora::devmode`, parta do salvo quando houver, passe pelo driver com `DM_IN_BUFFER | DM_OUT_BUFFER` e só então aplique os campos que o PlotS fixa, conferindo depois como já é feito (`conferir_simplex`, `conferir_aplicado`). **Um ajuste de duas faces, de várias páginas por folha, de cópias ou de outro papel nunca pode vencer o que o PlotS fixa.** Se o driver recusar o salvo, use o padrão da fila, com aviso no cartão ou no resultado do comando, nunca em silêncio. O trabalho em andamento não muda.
4. **Interface:** engrenagem (`lucide-react` `Settings`) em cada impressora real da lista, com rótulo acessível e `title` "Preferências da impressora" / "Printer preferences". Simuladas não têm. Marcar ou desmarcar a impressora continua funcionando como antes, e clicar na engrenagem não muda a marcação. Com ajuste salvo, mostre isso de modo discreto, por mais do que cor, e permita voltar ao padrão da fila. Foco, hover, cores forçadas e janela mínima, como na norma.

## Limites

- O envio de testes segue só para as simuladas e a fila "Microsoft Print to PDF".
- **A janela do driver é nativa:** não capture a tela do Windows nem simule teclas nela. Se não conseguir fechá-la de forma limpa no teste, deixe o gesto real como não verificado e diga como o usuário confere. A lógica fica atrás de uma interface testável com backend falso.
- Não altere o padrão do Windows: nada de `SetPrinter`.

## Testes

- Rust com falso: cancelar não grava; aceitar grava; salvo usado no próximo envio; salvo de duas faces continua saindo em uma face; salvo recusado cai no padrão com aviso; buffer curto, `dmSize` absurdo e texto corrompido não travam.
- Com a fila "Microsoft Print to PDF", sem janela: `DEVMODE` real lido, gravado, relido e aceito pelo driver, e um envio a arquivo sai certo.
- Perfis: ida e volta, arquivo antigo, teto de tamanho.
- Vitest: engrenagem só em fila real; clicar nela não marca a impressora.
- App real por CDP: lista com impressoras reais e simuladas, nos dois temas, em inglês, com foco e hover, com ajuste salvo e na largura mínima. Leia cada imagem.
