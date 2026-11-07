//! Provas da análise: contas de tamanho sem a biblioteca e, com `pdfium.dll`, páginas reais.

use super::*;
use lopdf::{dictionary, Document, Object, Stream};

fn pagina_de_conteudo(conteudo: String, largura: i64, altura: i64) -> ConteudoDePagina {
    ConteudoDePagina {
        largura,
        altura,
        conteudo,
    }
}

struct ConteudoDePagina {
    largura: i64,
    altura: i64,
    conteudo: String,
}

/// Um PDF com uma página por conteúdo, todas do mesmo tamanho.
fn pdf_de_paginas(paginas: Vec<ConteudoDePagina>) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let raiz = doc.new_object_id();
    let mut folhas = Vec::new();
    for pagina in paginas {
        let stream = doc.add_object(Stream::new(dictionary! {}, pagina.conteudo.into_bytes()));
        let objeto = dictionary! {
            "Type" => "Page",
            "Parent" => raiz,
            "MediaBox" => vec![0.into(), 0.into(), pagina.largura.into(), pagina.altura.into()],
            "Resources" => dictionary! {},
            "Contents" => stream
        };
        folhas.push(Object::Reference(doc.add_object(objeto)));
    }
    let total = folhas.len() as i64;
    doc.objects.insert(
        raiz,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => total,
            "Kids" => folhas
        }),
    );
    let catalogo = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => raiz});
    doc.trailer.set("Root", catalogo);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

/// Três páginas inteiras em ciano 10%, 20% e 30%.
fn pdf_de_tres_cianos() -> Vec<u8> {
    pdf_de_paginas(
        [0.1, 0.2, 0.3]
            .into_iter()
            .map(|c| pagina_de_conteudo(format!("{c} 0 0 0 k 0 0 300 300 re f"), 300, 300))
            .collect(),
    )
}

fn pixel(pagina: &PaginaRgb, x: usize, y: usize) -> [u8; 4] {
    let inicio = y * pagina.passo + x * 4;
    pagina.pixels[inicio..inicio + 4].try_into().unwrap()
}

/// Ciano simples de um pixel RGB: o preto é o que sobra do maior componente.
fn ciano_do_pixel(pixel: [u8; 4]) -> f64 {
    let r = f64::from(pixel[2]) / 255.0;
    let g = f64::from(pixel[1]) / 255.0;
    let b = f64::from(pixel[0]) / 255.0;
    let preto = 1.0 - r.max(g).max(b);
    if preto >= 1.0 {
        return 0.0;
    }
    (1.0 - r - preto) / (1.0 - preto) * 100.0
}

fn bloco_png(png: &mut Vec<u8>, tipo: &[u8; 4], dados: &[u8]) {
    png.extend((dados.len() as u32).to_be_bytes());
    png.extend_from_slice(tipo);
    png.extend_from_slice(dados);
    let mut crc = 0xffff_ffffu32;
    for byte in tipo.iter().chain(dados) {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    png.extend((!crc).to_be_bytes());
}

/// PNG RGB a partir de um bitmap BGRA, sem compressão.
fn png_rgb(pagina: &PaginaRgb) -> Vec<u8> {
    let (largura, altura) = (pagina.largura as usize, pagina.altura as usize);
    let mut cru = Vec::with_capacity(altura * (largura * 3 + 1));
    for linha in 0..altura {
        cru.push(0);
        for x in 0..largura {
            let inicio = linha * pagina.passo + x * 4;
            cru.extend([
                pagina.pixels[inicio + 2],
                pagina.pixels[inicio + 1],
                pagina.pixels[inicio],
            ]);
        }
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
    cabecalho.extend((largura as u32).to_be_bytes());
    cabecalho.extend((altura as u32).to_be_bytes());
    cabecalho.extend([8, 2, 0, 0, 0]);
    bloco_png(&mut png, b"IHDR", &cabecalho);
    bloco_png(&mut png, b"IDAT", &zlib);
    bloco_png(&mut png, b"IEND", &[]);
    png
}

#[test]
fn a4_a_300_dpi_cabe_no_limite_e_medidas_absurdas_nao_estouram() {
    assert_eq!(tamanho_do_bitmap(595.0, 842.0, 300).unwrap(), (2479, 3508));
    assert_eq!(tamanho_do_bitmap(300.0, 300.0, 72).unwrap(), (300, 300));
    for (largura, altura, dpi) in [
        (f32::NAN, 842.0, 300),
        (595.0, f32::INFINITY, 300),
        (0.0, 842.0, 300),
        (f32::MAX, f32::MAX, 300),
    ] {
        assert!(
            matches!(
                tamanho_do_bitmap(largura, altura, dpi),
                Err(ErroAnalise::TamanhoInvalido | ErroAnalise::MemoriaExcedida { .. })
            ),
            "{largura} × {altura} @ {dpi}"
        );
    }
    assert!(matches!(
        tamanho_do_bitmap(595.0, 842.0, 1200),
        Err(ErroAnalise::MemoriaExcedida { .. })
    ));
}

#[test]
fn resolucao_zero_ou_acima_do_teto_e_recusada() {
    assert_eq!(dpi_valido(0), Err(ErroAnalise::DpiInvalido(0)));
    assert_eq!(
        dpi_valido(DPI_MAXIMO + 1),
        Err(ErroAnalise::DpiInvalido(DPI_MAXIMO + 1))
    );
    assert!(dpi_valido(150).is_ok());
    assert!(dpi_valido(DPI_MAXIMO).is_ok());
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn tres_paginas_cmyk_tem_o_ciano_certo_no_pixel_central() {
    let sessao = SessaoDeAnalise::abrir(pdf_de_tres_cianos()).unwrap();
    assert_eq!(sessao.total_de_paginas(), 3);
    let mut ultima = None;
    for (numero, esperado) in [(1, 10.0), (2, 20.0), (3, 30.0)] {
        let pagina = sessao.renderizar(numero, 72).unwrap();
        assert_eq!((pagina.largura, pagina.altura), (300, 300));
        assert_eq!(pagina.passo, 300 * 4);
        let ciano = ciano_do_pixel(pixel(&pagina, 150, 150));
        assert!(
            (ciano - esperado).abs() < 2.5,
            "página {numero}: ciano {ciano:.1}%, esperado {esperado}%"
        );
        eprintln!("página {numero}: ciano {ciano:.1}% no pixel central");
        ultima = Some(pagina);
    }
    if let Some(destino) = std::env::var_os("PLOTS_ANALISE_PNG") {
        let pagina = ultima.expect("a terceira página foi desenhada");
        std::fs::write(&destino, png_rgb(&pagina)).unwrap();
        eprintln!("PNG: {}", std::path::Path::new(&destino).display());
    }
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn pagina_fora_do_documento_e_recusada() {
    let sessao = SessaoDeAnalise::abrir(pdf_de_tres_cianos()).unwrap();
    assert_eq!(
        sessao.renderizar(0, 72),
        Err(ErroAnalise::PaginaInexistente(0))
    );
    assert_eq!(
        sessao.renderizar(4, 72),
        Err(ErroAnalise::PaginaInexistente(4))
    );
    assert_eq!(
        sessao.renderizar(usize::MAX, 72),
        Err(ErroAnalise::PaginaInexistente(usize::MAX))
    );
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn resolucao_hostil_e_recusada_antes_de_alocar() {
    let sessao = SessaoDeAnalise::abrir(pdf_de_tres_cianos()).unwrap();
    assert_eq!(sessao.renderizar(1, 0), Err(ErroAnalise::DpiInvalido(0)));
    assert_eq!(
        sessao.renderizar(1, DPI_MAXIMO + 1),
        Err(ErroAnalise::DpiInvalido(DPI_MAXIMO + 1))
    );
    let a4 = pdf_de_paginas(vec![pagina_de_conteudo(
        "0.1 0 0 0 k 0 0 595 842 re f".to_string(),
        595,
        842,
    )]);
    let sessao = SessaoDeAnalise::abrir(a4).unwrap();
    assert!(matches!(
        sessao.renderizar(1, 1200),
        Err(ErroAnalise::MemoriaExcedida { .. })
    ));
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn conteudo_que_nao_e_pdf_nao_abre() {
    let erro = match SessaoDeAnalise::abrir(b"nao sou um pdf".to_vec()) {
        Ok(_) => panic!("o conteúdo inválido abriu"),
        Err(erro) => erro,
    };
    assert!(matches!(erro, ErroAnalise::Pdf(_)), "{erro:?}");
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn as_paginas_saem_na_ordem_pedida_e_o_callback_pode_parar() {
    let mut vistas = Vec::new();
    rasterizar_paginas(pdf_de_tres_cianos(), &[3, 1, 2], 72, |pagina| {
        vistas.push(pagina.numero);
        true
    })
    .unwrap();
    assert_eq!(vistas, vec![3, 1, 2]);

    let mut vistas = Vec::new();
    rasterizar_paginas(pdf_de_tres_cianos(), &[1, 2, 3], 72, |pagina| {
        vistas.push(pagina.numero);
        false
    })
    .unwrap();
    assert_eq!(vistas, vec![1]);
}
