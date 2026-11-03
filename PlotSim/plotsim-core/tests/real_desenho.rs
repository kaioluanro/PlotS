//! Desenha as páginas de um PDF recebido em PNG, para olhar à mão o que a fila gravou.
//! Roda com `--ignored`, `PLOTS_PDFIUM_DIR` e `PLOTSIM_PDF_PARA_OLHAR`.
#![cfg(windows)]

use pdfium_render::prelude::{PdfBitmapFormat, PdfRenderConfig, Pdfium};
use std::path::{Path, PathBuf};

fn crc32(bytes: &[u8]) -> u32 {
    !bytes.iter().fold(!0u32, |mut crc, byte| {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
        crc
    })
}

fn bloco(png: &mut Vec<u8>, tipo: &[u8; 4], dados: &[u8]) {
    png.extend((dados.len() as u32).to_be_bytes());
    let mut com_tipo = tipo.to_vec();
    com_tipo.extend(dados);
    png.extend(&com_tipo);
    png.extend(crc32(&com_tipo).to_be_bytes());
}

/// PNG em tons de cinza, sem compressão: blocos guardados do deflate.
fn png_cinza(largura: u32, altura: u32, cinza: &[u8]) -> Vec<u8> {
    let mut cru = Vec::new();
    for linha in cinza.chunks(largura as usize) {
        cru.push(0);
        cru.extend(linha);
    }
    let mut zlib = vec![0x78, 0x01];
    let pedacos: Vec<&[u8]> = cru.chunks(0xffff).collect();
    for (indice, pedaco) in pedacos.iter().enumerate() {
        zlib.push(u8::from(indice + 1 == pedacos.len()));
        zlib.extend((pedaco.len() as u16).to_le_bytes());
        zlib.extend((!(pedaco.len() as u16)).to_le_bytes());
        zlib.extend(*pedaco);
    }
    let (a, b) = cru.iter().fold((1u32, 0u32), |(a, b), byte| {
        let a = (a + u32::from(*byte)) % 65_521;
        (a, (b + a) % 65_521)
    });
    zlib.extend(((b << 16) | a).to_be_bytes());
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut cabecalho = Vec::new();
    cabecalho.extend(largura.to_be_bytes());
    cabecalho.extend(altura.to_be_bytes());
    cabecalho.extend([8, 0, 0, 0, 0]);
    bloco(&mut png, b"IHDR", &cabecalho);
    bloco(&mut png, b"IDAT", &zlib);
    bloco(&mut png, b"IEND", &[]);
    png
}

#[test]
#[ignore = "precisa da pdfium.dll e de um PDF indicado"]
fn desenha_o_pdf_indicado_em_png() {
    let pdf = PathBuf::from(std::env::var_os("PLOTSIM_PDF_PARA_OLHAR").unwrap());
    let pasta = PathBuf::from(std::env::var_os("PLOTS_PDFIUM_DIR").unwrap());
    let ligacoes =
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&pasta)).unwrap();
    let pdfium = Pdfium::new(ligacoes);
    let documento = pdfium.load_pdf_from_file(&pdf, None).unwrap();
    for (indice, pagina) in documento.pages().iter().enumerate() {
        let configuracao = PdfRenderConfig::new()
            .set_target_width(300)
            .set_format(PdfBitmapFormat::BGRA);
        let desenho = pagina.render_with_config(&configuracao).unwrap();
        let (largura, altura) = (desenho.width() as u32, desenho.height() as u32);
        let bytes = desenho.as_raw_bytes();
        let passo = bytes.len() / altura as usize;
        let verde = |linha: &[u8]| {
            linha
                .chunks(4)
                .take(largura as usize)
                .map(|p| p[1])
                .collect::<Vec<u8>>()
        };
        let cinza: Vec<u8> = bytes.chunks(passo).flat_map(verde).collect();
        let tinta = cinza.iter().filter(|valor| **valor < 128).count();
        println!(
            "página {}: {tinta} de {} pontos escuros",
            indice + 1,
            cinza.len()
        );
        let saida = Path::new(&pdf).with_extension(format!("{}.png", indice + 1));
        std::fs::write(&saida, png_cinza(largura, altura, &cinza)).unwrap();
        println!("{}", saida.display());
    }
}
