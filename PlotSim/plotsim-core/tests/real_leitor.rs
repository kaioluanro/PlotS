//! Leitura de PDF com o PDFium de verdade. Pedem a DLL, então só rodam à mão, com `--ignored` e
//! `PLOTS_PDFIUM_DIR`. Não tocam em filas.
#![cfg(windows)]

mod apoio;

use apoio::{mm, pdf_com, pdf_com_formatos, A4, A5, CARTA};
use lopdf::{dictionary, Document};
use plotsim_core::leitor::{inspecionar_bytes, ler_bytes, ErroLeitura};

fn perto(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.1
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn le_paginas_tamanho_e_texto() {
    let paginas = ler_bytes(&pdf_com_formatos(&[A4, A5, CARTA])).unwrap();
    assert_eq!(paginas.len(), 3);
    for (indice, (pagina, formato)) in paginas.iter().zip([A4, A5, CARTA]).enumerate() {
        assert_eq!(pagina.numero, indice + 1);
        assert!(perto(pagina.largura_mm, mm(formato.0)), "{pagina:?}");
        assert!(perto(pagina.altura_mm, mm(formato.1)), "{pagina:?}");
        assert_eq!(pagina.rotacao, 0);
        assert!(
            pagina.texto.contains(&format!("Pagina {}", indice + 1)),
            "{pagina:?}"
        );
    }
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn rotacao_da_pagina_vem_em_graus_e_o_tamanho_ja_girado() {
    for (graus, deitada) in [(0, false), (90, true), (180, false), (270, true)] {
        let bytes = pdf_com(&[A4], graus, |_| "Girada".to_string());
        let paginas = ler_bytes(&bytes).unwrap();
        assert_eq!(i64::from(paginas[0].rotacao), graus);
        let (largura, altura) = if deitada { (A4.1, A4.0) } else { A4 };
        assert!(perto(paginas[0].largura_mm, mm(largura)), "{paginas:?}");
        assert!(perto(paginas[0].altura_mm, mm(altura)), "{paginas:?}");
    }
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn pagina_sem_texto_e_erro_claro_e_inspecionar_ainda_mede() {
    let bytes = pdf_com(&[A4, A4, A4], 0, |numero| {
        if numero == 2 {
            String::new()
        } else {
            format!("Pagina {numero}")
        }
    });
    let erro = ler_bytes(&bytes).unwrap_err();
    assert!(matches!(erro, ErroLeitura::PaginaSemTexto { pagina: 2 }));
    assert!(erro.to_string().contains("página 2"), "{erro}");
    let paginas = inspecionar_bytes(&bytes).unwrap();
    assert_eq!(paginas.len(), 3);
    assert_eq!(paginas[1].texto.trim(), "");
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn pdf_hostil_vira_erro_sem_travar() {
    let bom = pdf_com_formatos(&[A4]);
    let mut casos: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"nao e pdf".to_vec(),
        bom[..bom.len() / 2].to_vec(),
        bom.iter().rev().copied().collect(),
    ];
    // Bytes trocados pelo arquivo inteiro: pode abrir ou não, mas nunca derrubar o programa.
    for posicao in (0..bom.len()).step_by(37) {
        let mut estragado = bom.clone();
        estragado[posicao] ^= 0xff;
        casos.push(estragado);
    }
    for caso in &casos {
        let _ = inspecionar_bytes(caso);
    }
    assert!(inspecionar_bytes(&casos[0]).is_err());
    assert!(inspecionar_bytes(&casos[1]).is_err());
}

#[test]
#[ignore = "precisa da pdfium.dll"]
fn pdf_criptografado_e_recusado() {
    let mut doc = Document::load_mem(&pdf_com_formatos(&[A4])).unwrap();
    let encrypt = doc.add_object(dictionary! { "Filter" => "Standard", "V" => 1 });
    doc.trailer.set("Encrypt", encrypt);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    assert!(inspecionar_bytes(&bytes).is_err());
}
