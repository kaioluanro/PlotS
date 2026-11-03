//! Documentos de teste, gerados aqui mesmo e sem fonte: a página tem um número grande, uma seta para
//! cima e a marca que o leitor acha no desenho. Tudo é retângulo e polígono, para sobreviver à
//! rasterização.

use plotsim_core::marca::{matriz_da_marca, Matriz, MAIOR_PAGINA, MODULOS};

/// Formatos da matriz, em pontos.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Formato {
    A4,
    A5,
    Carta,
    Oficio,
}

impl Formato {
    pub const fn pontos(self) -> (f64, f64) {
        match self {
            Self::A4 => (595.276, 841.89),
            Self::A5 => (419.528, 595.276),
            Self::Carta => (612.0, 792.0),
            Self::Oficio => (612.283, 935.433),
        }
    }

    /// Medidas em milímetros.
    pub fn milimetros(self) -> (f64, f64) {
        let (l, a) = self.pontos();
        (l * 25.4 / 72.0, a * 25.4 / 72.0)
    }

    pub const fn nome(self) -> &'static str {
        match self {
            Self::A4 => "A4",
            Self::A5 => "A5",
            Self::Carta => "Carta",
            Self::Oficio => "Ofício",
        }
    }
}

/// Como a página está no PDF.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GiroNoPdf {
    Nenhum,
    /// A página declara 180° de rotação.
    Declarado180,
    /// A página declara 90° de rotação. Fica deitada no documento.
    Declarado90,
    /// A folha é em pé, e o desenho vem deitado no conteúdo.
    ConteudoDeitado,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaginaDeTeste {
    pub numero: u32,
    pub formato: Formato,
    pub giro: GiroNoPdf,
}

fn retangulo(saida: &mut String, x: f64, y: f64, largura: f64, altura: f64) {
    saida.push_str(&format!("{x:.2} {y:.2} {largura:.2} {altura:.2} re f\n"));
}

/// A marca, com a origem no canto de baixo e à esquerda.
fn desenhar_marca(saida: &mut String, matriz: &Matriz, x: f64, y: f64, lado: f64) {
    let modulo = lado / MODULOS as f64;
    for (l, linha) in matriz.iter().enumerate() {
        for (c, preto) in linha.iter().enumerate() {
            if *preto {
                // Um pouco maior que o módulo, para não sobrar fresta entre vizinhos no desenho.
                retangulo(
                    saida,
                    x + c as f64 * modulo,
                    y + (MODULOS - 1 - l) as f64 * modulo,
                    modulo + 0.05,
                    modulo + 0.05,
                );
            }
        }
    }
}

/// Segmentos de cada dígito: topo, cima à direita, baixo à direita, base, baixo à esquerda, cima à
/// esquerda e meio.
const SEGMENTOS: [[bool; 7]; 10] = [
    [true, true, true, true, true, true, false],
    [false, true, true, false, false, false, false],
    [true, true, false, true, true, false, true],
    [true, true, true, true, false, false, true],
    [false, true, true, false, false, true, true],
    [true, false, true, true, false, true, true],
    [true, false, true, true, true, true, true],
    [true, true, true, false, false, false, false],
    [true, true, true, true, true, true, true],
    [true, true, true, true, false, true, true],
];

fn desenhar_digito(saida: &mut String, digito: usize, x: f64, y: f64, l: f64, a: f64) {
    let Some(ligados) = SEGMENTOS.get(digito) else {
        return;
    };
    let t = l / 6.0;
    let meio = a / 2.0;
    let pecas = [
        (x, y + a - t, l, t),
        (x + l - t, y + meio, t, meio),
        (x + l - t, y, t, meio),
        (x, y, l, t),
        (x, y, t, meio),
        (x, y + meio, t, meio),
        (x, y + meio - t / 2.0, l, t),
    ];
    for (peca, ligado) in pecas.iter().zip(ligados) {
        if *ligado {
            retangulo(saida, peca.0, peca.1, peca.2, peca.3);
        }
    }
}

/// O número no meio da área dada.
fn desenhar_numero(saida: &mut String, numero: u32, largura: f64, base: f64, altura: f64) {
    let texto = numero.to_string();
    let quantos = texto.len() as f64;
    let digito_l = (largura * 0.7 / quantos).min(altura * 0.55);
    let folga = digito_l * 0.25;
    let total = quantos * digito_l + (quantos - 1.0) * folga;
    let mut x = (largura - total) / 2.0;
    for algarismo in texto.bytes() {
        let digito = usize::from(algarismo.saturating_sub(b'0'));
        desenhar_digito(saida, digito, x, base, digito_l, altura);
        x += digito_l + folga;
    }
}

/// Seta para cima, no alto da página.
fn desenhar_seta(saida: &mut String, largura: f64, altura: f64) {
    let centro = largura / 2.0;
    let topo = altura * 0.95;
    let lado = altura * 0.04;
    saida.push_str(&format!(
        "{:.2} {:.2} m {:.2} {:.2} l {:.2} {:.2} l h f\n",
        centro,
        topo,
        centro - lado,
        topo - lado,
        centro + lado,
        topo - lado,
    ));
    retangulo(
        saida,
        centro - lado * 0.3,
        topo - lado * 2.2,
        lado * 0.6,
        lado * 1.3,
    );
}

/// O conteúdo de uma página de `largura` por `altura`, em pé.
fn conteudo_em_pe(numero: u32, largura: f64, altura: f64) -> Option<String> {
    let matriz = matriz_da_marca(numero)?;
    let mut saida = String::from("0 0 0 rg\n");
    desenhar_seta(&mut saida, largura, altura);
    desenhar_numero(&mut saida, numero, largura, altura * 0.40, altura * 0.40);
    let lado = largura.min(altura) * 0.22;
    desenhar_marca(
        &mut saida,
        &matriz,
        (largura - lado) / 2.0,
        altura * 0.08,
        lado,
    );
    Some(saida)
}

fn conteudo(pagina: &PaginaDeTeste) -> Option<String> {
    let (largura, altura) = pagina.formato.pontos();
    match pagina.giro {
        GiroNoPdf::ConteudoDeitado => {
            let dentro = conteudo_em_pe(pagina.numero, altura, largura)?;
            // Gira 90° no sentido horário: o desenho fica deitado, ocupando a folha em pé.
            Some(format!("q 0 -1 1 0 0 {altura:.2} cm\n{dentro}Q\n"))
        }
        GiroNoPdf::Nenhum | GiroNoPdf::Declarado180 | GiroNoPdf::Declarado90 => {
            conteudo_em_pe(pagina.numero, largura, altura)
        }
    }
}

/// Escreve um PDF com as páginas e conteúdos dados: `None` é página em branco.
fn escrever_pdf(paginas: &[(Formato, GiroNoPdf, Option<String>)]) -> Vec<u8> {
    let mut objetos: Vec<Vec<u8>> = Vec::with_capacity(paginas.len() * 2 + 2);
    let filhos: Vec<String> = (0..paginas.len())
        .map(|i| format!("{} 0 R", 3 + i * 2))
        .collect();
    objetos.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objetos.push(
        format!(
            "<< /Type /Pages /Count {} /Kids [{}] >>",
            paginas.len(),
            filhos.join(" ")
        )
        .into_bytes(),
    );
    for (indice, (formato, giro, texto)) in paginas.iter().enumerate() {
        let (largura, altura) = formato.pontos();
        let rotacao = match giro {
            GiroNoPdf::Declarado180 => " /Rotate 180",
            GiroNoPdf::Declarado90 => " /Rotate 90",
            GiroNoPdf::Nenhum | GiroNoPdf::ConteudoDeitado => "",
        };
        objetos.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {largura:.3} {altura:.3}]{rotacao} /Resources << >> /Contents {} 0 R >>",
                4 + indice * 2
            )
            .into_bytes(),
        );
        let corpo = texto.as_deref().unwrap_or("");
        let mut fluxo = format!("<< /Length {} >>\nstream\n", corpo.len()).into_bytes();
        fluxo.extend_from_slice(corpo.as_bytes());
        fluxo.extend_from_slice(b"\nendstream");
        objetos.push(fluxo);
    }
    let mut saida = b"%PDF-1.5\n".to_vec();
    let mut posicoes = Vec::with_capacity(objetos.len());
    for (indice, objeto) in objetos.iter().enumerate() {
        posicoes.push(saida.len());
        saida.extend_from_slice(format!("{} 0 obj\n", indice + 1).as_bytes());
        saida.extend_from_slice(objeto);
        saida.extend_from_slice(b"\nendobj\n");
    }
    let inicio_tabela = saida.len();
    saida.extend_from_slice(format!("xref\n0 {}\n", objetos.len() + 1).as_bytes());
    saida.extend_from_slice(b"0000000000 65535 f \n");
    for posicao in &posicoes {
        saida.extend_from_slice(format!("{posicao:010} 00000 n \n").as_bytes());
    }
    saida.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{inicio_tabela}\n%%EOF\n",
            objetos.len() + 1
        )
        .as_bytes(),
    );
    saida
}

/// O documento de teste. Falha com número que não cabe na marca.
pub fn pdf_de_teste(paginas: &[PaginaDeTeste]) -> Result<Vec<u8>, String> {
    if paginas.is_empty() {
        return Err("o documento de teste precisa de ao menos uma página".to_string());
    }
    let mut itens = Vec::with_capacity(paginas.len());
    for pagina in paginas {
        let texto = conteudo(pagina).ok_or_else(|| {
            format!(
                "a página {} não cabe na marca; o limite é {MAIOR_PAGINA}",
                pagina.numero
            )
        })?;
        itens.push((pagina.formato, pagina.giro, Some(texto)));
    }
    Ok(escrever_pdf(&itens))
}

/// Um PDF só com páginas em branco, para os testes do leitor.
pub fn pdf_em_branco(paginas: usize, formato: Formato) -> Vec<u8> {
    let itens: Vec<_> = (0..paginas)
        .map(|_| (formato, GiroNoPdf::Nenhum, None))
        .collect();
    escrever_pdf(&itens)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pagina(numero: u32) -> PaginaDeTeste {
        PaginaDeTeste {
            numero,
            formato: Formato::A4,
            giro: GiroNoPdf::Nenhum,
        }
    }

    #[test]
    fn a_tabela_de_referencias_bate_com_as_posicoes() {
        let pdf = pdf_de_teste(&[pagina(1), pagina(2), pagina(101)]).unwrap();
        let texto = String::from_utf8_lossy(&pdf).into_owned();
        assert!(texto.starts_with("%PDF-1.5"));
        assert!(texto.ends_with("%%EOF\n"));
        let inicio: usize = texto
            .rsplit("startxref\n")
            .next()
            .and_then(|fim| fim.lines().next())
            .and_then(|linha| linha.parse().ok())
            .unwrap();
        assert!(texto[inicio..].starts_with("xref\n0 9\n"));
        let tabela: Vec<&str> = texto[inicio..].lines().skip(3).take(8).collect();
        for (indice, linha) in tabela.iter().enumerate() {
            let posicao: usize = linha[..10].parse().unwrap();
            assert!(
                texto[posicao..].starts_with(&format!("{} 0 obj", indice + 1)),
                "objeto {}",
                indice + 1
            );
        }
    }

    #[test]
    fn a_rotacao_declarada_vai_para_a_pagina() {
        let com = pdf_de_teste(&[PaginaDeTeste {
            giro: GiroNoPdf::Declarado180,
            ..pagina(1)
        }])
        .unwrap();
        assert!(String::from_utf8_lossy(&com).contains("/Rotate 180"));
        let sem = pdf_de_teste(&[pagina(1)]).unwrap();
        assert!(!String::from_utf8_lossy(&sem).contains("/Rotate"));
    }

    #[test]
    fn numero_que_nao_cabe_e_erro() {
        assert!(pdf_de_teste(&[pagina(0)]).is_err());
        assert!(pdf_de_teste(&[pagina(MAIOR_PAGINA + 1)]).is_err());
        assert!(pdf_de_teste(&[]).is_err());
    }
}
