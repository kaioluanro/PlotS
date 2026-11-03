//! Leitura de um PDF recebido. O arquivo veio de fora: passa primeiro pela mesma barreira do PlotS
//! e só depois vai para o PDFium, sempre com limite de tamanho.

use crate::marca::{ErroMarca, MarcaLida};
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
/// Maior desenho de uma página para ler a marca, em pixels: o mesmo teto que a leitura da marca aceita.
pub const LIMITE_PIXELS_DO_DESENHO: usize = 100_000_000;

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
    /// O desenho de uma página veio com medidas que não fecham.
    Medidas,
    /// A página é tão alongada que o desenho para ler a marca passaria do limite de pixels.
    DesenhoGrande {
        largura: usize,
        altura: usize,
    },
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
            Self::Medidas => f.write_str("o desenho de uma página veio com medidas inválidas"),
            Self::DesenhoGrande { largura, altura } => write!(
                f,
                "a página daria um desenho de {largura} × {altura} pixels para ler a marca, acima de {LIMITE_PIXELS_DO_DESENHO} pixels; a proporção da página não cabe na leitura"
            ),
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

/// Uma página recebida, com o que a marca dela disse.
#[derive(Clone, Debug, PartialEq)]
pub struct PaginaMarcada {
    /// Número da página no arquivo, começando em 1.
    pub numero: usize,
    /// Tamanho em milímetros como a página é vista, já com a rotação aplicada.
    pub largura_mm: f64,
    pub altura_mm: f64,
    pub leitura: Result<MarcaLida, ErroMarca>,
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

/// Passa um desenho BGRA para cinza pelo canal verde, respeitando o passo de cada linha. Medidas que
/// não fecham dão `None`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn cinza_de_bgra(bytes: &[u8], colunas: usize, linhas: usize) -> Option<Vec<u8>> {
    let minimo = colunas.checked_mul(4)?;
    if linhas == 0 || minimo == 0 {
        return None;
    }
    let passo = bytes.len() / linhas;
    if passo < minimo {
        return None;
    }
    let mut cinza = Vec::new();
    cinza.try_reserve_exact(colunas.checked_mul(linhas)?).ok()?;
    for linha in bytes.chunks_exact(passo).take(linhas) {
        cinza.extend(
            linha
                .chunks_exact(4)
                .take(colunas)
                .filter_map(|ponto| ponto.get(1).copied()),
        );
    }
    Some(cinza)
}

/// Altura e largura, em pixels, do desenho de uma página com a largura pedida. Medidas que não fecham
/// e desenhos grandes demais são recusados antes de qualquer alocação.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn medidas_do_desenho(
    largura_pt: f64,
    altura_pt: f64,
    largura_px: i32,
) -> Result<(usize, usize), ErroLeitura> {
    let largura = usize::try_from(largura_px).map_err(|_| ErroLeitura::Medidas)?;
    let medidas_validas = largura > 0
        && largura_pt.is_finite()
        && altura_pt.is_finite()
        && largura_pt > 0.0
        && altura_pt > 0.0;
    if !medidas_validas {
        return Err(ErroLeitura::Medidas);
    }
    let largura_f = f64::from(largura_px);
    let altura = (largura_f * altura_pt / largura_pt).ceil();
    if !(altura >= 1.0) {
        return Err(ErroLeitura::Medidas);
    }
    // Com a conta feita em ponto flutuante, o teste vem antes do arredondamento para inteiro.
    if !(largura_f * altura <= LIMITE_PIXELS_DO_DESENHO as f64) {
        return Err(ErroLeitura::DesenhoGrande {
            largura,
            altura: altura as usize,
        });
    }
    Ok((largura, altura as usize))
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
    use crate::marca::{ler_marca, Cinza};
    use pdfium_render::prelude::{
        PdfBitmapFormat, PdfColor, PdfPage, PdfPageRenderRotation, PdfRenderConfig, Pdfium,
    };
    use std::sync::{Mutex, OnceLock, PoisonError};

    /// A biblioteca só pode ser iniciada uma vez por processo.
    static BIBLIOTECA: OnceLock<Pdfium> = OnceLock::new();
    /// Garante uma só ligação mesmo com chamadas simultâneas na primeira vez.
    static INICIO: Mutex<()> = Mutex::new(());

    /// A biblioteca do PDFium, ligada uma só vez por processo: a janela e a leitura usam esta mesma.
    pub fn biblioteca() -> Result<&'static Pdfium, ErroLeitura> {
        // Pelos mesmos caminhos do PlotS, nunca pela busca do sistema.
        if let Some(pronta) = BIBLIOTECA.get() {
            return Ok(pronta);
        }
        let _inicio = INICIO.lock().unwrap_or_else(PoisonError::into_inner);
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

    /// Largura do desenho usado para ler a marca, em pontos. Basta para os módulos de uma marca de um
    /// quinto da página; se a marca vier menor, a segunda largura tenta de novo.
    const LARGURAS_DE_LEITURA: [i32; 2] = [1000, 3000];

    /// Desenha a página em cinza, sobre branco, e lê a marca.
    fn ler_marca_da_pagina(
        pagina: &PdfPage<'_>,
    ) -> Result<Result<MarcaLida, ErroMarca>, ErroLeitura> {
        let mut leitura = Err(ErroMarca::SemMarca);
        let (largura_pt, altura_pt) = (
            f64::from(pagina.width().value),
            f64::from(pagina.height().value),
        );
        for largura in LARGURAS_DE_LEITURA {
            medidas_do_desenho(largura_pt, altura_pt, largura)?;
            let configuracao = PdfRenderConfig::new()
                .set_target_width(largura)
                .set_format(PdfBitmapFormat::BGRA)
                .clear_before_rendering(true)
                .set_clear_color(PdfColor::WHITE);
            let desenho = pagina
                .render_with_config(&configuracao)
                .map_err(|erro| ErroLeitura::Pdf(erro.to_string()))?;
            let (colunas, linhas) = (
                usize::try_from(desenho.width()).map_err(|_| ErroLeitura::Medidas)?,
                usize::try_from(desenho.height()).map_err(|_| ErroLeitura::Medidas)?,
            );
            let bytes = desenho.as_raw_bytes();
            let cinza = cinza_de_bgra(&bytes, colunas, linhas).ok_or(ErroLeitura::Medidas)?;
            leitura = ler_marca(Cinza {
                largura: colunas,
                altura: linhas,
                pontos: &cinza,
            });
            if !matches!(leitura, Err(ErroMarca::MarcaPequena { .. })) {
                break;
            }
        }
        Ok(leitura)
    }

    /// Lê a marca de cada página de um PDF já em memória.
    pub fn ler_marcas_de_bytes(bytes: &[u8]) -> Result<Vec<PaginaMarcada>, ErroLeitura> {
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
            lidas.push(PaginaMarcada {
                numero: indice.saturating_add(1),
                largura_mm: f64::from(pagina.width().value) * MM_POR_PONTO,
                altura_mm: f64::from(pagina.height().value) * MM_POR_PONTO,
                leitura: ler_marca_da_pagina(&pagina)?,
            });
        }
        Ok(lidas)
    }

    /// Lê a marca de cada página de um arquivo recebido.
    pub fn ler_marcas(caminho: &Path) -> Result<Vec<PaginaMarcada>, ErroLeitura> {
        ler_marcas_de_bytes(&ler_arquivo(caminho, LIMITE_BYTES_PDF)?)
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
pub use comum::{
    biblioteca, inspecionar, inspecionar_bytes, ler_bytes, ler_marcas, ler_marcas_de_bytes,
    ler_paginas,
};
/// O tipo da biblioteca devolvida por `biblioteca()`, para quem usa a mesma versão.
#[cfg(windows)]
pub use pdfium_render::prelude::Pdfium;

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
    fn cinza_respeita_o_passo_da_linha_e_recusa_medidas_ruins() {
        // Dois pontos por linha, com passo de doze bytes.
        let bytes = [
            0, 10, 0, 255, 0, 20, 0, 255, 9, 9, 9, 9, 0, 30, 0, 255, 0, 40, 0, 255, 9, 9, 9, 9,
        ];
        assert_eq!(cinza_de_bgra(&bytes, 2, 2), Some(vec![10, 20, 30, 40]));
        assert_eq!(cinza_de_bgra(&bytes, 7, 2), None);
        assert_eq!(cinza_de_bgra(&bytes, 0, 2), None);
        assert_eq!(cinza_de_bgra(&bytes, 2, 0), None);
        assert_eq!(cinza_de_bgra(&bytes, usize::MAX, 1), None);
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

    #[test]
    fn o_desenho_segue_a_proporcao_da_pagina() {
        assert_eq!(
            medidas_do_desenho(595.0, 842.0, 1000).ok(),
            Some((1000, 1416))
        );
        assert_eq!(medidas_do_desenho(1000.0, 1.0, 1000).ok(), Some((1000, 1)));
        assert_eq!(
            medidas_do_desenho(612.0, 792.0, 3000).ok(),
            Some((3000, 3883))
        );
    }

    #[test]
    fn pagina_alongada_demais_e_recusada_antes_de_desenhar() {
        for (largura, altura) in [
            (1.0, 1_000_000.0),
            (0.5, 1e12),
            (1.0, 1e300),
            (1e-300, 1e300),
        ] {
            let erro = medidas_do_desenho(largura, altura, 3000).unwrap_err();
            assert!(
                matches!(erro, ErroLeitura::DesenhoGrande { .. }),
                "{largura} x {altura}: {erro:?}"
            );
            assert!(erro.to_string().contains("pixels"));
        }
    }

    #[test]
    fn medidas_que_nao_fecham_sao_recusadas_sem_panico() {
        for (largura, altura, px) in [
            (0.0, 842.0, 1000),
            (595.0, 0.0, 1000),
            (-595.0, 842.0, 1000),
            (f64::NAN, 842.0, 1000),
            (595.0, f64::INFINITY, 1000),
            (595.0, 842.0, 0),
            (595.0, 842.0, -1000),
            (1e300, 1e-300, 1000),
        ] {
            assert!(
                matches!(
                    medidas_do_desenho(largura, altura, px),
                    Err(ErroLeitura::Medidas)
                ),
                "{largura} x {altura} em {px} px"
            );
        }
    }

    #[test]
    fn o_limite_de_pixels_e_inclusivo() {
        // Largura de 1000 px: 100 000 px de altura dão exatamente o limite de 100 milhões de pixels.
        assert_eq!(
            medidas_do_desenho(1.0, 100.0, 1000).ok(),
            Some((1000, 100_000))
        );
        assert!(matches!(
            medidas_do_desenho(1.0, 100.1, 1000),
            Err(ErroLeitura::DesenhoGrande { .. })
        ));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "precisa da pdfium.dll"]
    fn chamadas_simultaneas_recebem_a_mesma_biblioteca() {
        let barreira = std::sync::Arc::new(std::sync::Barrier::new(4));
        let enderecos: Vec<usize> = (0..4)
            .map(|_| {
                let barreira = std::sync::Arc::clone(&barreira);
                std::thread::spawn(move || {
                    barreira.wait();
                    biblioteca().map(|pdfium| std::ptr::from_ref(pdfium) as usize)
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|fio| fio.join().unwrap().unwrap())
            .collect();
        assert!(enderecos.windows(2).all(|par| par[0] == par[1]));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "precisa da pdfium.dll"]
    fn a_biblioteca_e_a_mesma_em_chamadas_seguidas() {
        let primeira = biblioteca().unwrap();
        let segunda = biblioteca().unwrap();
        assert!(std::ptr::eq(primeira, segunda));
    }
}
