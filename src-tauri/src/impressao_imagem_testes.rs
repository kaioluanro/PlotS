//! Uso real: abre uma imagem colorida, envia a página derivada para a fila de PDF do Windows e
//! confere as cores no PDF que a própria fila gravou.
//!
//! Precisa da pdfium.dll e de uma fila de PDF instalada. Fica fora do CI e roda com `--ignored`.

use crate::documento::{abrir_para_leitura, TipoDocumento};
use crate::leitura_pdf;
use image::{Rgb, RgbImage};
use pdfium_render::prelude::{PdfBitmapFormat, PdfRenderConfig, Pdfium, PdfiumError};
use plots_core::Pagina;
use plots_windows::plots_print::job::{Backend, EstadoEnvio, Passada};
use plots_windows::BackendWindows;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A única fila em que este teste pode enviar.
const FILA_DE_PDF: &str = "Microsoft Print to PDF";
const LARGURA: u32 = 800;
const ALTURA: u32 = 600;

/// Quatro faixas verticais iguais: vermelho, laranja, amarelo e azul.
fn png_de_faixas() -> Vec<u8> {
    let faixa = LARGURA / 4;
    let imagem = RgbImage::from_fn(LARGURA, ALTURA, |x, _| {
        let cor = match x / faixa {
            0 => [255, 0, 0],
            1 => [255, 128, 0],
            2 => [255, 255, 0],
            _ => [0, 0, 255],
        };
        Rgb(cor)
    });
    let mut bytes = std::io::Cursor::new(Vec::new());
    imagem
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("codificar o PNG do teste");
    bytes.into_inner()
}

fn biblioteca() -> Pdfium {
    let pasta = std::env::var_os("PLOTS_PDFIUM_DIR").expect("PLOTS_PDFIUM_DIR definida");
    let caminho = Path::new(&pasta).join("pdfium.dll");
    match Pdfium::bind_to_library(&caminho) {
        Ok(ligacoes) => Pdfium::new(ligacoes),
        // O envio já iniciou a biblioteca neste processo; a instância reusa as mesmas ligações.
        Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Pdfium::default(),
        Err(erro) => panic!("carregar {}: {erro}", caminho.display()),
    }
}

fn original(numero: usize) -> Pagina {
    Pagina {
        numero: Some(numero),
        girar_180: false,
        copia: 0,
    }
}

#[test]
#[ignore = "precisa da pdfium.dll e envia à fila de PDF do Windows"]
fn imagem_colorida_chega_ao_pdf_da_fila_com_as_cores_certas() {
    let Some(raiz) = std::env::var_os("PLOTS_ARQUIVO_SAIDA").map(PathBuf::from) else {
        eprintln!("PLOTS_ARQUIVO_SAIDA não definida: nada foi enviado");
        return;
    };
    let Some(fila) = plots_windows::listar_filas()
        .unwrap()
        .into_iter()
        .find(|fila| fila.nome == FILA_DE_PDF)
    else {
        eprintln!("fila de PDF ausente: nada foi enviado");
        return;
    };

    let pasta = raiz.join(format!("teste-imagem-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&pasta);
    std::fs::create_dir_all(&pasta).unwrap();
    let imagem = pasta.join("faixas.png");
    let bytes = png_de_faixas();
    std::fs::write(&imagem, &bytes).unwrap();
    let caminho = imagem.to_string_lossy().into_owned();
    eprintln!(
        "imagem: {} ({LARGURA}x{ALTURA}), {} bytes",
        imagem.display(),
        bytes.len()
    );

    // A mesma abertura que a interface usa: a imagem vira um PDF derivado e o nome exibido
    // continua o do arquivo original.
    let (documento, derivado) = abrir_para_leitura(&caminho).expect("abrir a imagem");
    assert_eq!(documento.tipo, TipoDocumento::Imagem);
    assert_eq!(documento.nome, "faixas.png");
    eprintln!(
        "derivado: {} ({} bytes)",
        derivado,
        std::fs::metadata(&derivado).unwrap().len()
    );

    // A impressão lê o mesmo conteúdo derivado, pela mesma barreira da abertura.
    let lido = leitura_pdf::ler_pdf(&caminho).expect("ler o derivado da imagem");
    assert_eq!(lido.dimensoes.len(), 1, "uma página por imagem");
    let medida = lido.dimensoes[0];
    eprintln!(
        "página do derivado: {:.1}x{:.1} mm",
        medida.largura_mm, medida.altura_mm
    );
    assert!(
        (medida.largura_mm - 297.0).abs() < 1.0 && (medida.altura_mm - 210.0).abs() < 1.0,
        "imagem deitada deveria cair em A4 paisagem, e caiu em {:.1}x{:.1} mm",
        medida.largura_mm,
        medida.altura_mm
    );

    let mut backend =
        BackendWindows::novo(&fila, lido.origem, lido.dimensoes, Some(&pasta)).unwrap();
    backend.enviar(Passada::Frente, &[original(1)]).unwrap();

    let mut estado = EstadoEnvio::EmAndamento;
    for _ in 0..600 {
        estado = backend.consultar().unwrap();
        if estado != EstadoEnvio::EmAndamento {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(estado, EstadoEnvio::Concluido);

    let arquivo = pasta.join("lote-0001-frente.pdf");
    let saida = std::fs::read(&arquivo).expect("o PDF gravado pela fila");
    eprintln!("saída: {} ({} bytes)", arquivo.display(), saida.len());

    // Reabre o PDF que o driver gravou e desenha como o envio desenha: bytes BGRA, sem a
    // inversão que o pdfium-render liga por padrão.
    let pdfium = biblioteca();
    let documento = pdfium.load_pdf_from_byte_slice(&saida, None).unwrap();
    let pagina = documento.pages().get(0).unwrap();
    let configuracao = PdfRenderConfig::new()
        .set_reverse_byte_order(false)
        .set_format(PdfBitmapFormat::BGRA);
    let desenho = pagina.render_with_config(&configuracao).unwrap();
    let (largura, altura) = (desenho.width() as usize, desenho.height() as usize);
    let pixels = desenho.as_raw_bytes();
    assert_eq!(pixels.len(), largura * altura * 4, "bitmap do PDF gerado");

    let (mut vermelho, mut laranja, mut amarelo, mut azul) = (0u64, 0u64, 0u64, 0u64);
    let (mut laranja_azulado, mut ciano) = (0u64, 0u64);
    for pixel in pixels.chunks_exact(4) {
        let (b, g, r) = (pixel[0], pixel[1], pixel[2]);
        if r > 200 && g < 60 && b < 60 {
            vermelho += 1;
        }
        if r > 200 && g > 80 && g < 180 && b < 60 {
            laranja += 1;
        }
        if r > 200 && g > 200 && b < 60 {
            amarelo += 1;
        }
        if b > 200 && g < 60 && r < 60 {
            azul += 1;
        }
        if b > 200 && g > 80 && g < 180 && r < 60 {
            laranja_azulado += 1;
        }
        if b > 200 && g > 200 && r < 60 {
            ciano += 1;
        }
    }
    // A página deitada vai para o papel em pé girada, então a faixa varia ao longo de um eixo ou
    // do outro; o que importa aqui é a cor de cada faixa, não a direção do giro.
    eprintln!(
        "PDF: {largura}x{altura}, vermelho {vermelho}, laranja {laranja}, amarelo {amarelo}, \
         azul {azul}, laranja azulado {laranja_azulado}, ciano {ciano}"
    );

    if let Some(destino) = std::env::var_os("PLOTS_COR_PNG").map(PathBuf::from) {
        let mut rgb = RgbImage::new(largura as u32, altura as u32);
        for (indice, pixel) in pixels.chunks_exact(4).enumerate() {
            let x = (indice % largura) as u32;
            let y = (indice / largura) as u32;
            rgb.put_pixel(x, y, Rgb([pixel[2], pixel[1], pixel[0]]));
        }
        rgb.save(&destino).expect("gravar o PNG do PDF gerado");
        eprintln!("PNG: {}", destino.display());
    }

    assert!(vermelho > 1000, "vermelho com só {vermelho} pixels");
    assert!(laranja > 1000, "laranja com só {laranja} pixels");
    assert!(amarelo > 1000, "amarelo com só {amarelo} pixels");
    assert!(azul > 1000, "azul com só {azul} pixels");
    assert_eq!(laranja_azulado, 0, "laranja saiu azulado");
    assert_eq!(ciano, 0, "amarelo saiu ciano");
}
