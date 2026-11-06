# Opções da impressora na lateral

**Estado:** Concluído no código

## Objetivo

Ter, ao lado da engrenagem de cada impressora real, uma chave de boca que abre um painel com as
opções que o **driver daquela fila** declara. O painel aplica no ajuste salvo da fila, sem abrir
a janela do fabricante e sem mudar o padrão do Windows.

## Decisão de produto

A pesquisa `plan/research/pdfjs-printing.md` mostra que o pdf.js não conhece impressoras: quem
expõe papel, bandeja, cor e resolução é a camada nativa do navegador/SO. O PlotS é nativo e já
fala `DeviceCapabilities`/`DocumentProperties`, então o painel usa essas APIs.

O PlotS **decide** papel, uma face por passada, uma cópia, escala 100%, retrato e uma página por
folha. Esses campos não entram no painel: o envio continua fixando-os por cima do ajuste. O
painel mostra apenas o que o driver declara e o PlotS não força:

- **Bandeja de entrada** (`dmDefaultSource`, `DC_BINS`/`DC_BINNAMES`).
- **Resolução/qualidade** (`dmPrintQuality` + `dmYResolution`, `DC_ENUMRESOLUTIONS`).
- **Cor** (`dmColor`, `DC_COLORDEVICE`).

Tipo de mídia fica de fora nesta versão: `DeviceCapabilities` devolve códigos sem nomes
confiáveis, e um seletor de números não ajuda ninguém. Se o driver não declarar nenhuma das três,
o painel diz que não há opções ajustáveis e aponta a engrenagem.

## Comportamento

- **Chave de boca** ao lado da engrenagem, só em fila real. Ícone `Wrench` de 14 px, 24 px de
  alvo, `aria-label` "Opções da impressora <nome>", `title` "Opções da impressora", sem marcar a
  impressora.
- **Painel** modal com o nome da fila, uma seção por opção disponível e a nota de que papel,
  face, cópias, escala e orientação continuam com o PlotS.
- **Valores atuais:** o painel parte do ajuste salvo; sem ajuste, do padrão do driver. Cada
  seletor tem "Padrão da impressora" para não fixar aquele campo.
- **Aplicar** grava o `DEVMODE` validado no arquivo de perfis, como a engrenagem já faz. O
  próximo envio parte dele; trabalho em andamento não muda.
- **Cancelar/Esc** não grava. **Usar padrão** apaga o ajuste da fila e fecha.
- **Driver recusou:** o painel mostra a falha e não grava; o ajuste anterior fica.
- **Fila sumiu:** mensagem e o painel fecha; a lista se atualiza.
- **Sem opções:** painel com a frase e um botão para abrir a engrenagem.
- **Foco:** ao fechar, volta para a chave de boca; ao aplicar, a linha mostra o selo de ajuste
  salvo. O painel cabe em 640×420, com o corpo rolando e os botões fixos.

## Backend

- `plots-windows`: leitura das capacidades e aplicação no `DEVMODE`.
  - `opcoes_da_fila(fila, salvo) -> OpcoesDaFila` usa `listar_filas` para a porta e
    `DeviceCapabilitiesW`, e o ajuste salvo (ou o padrão) para os valores atuais.
  - `aplicar_opcoes_da_fila(fila, salvo, escolhas) -> Vec<u8>` monta o `DEVMODE` a partir do
    salvo/padrão, marca só os campos escolhidos, valida com `DocumentPropertiesW` e devolve os
    bytes; recusa vira `ErroPreferencias::Envio`.
  - Nada de `DM_DUPLEX`, `DM_PAPERSIZE`, `DM_COPIES`, `DM_SCALE`, `DM_ORIENTATION` ou `DM_NUP`
    no painel.
- `src-tauri`: comandos `opcoes_da_impressora` e `aplicar_opcoes_da_impressora`, com os mesmos
  códigos de falha de `preferencias.rs`; grava com `Perfis::definir_ajuste_do_driver` e a trava
  `GRAVACAO_DE_PERFIS`. "Usar padrão" reusa `voltar_ao_padrao_da_fila`.

## UI

- `ui/src/components/OpcoesDaImpressoraDialog.tsx` + CSS module; Radix Dialog, tokens de
  `theme.css`, mesmos botões de `Botao.module.css`.
- `ui/src/opcoes-impressora.ts` com tipos, `invoke` e normalização (códigos fora do esperado
  caem em falha).
- `Sidebar.tsx`: botão `Wrench` ao lado de `Settings`, prop `aoAbrirOpcoes`; `App.tsx` guarda a
  fila aberta e renderiza o diálogo.
- `usePreferencias` ganha `recarregar` para atualizar o selo depois de aplicar/voltar ao padrão.
- Textos pt-BR e en em `i18n.ts`.

## Testes

- Rust: parsing de `DC_BINS`/`DC_BINNAMES`/`DC_ENUMRESOLUTIONS`/`DC_COLORDEVICE` com arrays
  puros; aplicar escolhe só os campos pedidos, valida com um driver falso e recusa sem gravar;
  "sem opções" devolve listas vazias; nenhum campo forçado é marcado.
- Tauri: comando aplica e grava no perfil temporário; falha do driver não grava; fila sumida
  devolve código próprio.
- UI: o botão aparece só em fila real, tem nome acessível e abre o painel; o painel mostra só as
  opções disponíveis, aplica, cancela, usa padrão, mostra erro e volta o foco. Testes de teclado
  e de estado vazio.
- Uso real com CDP: fila "Microsoft Print to PDF" (ou impressora real, se houver), abrir a chave
  de boca, conferir as opções declaradas, aplicar uma resolução/bandeja e ver o
  `perfis.toml` temporário mudar; capturas em 640×420 e no tamanho normal, temas escuro e cinza.
- Falha anterior registrada para os testes novos.

## Depende de

`08-impressao-windows`, `11-lateral-impressoras`, `20-preferencias-do-driver`.

## Pronto quando

A chave de boca abre um painel com as opções que o driver declara, aplica no ajuste da fila e o
envio seguinte usa o que foi escolhido, sem tocar no padrão do Windows.


## Resultado do lote

- `plots-windows/src/opcoes.rs` lê `DC_BINS`/`DC_BINNAMES`, `DC_ENUMRESOLUTIONS` e
  `DC_COLORDEVICE`; `opcoes_da_fila(fila, salvo)` parte do ajuste salvo ou do padrão.
- `aplicar_opcoes_da_fila` marca só bandeja, resolução/qualidade e cor, valida com
  `DocumentPropertiesW`, confere o que voltou e repõe o padrão do driver nos campos que a
  pessoa deixou em “Padrão da impressora”. Papel, face, cópias, escala, orientação e N-up
  continuam fora do painel e do ajuste.
- A chave de boca fica ao lado da engrenagem, só em fila real; o painel aplica, cancela, usa
  padrão, mostra falha e devolve o foco. O estado vazio e o corpo rolável foram provados.
- Evidência: commit `559a9c8`; CI em
  `%LOCALAPPDATA%\plots-build\integracao-alpha5\saidas\ci.log`; capturas em
  `%LOCALAPPDATA%\plots-build\opcoes-impressora\saidas` (nativas 640×420, temas escuro e
  cinza) e `falha-antes-padrao.log`.
- Falta: nenhum driver instalado declara bandeja ou zero opções, então a bandeja real e o
  estado vazio real não foram exercitados; o envio seguinte usando o `DEVMODE` salvo foi
  provado nos bytes e na releitura, não em uma impressão.