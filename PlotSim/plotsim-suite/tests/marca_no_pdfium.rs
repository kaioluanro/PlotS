//! A marca pelo caminho de verdade: PDF gerado, desenhado pelo PDFium e lido do bitmap.
//! Roda com `--ignored` e `PLOTS_PDFIUM_DIR`.
#![cfg(windows)]

use plotsim_core::leitor::ler_marcas_de_bytes;
use plotsim_core::marca::{ErroMarca, MarcaLida};
use plotsim_suite::documento::{pdf_de_teste, pdf_em_branco, Formato, GiroNoPdf, PaginaDeTeste};

fn pagina(numero: u32, formato: Formato, giro: GiroNoPdf) -> PaginaDeTeste {
    PaginaDeTeste {
        numero,
        formato,
        giro,
    }
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn cada_formato_e_numero_e_lido_do_desenho() {
    let mut paginas = Vec::new();
    for formato in [Formato::A4, Formato::A5, Formato::Carta, Formato::Oficio] {
        for numero in [1, 2, 9, 10, 11, 24, 101, 1000] {
            paginas.push(pagina(numero, formato, GiroNoPdf::Nenhum));
        }
    }
    let pdf = pdf_de_teste(&paginas).unwrap();
    let lidas = ler_marcas_de_bytes(&pdf).unwrap();
    assert_eq!(lidas.len(), paginas.len());
    for (lida, enviada) in lidas.iter().zip(&paginas) {
        assert_eq!(
            lida.leitura,
            Ok(MarcaLida {
                pagina: enviada.numero,
                rotacao: 0
            }),
            "{enviada:?}"
        );
        let (largura, altura) = enviada.formato.milimetros();
        assert!((lida.largura_mm - largura).abs() < 0.5, "{lida:?}");
        assert!((lida.altura_mm - altura).abs() < 0.5, "{lida:?}");
    }
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn pagina_girada_diz_a_rotacao() {
    let pdf = pdf_de_teste(&[
        pagina(5, Formato::A4, GiroNoPdf::Nenhum),
        pagina(6, Formato::A4, GiroNoPdf::Declarado180),
        pagina(7, Formato::A4, GiroNoPdf::Declarado90),
        pagina(8, Formato::A4, GiroNoPdf::ConteudoDeitado),
        pagina(9, Formato::A5, GiroNoPdf::ConteudoDeitado),
    ])
    .unwrap();
    let rotacoes: Vec<_> = ler_marcas_de_bytes(&pdf)
        .unwrap()
        .into_iter()
        .map(|lida| lida.leitura.unwrap())
        .map(|marca| (marca.pagina, marca.rotacao))
        .collect();
    assert_eq!(rotacoes, vec![(5, 0), (6, 180), (7, 90), (8, 90), (9, 90)]);
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn pdf_em_branco_nao_vira_palpite() {
    let lidas = ler_marcas_de_bytes(&pdf_em_branco(2, Formato::A4)).unwrap();
    assert_eq!(lidas.len(), 2);
    for lida in lidas {
        assert_eq!(lida.leitura, Err(ErroMarca::PaginaEmBranco));
    }
}

/// A página cabe em uma folha muito maior: o mesmo que um driver que reduz a imagem.
#[test]
#[ignore = "precisa da pdfium.dll"]
fn a_marca_sobrevive_a_escala_no_desenho() {
    let original = pdf_de_teste(&[pagina(42, Formato::A5, GiroNoPdf::Nenhum)]).unwrap();
    let texto = String::from_utf8_lossy(&original).into_owned();
    // A mesma página, em uma caixa dez vezes maior: a marca fica com um décimo do tamanho relativo.
    let reduzida = texto.replace(
        "/MediaBox [0 0 419.528 595.276]",
        "/MediaBox [0 0 4195.28 5952.76]",
    );
    assert_ne!(texto, reduzida);
    let lidas = ler_marcas_de_bytes(reduzida.as_bytes());
    match lidas {
        Ok(lidas) => assert_eq!(
            lidas[0].leitura,
            Ok(MarcaLida {
                pagina: 42,
                rotacao: 0
            })
        ),
        Err(erro) => panic!("a estrutura mudou de tamanho e a barreira recusou: {erro}"),
    }
}
