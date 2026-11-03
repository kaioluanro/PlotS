# Acabamento e instalador

**Estado:** Em andamento · OK 1 · PARTIAL 1 · MISSING 1 · BUG 0 · N/A 0

## Objetivo

Deixar o programa pronto para outra pessoa usar.

## Plano de implementação

- [x] Reiniciar lote no controlador de job
- [ ] "Reimprimir lote" e histórico na interface
- [ ] Folha de controle opcional
- [x] Instalador com `tauri bundle` (`1a1f6d7`), gerado por `node scripts/empacotar.mjs`
- [x] Avisos de terceiros, gerados dos arquivos de licença de cada crate
- [ ] Busca do nome no INPI e USPTO
- [ ] Executar o instalador em máquina ou conta de teste: instalação, atalhos, "Abrir com", desinstalação, permanência dos perfis, baixa do WebView2 Runtime
- [x] Dizer na documentação onde os perfis ficam e como apagá-los, porque a pasta Roaming pode acompanhar perfis móveis

## Decisões

- **Instalador por usuário**, sem administrador e sem assinatura de código, então o SmartScreen pode avisar. O arquivo do Windows passa a ser `PlotS.exe`; o crate continua `plots-app`.
- **"Abrir com":** só `OpenWithProgids` e `Applications\PlotS.exe`, gravados por `scripts/instalador/hooks.nsh`. A opção `fileAssociations` do Tauri foi evitada por suspeita, não comprovada, de que grava o padrão de `.pdf`.
- **Não executado nesta máquina:** instalar mexe no registro e no menu Iniciar do usuário. O conteúdo foi conferido pelo script gerado do NSIS, e o exe de release foi testado sem instalar.

## Pronto quando

Instalado em máquina limpa e usado por outra pessoa sem ajuda.

## Depende de

Todas as funcionalidades anteriores.

## Requisitos

| ID | Requisito | Situação |
|---|---|---|
| PS-A017 | Folha de controle opcional no canto do verso | MISSING |
| PS-A018 | "Reimprimir lote" refaz só aquele lote | OK |
| PS-A081 | Instalador (tauri bundle), reimprimir lote, folha de controle, histórico | PARTIAL |

## Especificação

- Se o usuário errar, "Reimprimir lote" refaz só aquele lote.
| Fase | Entrega | Pronto quando |
| --- | --- | --- |
| 6. Acabamento | Instalador (`tauri bundle`), reimprimir lote, folha de controle, histórico | Instalado em máquina limpa e usado por outra pessoa sem ajuda |
