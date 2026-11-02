//! Pedem a DLL e, no último, o spooler: ficam fora do CI e rodam com `--ignored`.

use super::*;
use crate::raster::Dispositivo;
use lopdf::{dictionary, Document, Object, Stream};

/// A única fila em que estes testes podem enviar.
const FILA_DE_PDF: &str = "Microsoft Print to PDF";
const PRETO: [u8; 4] = [0, 0, 0, 255];
const BRANCO: [u8; 4] = [255, 255, 255, 255];

/// A 72 dpi, um ponto do PDF vale um pixel.
fn fila_a_72_dpi(largura: i32, altura: i32) -> Dispositivo {
    Dispositivo {
        dpi_x: 72,
        dpi_y: 72,
        papel_largura: largura,
        papel_altura: altura,
        margem_x: 0,
        margem_y: 0,
        area_largura: largura,
        area_altura: altura,
    }
}

/// Uma página com um quadrado preto de 40 pontos no canto superior esquerdo.
fn pdf_com_marca(largura: i64, altura: i64, rotacao: i64, unidade: Option<f32>) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let raiz = doc.new_object_id();
    let conteudo = format!("0 g 0 {} 40 40 re f", altura - 40);
    let stream = doc.add_object(Stream::new(dictionary! {}, conteudo.into_bytes()));
    let mut pagina = dictionary! {
        "Type" => "Page", "Parent" => raiz,
        "MediaBox" => vec![0.into(), 0.into(), largura.into(), altura.into()],
        "Resources" => dictionary! {}, "Contents" => stream, "Rotate" => rotacao
    };
    if let Some(unidade) = unidade {
        pagina.set("UserUnit", unidade);
    }
    let folha = doc.add_object(pagina);
    doc.objects.insert(
        raiz,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Count" => 1, "Kids" => vec![Object::Reference(folha)]
        }),
    );
    let catalogo = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => raiz});
    doc.trailer.set("Root", catalogo);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

/// Desenha a página 1 do PDF como o envio faria, passando pela barreira e pela passada.
fn desenhar(pdf: &[u8], pagina: Pagina, fila: &Dispositivo) -> (PlanoPagina, Vec<u8>) {
    let origem = crate::abrir_origem(pdf).unwrap();
    let dimensoes = origem.dimensoes_paginas().unwrap();
    let passada = origem.gerar_passada(&[pagina], 1).unwrap();
    let pdfium = biblioteca().unwrap();
    let documento = pdfium.load_pdf_from_byte_slice(&passada, None).unwrap();
    let planos = planejar_paginas(&documento, &[pagina], &dimensoes, fila).unwrap();
    let pixels = rasterizar(&documento, 0, pagina, &planos[0], fila).unwrap();
    (planos[0], pixels)
}

fn pixel(plano: &PlanoPagina, pixels: &[u8], x: i32, y: i32) -> [u8; 4] {
    let inicio = ((y * plano.largura_px + x) * 4) as usize;
    pixels[inicio..inicio + 4].try_into().unwrap()
}

fn original(numero: usize) -> Pagina {
    Pagina {
        numero: Some(numero),
        girar_180: false,
    }
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn pagina_retrato_sai_no_tamanho_certo_sobre_branco() {
    let (plano, pixels) = desenhar(
        &pdf_com_marca(200, 300, 0, None),
        original(1),
        &fila_a_72_dpi(200, 300),
    );
    assert_eq!((plano.largura_px, plano.altura_px), (200, 300));
    assert_eq!(pixels.len(), 200 * 300 * 4);
    assert_eq!(pixel(&plano, &pixels, 10, 10), PRETO);
    assert_eq!(pixel(&plano, &pixels, 190, 290), BRANCO);
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn giro_de_180_da_passada_leva_a_marca_para_o_canto_oposto() {
    let pagina = Pagina {
        numero: Some(1),
        girar_180: true,
    };
    let (plano, pixels) = desenhar(
        &pdf_com_marca(200, 300, 0, None),
        pagina,
        &fila_a_72_dpi(200, 300),
    );
    assert_eq!(pixel(&plano, &pixels, 190, 290), PRETO);
    assert_eq!(pixel(&plano, &pixels, 10, 10), BRANCO);
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn rotacao_de_180_da_propria_pagina_e_respeitada() {
    let (plano, pixels) = desenhar(
        &pdf_com_marca(200, 300, 180, None),
        original(1),
        &fila_a_72_dpi(200, 300),
    );
    assert!(!plano.girar_90);
    assert_eq!(pixel(&plano, &pixels, 190, 290), PRETO);
    assert_eq!(pixel(&plano, &pixels, 10, 10), BRANCO);
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn rotacao_de_90_faz_a_pagina_em_pe_ser_tratada_como_deitada() {
    // Com /Rotate 90, a marca vai para o canto superior direito da página, agora de 300 × 200.
    // O giro de 270° para caber no papel em pé a leva para o canto superior esquerdo do papel.
    let (plano, pixels) = desenhar(
        &pdf_com_marca(200, 300, 90, None),
        original(1),
        &fila_a_72_dpi(200, 300),
    );
    assert!(plano.girar_90);
    assert_eq!((plano.largura_px, plano.altura_px), (200, 300));
    assert_eq!(pixel(&plano, &pixels, 10, 10), PRETO);
    assert_eq!(pixel(&plano, &pixels, 190, 290), BRANCO);
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn pagina_deitada_gira_para_o_papel_em_pe_com_o_topo_a_esquerda() {
    let (plano, pixels) = desenhar(
        &pdf_com_marca(300, 200, 0, None),
        original(1),
        &fila_a_72_dpi(200, 300),
    );
    assert!(plano.girar_90);
    assert_eq!((plano.largura_px, plano.altura_px), (200, 300));
    // O canto superior esquerdo da página deitada fica embaixo, à esquerda, no papel.
    assert_eq!(pixel(&plano, &pixels, 10, 290), PRETO);
    assert_eq!(pixel(&plano, &pixels, 10, 10), BRANCO);
    assert_eq!(pixel(&plano, &pixels, 190, 10), BRANCO);
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn pagina_em_branco_sai_quase_toda_branca_com_o_ponto_discreto() {
    let pagina = Pagina {
        numero: None,
        girar_180: false,
    };
    let (plano, pixels) = desenhar(
        &pdf_com_marca(200, 300, 0, None),
        pagina,
        &fila_a_72_dpi(200, 300),
    );
    assert_eq!(pixel(&plano, &pixels, 10, 10), BRANCO);
    assert_eq!(pixel(&plano, &pixels, 100, 150), BRANCO);
    let marcados = pixels
        .chunks_exact(4)
        .filter(|cor| cor[..3] != BRANCO[..3])
        .count();
    assert!((1..=16).contains(&marcados), "{marcados} pixels marcados");
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn unidade_da_pagina_entra_no_tamanho() {
    let (plano, _) = desenhar(
        &pdf_com_marca(100, 150, 0, Some(2.0)),
        original(1),
        &fila_a_72_dpi(200, 300),
    );
    assert_eq!((plano.largura_px, plano.altura_px), (200, 300));
}

#[test]
#[ignore = "envia à fila de PDF do Windows"]
fn frente_vai_para_o_arquivo_na_fila_de_pdf_e_o_trabalho_termina() {
    let Some(fila) = crate::listar_filas()
        .unwrap()
        .into_iter()
        .find(|fila| fila.nome == FILA_DE_PDF)
    else {
        eprintln!("fila de PDF ausente: nada foi enviado");
        return;
    };
    assert_eq!(fila.nome, FILA_DE_PDF);
    let pasta = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target")
        .join(format!("teste-envio-{}", std::process::id()));
    std::fs::create_dir_all(&pasta).unwrap();
    let pdf = plots_pdf::pdf_calibracao(plots_pdf::PapelCalibracao::A4).unwrap();
    let origem = crate::abrir_origem(&pdf).unwrap();
    let dimensoes = origem.dimensoes_paginas().unwrap();
    let mut backend = BackendWindows::novo(&fila, origem, dimensoes, Some(&pasta)).unwrap();
    backend
        .enviar(Passada::Frente, &[original(1), original(3)])
        .unwrap();
    let mut leituras = Vec::new();
    for _ in 0..600 {
        let estado = backend.consultar().unwrap();
        let leitura = (backend.ultimo_status(), estado);
        if leituras.last() != Some(&leitura) {
            eprintln!("estado do trabalho: {leitura:?}");
            leituras.push(leitura);
        }
        if estado != EstadoEnvio::EmAndamento {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let ultima = leituras.last().unwrap();
    assert_eq!(ultima.1, EstadoEnvio::Concluido);
    let arquivo = pasta.join("lote-0001-frente.pdf");
    assert!(std::fs::metadata(&arquivo).unwrap().len() > 0);
    assert_eq!(
        Impressora::abrir(FILA_DE_PDF)
            .unwrap()
            .trabalhos_na_fila()
            .unwrap(),
        0
    );
    std::fs::remove_dir_all(&pasta).unwrap();
}

#[test]
#[ignore = "lê a fila de PDF do Windows, sem enviar"]
fn fila_de_pdf_nao_tem_trabalho_sobrando() {
    let existe = crate::listar_filas()
        .unwrap()
        .iter()
        .any(|fila| fila.nome == FILA_DE_PDF);
    if !existe {
        eprintln!("fila de PDF ausente");
        return;
    }
    let trabalhos = Impressora::abrir(FILA_DE_PDF)
        .unwrap()
        .trabalhos_na_fila()
        .unwrap();
    eprintln!("trabalhos na fila de PDF: {trabalhos}");
    assert_eq!(trabalhos, 0);
}
