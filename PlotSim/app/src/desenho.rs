//! Desenha uma página recebida para a tela. O PDF é entrada hostil: passa pelo mesmo limite de
//! tamanho do núcleo e o desenho tem largura e altura máximas.

/// Largura do desenho, em pixels: o dobro do que a folha ocupa na tela.
#[cfg_attr(not(windows), allow(dead_code))]
const LARGURA: i32 = 320;
#[cfg_attr(not(windows), allow(dead_code))]
const ALTURA_MAXIMA: i32 = 640;
const CABECALHO_BMP: u32 = 54;
const BYTES_POR_PONTO: u32 = 4;
const ALFABETO: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Por que a página não pôde ser desenhada.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErroDesenho {
    Biblioteca(String),
    Pdf(String),
    PaginaAusente,
    Medidas,
}

/// Base64 padrão. Escrito aqui para não trazer dependência por isso.
pub fn base64(bytes: &[u8]) -> String {
    let mut saida = String::with_capacity(bytes.len().div_ceil(3).saturating_mul(4));
    let letra = |valor: u8| char::from(ALFABETO[usize::from(valor & 0x3f)]);
    for bloco in bytes.chunks(3) {
        let a = bloco.first().copied().unwrap_or(0);
        let b = bloco.get(1).copied().unwrap_or(0);
        let c = bloco.get(2).copied().unwrap_or(0);
        saida.push(letra(a >> 2));
        saida.push(letra((a << 4) | (b >> 4)));
        saida.push(if bloco.len() > 1 {
            letra((b << 2) | (c >> 6))
        } else {
            '='
        });
        saida.push(if bloco.len() > 2 { letra(c) } else { '=' });
    }
    saida
}

/// Imagem BMP de 32 bits, de cima para baixo, a partir de linhas BGRA com `passo` bytes por linha.
pub fn bmp(largura: u32, altura: u32, passo: usize, pontos: &[u8]) -> Result<Vec<u8>, ErroDesenho> {
    let bytes_da_linha = largura
        .checked_mul(BYTES_POR_PONTO)
        .ok_or(ErroDesenho::Medidas)?;
    let tamanho_imagem = bytes_da_linha
        .checked_mul(altura)
        .ok_or(ErroDesenho::Medidas)?;
    let tamanho = tamanho_imagem
        .checked_add(CABECALHO_BMP)
        .ok_or(ErroDesenho::Medidas)?;
    let altura_negativa = i32::try_from(altura)
        .map_err(|_| ErroDesenho::Medidas)?
        .checked_neg()
        .ok_or(ErroDesenho::Medidas)?;
    let largura_assinada = i32::try_from(largura).map_err(|_| ErroDesenho::Medidas)?;
    let linha = usize::try_from(bytes_da_linha).map_err(|_| ErroDesenho::Medidas)?;
    if passo < linha || largura == 0 || altura == 0 {
        return Err(ErroDesenho::Medidas);
    }
    let mut saida = Vec::with_capacity(usize::try_from(tamanho).unwrap_or(0));
    saida.extend_from_slice(b"BM");
    saida.extend_from_slice(&tamanho.to_le_bytes());
    saida.extend_from_slice(&[0; 4]);
    saida.extend_from_slice(&CABECALHO_BMP.to_le_bytes());
    saida.extend_from_slice(&40u32.to_le_bytes());
    saida.extend_from_slice(&largura_assinada.to_le_bytes());
    saida.extend_from_slice(&altura_negativa.to_le_bytes());
    saida.extend_from_slice(&1u16.to_le_bytes());
    saida.extend_from_slice(&32u16.to_le_bytes());
    saida.extend_from_slice(&[0; 4]);
    saida.extend_from_slice(&tamanho_imagem.to_le_bytes());
    saida.extend_from_slice(&[0; 16]);
    let mut linhas = 0u32;
    for pedaco in pontos.chunks_exact(passo) {
        let conteudo = pedaco.get(..linha).ok_or(ErroDesenho::Medidas)?;
        saida.extend_from_slice(conteudo);
        linhas = linhas.saturating_add(1);
        if linhas == altura {
            break;
        }
    }
    if linhas != altura {
        return Err(ErroDesenho::Medidas);
    }
    Ok(saida)
}

#[cfg(windows)]
mod comum {
    use super::*;
    use pdfium_render::prelude::{PdfBitmapFormat, PdfRenderConfig, PdfiumError};
    use plotsim_core::leitor::{biblioteca, ler_arquivo, LIMITE_BYTES_PDF};
    use std::path::Path;

    /// A página como imagem em endereço de dados, pronta para a tela. A biblioteca é a mesma que a
    /// leitura da marca usa: o PDFium só aceita uma ligação por processo.
    pub fn desenhar(arquivo: &Path, indice: usize) -> Result<String, ErroDesenho> {
        let bytes = ler_arquivo(arquivo, LIMITE_BYTES_PDF)
            .map_err(|erro| ErroDesenho::Pdf(erro.to_string()))?;
        let invalido = |erro: PdfiumError| ErroDesenho::Pdf(erro.to_string());
        let documento = biblioteca()
            .map_err(|erro| ErroDesenho::Biblioteca(erro.to_string()))?
            .load_pdf_from_byte_slice(&bytes, None)
            .map_err(invalido)?;
        let pagina = documento
            .pages()
            .iter()
            .nth(indice)
            .ok_or(ErroDesenho::PaginaAusente)?;
        let configuracao = PdfRenderConfig::new()
            .set_target_width(LARGURA)
            .set_maximum_height(ALTURA_MAXIMA)
            .set_format(PdfBitmapFormat::BGRA);
        let desenho = pagina.render_with_config(&configuracao).map_err(invalido)?;
        let largura = u32::try_from(desenho.width()).map_err(|_| ErroDesenho::Medidas)?;
        let altura = u32::try_from(desenho.height()).map_err(|_| ErroDesenho::Medidas)?;
        let pontos = desenho.as_raw_bytes();
        let linhas = usize::try_from(altura).map_err(|_| ErroDesenho::Medidas)?;
        let passo = pontos
            .len()
            .checked_div(linhas)
            .ok_or(ErroDesenho::Medidas)?;
        let imagem = bmp(largura, altura, passo, &pontos)?;
        Ok(format!("data:image/bmp;base64,{}", base64(&imagem)))
    }
}

#[cfg(windows)]
pub use comum::desenhar;

#[cfg(not(windows))]
pub fn desenhar(_: &std::path::Path, _: usize) -> Result<String, ErroDesenho> {
    Err(ErroDesenho::Biblioteca("só existe no Windows".to_string()))
}

#[cfg(test)]
mod testes {
    use super::*;

    /// PDF de duas páginas com número em texto e sem marca: a leitura não pode tirar número daí.
    fn pdf_de_duas_paginas() -> Vec<u8> {
        let mut pdf = b"%PDF-1.5\n".to_vec();
        let conteudo = "BT /F1 48 Tf 40 300 Td (7) Tj ET";
        let objetos = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Count 2 /Kids [4 0 R 5 0 R] >>".to_string(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents 6 0 R >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Rotate 180 /Resources << /Font << /F1 3 0 R >> >> /Contents 6 0 R >>".to_string(),
            format!("<< /Length {} >>\nstream\n{conteudo}\nendstream", conteudo.len()),
        ];
        let mut posicoes = Vec::new();
        for (indice, corpo) in objetos.iter().enumerate() {
            posicoes.push(pdf.len());
            pdf.extend(format!("{} 0 obj\n{corpo}\nendobj\n", indice + 1).into_bytes());
        }
        let xref = pdf.len();
        pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objetos.len() + 1).into_bytes());
        for posicao in posicoes {
            pdf.extend(format!("{posicao:010} 00000 n \n").into_bytes());
        }
        pdf.extend(
            format!(
                "trailer << /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objetos.len() + 1
            )
            .into_bytes(),
        );
        pdf
    }

    /// A biblioteca de PDF só aceita uma ligação por processo: ler e desenhar têm de usar a mesma.
    #[test]
    #[ignore = "precisa da pdfium.dll em PLOTS_PDFIUM_DIR"]
    fn le_e_desenha_o_mesmo_pdf_no_mesmo_processo() {
        let arquivo =
            std::env::temp_dir().join(format!("plotsim-desenho-{}.pdf", std::process::id()));
        std::fs::write(&arquivo, pdf_de_duas_paginas()).unwrap();
        let paginas = crate::plataforma::ler_marcas(&arquivo).unwrap();
        assert_eq!(paginas.len(), 2);
        // Sem marca, nenhuma página ganha número, mesmo com o número escrito no texto.
        assert!(paginas.iter().all(|pagina| pagina.leitura.is_err()));
        let desenho = desenhar(&arquivo, 1).unwrap();
        assert!(desenho.starts_with("data:image/bmp;base64,Qk"));
        assert_eq!(desenhar(&arquivo, 2), Err(ErroDesenho::PaginaAusente));
        std::fs::remove_file(&arquivo).unwrap();
    }

    #[test]
    fn base64_segue_os_vetores_de_referencia() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
    }

    #[test]
    fn bmp_tem_o_cabecalho_certo_e_tira_o_excesso_de_cada_linha() {
        // Duas linhas de um ponto, com quatro bytes de folga por linha.
        let pontos = [1, 2, 3, 4, 9, 9, 9, 9, 5, 6, 7, 8, 9, 9, 9, 9];
        let imagem = bmp(1, 2, 8, &pontos).unwrap();
        assert_eq!(&imagem[..2], b"BM");
        assert_eq!(imagem.len(), 54 + 8);
        assert_eq!(&imagem[2..6], &62u32.to_le_bytes());
        assert_eq!(&imagem[22..26], &(-2i32).to_le_bytes());
        assert_eq!(&imagem[54..], &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn medidas_absurdas_sao_recusadas_sem_travar() {
        assert_eq!(bmp(0, 1, 4, &[0; 4]), Err(ErroDesenho::Medidas));
        assert_eq!(bmp(1, 0, 4, &[0; 4]), Err(ErroDesenho::Medidas));
        assert_eq!(bmp(u32::MAX, 2, 4, &[0; 8]), Err(ErroDesenho::Medidas));
        assert_eq!(bmp(2, u32::MAX, 8, &[0; 8]), Err(ErroDesenho::Medidas));
        // A imagem diz ter mais linhas do que os dados trazem.
        assert_eq!(bmp(1, 3, 4, &[0; 8]), Err(ErroDesenho::Medidas));
        // Passo menor que a linha.
        assert_eq!(bmp(2, 1, 4, &[0; 8]), Err(ErroDesenho::Medidas));
    }
}
