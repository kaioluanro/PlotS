# Pesquisa: rasterização com PDFium para impressão GDI

Consultado em 2026-11-02, só em fontes públicas; nada foi baixado.

## Recomendação

`pdfium-render` com a `pdfium.dll` de `bblanchon/pdfium-binaries`, variante sem V8, com tag e sha256 fixos, conferidos no download. Carregar por caminho explícito, ao lado do executável.

| Item | Valor | Licença |
|---|---|---|
| `pdfium-render` | 0.9.5 no crates.io; carregamento dinâmico; feature `pdfium_latest` corresponde à build 8125 | MIT OR Apache-2.0 |
| `pdfium-win-x64.tgz` | tag `chromium/8125`, 3 904 987 bytes, sha256 `9a964261760cff52570aabf038ab346d3a5034dd8b30d0d5fe8861235d356b8b` | Scripts MIT; PDFium BSD-3 e Apache-2.0; terceiros permissivos, com textos na pasta `licenses/` do pacote |

Fontes: https://github.com/ajrcarey/pdfium-render, https://github.com/bblanchon/pdfium-binaries, https://pdfium.googlesource.com/pdfium/+/refs/heads/main/LICENSE

## Avisos de terceiros

Copiar os textos exatos da pasta `licenses/` do pacote para `THIRD-PARTY-NOTICES`, incluindo FreeType (FTL, exige crédito), e registrar tag e sha256. A ausência de copyleft foi concluída pelos nomes das licenças, sem leitura de cada texto.

## Patentes de codec (para revisão jurídica)

Não há licença de patente nas licenças do OpenJPEG; o `LICENSE` dele ressalva expressamente direitos de terceiros. As Partes 1 de JPEG2000 e JBIG2 são normas de cerca de 2000-2001: patentes essenciais daquela época tendem a ter expirado (inferência, sem verificação por número). A DLL inclui também o decodificador HTJ2K (Parte 15), mais recente, com intenção declarada de isenção de royalties, sem garantia encontrada. Decodificar não isenta de patente. Situação não verificada em base oficial: decisão antes da distribuição.

Opções antes da primeira distribuição a terceiros, mesmo gratuita: aceitar e registrar o risco residual; consultar advogado para análise de liberdade de operação, o recomendado se for comercial ou de grande alcance; trocar para `Windows.Data.Pdf`; ou recusar PDFs com esses formatos de imagem, último recurso.

## Alternativas

| Opção | Licença | Situação |
|---|---|---|
| hayro (Rust puro) | MIT e Apache-2.0 | Plano B; sem DLL; robustez incompleta, sem CMYK blending; não testado |
| `Windows.Data.Pdf` | Componente do Windows | Plano B; nada a empacotar; qualidade e limites a 600 dpi não confirmados |
| MuPDF | AGPL ou comercial | Só com AGPL aceita ou licença paga da Artifex |
| SumatraPDF, Poppler, Ghostscript | GPL/AGPL | Descartados por copyleft |

## Windows

- **Desenho:** `StartDocW`, `StartPage`, `StretchDIBits`, `EndPage`, `EndDoc`; `AbortDoc` em falha. `StartDocW` pode bloquear: nunca na thread da interface. Pixels vêm de `GetDeviceCaps` (`HORZRES`, `VERTRES`, `LOGPIXELSX`). Conferir `RC_STRETCHDIB`.
- **`pdfium-render`:** não desenha em HDC e não tem DPI explícito. Calcular os pixels (pontos / 72 × dpi), renderizar com `use_print_quality` e compor sobre branco antes de enviar. A4 a 600 dpi tem cerca de 4960 × 7016 px, ou 139 MB em BGRA: uma página por vez e possível renderização por faixas.
- **`DEVMODE`:** nunca montar do zero. `DocumentPropertiesW` em dois passos e alterar só `dmDuplex`, `dmCollate`, `dmDefaultSource` e `dmPaperSize`, marcando `dmFields`.
- **`GetJobW`, limite importante:** `JOB_STATUS_COMPLETE` só garante a entrega ao destino seguinte, não a saída do papel, e monitores de porta sem *TrueEndOfJob* marcam `PRINTED` logo após o envio. Em USB local, o spooler pode confirmar antes de o papel sair. O botão do verso precisa dizer ao usuário para esperar a última folha, e isso só se decide com impressora real.
- **"Microsoft Print to PDF":** `DOCINFOW.lpszOutput` com caminho absoluto deve evitar o diálogo. Base: blog arquivado da Microsoft, sobre o XPS Writer, e um blog de terceiros. Não confirmado em documentação oficial: testar antes de depender disso.

Referência: https://learn.microsoft.com/en-us/windows/win32/printdocs/getjob e demais páginas de `wingdi` e `printdocs` em learn.microsoft.com.

## Não confirmado

Estrutura real do `.tgz`; `lpszOutput` no Windows 11; renderização por faixas; semântica de `PRINTED` e `COMPLETE` em USB real; se `GetJob` falha quando o trabalho sai da fila.
