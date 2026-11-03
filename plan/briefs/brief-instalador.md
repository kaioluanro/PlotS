# instalador: gerar o instalador do PlotS, com licença e avisos de terceiros

Responda em português brasileiro. Trabalhe no worktree isolado em que você foi iniciado. Leia primeiro `E:\PlotS\plan\briefs\coordenacao-paralela-2.md`. Fontes: `plan/17-acabamento.md` e `plan/licencas-terceiros.md`, no worktree.

**Decisões do usuário, não reabra:** o projeto é MIT, publicado só de graça no GitHub; a licença do pacote PDFium está decidida.

## Escopo

1. **Licença:** `license = "MIT"` em todo `Cargo.toml` de crate e em `ui/package.json`. Confira que o `Cargo.lock` não muda ou, se mudar, relate.
2. **Avisos de terceiros:** um script em `scripts/` gera `THIRD-PARTY-NOTICES` a partir dos arquivos de licença reais de cada crate de produção (`cargo metadata`, cache do registro) e de cada pacote npm de produção, seguindo tudo o que está em `plan/licencas-terceiros.md`: Feather em `lucide-react`, cada arquivo de licença do `pdfjs-dist` listado lá sem `LICENSE_LIBERATION`, `option-ext` com texto e origem do código-fonte, Inter, PDFium e terceiros dele. Crate ou pacote sem arquivo de licença faz o script falhar com o nome dele, nunca inventar texto. Sem ferramenta nova instalada. Rode o script e versione o arquivo gerado.
3. **Bundle:** `tauri bundle` com instalador NSIS por usuário, sem pedir administrador, produto `PlotS`, com `pdfium.dll`, `THIRD-PARTY-NOTICES` e `LICENSE` junto do programa, de um jeito que a procura da DLL já existente encontre. A DLL não entra no repositório: o build lê de `PLOTS_PDFIUM_DIR`. Sem assinatura de código. Ative o bundle só no comando de release, se ativá-lo sempre deixar `cargo test` lento ou dependente da DLL.
4. **Sem dados da máquina no binário:** caminhos do build remapeados, sem nome de usuário nem caminho do disco. Prove com busca em bytes no exe de release por `Kaio`, `Users` e `E:\PlotS`, e relate o que restar.
5. **Associação "Abrir com":** registre PDF como tipo que o PlotS abre **sem tomar o lugar do leitor padrão**. Se o instalador do Tauri não permitir garantir isso, deixe de fora e relate.
6. **`README.md`** curto, em português: o que é, como instalar, licença, onde os perfis ficam e como apagá-los, que a pasta Roaming pode acompanhar perfis móveis, e que nada foi medido em impressora real.

## Limites

- **Não execute o instalador nesta máquina**: instalar mexe no registro e no menu Iniciar do usuário. Confira o conteúdo abrindo o pacote com ferramenta de listagem, se houver, ou pelo log do NSIS, e teste o exe de release sem instalar, com a DLL ao lado.
- O Tauri baixa o NSIS na primeira vez. Se o sistema negar, pare e relate; não contorne.
- Busca do nome no INPI e no USPTO fica fora: é do coordenador.
- Instalador e bundle ficam em `plots-build/instalador`, nunca no E:. Guarde o instalador final em `saidas/` e informe o caminho.

## Testes

- Teste do script: crate fictício sem licença falha; a lista gerada cobre todo crate e pacote de produção; `LICENSE_LIBERATION` não aparece.
- Teste de uso: exe de release, abre PDF de teste, página e miniatura aparecem, sem `__plotsE2e`.
