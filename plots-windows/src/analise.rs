//! Rasterização de páginas para a estimativa de tinta.
//!
//! A análise abre o PDF uma vez e desenha uma página por vez, sobre branco, em BGRA. Cada buffer
//! é entregue sozinho: quem recebe calcula a cobertura e o descarta antes da página seguinte. Não
//! há qualidade de impressão aqui; a estimativa usa a média dos pixels.
//!
//! A biblioteca é a mesma do envio, carregada uma única vez no processo. Um PDF protegido não
//! chega até aqui: a leitura do documento já o recusa.

use std::fmt;

/// Maior resolução aceita para a análise. A interface usa 150 ou 300 dpi; valores acima só
/// poderiam vir de uma chamada direta e não têm utilidade prática.
pub const DPI_MAXIMO: u32 = 1200;

/// Maior bitmap aceito para uma página da análise (256 MiB). Em BGRA, uma folha A4 a 300 dpi
/// ocupa cerca de 8,7 MB; o limite comporta páginas grandes sem deixar o processo sem memória.
pub const LIMITE_BYTES_DA_ANALISE: u64 = 256 * 1024 * 1024;

/// Uma página desenhada, em BGRA de cima para baixo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaginaRgb {
    /// Número da página no documento, começando em 1.
    pub numero: usize,
    pub largura: u32,
    pub altura: u32,
    /// Distância entre o começo de duas linhas do buffer.
    pub passo: usize,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErroAnalise {
    /// A rasterização só existe no Windows.
    NaoSuportado,
    /// A biblioteca de rasterização não pôde ser carregada.
    Biblioteca(String),
    /// O conteúdo não abriu como PDF.
    Pdf(String),
    /// A página pedida não existe no documento.
    PaginaInexistente(usize),
    /// Resolução fora da faixa aceita.
    DpiInvalido(u32),
    /// Medidas de página que não geram um bitmap.
    TamanhoInvalido,
    /// Os pixels da página passariam do limite de memória.
    MemoriaExcedida { largura: u32, altura: u32 },
    /// A biblioteca recusou o desenho.
    Rasterizacao(String),
    /// O bitmap voltou com um tamanho que não forma as linhas pedidas.
    RespostaInvalida,
}

impl fmt::Display for ErroAnalise {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NaoSuportado => write!(f, "a análise de tinta só está disponível no Windows"),
            Self::Biblioteca(detalhe) => {
                write!(
                    f,
                    "não foi possível carregar a biblioteca de rasterização: {detalhe}"
                )
            }
            Self::Pdf(detalhe) => write!(f, "o conteúdo não abriu como PDF: {detalhe}"),
            Self::PaginaInexistente(numero) => {
                write!(f, "a página {numero} não existe no documento")
            }
            Self::DpiInvalido(dpi) => {
                write!(f, "a resolução {dpi} dpi não é aceita pela análise")
            }
            Self::TamanhoInvalido => write!(f, "as medidas da página não geram um bitmap"),
            Self::MemoriaExcedida { largura, altura } => write!(
                f,
                "a página em {largura} × {altura} pixels passa do limite de memória"
            ),
            Self::Rasterizacao(detalhe) => write!(f, "a rasterização da página falhou: {detalhe}"),
            Self::RespostaInvalida => {
                write!(
                    f,
                    "a rasterização devolveu um bitmap com tamanho inesperado"
                )
            }
        }
    }
}

impl std::error::Error for ErroAnalise {}

fn erro_de_desenho(erro: impl fmt::Display) -> ErroAnalise {
    ErroAnalise::Rasterizacao(erro.to_string())
}

fn pixels_da_pagina(pontos: f32, dpi: u32) -> Result<u32, ErroAnalise> {
    let valor = f64::from(pontos) / 72.0 * f64::from(dpi);
    let arredondado = valor.round();
    if !arredondado.is_finite() || !(1.0..=f64::from(u32::MAX)).contains(&arredondado) {
        return Err(ErroAnalise::TamanhoInvalido);
    }
    Ok(arredondado as u32)
}

fn tamanho_do_bitmap(largura_pt: f32, altura_pt: f32, dpi: u32) -> Result<(i32, i32), ErroAnalise> {
    let largura = pixels_da_pagina(largura_pt, dpi)?;
    let altura = pixels_da_pagina(altura_pt, dpi)?;
    let medidas = (largura, altura);
    let bytes = u64::from(largura)
        .checked_mul(u64::from(altura))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(ErroAnalise::MemoriaExcedida { largura, altura })?;
    if bytes > LIMITE_BYTES_DA_ANALISE {
        return Err(ErroAnalise::MemoriaExcedida { largura, altura });
    }
    let largura = i32::try_from(medidas.0).map_err(|_| ErroAnalise::MemoriaExcedida {
        largura: medidas.0,
        altura: medidas.1,
    })?;
    let altura = i32::try_from(medidas.1).map_err(|_| ErroAnalise::MemoriaExcedida {
        largura: medidas.0,
        altura: medidas.1,
    })?;
    Ok((largura, altura))
}

fn dpi_valido(dpi: u32) -> Result<(), ErroAnalise> {
    if dpi == 0 || dpi > DPI_MAXIMO {
        return Err(ErroAnalise::DpiInvalido(dpi));
    }
    Ok(())
}

#[cfg(windows)]
mod plataforma {
    use super::{dpi_valido, erro_de_desenho, tamanho_do_bitmap, ErroAnalise, PaginaRgb};
    use crate::backend::biblioteca;
    use pdfium_render::prelude::{
        PdfBitmapFormat, PdfColor, PdfDocument, PdfRenderConfig, PdfiumError,
    };

    /// Documento aberto, pronto para desenhar páginas em ordem.
    pub struct SessaoDeAnalise {
        documento: PdfDocument<'static>,
    }

    impl SessaoDeAnalise {
        /// Abre o conteúdo uma vez. A biblioteca toma a posse dos bytes, então o buffer vive
        /// enquanto o documento.
        pub fn abrir(bytes: Vec<u8>) -> Result<Self, ErroAnalise> {
            let pdfium = biblioteca().map_err(|erro| ErroAnalise::Biblioteca(erro.to_string()))?;
            let documento = pdfium
                .load_pdf_from_byte_vec(bytes, None)
                .map_err(|erro: PdfiumError| ErroAnalise::Pdf(erro.to_string()))?;
            Ok(Self { documento })
        }

        /// Quantidade de páginas que o desenho enxerga.
        pub fn total_de_paginas(&self) -> usize {
            usize::try_from(self.documento.pages().len()).unwrap_or(0)
        }

        /// Desenha uma página sobre branco, com o tamanho calculado pela página e pela resolução.
        pub fn renderizar(&self, numero: usize, dpi: u32) -> Result<PaginaRgb, ErroAnalise> {
            dpi_valido(dpi)?;
            let zero = numero
                .checked_sub(1)
                .ok_or(ErroAnalise::PaginaInexistente(numero))?;
            let indice = i32::try_from(zero).map_err(|_| ErroAnalise::PaginaInexistente(numero))?;
            if usize::try_from(indice).map_or(true, |indice| indice >= self.total_de_paginas()) {
                return Err(ErroAnalise::PaginaInexistente(numero));
            }
            let pagina = self
                .documento
                .pages()
                .get(indice)
                .map_err(erro_de_desenho)?;
            let (largura, altura) =
                tamanho_do_bitmap(pagina.width().value, pagina.height().value, dpi)?;
            let configuracao = PdfRenderConfig::new()
                .set_fixed_size(largura, altura)
                .set_format(PdfBitmapFormat::BGRA)
                // O DIB do GDI e a cobertura esperam BGRA; a inversão que a biblioteca liga por
                // padrão entregaria RGBA.
                .set_reverse_byte_order(false)
                .clear_before_rendering(true)
                .set_clear_color(PdfColor::WHITE);
            let bitmap = pagina
                .render_with_config(&configuracao)
                .map_err(erro_de_desenho)?;
            let pixels = bitmap.as_raw_bytes();
            let altura = u32::try_from(altura).map_err(|_| ErroAnalise::RespostaInvalida)?;
            let largura = u32::try_from(largura).map_err(|_| ErroAnalise::RespostaInvalida)?;
            let linha = usize::try_from(largura)
                .ok()
                .and_then(|largura| largura.checked_mul(4))
                .ok_or(ErroAnalise::RespostaInvalida)?;
            let linhas = usize::try_from(altura).map_err(|_| ErroAnalise::RespostaInvalida)?;
            if linhas == 0 || pixels.len() % linhas != 0 {
                return Err(ErroAnalise::RespostaInvalida);
            }
            let passo = pixels.len() / linhas;
            if passo < linha {
                return Err(ErroAnalise::RespostaInvalida);
            }
            Ok(PaginaRgb {
                numero,
                largura,
                altura,
                passo,
                pixels,
            })
        }
    }
}

#[cfg(not(windows))]
mod plataforma {
    use super::{ErroAnalise, PaginaRgb};

    /// Fora do Windows não há rasterização.
    pub struct SessaoDeAnalise;

    impl SessaoDeAnalise {
        pub fn abrir(_bytes: Vec<u8>) -> Result<Self, ErroAnalise> {
            Err(ErroAnalise::NaoSuportado)
        }

        pub fn total_de_paginas(&self) -> usize {
            0
        }

        pub fn renderizar(&self, _numero: usize, _dpi: u32) -> Result<PaginaRgb, ErroAnalise> {
            Err(ErroAnalise::NaoSuportado)
        }
    }
}

pub use plataforma::SessaoDeAnalise;

/// Abre o conteúdo e entrega cada página pedida, em ordem, ao callback. `false` interrompe sem
/// erro; o buffer entregue é liberado antes de a próxima página ser desenhada.
pub fn rasterizar_paginas<F>(
    bytes: Vec<u8>,
    paginas: &[usize],
    dpi: u32,
    mut entregar: F,
) -> Result<(), ErroAnalise>
where
    F: FnMut(PaginaRgb) -> bool,
{
    let sessao = SessaoDeAnalise::abrir(bytes)?;
    for numero in paginas {
        let pagina = sessao.renderizar(*numero, dpi)?;
        if !entregar(pagina) {
            break;
        }
    }
    Ok(())
}

#[cfg(all(test, windows))]
#[path = "analise_testes.rs"]
mod testes_com_a_biblioteca;
