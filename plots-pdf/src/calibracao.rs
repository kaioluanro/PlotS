//! PDF de calibração: quatro páginas numeradas de 1 a 4, cada uma com o número
//! grande no centro e uma seta para cima acima dele. Tudo é desenhado com
//! caminhos vetoriais, sem fonte, para que a rotação e a ordem das páginas
//! apareçam sem depender de nenhuma tipografia instalada.

use lopdf::{dictionary, Document, Object, Stream};

use crate::ErroPdf;

/// Papéis aceitos pelo PDF de calibração.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PapelCalibracao {
    A4,
    A5,
    Carta,
    Oficio,
    Legal,
}

impl PapelCalibracao {
    /// Largura e altura da caixa de página, em pontos (1/72 de polegada).
    pub const fn dimensoes_pt(self) -> (u32, u32) {
        match self {
            Self::A4 => (595, 842),
            Self::A5 => (420, 595),
            Self::Carta => (612, 792),
            Self::Oficio => (612, 936),
            Self::Legal => (612, 1008),
        }
    }
}

/// Número de páginas do PDF de calibração.
pub const PAGINAS_CALIBRACAO: usize = 4;

/// Traços de cada dígito em um quadro unitário (x e y entre 0 e 1, y para cima).
/// Cada traço é uma linha poligonal desenhada com a espessura do dígito.
fn tracos(numero: usize) -> &'static [&'static [(f32, f32)]] {
    match numero {
        1 => &[&[(0.5, 0.0), (0.5, 1.0), (0.12, 0.8)]],
        2 => &[&[
            (0.0, 0.8),
            (0.2, 1.0),
            (0.8, 1.0),
            (1.0, 0.8),
            (1.0, 0.6),
            (0.0, 0.0),
            (1.0, 0.0),
        ]],
        3 => &[
            &[
                (0.0, 0.85),
                (0.2, 1.0),
                (0.8, 1.0),
                (1.0, 0.8),
                (1.0, 0.65),
                (0.8, 0.5),
                (0.3, 0.5),
            ],
            &[
                (0.8, 0.5),
                (1.0, 0.35),
                (1.0, 0.2),
                (0.8, 0.0),
                (0.2, 0.0),
                (0.0, 0.15),
            ],
        ],
        4 => &[&[(0.75, 0.0), (0.75, 1.0), (0.0, 0.3), (1.0, 0.3)]],
        _ => &[],
    }
}

/// Conteúdo de uma página: o dígito centralizado, com altura de 40% da página,
/// e uma seta para cima logo acima dele.
fn conteudo_da_pagina(numero: usize, largura: f32, altura: f32) -> String {
    let mut conteudo = String::new();
    conteudo.push_str("1 J 1 j\n");
    let alto = altura * 0.4;
    let largo = alto * 0.6;
    let x0 = largura / 2.0 - largo / 2.0;
    let y0 = altura / 2.0 - alto / 2.0;
    conteudo.push_str(&format!("{:.3} w\n", alto * 0.08));
    for traco in tracos(numero) {
        for (indice, &(u, v)) in traco.iter().enumerate() {
            let operador = if indice == 0 { "m" } else { "l" };
            conteudo.push_str(&format!(
                "{:.2} {:.2} {operador}\n",
                x0 + u * largo,
                y0 + v * alto
            ));
        }
        conteudo.push_str("S\n");
    }
    let cx = largura / 2.0;
    let base = altura * 0.74;
    let ponta = altura * 0.90;
    let cabeca = altura * 0.045;
    let meia_cabeca = altura * 0.035;
    conteudo.push_str(&format!("{:.3} w\n", altura * 0.018));
    conteudo.push_str(&format!("{cx:.2} {base:.2} m\n{cx:.2} {ponta:.2} l\nS\n"));
    conteudo.push_str(&format!(
        "{:.2} {:.2} m\n{cx:.2} {ponta:.2} l\n{:.2} {:.2} l\nS\n",
        cx - meia_cabeca,
        ponta - cabeca,
        cx + meia_cabeca,
        ponta - cabeca
    ));
    conteudo
}

/// Gera o PDF de calibração com quatro páginas no papel pedido. A função não
/// grava arquivos nem acessa impressoras.
pub fn pdf_calibracao(papel: PapelCalibracao) -> Result<Vec<u8>, ErroPdf> {
    let (largura, altura) = papel.dimensoes_pt();
    let mut documento = Document::with_version("1.5");
    let raiz = documento.new_object_id();
    let mut paginas = Vec::with_capacity(PAGINAS_CALIBRACAO);
    for numero in 1..=PAGINAS_CALIBRACAO {
        let conteudo = conteudo_da_pagina(numero, largura as f32, altura as f32);
        let fluxo = documento.add_object(Stream::new(dictionary! {}, conteudo.into_bytes()));
        let pagina = documento.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => raiz,
            "MediaBox" => vec![0.into(), 0.into(), (largura as i64).into(), (altura as i64).into()],
            "Resources" => dictionary! {},
            "Contents" => fluxo,
        });
        paginas.push(Object::Reference(pagina));
    }
    documento.objects.insert(
        raiz,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => PAGINAS_CALIBRACAO as i64,
            "Kids" => paginas,
        }),
    );
    let catalogo = documento.add_object(dictionary! {"Type" => "Catalog", "Pages" => raiz});
    documento.trailer.set("Root", catalogo);
    let mut bytes = Vec::new();
    documento.save_to(&mut bytes).map_err(ErroPdf::Escrita)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caixa(doc: &Document, pagina: lopdf::ObjectId) -> Vec<f32> {
        doc.get_object(pagina)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_float().unwrap())
            .collect()
    }

    #[test]
    fn quatro_paginas_no_papel_pedido() {
        for (papel, largura, altura) in [
            (PapelCalibracao::A4, 595.0, 842.0),
            (PapelCalibracao::A5, 420.0, 595.0),
            (PapelCalibracao::Carta, 612.0, 792.0),
            (PapelCalibracao::Oficio, 612.0, 936.0),
            (PapelCalibracao::Legal, 612.0, 1008.0),
        ] {
            let bytes = pdf_calibracao(papel).unwrap();
            let doc = Document::load_mem(&bytes).unwrap();
            let paginas: Vec<_> = doc.get_pages().into_values().collect();
            assert_eq!(paginas.len(), 4, "{papel:?}");
            for pagina in paginas {
                assert_eq!(caixa(&doc, pagina), vec![0.0, 0.0, largura, altura]);
            }
        }
    }

    #[test]
    fn paginas_sao_diferentes_e_nao_usam_fonte() {
        let bytes = pdf_calibracao(PapelCalibracao::A4).unwrap();
        let doc = Document::load_mem(&bytes).unwrap();
        let paginas: Vec<_> = doc.get_pages().into_values().collect();
        let conteudos: Vec<Vec<u8>> = paginas
            .iter()
            .map(|pagina| doc.get_page_content(*pagina))
            .collect();
        for i in 0..conteudos.len() {
            for j in (i + 1)..conteudos.len() {
                assert_ne!(conteudos[i], conteudos[j]);
            }
        }
        for pagina in &paginas {
            let dicionario = doc.get_object(*pagina).unwrap().as_dict().unwrap();
            let recursos = dicionario.get(b"Resources").unwrap().as_dict().unwrap();
            assert!(!recursos.has(b"Font"));
            let operadores = lopdf::content::Content::decode(&doc.get_page_content(*pagina))
                .unwrap()
                .operations;
            assert!(operadores
                .iter()
                .all(|op| !matches!(op.operator.as_str(), "Tf" | "Tj" | "TJ" | "BT")));
        }
    }

    #[test]
    fn numero_ocupa_quarenta_por_cento_e_seta_fica_acima() {
        let bytes = pdf_calibracao(PapelCalibracao::A4).unwrap();
        let doc = Document::load_mem(&bytes).unwrap();
        let paginas: Vec<_> = doc.get_pages().into_values().collect();
        for pagina in paginas {
            let altura = 842.0_f32;
            let operadores = lopdf::content::Content::decode(&doc.get_page_content(pagina))
                .unwrap()
                .operations;
            let mut ys_do_numero = Vec::new();
            let mut ys_da_seta = Vec::new();
            for op in &operadores {
                if !matches!(op.operator.as_str(), "m" | "l") {
                    continue;
                }
                let y = op.operands[1].as_float().unwrap();
                if y < altura * 0.72 {
                    ys_do_numero.push(y);
                } else {
                    ys_da_seta.push(y);
                }
            }
            let minimo = ys_do_numero.iter().copied().fold(f32::MAX, f32::min);
            let maximo = ys_do_numero.iter().copied().fold(f32::MIN, f32::max);
            assert!((minimo - altura * 0.3).abs() < 0.5, "base {minimo}");
            assert!((maximo - altura * 0.7).abs() < 0.5, "topo {maximo}");
            assert!(((maximo - minimo) / altura - 0.4).abs() < 0.01);
            assert!(!ys_da_seta.is_empty());
            assert!(ys_da_seta.iter().all(|y| *y > altura * 0.72 && *y < altura));
        }
    }
}
