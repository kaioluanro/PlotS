# pdfs-reais, correção 1: deixar claro que a cópia não contém a assinatura

Responda em português brasileiro. Trabalhe no mesmo worktree: `E:\PlotS\.claude\worktrees\agent-a973cdb65813dca16`. Atenção: o coordenador fez rebase sobre `master`; o commit agora é `ec853bc` sobre `cc9d4f8`. Faça commits wip; nunca dê push nem squash.

Valem as regras do brief `E:\PlotS\plan\briefs\brief-pdfs-reais.md`, inclusive as dos PDFs reais. Use `CARGO_TARGET_DIR="C:/Users/Kaio Luan/AppData/Local/plots-build/pdfs-reais"`. Autoverificação: `sh E:/PlotS/.claude/team/selfcheck.sh cc9d4f8 plots-cli plots-pdf`.

## Escopo
1. **Texto de uso.** Acrescente a `USO_GERAR_PDF`: `Se o PDF tiver assinatura digital, as passadas mostram a aparência dela, mas não a contêm: servem só para imprimir e não comprovam a assinatura. Para enviar ou guardar o documento, use o arquivo original.`
2. **Aviso na saída.** Exponha em `PdfOrigem` um método como `tem_assinatura_aplicada()`, reaproveitando `campo_assinado` nas anotações de todas as páginas. Quando for verdadeiro, `gerar-pdf` escreve antes da linha "Gerados...": `Aviso: este PDF tem assinatura digital. As cópias geradas servem só para imprimir: mostram a aparência da assinatura, mas não a contêm. Para enviar ou guardar o documento, use o arquivo original.`
3. **`SigFlags`.** Tire `b"SigFlags"` da lista copiada para o `/AcroForm` novo: o arquivo não pode declarar assinaturas que não contém.
4. **Comentário no carimbo.** Em `assinatura_como_carimbo`, defina sempre `/Contents` como `Cópia para impressão: a assinatura digital original não está neste arquivo. / Print copy: the original digital signature is not in this file.`, codificado corretamente como texto do PDF. Confirme com a conferência de imagens que a impressão não muda.
5. **Assinatura sem `/Type`.** Reconheça também pelo `/FT` igual a `Sig`, herdado na mesma subida até o campo pai, com `/V` presente, para que os dados do signatário nunca sigam na cópia.

## Testes
- CLI, com fixture sintética assinada: exige o aviso. Sem assinatura: exige a ausência dele. `--help` mostra o texto de uso.
- Fixture com assinatura mais campo de texto na mesma página: `/AcroForm` sem `SigFlags` e campo comum preservado.
- Carimbo com o `/Contents` novo, inclusive quando o widget original já tinha `/Contents`.
- Assinatura com `/FT` igual a `Sig` sem `/Type`, no widget e no campo pai.
- Remoção de `/ID` do trailer nas duas rotas.
Registre a falha prévia em `target/fail-before-fix.log`.

## Teste de uso
Repita a conferência de imagens em todos os PDFs reais com o binário final e relate aceitos, recusados e divergentes, sem nomes nem conteúdo, e quantos geraram o aviso de assinatura.

## Limpeza
Ao final, apague de `target/` do worktree os scripts e logs que mencionem a pasta dos PDFs, depois de relatar os resultados. Mantenha só `ci.log` e os logs de falha prévia, confirmando que eles não têm nomes de arquivo nem títulos.

## Relatório
Cada item com testes e grupo de falha prévia, contagem final nos PDFs reais, linha do CI (`cargo fmt --all --check` e `cargo test --workspace --locked`), autoverificação vazia, textos novos, limpeza e o que não foi verificado. Termine com `git status` limpo.
