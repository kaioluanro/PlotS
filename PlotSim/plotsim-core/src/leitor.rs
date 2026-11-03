//! Leitura de um PDF recebido. O arquivo veio de fora: passa primeiro pela mesma barreira do PlotS
//! e só depois vai para o PDFium, sempre com limite de tamanho.

use plots_pdf::{ErroPdf, PdfOrigem};
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Maior PDF aceito, em bytes.
pub const LIMITE_BYTES_PDF: u64 = 512 * 1024 * 1024;
/// Maior texto aceito de uma página, em caracteres. Acima disso a página não é uma página de teste.
pub const LIMITE_TEXTO_PAGINA: usize = 1024 * 1024;

const MM_POR_PONTO: f64 = 25.4 / 72.0;

/// Por que um PDF recebido não pôde ser lido.
#[derive(Debug)]
pub enum ErroLeitura {
    Arquivo(PathBuf, io::Error),
    Grande {
        bytes: u64,
    },
    Pdf(String),
    Criptografado,
    /// A biblioteca de PDF não está disponível.
    Biblioteca(String),
    /// A página existe, mas não tem nenhuma letra ou número que dê para ler.
    PaginaSemTexto {
        pagina: usize,
    },
    TextoGrande {
        pagina: usize,
    },
    /// O PDFium e a barreira discordam sobre o documento.
    Divergencia(String),
}

impl fmt::Display for ErroLeitura {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arquivo(caminho, erro) => {
                write!(f, "não deu para ler {}: {erro}", caminho.display())
            }
            Self::Grande { bytes } => {
                write!(f, "o PDF tem {bytes} bytes, acima de {LIMITE_BYTES_PDF}")
            }
            Self::Pdf(detalhe) => write!(f, "o PDF recebido é inválido: {detalhe}"),
            Self::Criptografado => f.write_str("o PDF recebido está criptografado"),
            Self::Biblioteca(detalhe) => write!(f, "biblioteca de PDF indisponível: {detalhe}"),
            Self::PaginaSemTexto { pagina } => write!(
                f,
                "a página {pagina} do PDF recebido não tem texto legível; sem ele não dá para conferir qual página é"
            ),
            Self::TextoGrande { pagina } => write!(
                f,
                "a página {pagina} do PDF recebido passa de {LIMITE_TEXTO_PAGINA} caracteres de texto"
            ),
            Self::Divergencia(detalhe) => {
                write!(f, "o PDF recebido abriu de forma inconsistente: {detalhe}")
            }
        }
    }
}

impl std::error::Error for ErroLeitura {}

/// Uma página como ela chegou.
#[derive(Clone, Debug, PartialEq)]
pub struct PaginaLida {
    /// Número da página no arquivo, começando em 1.
    pub numero: usize,
    /// Tamanho em milímetros como a página é vista, já com a rotação aplicada.
    pub largura_mm: f64,
    pub altura_mm: f64,
    /// Rotação da própria página em graus: 0, 90, 180 ou 270.
    pub rotacao: u16,
    /// Texto da página; vazio quando ela não tem texto.
    pub texto: String,
}

/// O texto dá para ler? Precisa ter ao menos uma letra ou um número: espaços e símbolos de
/// substituição não dizem qual página é.
pub fn texto_legivel(texto: &str) -> bool {
    texto.chars().any(char::is_alphanumeric)
}

/// Exige texto legível em todas as páginas. O erro diz qual foi a primeira sem texto.
pub fn exigir_texto(paginas: &[PaginaLida]) -> Result<(), ErroLeitura> {
    match paginas.iter().find(|pagina| !texto_legivel(&pagina.texto)) {
        Some(pagina) => Err(ErroLeitura::PaginaSemTexto {
            pagina: pagina.numero,
        }),
        None => Ok(()),
    }
}

/// Lê o arquivo inteiro, sem passar do limite mesmo que ele cresça durante a leitura.
pub fn ler_arquivo(caminho: &Path, limite: u64) -> Result<Vec<u8>, ErroLeitura> {
    let falha = |erro| ErroLeitura::Arquivo(caminho.to_path_buf(), erro);
    let arquivo = File::open(caminho).map_err(falha)?;
    let tamanho = arquivo.metadata().map_err(falha)?.len();
    if tamanho > limite {
        return Err(ErroLeitura::Grande { bytes: tamanho });
    }
    let capacidade =
        usize::try_from(tamanho).map_err(|_| ErroLeitura::Grande { bytes: tamanho })?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacidade)
        .map_err(|_| ErroLeitura::Grande { bytes: tamanho })?;
    arquivo
        .take(limite.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(falha)?;
    let lidos = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if lidos > limite {
        return Err(ErroLeitura::Grande { bytes: lidos });
    }
    Ok(bytes)
}

/// A barreira do PlotS: documento protegido ou malformado para aqui. Devolve quantas páginas há.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn barreira(bytes: &[u8]) -> Result<usize, ErroLeitura> {
    let origem = PdfOrigem::carregar(bytes).map_err(|erro| match erro {
        ErroPdf::Criptografado => ErroLeitura::Criptografado,
        outro => ErroLeitura::Pdf(outro.to_string()),
    })?;
    Ok(origem.total_paginas())
}

#[cfg(windows)]
mod comum {
    use super::*;
    use pdfium_render::prelude::{PdfPageRenderRotation, Pdfium};
    use std::sync::OnceLock;

    /// A biblioteca só pode ser iniciada uma vez por processo.
    static BIBLIOTECA: OnceLock<Pdfium> = OnceLock::new();

    /// Carrega a biblioteca pelos mesmos caminhos do PlotS, nunca pela busca do sistema.
    fn biblioteca() -> Result<&'static Pdfium, ErroLeitura> {
        if let Some(pronta) = BIBLIOTECA.get() {
            return Ok(pronta);
        }
        let falha = |erro: plots_windows::ErroEnvio| ErroLeitura::Biblioteca(erro.to_string());
        let indicada = std::env::var_os(plots_windows::VARIAVEL_PASTA_BIBLIOTECA);
        let executavel = std::env::current_exe().ok();
        let pasta = executavel.as_deref().and_then(Path::parent);
        let caminhos =
            plots_windows::caminhos_da_biblioteca(indicada.as_deref(), pasta).map_err(falha)?;
        let caminho = plots_windows::escolher_biblioteca(caminhos, Path::is_file).map_err(falha)?;
        let ligacoes = Pdfium::bind_to_library(&caminho)
            .map_err(|erro| ErroLeitura::Biblioteca(erro.to_string()))?;
        Ok(BIBLIOTECA.get_or_init(|| Pdfium::new(ligacoes)))
    }

    fn graus(rotacao: PdfPageRenderRotation) -> u16 {
        match rotacao {
            PdfPageRenderRotation::None => 0,
            PdfPageRenderRotation::Degrees90 => 90,
            PdfPageRenderRotation::Degrees180 => 180,
            PdfPageRenderRotation::Degrees270 => 270,
        }
    }

    /// Lê páginas, tamanho, rotação e texto, sem exigir que haja texto.
    pub fn inspecionar_bytes(bytes: &[u8]) -> Result<Vec<PaginaLida>, ErroLeitura> {
        let total = barreira(bytes)?;
        let invalido =
            |erro: pdfium_render::prelude::PdfiumError| ErroLeitura::Pdf(erro.to_string());
        let documento = biblioteca()?
            .load_pdf_from_byte_slice(bytes, None)
            .map_err(invalido)?;
        let paginas = documento.pages();
        if usize::try_from(paginas.len()).ok() != Some(total) {
            return Err(ErroLeitura::Divergencia(format!(
                "{total} páginas na estrutura, {} no desenho",
                paginas.len()
            )));
        }
        let mut lidas = Vec::with_capacity(total);
        for (indice, pagina) in paginas.iter().enumerate() {
            let numero = indice.saturating_add(1);
            let texto = pagina.text().map_err(invalido)?;
            if usize::try_from(texto.len()).map_or(true, |tamanho| tamanho > LIMITE_TEXTO_PAGINA) {
                return Err(ErroLeitura::TextoGrande { pagina: numero });
            }
            lidas.push(PaginaLida {
                numero,
                largura_mm: f64::from(pagina.width().value) * MM_POR_PONTO,
                altura_mm: f64::from(pagina.height().value) * MM_POR_PONTO,
                rotacao: graus(pagina.rotation().map_err(invalido)?),
                texto: texto.all(),
            });
        }
        Ok(lidas)
    }

    /// Lê as páginas e exige texto legível em cada uma.
    pub fn ler_bytes(bytes: &[u8]) -> Result<Vec<PaginaLida>, ErroLeitura> {
        let paginas = inspecionar_bytes(bytes)?;
        exigir_texto(&paginas)?;
        Ok(paginas)
    }

    /// Lê um arquivo recebido e exige texto legível em cada página.
    pub fn ler_paginas(caminho: &Path) -> Result<Vec<PaginaLida>, ErroLeitura> {
        ler_bytes(&ler_arquivo(caminho, LIMITE_BYTES_PDF)?)
    }

    /// Como `ler_paginas`, para quem só quer contar e medir.
    pub fn inspecionar(caminho: &Path) -> Result<Vec<PaginaLida>, ErroLeitura> {
        inspecionar_bytes(&ler_arquivo(caminho, LIMITE_BYTES_PDF)?)
    }
}

#[cfg(windows)]
pub use comum::{inspecionar, inspecionar_bytes, ler_bytes, ler_paginas};

#[cfg(test)]
mod testes {
    use super::*;
    use crate::testes_comuns::PastaTemporaria;

    #[test]
    fn texto_legivel_pede_letra_ou_numero() {
        assert!(texto_legivel("Página 3"));
        assert!(texto_legivel("7"));
        assert!(!texto_legivel(""));
        assert!(!texto_legivel(" \r\n\t"));
        assert!(!texto_legivel("\u{fffd}\u{fffe}--"));
    }

    fn pagina(numero: usize, texto: &str) -> PaginaLida {
        PaginaLida {
            numero,
            largura_mm: 210.0,
            altura_mm: 297.0,
            rotacao: 0,
            texto: texto.to_string(),
        }
    }

    #[test]
    fn pagina_sem_texto_diz_qual_e() {
        assert!(exigir_texto(&[pagina(1, "1"), pagina(2, "2")]).is_ok());
        let erro = exigir_texto(&[pagina(1, "1"), pagina(2, " "), pagina(3, "")]).unwrap_err();
        assert!(matches!(erro, ErroLeitura::PaginaSemTexto { pagina: 2 }));
        assert!(erro.to_string().contains("página 2"));
    }

    #[test]
    fn arquivo_acima_do_limite_e_recusado_sem_ler() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.caminho("a.pdf");
        std::fs::write(&caminho, vec![0u8; 11]).unwrap();
        assert!(matches!(
            ler_arquivo(&caminho, 10),
            Err(ErroLeitura::Grande { bytes: 11 })
        ));
        assert_eq!(ler_arquivo(&caminho, 11).unwrap().len(), 11);
        assert!(matches!(
            ler_arquivo(&pasta.caminho("nao-existe.pdf"), 10),
            Err(ErroLeitura::Arquivo(..))
        ));
    }

    #[test]
    fn barreira_recusa_lixo_e_pdf_truncado_sem_travar() {
        for lixo in [
            &b""[..],
            &b"nao e pdf"[..],
            &b"%PDF-1.7\n"[..],
            &b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 9 0 R >>\nendobj\ntrailer << /Root 1 0 R >>\n"[..],
        ] {
            assert!(barreira(lixo).is_err());
        }
    }
}
