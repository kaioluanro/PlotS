//! Aceitação do caminho de um documento e leitura do conteúdo que a interface e a impressão usam.
//!
//! Um PDF é lido no próprio arquivo. Uma imagem vira um PDF derivado, guardado em cache pelo
//! conteúdo: prévia, contagem de páginas e impressão seguem o mesmo caminho, e o nome, o caminho
//! e o histórico continuam os do arquivo original.

use plots_pdf::imagem::{pdf_de_imagem, ErroImagem, OpcoesDeImagem};
use plots_pdf::{ErroPdf, PdfOrigem};
use serde::Serialize;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime};
use tauri::ipc::Response;

/// Tipo do arquivo aberto; decide de onde vem o conteúdo lido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoDocumento {
    Pdf,
    Imagem,
}

/// Documento aceito para abrir em uma aba.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Documento {
    pub caminho: String,
    pub nome: String,
    pub tipo: TipoDocumento,
}

/// Motivo da recusa. A interface traduz o codigo; o nome identifica o arquivo na mensagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodigoErro {
    NaoEncontrado,
    NaoEArquivo,
    NaoSuportado,
    Inacessivel,
    PdfInvalido,
    ImagemInvalida,
    Criptografado,
    Grande,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErroDocumento {
    pub codigo: CodigoErro,
    pub nome: String,
}

/// Maior PDF aceito para leitura (1 GiB). Um arquivo maior e recusado antes de ocupar a memoria.
pub const LIMITE_BYTES_PDF: u64 = 1024 * 1024 * 1024;

/// Maior imagem aceita para virar PDF derivado (1 GiB), igual ao limite do proprio conversor.
pub const LIMITE_BYTES_IMAGEM: u64 = plots_pdf::imagem::LIMITE_BYTES_IMAGEM;

/// Extensões de imagem que o PlotS converte, em qualquer caixa.
const EXTENSOES_DE_IMAGEM: [&str; 8] = ["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp"];

/// Idade a partir da qual um PDF derivado é apagado do cache.
const IDADE_MAXIMA_DO_CACHE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Versão do formato do PDF derivado. Entra no material do hash, então uma mudança na conversão
/// invalida os derivados antigos sem renomear nem apagar o cache.
const VERSAO_DO_DERIVADO: u8 = 1;

/// Aceita o caminho quando ele existe, é um arquivo e tem extensão de PDF ou de imagem aceita.
pub fn abrir(caminho: &str) -> Result<Documento, ErroDocumento> {
    let alvo = Path::new(caminho);
    let nome = nome_de_exibicao(caminho);
    let erro = |codigo| ErroDocumento {
        codigo,
        nome: nome.clone(),
    };

    let metadados = std::fs::metadata(alvo).map_err(|falha| {
        erro(match falha.kind() {
            ErrorKind::NotFound => CodigoErro::NaoEncontrado,
            _ => CodigoErro::Inacessivel,
        })
    })?;
    if !metadados.is_file() {
        return Err(erro(CodigoErro::NaoEArquivo));
    }
    let tipo = tipo_de(alvo).ok_or_else(|| erro(CodigoErro::NaoSuportado))?;

    Ok(Documento {
        caminho: caminho.to_owned(),
        nome,
        tipo,
    })
}

fn tipo_de(alvo: &Path) -> Option<TipoDocumento> {
    let extensao = alvo.extension()?.to_str()?;
    if extensao.eq_ignore_ascii_case("pdf") {
        return Some(TipoDocumento::Pdf);
    }
    EXTENSOES_DE_IMAGEM
        .iter()
        .any(|aceita| extensao.eq_ignore_ascii_case(aceita))
        .then_some(TipoDocumento::Imagem)
}

fn extensao_de(alvo: &Path) -> Option<String> {
    alvo.extension()
        .and_then(|extensao| extensao.to_str())
        .map(str::to_ascii_lowercase)
}

fn extensao_e_imagem(caminho: &str) -> bool {
    Path::new(caminho)
        .extension()
        .and_then(|extensao| extensao.to_str())
        .is_some_and(|extensao| {
            EXTENSOES_DE_IMAGEM
                .iter()
                .any(|aceita| extensao.eq_ignore_ascii_case(aceita))
        })
}

/// Abre o documento e devolve também o caminho do conteúdo lido: o próprio PDF, ou o PDF
/// derivado de uma imagem.
pub fn abrir_para_leitura(caminho: &str) -> Result<(Documento, String), ErroDocumento> {
    abrir_para_leitura_em(caminho, &pasta_do_cache())
}

/// Igual a [`abrir_para_leitura`], com a pasta de cache indicada. A validação e a derivação
/// acontecem com a vez da carga tomada, de modo que duas abas não decodifiquem imagens grandes ao
/// mesmo tempo.
pub fn abrir_para_leitura_em(
    caminho: &str,
    cache: &Path,
) -> Result<(Documento, String), ErroDocumento> {
    let _vez = CARGA_EM_ANDAMENTO
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    abrir_para_leitura_sem_lock(caminho, cache)
}

/// O mesmo caminho de abertura sem tomar a vez da carga: quem chama já a segurou, e tomar de novo
/// travaria a própria thread.
fn abrir_para_leitura_sem_lock(
    caminho: &str,
    cache: &Path,
) -> Result<(Documento, String), ErroDocumento> {
    abrir_para_leitura_com(caminho, cache, pdf_de_imagem)
}

/// Abertura com o conversor indicado. O derivado já gravado no cache é devolvido sem decodificar a
/// imagem de novo; a conversão só roda quando o arquivo falta ou está vazio.
fn abrir_para_leitura_com<C>(
    caminho: &str,
    cache: &Path,
    converter: C,
) -> Result<(Documento, String), ErroDocumento>
where
    C: FnOnce(&[u8], &str, &OpcoesDeImagem) -> Result<Vec<u8>, ErroImagem>,
{
    let documento = abrir(caminho)?;
    if documento.tipo == TipoDocumento::Pdf {
        return Ok((documento, caminho.to_owned()));
    }
    let erro = |codigo| ErroDocumento {
        codigo,
        nome: documento.nome.clone(),
    };
    let extensao = extensao_de(Path::new(caminho)).ok_or_else(|| erro(CodigoErro::NaoSuportado))?;
    let bytes = ler_com_limite(Path::new(caminho), LIMITE_BYTES_IMAGEM).map_err(erro)?;
    let opcoes = OpcoesDeImagem::default();
    let destino = cache.join(nome_do_derivado(&bytes, &extensao, &opcoes));
    if !derivado_pronto(&destino) {
        let pdf =
            converter(&bytes, &extensao, &opcoes).map_err(|falha| erro(codigo_da_imagem(falha)))?;
        std::fs::create_dir_all(cache).map_err(|_| erro(CodigoErro::Inacessivel))?;
        gravar_derivado(&destino, &pdf).map_err(erro)?;
        limpar_cache(cache);
    }
    Ok((documento, destino.to_string_lossy().into_owned()))
}

/// O derivado está no cache e tem conteúdo, então serve sem passar pela conversão.
fn derivado_pronto(destino: &Path) -> bool {
    std::fs::metadata(destino).is_ok_and(|metadados| metadados.is_file() && metadados.len() > 0)
}

fn codigo_da_imagem(falha: ErroImagem) -> CodigoErro {
    match falha {
        ErroImagem::FormatoNaoSuportado => CodigoErro::NaoSuportado,
        ErroImagem::Grande => CodigoErro::Grande,
        _ => CodigoErro::ImagemInvalida,
    }
}

/// Nome do derivado: o conteúdo, a extensão e as opções definem o hash, e o tamanho deixa o
/// nome legível para diagnóstico.
fn nome_do_derivado(bytes: &[u8], extensao: &str, opcoes: &OpcoesDeImagem) -> String {
    let hash = hash_do_conteudo(bytes, extensao, opcoes);
    format!("{hash:016x}-{:x}.pdf", bytes.len())
}

/// FNV-1a de 64 bits sobre a versao do formato, o conteúdo, a extensão e a opção de DPI.
fn hash_do_conteudo(bytes: &[u8], extensao: &str, opcoes: &OpcoesDeImagem) -> u64 {
    hash_com_versao(bytes, extensao, opcoes, VERSAO_DO_DERIVADO)
}

/// Mesmo hash com outra versão do formato, para os derivados de versões antigas deixarem de valer.
fn hash_com_versao(bytes: &[u8], extensao: &str, opcoes: &OpcoesDeImagem, versao: u8) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut misturar = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for byte in bytes {
        misturar(*byte);
    }
    for byte in extensao.bytes() {
        misturar(byte);
    }
    misturar(u8::from(opcoes.usar_dpi));
    misturar(versao);
    hash
}

/// Pasta do cache dos PDFs derivados. `PLOTS_CACHE_IMAGENS` permite apontar outro lugar em
/// diagnóstico e nos testes.
pub fn pasta_do_cache() -> PathBuf {
    if let Some(pasta) = std::env::var_os("PLOTS_CACHE_IMAGENS") {
        return PathBuf::from(pasta);
    }
    if let Some(base) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(base)
            .join("plots")
            .join("cache")
            .join("imagens");
    }
    std::env::temp_dir()
        .join("plots")
        .join("cache")
        .join("imagens")
}

/// Escreve o derivado por um arquivo temporário na mesma pasta e então renomeia. Um derivado vazio
/// deixado no cache é tratado como ausente e substituído.
fn gravar_derivado(destino: &Path, bytes: &[u8]) -> Result<(), CodigoErro> {
    if derivado_pronto(destino) {
        return Ok(());
    }
    let pasta = destino.parent().ok_or(CodigoErro::Inacessivel)?;
    std::fs::create_dir_all(pasta).map_err(|_| CodigoErro::Inacessivel)?;
    if destino.exists() {
        // Renomear por cima de um arquivo existente falha no Windows; só o derivado vazio cai aqui.
        std::fs::remove_file(destino).map_err(|_| CodigoErro::Inacessivel)?;
    }
    let numero = CONTADOR_DERIVADO.fetch_add(1, Ordering::Relaxed);
    let temporario = pasta.join(format!(".plots-{}-{numero}.tmp", std::process::id()));
    let resultado = (|| -> std::io::Result<()> {
        std::fs::write(&temporario, bytes)?;
        match std::fs::rename(&temporario, destino) {
            Ok(()) => Ok(()),
            Err(_) if derivado_pronto(destino) => Ok(()),
            Err(falha) => Err(falha),
        }
    })()
    .map_err(|_| CodigoErro::Inacessivel);
    let _ = std::fs::remove_file(&temporario);
    resultado
}

static CONTADOR_DERIVADO: AtomicU32 = AtomicU32::new(0);

/// Apaga, sem falhar a abertura, os derivados e temporários com mais de 30 dias.
fn limpar_cache(pasta: &Path) {
    let Ok(entradas) = std::fs::read_dir(pasta) else {
        return;
    };
    let agora = SystemTime::now();
    for entrada in entradas.flatten() {
        let Ok(metadados) = entrada.metadata() else {
            continue;
        };
        if !metadados.is_file() {
            continue;
        }
        let nome = entrada.file_name();
        let nome = nome.to_string_lossy();
        if !nome.ends_with(".pdf") && !nome.ends_with(".tmp") {
            continue;
        }
        let Ok(modificado) = metadados.modified() else {
            continue;
        };
        let Ok(idade) = agora.duration_since(modificado) else {
            continue;
        };
        if idade > IDADE_MAXIMA_DO_CACHE {
            let _ = std::fs::remove_file(entrada.path());
        }
    }
}

/// Total de paginas, depois de o conteúdo passar pela mesma barreira que a impressão usa.
pub fn contar_paginas(caminho: &str) -> Result<usize, ErroDocumento> {
    contar_com_limite(caminho, LIMITE_BYTES_PDF)
}

fn contar_com_limite(caminho: &str, limite: u64) -> Result<usize, ErroDocumento> {
    contar_com_limite_em(caminho, limite, &pasta_do_cache())
}

fn contar_com_limite_em(caminho: &str, limite: u64, cache: &Path) -> Result<usize, ErroDocumento> {
    carregar_validado_em(caminho, limite, cache).map(|(_, total)| total)
}

/// Bytes do conteúdo para a interface mostrar, sempre depois da mesma barreira da impressao. Um
/// PDF recusado aqui nunca chega ao leitor da interface, que nao recebe senha nem abre PDF
/// protegido.
///
/// Devolve os bytes crus, sem passar por JSON. A leitura roda em uma thread propria, e o que a
/// interromper vira recusa.
#[tauri::command]
pub async fn ler_documento(caminho: String) -> Result<Response, ErroDocumento> {
    let falha = falha_inesperada(&caminho);
    tauri::async_runtime::spawn_blocking(move || ler_validado(&caminho))
        .await
        .unwrap_or(Err(falha))
        .map(Response::new)
}

fn ler_validado(caminho: &str) -> Result<Vec<u8>, ErroDocumento> {
    ler_com_limite_validado(caminho, LIMITE_BYTES_PDF)
}

fn ler_com_limite_validado(caminho: &str, limite: u64) -> Result<Vec<u8>, ErroDocumento> {
    ler_com_limite_em(caminho, limite, &pasta_do_cache())
}

fn ler_com_limite_em(caminho: &str, limite: u64, cache: &Path) -> Result<Vec<u8>, ErroDocumento> {
    carregar_validado_em(caminho, limite, cache).map(|(bytes, _)| bytes)
}

/// Um documento por vez: a contagem, a leitura de varias abas e a conversao de uma imagem nao
/// disputam a memoria ao mesmo tempo.
static CARGA_EM_ANDAMENTO: Mutex<()> = Mutex::new(());

fn carregar_validado_em(
    caminho: &str,
    limite: u64,
    cache: &Path,
) -> Result<(Vec<u8>, usize), ErroDocumento> {
    let _vez = CARGA_EM_ANDAMENTO
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let (documento, conteudo) = abrir_para_leitura_sem_lock(caminho, cache)?;
    let erro = |codigo| ErroDocumento {
        codigo,
        nome: documento.nome.clone(),
    };
    let bytes = ler_com_limite(Path::new(&conteudo), limite).map_err(erro)?;
    let total = PdfOrigem::carregar(&bytes)
        .map_err(|falha| {
            erro(match falha {
                ErroPdf::Criptografado => CodigoErro::Criptografado,
                _ => CodigoErro::PdfInvalido,
            })
        })?
        .total_paginas();
    Ok((bytes, total))
}

/// Le o arquivo sem passar do limite, mesmo que ele cresca depois de medido. Faltar memoria para o
/// conteudo vira recusa em vez de encerrar o programa.
fn ler_com_limite(alvo: &Path, limite: u64) -> Result<Vec<u8>, CodigoErro> {
    let de_leitura = |falha: std::io::Error| match falha.kind() {
        ErrorKind::NotFound => CodigoErro::NaoEncontrado,
        _ => CodigoErro::Inacessivel,
    };
    let arquivo = File::open(alvo).map_err(de_leitura)?;
    let tamanho = arquivo.metadata().map_err(de_leitura)?.len();
    if tamanho > limite {
        return Err(CodigoErro::Grande);
    }
    let capacidade = usize::try_from(tamanho).map_err(|_| CodigoErro::Grande)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacidade)
        .map_err(|_| CodigoErro::Grande)?;
    arquivo
        .take(limite.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(de_leitura)?;
    let lidos = u64::try_from(bytes.len()).map_err(|_| CodigoErro::Grande)?;
    if lidos > limite {
        return Err(CodigoErro::Grande);
    }
    Ok(bytes)
}

/// Recusa para quando a propria contagem nao chega ao fim. O código combina com o tipo provável
/// do arquivo, para a mensagem não falar de PDF danificado quando era uma imagem.
pub fn falha_inesperada(caminho: &str) -> ErroDocumento {
    ErroDocumento {
        codigo: if extensao_e_imagem(caminho) {
            CodigoErro::ImagemInvalida
        } else {
            CodigoErro::PdfInvalido
        },
        nome: nome_de_exibicao(caminho),
    }
}

/// Ultimo componente do caminho; sem ele (raiz de disco, texto vazio) devolve o caminho inteiro.
fn nome_de_exibicao(caminho: &str) -> String {
    Path::new(caminho).file_name().map_or_else(
        || caminho.to_owned(),
        |nome| nome.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod testes {
    use super::*;
    use image::{DynamicImage, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
    use std::fs;
    use std::path::PathBuf;

    static CONTADOR: AtomicU32 = AtomicU32::new(0);

    /// Pasta temporaria exclusiva do teste, removida ao sair de escopo.
    struct PastaTemporaria(PathBuf);

    impl PastaTemporaria {
        fn nova() -> Self {
            let numero = CONTADOR.fetch_add(1, Ordering::Relaxed);
            let pasta = std::env::temp_dir().join(format!(
                "plots-app-documento-{}-{numero}",
                std::process::id()
            ));
            fs::create_dir_all(&pasta).expect("criar pasta temporaria");
            Self(pasta)
        }

        fn arquivo(&self, nome: &str) -> String {
            let caminho = self.0.join(nome);
            fs::write(&caminho, b"%PDF-1.7\n").expect("criar arquivo");
            caminho.to_string_lossy().into_owned()
        }

        fn cache(&self) -> PathBuf {
            let pasta = self.0.join("cache");
            fs::create_dir_all(&pasta).expect("criar cache");
            pasta
        }

        fn gravar(&self, nome: &str, bytes: &[u8]) -> String {
            let caminho = self.0.join(nome);
            fs::write(&caminho, bytes).expect("gravar arquivo");
            caminho.to_string_lossy().into_owned()
        }
    }

    impl Drop for PastaTemporaria {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn imagem_rgb(largura: u32, altura: u32) -> DynamicImage {
        let imagem = RgbImage::from_fn(largura, altura, |x, _| Rgb([(x % 256) as u8, 128, 40]));
        DynamicImage::ImageRgb8(imagem)
    }

    fn bytes_de(imagem: &DynamicImage, formato: ImageFormat) -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        imagem
            .write_to(&mut bytes, formato)
            .expect("codificar imagem");
        bytes.into_inner()
    }

    fn arquivos_do_cache(cache: &Path) -> Vec<String> {
        fs::read_dir(cache)
            .unwrap()
            .flatten()
            .map(|entrada| entrada.file_name().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn aceita_png_como_documento_de_imagem() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.gravar("faixas.png", &bytes_de(&imagem_rgb(8, 6), ImageFormat::Png));
        let documento = abrir(&caminho).expect("PNG deveria ser aceito como imagem");
        assert_eq!(documento.tipo, TipoDocumento::Imagem);
        assert_eq!(documento.nome, "faixas.png");
    }

    #[test]
    fn aceita_arquivo_pdf_e_devolve_o_nome_de_exibicao() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.arquivo("relatorio.pdf");
        assert_eq!(
            abrir(&caminho),
            Ok(Documento {
                caminho: caminho.clone(),
                nome: "relatorio.pdf".to_owned(),
                tipo: TipoDocumento::Pdf,
            })
        );
    }

    #[test]
    fn aceita_extensao_em_maiusculas() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.arquivo("SCAN.PDF");
        assert_eq!(
            abrir(&caminho).map(|doc| doc.nome),
            Ok("SCAN.PDF".to_owned())
        );
    }

    #[test]
    fn aceita_caminho_com_acento_e_espaco() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.arquivo("Relatório de produção – março.pdf");
        assert_eq!(
            abrir(&caminho).map(|doc| doc.nome),
            Ok("Relatório de produção – março.pdf".to_owned())
        );
    }

    #[test]
    fn recusa_arquivo_inexistente() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.0.join("sumiu.pdf");
        assert_eq!(
            abrir(&caminho.to_string_lossy()),
            Err(ErroDocumento {
                codigo: CodigoErro::NaoEncontrado,
                nome: "sumiu.pdf".to_owned(),
            })
        );
    }

    #[test]
    fn recusa_pasta_mesmo_com_nome_terminado_em_pdf() {
        let pasta = PastaTemporaria::nova();
        let subpasta = pasta.0.join("capitulos.pdf");
        fs::create_dir_all(&subpasta).expect("criar subpasta");
        assert_eq!(
            abrir(&subpasta.to_string_lossy()),
            Err(ErroDocumento {
                codigo: CodigoErro::NaoEArquivo,
                nome: "capitulos.pdf".to_owned(),
            })
        );
    }

    #[test]
    fn recusa_outra_extensao_e_arquivo_sem_extensao() {
        let pasta = PastaTemporaria::nova();
        for nome in ["notas.txt", "pdf", "falso.pdf.exe"] {
            let caminho = pasta.arquivo(nome);
            assert_eq!(
                abrir(&caminho),
                Err(ErroDocumento {
                    codigo: CodigoErro::NaoSuportado,
                    nome: nome.to_owned(),
                })
            );
        }
    }

    #[test]
    fn caminho_vazio_ou_invalido_vira_erro_sem_derrubar_o_app() {
        assert_eq!(
            abrir("").map_err(|erro| erro.codigo),
            Err(CodigoErro::NaoEncontrado)
        );
        assert_eq!(
            abrir("a\0b.pdf").map_err(|erro| erro.codigo),
            Err(CodigoErro::Inacessivel)
        );
    }

    /// PDF escrito byte a byte, com a tabela de referencias certa. `extra` entra como objetos
    /// adicionais antes da tabela e `trailer` dentro do dicionario final.
    fn pdf(paginas: usize, extra: &[&str], trailer: &str) -> Vec<u8> {
        let filhos: Vec<String> = (0..paginas).map(|n| format!("{} 0 R", n + 3)).collect();
        let mut objetos = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            format!(
                "<< /Type /Pages /Count {paginas} /Kids [{}] >>",
                filhos.join(" ")
            ),
        ];
        for _ in 0..paginas {
            objetos.push("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>".to_owned());
        }
        objetos.extend(extra.iter().map(|objeto| (*objeto).to_owned()));

        let mut bytes = b"%PDF-1.5\n".to_vec();
        let mut posicoes = Vec::new();
        for (indice, objeto) in objetos.iter().enumerate() {
            posicoes.push(bytes.len());
            bytes.extend_from_slice(format!("{} 0 obj\n{objeto}\nendobj\n", indice + 1).as_bytes());
        }
        let tabela = bytes.len();
        let total = objetos.len() + 1;
        bytes.extend_from_slice(format!("xref\n0 {total}\n0000000000 65535 f \n").as_bytes());
        for posicao in posicoes {
            bytes.extend_from_slice(format!("{posicao:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!(
                "trailer\n<< /Size {total} /Root 1 0 R {trailer} >>\nstartxref\n{tabela}\n%%EOF\n"
            )
            .as_bytes(),
        );
        bytes
    }

    fn gravar(pasta: &PastaTemporaria, nome: &str, bytes: &[u8]) -> String {
        pasta.gravar(nome, bytes)
    }

    fn codigo_ao_contar(caminho: &str) -> Option<CodigoErro> {
        contar_paginas(caminho).err().map(|erro| erro.codigo)
    }

    #[test]
    fn conta_as_paginas_de_um_pdf_valido() {
        let pasta = PastaTemporaria::nova();
        for total in [1, 2, 24] {
            let caminho = gravar(&pasta, &format!("{total}.pdf"), &pdf(total, &[], ""));
            assert_eq!(contar_paginas(&caminho), Ok(total));
        }
    }

    #[test]
    fn recusa_pdf_invalido_e_informa_o_nome() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.arquivo("quebrado.pdf");
        assert_eq!(
            contar_paginas(&caminho),
            Err(ErroDocumento {
                codigo: CodigoErro::PdfInvalido,
                nome: "quebrado.pdf".to_owned(),
            })
        );
        let lixo = gravar(&pasta, "lixo.pdf", &[0xFF; 4096]);
        assert_eq!(codigo_ao_contar(&lixo), Some(CodigoErro::PdfInvalido));
        let vazio = gravar(&pasta, "vazio.pdf", b"");
        assert_eq!(codigo_ao_contar(&vazio), Some(CodigoErro::PdfInvalido));
    }

    #[test]
    fn recusa_pdf_sem_paginas() {
        let pasta = PastaTemporaria::nova();
        let caminho = gravar(&pasta, "sem-paginas.pdf", &pdf(0, &[], ""));
        assert_eq!(codigo_ao_contar(&caminho), Some(CodigoErro::PdfInvalido));
    }

    #[test]
    fn recusa_pdf_criptografado() {
        let pasta = PastaTemporaria::nova();
        let hexa = "A".repeat(64);
        let cifra = format!("<< /Filter /Standard /V 1 /R 2 /O <{hexa}> /U <{hexa}> /P -4 >>");
        let caminho = gravar(
            &pasta,
            "cifrado.pdf",
            &pdf(2, &[&cifra], "/Encrypt 5 0 R /ID [<00> <00>]"),
        );
        assert_eq!(codigo_ao_contar(&caminho), Some(CodigoErro::Criptografado));
    }

    #[test]
    fn os_erros_do_caminho_valem_tambem_ao_contar() {
        let pasta = PastaTemporaria::nova();
        let sumiu = pasta.0.join("sumiu.pdf");
        assert_eq!(
            codigo_ao_contar(&sumiu.to_string_lossy()),
            Some(CodigoErro::NaoEncontrado)
        );
        let texto = pasta.arquivo("notas.txt");
        assert_eq!(codigo_ao_contar(&texto), Some(CodigoErro::NaoSuportado));
        assert_eq!(codigo_ao_contar(""), Some(CodigoErro::NaoEncontrado));
    }

    #[test]
    fn arquivo_acima_do_limite_nao_chega_a_ser_interpretado() {
        let pasta = PastaTemporaria::nova();
        let bytes = pdf(1, &[], "");
        let caminho = gravar(&pasta, "grande.pdf", &bytes);
        let limite = u64::try_from(bytes.len()).expect("tamanho do teste");
        assert_eq!(contar_com_limite(&caminho, limite), Ok(1));
        assert_eq!(
            contar_com_limite(&caminho, limite - 1).map_err(|erro| erro.codigo),
            Err(CodigoErro::Grande)
        );
    }

    #[test]
    fn leitura_devolve_exatamente_os_bytes_do_pdf_aceito() {
        let pasta = PastaTemporaria::nova();
        let bytes = pdf(3, &[], "");
        let caminho = gravar(&pasta, "tres.pdf", &bytes);
        assert_eq!(
            ler_com_limite_validado(&caminho, LIMITE_BYTES_PDF),
            Ok(bytes)
        );
    }

    #[test]
    fn leitura_recusa_arquivo_inexistente_com_o_nome() {
        let pasta = PastaTemporaria::nova();
        let sumiu = pasta.0.join("sumiu.pdf");
        assert_eq!(
            ler_validado(&sumiu.to_string_lossy()),
            Err(ErroDocumento {
                codigo: CodigoErro::NaoEncontrado,
                nome: "sumiu.pdf".to_owned(),
            })
        );
    }

    #[test]
    fn leitura_recusa_pdf_protegido_sem_entregar_bytes() {
        let pasta = PastaTemporaria::nova();
        let hexa = "A".repeat(64);
        let cifra = format!("<< /Filter /Standard /V 1 /R 2 /O <{hexa}> /U <{hexa}> /P -4 >>");
        let caminho = gravar(
            &pasta,
            "cifrado.pdf",
            &pdf(2, &[&cifra], "/Encrypt 5 0 R /ID [<00> <00>]"),
        );
        assert_eq!(
            ler_validado(&caminho).map_err(|erro| erro.codigo),
            Err(CodigoErro::Criptografado)
        );
    }

    #[test]
    fn leitura_recusa_o_que_a_barreira_do_pdf_nao_aceita() {
        let pasta = PastaTemporaria::nova();
        let quebrado = pasta.arquivo("quebrado.pdf");
        assert_eq!(
            ler_validado(&quebrado).map_err(|erro| erro.codigo),
            Err(CodigoErro::PdfInvalido)
        );
        let texto = pasta.arquivo("notas.txt");
        assert_eq!(
            ler_validado(&texto).map_err(|erro| erro.codigo),
            Err(CodigoErro::NaoSuportado)
        );
    }

    #[test]
    fn leitura_acima_do_limite_vira_recusa_antes_de_interpretar() {
        let pasta = PastaTemporaria::nova();
        let bytes = pdf(1, &[], "");
        let caminho = gravar(&pasta, "grande.pdf", &bytes);
        let limite = u64::try_from(bytes.len()).expect("tamanho do teste");
        assert_eq!(ler_com_limite_validado(&caminho, limite), Ok(bytes));
        assert_eq!(
            ler_com_limite_validado(&caminho, limite - 1).map_err(|erro| erro.codigo),
            Err(CodigoErro::Grande)
        );
    }

    #[test]
    fn aninhamento_profundo_nao_derruba_o_app() {
        let pasta = PastaTemporaria::nova();
        let fundo = format!("{}{}", "[".repeat(200_000), "]".repeat(200_000));
        let caminho = gravar(&pasta, "fundo.pdf", &pdf(1, &[&fundo], ""));
        // Basta nao derrubar o processo: o objeto nao e referenciado por pagina alguma.
        let _ = contar_paginas(&caminho);
    }

    #[test]
    fn falha_inesperada_identifica_o_arquivo() {
        assert_eq!(
            falha_inesperada("pasta/a.pdf"),
            ErroDocumento {
                codigo: CodigoErro::PdfInvalido,
                nome: "a.pdf".to_owned(),
            }
        );
        assert_eq!(
            falha_inesperada("pasta/a.png"),
            ErroDocumento {
                codigo: CodigoErro::ImagemInvalida,
                nome: "a.png".to_owned(),
            }
        );
    }

    #[test]
    fn cada_formato_de_imagem_gera_o_derivado() {
        let pasta = PastaTemporaria::nova();
        let cache = pasta.cache();
        let imagem = imagem_rgb(12, 7);
        for (formato, extensao) in [
            (ImageFormat::Png, "png"),
            (ImageFormat::Jpeg, "jpg"),
            (ImageFormat::Bmp, "bmp"),
            (ImageFormat::Gif, "gif"),
            (ImageFormat::WebP, "webp"),
        ] {
            let caminho = pasta.gravar(&format!("quadro.{extensao}"), &bytes_de(&imagem, formato));
            let (documento, derivado) = abrir_para_leitura_em(&caminho, &cache)
                .unwrap_or_else(|erro| panic!("{extensao}: {erro:?}"));
            assert_eq!(documento.tipo, TipoDocumento::Imagem, "{extensao}");
            assert_eq!(documento.caminho, caminho, "{extensao}");
            assert!(Path::new(&derivado).is_file(), "{extensao}");
            assert_eq!(
                contar_com_limite_em(&caminho, LIMITE_BYTES_PDF, &cache),
                Ok(1)
            );
        }
    }

    #[test]
    fn tiff_multipagina_conta_todos_os_quadros() {
        let pasta = PastaTemporaria::nova();
        let cache = pasta.cache();
        let mut bytes = std::io::Cursor::new(Vec::new());
        {
            let mut codificador = tiff::encoder::TiffEncoder::new(&mut bytes).unwrap();
            for cor in [[255, 128, 0], [0, 64, 255], [10, 200, 90]] {
                let quadro = RgbImage::from_pixel(5, 4, Rgb(cor));
                codificador
                    .write_image::<tiff::encoder::colortype::RGB8>(5, 4, quadro.as_raw())
                    .unwrap();
            }
        }
        let caminho = pasta.gravar("tres-paginas.tif", &bytes.into_inner());
        assert_eq!(
            contar_com_limite_em(&caminho, LIMITE_BYTES_PDF, &cache),
            Ok(3)
        );
    }

    #[test]
    fn derivado_e_reaproveitado_no_cache() {
        let pasta = PastaTemporaria::nova();
        let cache = pasta.cache();
        let caminho = pasta.gravar(
            "faixas.png",
            &bytes_de(&imagem_rgb(40, 30), ImageFormat::Png),
        );
        let (_, primeiro) = abrir_para_leitura_em(&caminho, &cache).unwrap();
        // Sentinela no PDF derivado: se a segunda abertura convertesse de novo e regravasse o
        // arquivo, ela sumiria.
        let mut com_sentinela = fs::read(&primeiro).unwrap();
        com_sentinela.extend_from_slice(b"\n% sentinela do teste\n");
        fs::write(&primeiro, &com_sentinela).unwrap();
        let modificado = fs::metadata(&primeiro).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(30));
        let (_, segundo) = abrir_para_leitura_com(&caminho, &cache, |_, _, _| {
            panic!("o derivado já estava no cache: a imagem não deveria ser convertida de novo")
        })
        .unwrap();
        assert_eq!(primeiro, segundo);
        assert_eq!(fs::read(&segundo).unwrap(), com_sentinela);
        assert_eq!(
            fs::metadata(&segundo).unwrap().modified().unwrap(),
            modificado
        );
        assert_eq!(arquivos_do_cache(&cache).len(), 1);
    }

    #[test]
    fn derivado_vazio_no_cache_e_convertido_de_novo() {
        let pasta = PastaTemporaria::nova();
        let cache = pasta.cache();
        let caminho = pasta.gravar(
            "faixas.png",
            &bytes_de(&imagem_rgb(16, 9), ImageFormat::Png),
        );
        let (_, derivado) = abrir_para_leitura_em(&caminho, &cache).unwrap();
        fs::write(&derivado, b"").unwrap();
        let (_, de_novo) = abrir_para_leitura_em(&caminho, &cache).unwrap();
        assert_eq!(derivado, de_novo);
        assert!(fs::metadata(&de_novo).unwrap().len() > 0);
        assert_eq!(
            contar_com_limite_em(&caminho, LIMITE_BYTES_PDF, &cache),
            Ok(1)
        );
    }

    #[test]
    fn abrir_imagem_espera_a_vez_de_quem_esta_carregando() {
        let pasta = PastaTemporaria::nova();
        let cache = pasta.cache();
        let caminho = pasta.gravar("faixas.png", &bytes_de(&imagem_rgb(8, 6), ImageFormat::Png));
        let vez = CARGA_EM_ANDAMENTO
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let (envio, recepcao) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            let abertura = abrir_para_leitura_em(&caminho, &cache);
            let _ = envio.send(abertura);
        });
        assert!(
            recepcao.recv_timeout(Duration::from_millis(300)).is_err(),
            "a abertura converteu a imagem sem esperar a vez da carga"
        );
        drop(vez);
        let (_, derivado) = recepcao
            .recv_timeout(Duration::from_secs(30))
            .expect("a abertura deveria seguir depois da vez")
            .expect("a imagem deveria virar derivado");
        assert!(Path::new(&derivado).is_file());
        thread.join().unwrap();
    }

    #[test]
    fn derivado_de_outra_versao_do_formato_nao_e_aproveitado() {
        let pasta = PastaTemporaria::nova();
        let cache = pasta.cache();
        let bytes = bytes_de(&imagem_rgb(12, 6), ImageFormat::Png);
        let caminho = pasta.gravar("faixas.png", &bytes);
        let opcoes = OpcoesDeImagem::default();
        let versao_anterior = VERSAO_DO_DERIVADO.wrapping_sub(1);
        let antigo = cache.join(format!(
            "{:016x}-{:x}.pdf",
            hash_com_versao(&bytes, "png", &opcoes, versao_anterior),
            bytes.len()
        ));
        fs::write(&antigo, b"derivado da versao anterior").unwrap();
        let (_, derivado) = abrir_para_leitura_em(&caminho, &cache).unwrap();
        assert_ne!(Path::new(&derivado), antigo);
        assert_eq!(
            contar_com_limite_em(&caminho, LIMITE_BYTES_PDF, &cache),
            Ok(1)
        );
    }

    #[test]
    fn bytes_da_leitura_sao_os_do_derivado() {
        let pasta = PastaTemporaria::nova();
        let cache = pasta.cache();
        let caminho = pasta.gravar(
            "faixas.png",
            &bytes_de(&imagem_rgb(20, 10), ImageFormat::Png),
        );
        let (_, derivado) = abrir_para_leitura_em(&caminho, &cache).unwrap();
        let do_cache = fs::read(&derivado).unwrap();
        assert_eq!(
            ler_com_limite_em(&caminho, LIMITE_BYTES_PDF, &cache),
            Ok(do_cache)
        );
    }

    #[test]
    fn imagem_quebrada_vira_recusa_sem_derivar() {
        let pasta = PastaTemporaria::nova();
        let cache = pasta.cache();
        let caminho = pasta.gravar("quebrada.png", &[0xFF; 128]);
        assert_eq!(
            abrir_para_leitura_em(&caminho, &cache).map_err(|erro| erro.codigo),
            Err(CodigoErro::ImagemInvalida)
        );
        assert!(arquivos_do_cache(&cache).is_empty());
    }

    #[test]
    fn pdf_continua_lido_no_arquivo_original() {
        let pasta = PastaTemporaria::nova();
        let cache = pasta.cache();
        let caminho = gravar(&pasta, "duas.pdf", &pdf(2, &[], ""));
        let (documento, conteudo) = abrir_para_leitura_em(&caminho, &cache).unwrap();
        assert_eq!(documento.tipo, TipoDocumento::Pdf);
        assert_eq!(conteudo, caminho);
        assert!(arquivos_do_cache(&cache).is_empty());
    }

    #[test]
    fn cache_antigo_e_limpo_sem_impedir_a_abertura() {
        let pasta = PastaTemporaria::nova();
        let cache = pasta.cache();
        let antigo = cache.join("0000000000000000-1.pdf");
        fs::write(&antigo, b"velho").unwrap();
        let tempo = SystemTime::now() - Duration::from_secs(31 * 24 * 60 * 60);
        fs::File::options()
            .write(true)
            .open(&antigo)
            .unwrap()
            .set_modified(tempo)
            .unwrap();
        let caminho = pasta.gravar("faixas.png", &bytes_de(&imagem_rgb(4, 4), ImageFormat::Png));
        let (_, derivado) = abrir_para_leitura_em(&caminho, &cache).unwrap();
        assert!(Path::new(&derivado).is_file());
        assert!(!antigo.exists());
    }

    #[test]
    fn png_com_alfa_gera_derivado_nao_vazio() {
        let pasta = PastaTemporaria::nova();
        let cache = pasta.cache();
        let imagem = RgbaImage::from_pixel(2, 1, Rgba([255, 0, 0, 128]));
        let caminho = pasta.gravar(
            "transparente.png",
            &bytes_de(&DynamicImage::ImageRgba8(imagem), ImageFormat::Png),
        );
        let (_, derivado) = abrir_para_leitura_em(&caminho, &cache).unwrap();
        let bytes = fs::read(derivado).unwrap();
        let origem = PdfOrigem::carregar(&bytes).unwrap();
        assert_eq!(origem.total_paginas(), 1);
    }
}
