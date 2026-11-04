//! Gera os PDFs de uso real com a mesma função que a suíte usa: cada página tem a marca que a janela
//! lê. Fica fora da execução comum; sem a pasta indicada, grava numa pasta temporária.

use plotsim_suite::documento::{pdf_de_teste, Formato, GiroNoPdf, PaginaDeTeste};
use std::path::PathBuf;

fn paginas(quantas: u32, girada: Option<u32>) -> Vec<PaginaDeTeste> {
    (1..=quantas)
        .map(|numero| PaginaDeTeste {
            numero,
            formato: Formato::A4,
            giro: if Some(numero) == girada {
                GiroNoPdf::Declarado180
            } else {
                GiroNoPdf::Nenhum
            },
        })
        .collect()
}

#[test]
#[ignore = "gera PDFs de uso real; PLOTSIM_DOCUMENTOS indica a pasta de destino"]
fn gera_documentos_de_uso_real() {
    let pasta = std::env::var_os("PLOTSIM_DOCUMENTOS")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("plotsim-documentos-de-uso"));
    std::fs::create_dir_all(&pasta).unwrap();
    // Total ímpar: o último verso fica em branco, e a janela não pode acusá-lo como sem marca.
    let cinco = pdf_de_teste(&paginas(5, None)).unwrap();
    std::fs::write(pasta.join("cinco.pdf"), cinco).unwrap();
    // Total par, com uma página declarada girada: a marca tem de dizer o giro.
    let seis = pdf_de_teste(&paginas(6, Some(3))).unwrap();
    std::fs::write(pasta.join("seis.pdf"), seis).unwrap();
}
