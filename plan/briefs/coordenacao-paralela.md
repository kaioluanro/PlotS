# Coordenação de quatro agentes em paralelo

Cada agente mexe só nos arquivos do seu lote. Se precisar de um arquivo de outro lote, não edite: relate ao coordenador. Quem integra em `master`, na ordem abaixo, é só o coordenador, com rebase e CI a cada entrada.

| Ordem | Lote | Pode mexer em | Não pode mexer em |
|---|---|---|---|
| 1 | `pdf-robustez` | `plots-pdf/` | todo o resto |
| 2 | `tela-abertura` | `src-tauri/src/abertura.rs` (novo), `src-tauri/tauri.conf.json`, `src-tauri/capabilities/`, `ui/abertura.html`, `ui/src/abertura/` (novos), `ui/vite.config.*`; em `src-tauri/src/main.rs`, só a linha do `mod`, o registro do comando e a chamada no `setup` | `ui/src/App.tsx`, `ui/src/tabs.ts`, `ui/src/components/` |
| 3 | `visualizador` | `ui/src/visualizador/` (novo), `ui/src/components/DocumentArea.*`, `ui/package.json`, `ui/package-lock.json`, `src-tauri/src/documento.rs`; em `src-tauri/src/main.rs`, só o registro do comando | `ui/src/App.tsx`, `ui/src/tabs.ts`, `Sidebar.*`, `plots-*` |
| 4 | `cartoes-impressao`, já em andamento | `plots-cli/`, `plots-print/`, `plots-windows/`, `plots-core/`, `src-tauri/src/` (exceto `abertura.rs` e `documento.rs`), `ui/src/App.tsx`, `ui/src/tabs.ts`, `ui/src/components/` (exceto `DocumentArea.*`) | `plots-pdf/`, `ui/package*.json`, `src-tauri/tauri.conf.json`, `ui/src/visualizador/`, `ui/src/abertura/` |

## Regras comuns
- **`ui/src/i18n.ts`:** todos podem acrescentar chaves, mas só com o prefixo do lote (`visualizador.*`, `abertura.*`, `erro.pdf_*`), em bloco próprio, no fim de cada idioma. Não reordene nem altere as chaves existentes.
- **`Cargo.toml` e `Cargo.lock`:** nenhum lote traz dependência Rust nova. Se achar que precisa, relate.
- **`plan/`:** agentes não editam. O coordenador atualiza planos e registros.
- **Compilação:** cada lote usa a própria pasta em `%LOCALAPPDATA%/plots-build/<lote>`, com `CARGO_INCREMENTAL=0` e `-j 4`. Vários agentes compilam ao mesmo tempo: se o sistema matar um build por memória, espere 2 minutos antes de repetir.
- **Disco E:** vigie com `df -h /e`. Abaixo de 500 MB, pare e relate. Apague `ui/node_modules` e os PDFs temporários do seu worktree ao terminar.
- **Processos:** encerre só os seus próprios PIDs, por número exato. Vários `plots-app` podem estar abertos ao mesmo tempo, então use uma porta de depuração e uma pasta do WebView2 só suas, e confira que a porta é do seu processo.
- **Capturas:** só por CDP (`Page.captureScreenshot`), nunca a tela do Windows.
- **Impressoras:** só `cartoes-impressao` envia, e somente às simuladas e à fila de PDF. Os outros lotes não enviam nada a nenhuma fila.
