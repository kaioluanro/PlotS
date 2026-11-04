//! O que muda em cada página de uma passada quando o usuário pede outro papel ou a marca de
//! controle. Tudo acontece no PDF da passada: a fila desenha esse mesmo PDF, então fila e arquivo
//! gerado saem iguais. Sem nenhuma opção, a página não é tocada.

use std::collections::BTreeMap;

use lopdf::{dictionary, Dictionary, Document, Object};

use crate::{caixa_visivel, Afim, ErroPdf};

const PONTOS_POR_MM: f64 = 72.0 / 25.4;
/// Tamanho da marca: legível de perto, sem disputar com o texto da página.
const TAMANHO_DA_MARCA_PT: f64 = 7.0;
/// Fora da margem que impressoras comuns não alcançam, que passa de 4 mm em algumas lasers.
const MARGEM_DA_MARCA_PT: f64 = 16.0;
/// Nomes tentados para a fonte da marca antes de desistir de um documento hostil.
const TENTATIVAS_DE_NOME: usize = 4096;
const CAIXAS_RETIRADAS: [&[u8]; 4] = [b"CropBox", b"TrimBox", b"BleedBox", b"ArtBox"];

/// O que aplicar a uma passada.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OpcoesDaPassada {
    /// Papel de destino em retrato, em milímetros; ausente, cada página fica no próprio tamanho.
    pub papel_mm: Option<(f64, f64)>,
    /// Texto da marca por página e cópia (`Pagina::identidade`); sem número é a face em branco.
    pub marcas: BTreeMap<plots_core::IdentidadeDaPagina, String>,
}

fn invalido(motivo: &'static str) -> ErroPdf {
    ErroPdf::DocumentoInvalido(motivo)
}

fn reais(valores: &[f64]) -> Vec<Object> {
    valores.iter().map(|v| Object::Real(*v as f32)).collect()
}

/// De coordenadas da página para o que o leitor vê, com a rotação dela e a unidade `s`.
fn para_exibicao(rotacao: i64, [x0, y0, x1, y1]: [f64; 4], s: f64) -> Afim {
    match rotacao {
        90 => [0.0, -s, s, 0.0, -s * y0, s * x1],
        180 => [-s, 0.0, 0.0, -s, s * x1, s * y1],
        270 => [0.0, s, -s, 0.0, s * y1, -s * x0],
        _ => [s, 0.0, 0.0, s, -s * x0, -s * y0],
    }
}

fn inverter([a, b, c, d, e, f]: Afim) -> Option<Afim> {
    let determinante = a * d - b * c;
    if !determinante.is_finite() || determinante == 0.0 {
        return None;
    }
    let inversa = [
        d / determinante,
        -b / determinante,
        -c / determinante,
        a / determinante,
        (c * f - d * e) / determinante,
        (b * e - a * f) / determinante,
    ];
    inversa
        .iter()
        .all(|valor| valor.is_finite())
        .then_some(inversa)
}

/// O desenho da página, com o novo começo e fim, na mesma ordem em que estava.
fn envolver_conteudo(
    saida: &mut Document,
    pagina: &mut Dictionary,
    inicio: String,
    fim: Option<String>,
) -> Result<(), ErroPdf> {
    let existentes = match pagina.get(b"Contents") {
        Err(_) => Vec::new(),
        Ok(valor) => match saida.dereference(valor)? {
            (_, Object::Array(itens)) => itens.clone(),
            _ => vec![valor.clone()],
        },
    };
    let abertura = saida.add_object(lopdf::Stream::new(dictionary! {}, inicio.into_bytes()));
    let mut conteudo = Vec::with_capacity(existentes.len().saturating_add(3));
    conteudo.push(Object::Reference(abertura));
    conteudo.extend(existentes);
    let fechamento = saida.add_object(lopdf::Stream::new(
        dictionary! {},
        fim.unwrap_or_default().into_bytes(),
    ));
    conteudo.push(Object::Reference(fechamento));
    pagina.set("Contents", conteudo);
    Ok(())
}

/// Troca a caixa pelo papel de destino e leva o desenho e as anotações para dentro dele, em escala
/// proporcional e centralizados. A rotação da página fica: o desenho só escala e anda, e as
/// anotações continuam casando com a própria aparência.
fn ajustar_ao_papel(
    saida: &mut Document,
    pagina: &mut Dictionary,
    rotacao: i64,
    (menor_mm, maior_mm): (f64, f64),
) -> Result<(), ErroPdf> {
    let [x0, y0, x1, y1] = caixa_visivel(saida, pagina)?;
    let (largura, altura) = (x1 - x0, y1 - y0);
    if !largura.is_finite() || !altura.is_finite() || largura <= 0.0 || altura <= 0.0 {
        return Err(invalido("caixa da página inválida"));
    }
    let (menor, maior) = (menor_mm * PONTOS_POR_MM, maior_mm * PONTOS_POR_MM);
    if !menor.is_finite()
        || !maior.is_finite()
        || menor <= 0.0
        || maior <= 0.0
        || menor > f64::from(f32::MAX)
        || maior > f64::from(f32::MAX)
    {
        return Err(invalido("papel de destino inválido"));
    }
    // Na rotação de 90 ou 270 graus, a largura da caixa é a altura que o leitor vê.
    let rotacao_de_lado = rotacao.rem_euclid(180) == 90;
    let (vista_largura, vista_altura) = if rotacao_de_lado {
        (altura, largura)
    } else {
        (largura, altura)
    };
    let (alvo_largura, alvo_altura) = if vista_largura > vista_altura {
        (maior, menor)
    } else {
        (menor, maior)
    };
    let (caixa_largura, caixa_altura) = if rotacao_de_lado {
        (alvo_altura, alvo_largura)
    } else {
        (alvo_largura, alvo_altura)
    };
    let fator = (caixa_largura / largura).min(caixa_altura / altura);
    if !fator.is_finite() || fator <= 0.0 {
        return Err(invalido("escala da página inválida"));
    }
    let e = (caixa_largura - fator * largura) / 2.0 - fator * x0;
    let f = (caixa_altura - fator * altura) / 2.0 - fator * y0;
    if !e.is_finite() || !f.is_finite() {
        return Err(invalido("escala da página inválida"));
    }
    // O recorte mantém de fora o que estava fora da caixa visível do documento.
    envolver_conteudo(
        saida,
        pagina,
        format!(
            "q {fator:.6} 0 0 {fator:.6} {e:.4} {f:.4} cm {x0:.4} {y0:.4} {largura:.4} {altura:.4} re W n\n"
        ),
        Some("\nQ\n".to_owned()),
    )?;
    for caixa in CAIXAS_RETIRADAS {
        pagina.remove(caixa);
    }
    pagina.remove(b"UserUnit");
    pagina.set("MediaBox", reais(&[0.0, 0.0, caixa_largura, caixa_altura]));
    mover_anotacoes(saida, pagina, fator, e, f)
}

/// A aparência de uma anotação segue o retângulo dela, então basta mover o retângulo.
fn mover_anotacoes(
    saida: &mut Document,
    pagina: &Dictionary,
    fator: f64,
    e: f64,
    f: f64,
) -> Result<(), ErroPdf> {
    let Ok(anotacoes) = pagina.get(b"Annots") else {
        return Ok(());
    };
    let ids: Vec<_> = saida
        .dereference(anotacoes)?
        .1
        .as_array()?
        .iter()
        .filter_map(|item| item.as_reference().ok())
        .collect();
    for id in ids {
        let retangulo = {
            let Ok(Object::Dictionary(anotacao)) = saida.get_object(id) else {
                continue;
            };
            let Ok(valor) = anotacao.get(b"Rect") else {
                continue;
            };
            crate::caixa(saida, valor)?.map(f64::from)
        };
        let [ax0, ay0, ax1, ay1] = retangulo;
        let novo = [
            fator * ax0 + e,
            fator * ay0 + f,
            fator * ax1 + e,
            fator * ay1 + f,
        ];
        if novo.iter().any(|valor| !valor.is_finite()) {
            return Err(invalido("anotação fora de escala"));
        }
        if let Ok(Object::Dictionary(anotacao)) = saida.get_object_mut(id) {
            anotacao.set("Rect", reais(&novo));
        }
    }
    Ok(())
}

/// Os recursos da página com a fonte da marca, sem tocar nos de outras páginas. Devolve o nome dela.
fn recursos_com_a_fonte(saida: &Document, pagina: &mut Dictionary) -> Result<String, ErroPdf> {
    let mut recursos = match pagina.get(b"Resources") {
        Ok(valor) => saida.dereference(valor)?.1.as_dict()?.clone(),
        Err(_) => Dictionary::new(),
    };
    let mut fontes = match recursos.get(b"Font") {
        Ok(valor) => saida.dereference(valor)?.1.as_dict()?.clone(),
        Err(_) => Dictionary::new(),
    };
    let nome = (0..TENTATIVAS_DE_NOME)
        .map(|indice| format!("PlotSControle{indice}"))
        .find(|nome| !fontes.has(nome.as_bytes()))
        .ok_or(invalido("recursos demais na página"))?;
    fontes.set(
        nome.as_bytes(),
        dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
            "Encoding" => "WinAnsiEncoding",
        },
    );
    recursos.set("Font", fontes);
    pagina.set("Resources", recursos);
    Ok(nome)
}

/// A marca é texto curto do próprio programa; o que não existir na codificação da fonte vira `?`.
fn texto_em_hexadecimal(texto: &str) -> String {
    texto
        .chars()
        .map(|letra| match letra {
            '\u{20}'..='\u{7e}' => letra as u8,
            '·' => 0xB7,
            _ => b'?',
        })
        .map(|byte| format!("{byte:02X}"))
        .collect()
}

/// Escreve a marca no canto inferior esquerdo que o leitor vê, de pé, com o mesmo tamanho físico em
/// qualquer rotação ou unidade. Sem fundo: cobre só o que estiver sob as letras.
fn escrever_marca(
    saida: &mut Document,
    pagina: &mut Dictionary,
    rotacao: i64,
    texto: &str,
) -> Result<(), ErroPdf> {
    let caixa = caixa_visivel(saida, pagina)?;
    let unidade = match pagina.get(b"UserUnit") {
        Ok(valor) => f64::from(saida.dereference(valor)?.1.as_float()?),
        Err(_) => 1.0,
    };
    let largura = (caixa[2] - caixa[0]) * unidade;
    let altura = (caixa[3] - caixa[1]) * unidade;
    let (vista_largura, vista_altura) = if rotacao.rem_euclid(180) == 90 {
        (altura, largura)
    } else {
        (largura, altura)
    };
    let menor_lado = vista_largura.min(vista_altura);
    let margem = MARGEM_DA_MARCA_PT.min(menor_lado * 0.06);
    let tamanho = TAMANHO_DA_MARCA_PT.min(menor_lado * 0.03).max(0.5);
    let [a, b, c, d, e, f] = inverter(para_exibicao(rotacao, caixa, unidade))
        .ok_or(invalido("caixa da página inválida"))?;
    let nome = recursos_com_a_fonte(saida, pagina)?;
    let comandos = format!(
        "q {a:.6} {b:.6} {c:.6} {d:.6} {e:.4} {f:.4} cm BT /{nome} {tamanho:.2} Tf 0 g {margem:.2} {margem:.2} Td <{}> Tj ET Q\n",
        texto_em_hexadecimal(texto)
    );
    let marca = saida.add_object(lopdf::Stream::new(dictionary! {}, comandos.into_bytes()));
    let mut conteudo = saida
        .dereference(pagina.get(b"Contents")?)?
        .1
        .as_array()?
        .clone();
    conteudo.push(Object::Reference(marca));
    pagina.set("Contents", conteudo);
    Ok(())
}

/// Aplica as opções a uma página já pronta para a passada.
pub(crate) fn aplicar_a_pagina(
    saida: &mut Document,
    pagina: &mut Dictionary,
    rotacao: i64,
    papel_mm: Option<(f64, f64)>,
    marca: Option<&str>,
) -> Result<(), ErroPdf> {
    if let Some(papel) = papel_mm {
        ajustar_ao_papel(saida, pagina, rotacao, papel)?;
    } else if marca.is_some() {
        // O desenho pode terminar com a matriz alterada: a marca não pode herdar isso.
        envolver_conteudo(saida, pagina, "q\n".to_owned(), Some("\nQ\n".to_owned()))?;
    }
    if let Some(texto) = marca {
        escrever_marca(saida, pagina, rotacao, texto)?;
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::compor;
    use crate::PdfOrigem;
    use plots_core::Pagina;

    /// Uma página por item: largura, altura, rotação e unidade.
    fn pdf(paginas: &[(i64, i64, i64, Option<f32>)]) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let raiz = doc.new_object_id();
        let mut filhos = Vec::new();
        for (largura, altura, rotacao, unidade) in paginas {
            let desenho = doc.add_object(lopdf::Stream::new(
                dictionary! {},
                b"0 g 10 10 20 20 re f 2 0 0 2 0 0 cm".to_vec(),
            ));
            let mut pagina = dictionary! {
                "Type" => "Page", "Parent" => raiz,
                "MediaBox" => vec![0.into(), 0.into(), (*largura).into(), (*altura).into()],
                "Resources" => dictionary! {}, "Contents" => desenho, "Rotate" => *rotacao
            };
            if let Some(unidade) = unidade {
                pagina.set("UserUnit", *unidade);
            }
            filhos.push(Object::Reference(doc.add_object(pagina)));
        }
        let total = i64::try_from(filhos.len()).unwrap();
        doc.objects.insert(
            raiz,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Count" => total, "Kids" => filhos
            }),
        );
        let catalogo = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => raiz});
        doc.trailer.set("Root", catalogo);
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        bytes
    }

    fn original(numero: usize) -> Pagina {
        Pagina {
            numero: Some(numero),
            girar_180: false,
            copia: 0,
        }
    }

    fn pagina_de(bytes: &[u8], indice: usize) -> (Document, Dictionary) {
        let doc = Document::load_mem(bytes).unwrap();
        let id = *doc.get_pages().get(&(indice as u32 + 1)).unwrap();
        let pagina = doc.get_object(id).unwrap().as_dict().unwrap().clone();
        (doc, pagina)
    }

    fn conteudo_junto(doc: &Document, pagina: &Dictionary) -> String {
        let itens = doc
            .dereference(pagina.get(b"Contents").unwrap())
            .unwrap()
            .1
            .as_array()
            .unwrap();
        itens
            .iter()
            .map(|item| {
                let stream = doc.dereference(item).unwrap().1.as_stream().unwrap();
                String::from_utf8_lossy(&stream.content).into_owned()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn caixa_em_pontos(doc: &Document, pagina: &Dictionary) -> [f32; 4] {
        crate::caixa(doc, pagina.get(b"MediaBox").unwrap()).unwrap()
    }

    fn opcoes_de_papel(largura: f64, altura: f64) -> OpcoesDaPassada {
        OpcoesDaPassada {
            papel_mm: Some((largura, altura)),
            ..OpcoesDaPassada::default()
        }
    }

    fn com_marca(numero: Option<usize>, texto: &str) -> OpcoesDaPassada {
        OpcoesDaPassada {
            marcas: BTreeMap::from([((0, numero), texto.to_owned())]),
            ..OpcoesDaPassada::default()
        }
    }

    #[test]
    fn sem_opcao_a_passada_sai_igual_a_de_sempre() {
        let origem =
            PdfOrigem::carregar(&pdf(&[(595, 842, 0, None), (595, 842, 0, None)])).unwrap();
        let paginas = [original(2), original(1)];
        let sem_opcao = origem
            .gerar_passada_com(&paginas, 2, &OpcoesDaPassada::default())
            .unwrap();
        assert_eq!(sem_opcao, origem.gerar_passada(&paginas, 2).unwrap());
    }

    #[test]
    fn a_marca_entra_so_na_pagina_pedida_e_nao_muda_a_ordem_nem_o_tamanho() {
        let origem =
            PdfOrigem::carregar(&pdf(&[(595, 842, 0, None), (595, 842, 0, None)])).unwrap();
        let saida = origem
            .gerar_passada_com(
                &[original(2), original(1)],
                2,
                &com_marca(Some(1), "Lote 2 · folha 3"),
            )
            .unwrap();
        let (doc_a, marcada) = pagina_de(&saida, 1);
        let texto = conteudo_junto(&doc_a, &marcada);
        assert!(
            texto.contains(&texto_em_hexadecimal("Lote 2 · folha 3")),
            "{texto}"
        );
        assert!(
            texto.contains("B7"),
            "o ponto do meio usa a codificação da fonte"
        );
        assert_eq!(caixa_em_pontos(&doc_a, &marcada), [0.0, 0.0, 595.0, 842.0]);
        let (doc_b, sem_marca) = pagina_de(&saida, 0);
        assert!(
            sem_marca.get(b"Contents").unwrap().as_reference().is_ok(),
            "a outra página não é tocada"
        );
        assert_eq!(doc_b.get_pages().len(), 2);
    }

    #[test]
    fn a_marca_nao_herda_a_matriz_que_o_desenho_deixou_alterada() {
        let origem = PdfOrigem::carregar(&pdf(&[(595, 842, 0, None)])).unwrap();
        let saida = origem
            .gerar_passada_com(&[original(1)], 1, &com_marca(Some(1), "Lote 1 · folha 1"))
            .unwrap();
        let (doc, pagina) = pagina_de(&saida, 0);
        let texto = conteudo_junto(&doc, &pagina);
        let fecha = texto.rfind("\nQ\n").unwrap();
        let marca = texto.find(" Tf 0 g ").unwrap();
        assert!(texto.starts_with("q\n"), "{texto}");
        assert!(fecha < marca, "{texto}");
    }

    #[test]
    fn a_marca_tem_o_mesmo_tamanho_fisico_e_fica_de_pe_em_qualquer_rotacao() {
        for rotacao in [0, 90, 180, 270] {
            let origem = PdfOrigem::carregar(&pdf(&[(595, 842, rotacao, Some(2.0))])).unwrap();
            let saida = origem
                .gerar_passada_com(&[original(1)], 1, &com_marca(Some(1), "x"))
                .unwrap();
            let (doc, pagina) = pagina_de(&saida, 0);
            let texto = conteudo_junto(&doc, &pagina);
            let matriz = para_exibicao(rotacao, [0.0, 0.0, 595.0, 842.0], 2.0);
            let inversa = inverter(matriz).unwrap();
            let identidade = compor(inversa, matriz);
            for (valor, esperado) in identidade.iter().zip([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]) {
                assert!((valor - esperado).abs() < 1e-9, "{rotacao}: {identidade:?}");
            }
            let [a, b, c, d, ..] = inversa;
            assert!(
                texto.contains(&format!("q {a:.6} {b:.6} {c:.6} {d:.6} ")),
                "{rotacao}: {texto}"
            );
        }
    }

    #[test]
    fn um_nome_de_fonte_que_ja_existe_nao_e_trocado() {
        let mut doc = Document::with_version("1.5");
        let mut pagina = dictionary! {
            "Resources" => dictionary! {
                "Font" => dictionary! { "PlotSControle0" => dictionary! { "Subtype" => "Type1" } }
            }
        };
        let nome = recursos_com_a_fonte(&doc, &mut pagina).unwrap();
        assert_eq!(nome, "PlotSControle1");
        let recursos = pagina.get(b"Resources").unwrap().as_dict().unwrap();
        let fontes = recursos.get(b"Font").unwrap().as_dict().unwrap();
        assert!(fontes.has(b"PlotSControle0") && fontes.has(b"PlotSControle1"));
        doc.objects.clear();
    }

    #[test]
    fn letra_fora_da_fonte_nao_quebra_o_texto() {
        assert_eq!(texto_em_hexadecimal("a·ç"), "61B73F");
    }

    #[test]
    fn carta_em_a4_fica_centralizada_na_caixa_do_a4() {
        let origem = PdfOrigem::carregar(&pdf(&[(612, 792, 0, None)])).unwrap();
        let saida = origem
            .gerar_passada_com(&[original(1)], 1, &opcoes_de_papel(210.0, 297.0))
            .unwrap();
        let (doc, pagina) = pagina_de(&saida, 0);
        let [_, _, largura, altura] = caixa_em_pontos(&doc, &pagina);
        assert!((largura - 595.28).abs() < 0.01 && (altura - 841.89).abs() < 0.01);
        let texto = conteudo_junto(&doc, &pagina);
        let fator = 595.2756 / 612.0;
        let f = (841.8898 - fator * 792.0) / 2.0;
        assert!(
            texto.starts_with(&format!("q {fator:.6} 0 0 {fator:.6} 0.0000 {f:.4} cm ")),
            "{texto}"
        );
        assert!(
            texto.contains("0.0000 0.0000 612.0000 792.0000 re W n"),
            "{texto}"
        );
    }

    #[test]
    fn paisagem_fica_com_o_papel_deitado_e_a_rotacao_e_respeitada() {
        for (largura, altura, rotacao, caixa_esperada) in [
            (842, 595, 0, (841.89, 595.28)),
            (595, 842, 90, (595.28, 841.89)),
            (842, 595, 90, (841.89, 595.28)),
        ] {
            let origem = PdfOrigem::carregar(&pdf(&[(largura, altura, rotacao, None)])).unwrap();
            let saida = origem
                .gerar_passada_com(&[original(1)], 1, &opcoes_de_papel(210.0, 297.0))
                .unwrap();
            let (doc, pagina) = pagina_de(&saida, 0);
            let [_, _, w, h] = caixa_em_pontos(&doc, &pagina);
            assert!(
                (f64::from(w) - caixa_esperada.0).abs() < 0.01
                    && (f64::from(h) - caixa_esperada.1).abs() < 0.01,
                "{largura}x{altura} a {rotacao}: {w}x{h}"
            );
            assert_eq!(pagina.get(b"Rotate").unwrap().as_i64().unwrap(), rotacao);
        }
    }

    #[test]
    fn a_unidade_da_pagina_some_e_a_medida_fisica_entra_na_conta() {
        let origem = PdfOrigem::carregar(&pdf(&[(100, 100, 0, Some(5.0))])).unwrap();
        let saida = origem
            .gerar_passada_com(&[original(1)], 1, &opcoes_de_papel(210.0, 297.0))
            .unwrap();
        let (doc, pagina) = pagina_de(&saida, 0);
        assert!(!pagina.has(b"UserUnit"));
        let [_, _, largura, _] = caixa_em_pontos(&doc, &pagina);
        assert!((largura - 595.28).abs() < 0.01, "{largura}");
    }

    #[test]
    fn a_face_em_branco_tambem_fica_no_papel_escolhido() {
        let origem = PdfOrigem::carregar(&pdf(&[(420, 595, 0, None)])).unwrap();
        let saida = origem
            .gerar_passada_com(
                &[Pagina {
                    numero: None,
                    girar_180: false,
                    copia: 0,
                }],
                1,
                &opcoes_de_papel(210.0, 297.0),
            )
            .unwrap();
        let (doc, pagina) = pagina_de(&saida, 0);
        let [_, _, largura, altura] = caixa_em_pontos(&doc, &pagina);
        assert!((largura - 595.28).abs() < 0.01 && (altura - 841.89).abs() < 0.01);
    }

    #[test]
    fn o_retangulo_da_anotacao_acompanha_a_escala() {
        let mut doc = Document::new();
        let anotacao = doc.add_object(dictionary! {
            "Rect" => vec![10.into(), 20.into(), 30.into(), 40.into()]
        });
        let pagina = dictionary! { "Annots" => vec![Object::Reference(anotacao)] };
        mover_anotacoes(&mut doc, &pagina, 2.0, 100.0, 200.0).unwrap();
        let lido = doc.get_object(anotacao).unwrap().as_dict().unwrap();
        assert_eq!(
            crate::caixa(&doc, lido.get(b"Rect").unwrap()).unwrap(),
            [120.0, 240.0, 160.0, 280.0]
        );
    }

    #[test]
    fn a_marca_sai_no_canto_do_papel_escolhido_e_nao_dentro_da_escala() {
        let origem = PdfOrigem::carregar(&pdf(&[(420, 595, 0, None)])).unwrap();
        let mut opcoes = opcoes_de_papel(210.0, 297.0);
        opcoes
            .marcas
            .insert((0, Some(1)), "Lote 1 · folha 1".to_owned());
        let saida = origem
            .gerar_passada_com(&[original(1)], 1, &opcoes)
            .unwrap();
        let (doc, pagina) = pagina_de(&saida, 0);
        let [_, _, largura, _] = caixa_em_pontos(&doc, &pagina);
        assert!((largura - 595.28).abs() < 0.01);
        let texto = conteudo_junto(&doc, &pagina);
        let fim_da_escala = texto
            .rfind(
                "
Q
",
            )
            .unwrap();
        let marca = texto.find(" Tf 0 g ").unwrap();
        assert!(fim_da_escala < marca, "{texto}");
        assert!(texto.contains(" 7.00 Tf 0 g 16.00 16.00 Td"), "{texto}");
    }

    #[test]
    fn a_mesma_pagina_em_copias_diferentes_leva_marcas_diferentes() {
        let origem =
            PdfOrigem::carregar(&pdf(&[(595, 842, 0, None), (595, 842, 0, None)])).unwrap();
        let da_copia = |copia| Pagina {
            copia,
            ..original(2)
        };
        let mut opcoes = OpcoesDaPassada::default();
        opcoes.marcas.insert((0, Some(2)), "Folha 1".to_owned());
        opcoes.marcas.insert((1, Some(2)), "Folha 2".to_owned());
        let saida = origem
            .gerar_passada_com(&[da_copia(0), da_copia(1)], 1, &opcoes)
            .unwrap();
        for (indice, esperado) in ["Folha 1", "Folha 2"].into_iter().enumerate() {
            let (doc, pagina) = pagina_de(&saida, indice);
            let texto = conteudo_junto(&doc, &pagina);
            assert!(texto.contains(&texto_em_hexadecimal(esperado)), "{texto}");
            let outro = if indice == 0 { "Folha 2" } else { "Folha 1" };
            assert!(!texto.contains(&texto_em_hexadecimal(outro)), "{texto}");
        }
    }

    #[test]
    fn papel_hostil_vira_erro_sem_travar() {
        let origem = PdfOrigem::carregar(&pdf(&[(595, 842, 0, None)])).unwrap();
        for (largura, altura) in [
            (0.0, 0.0),
            (-1.0, 297.0),
            (f64::NAN, 297.0),
            (f64::INFINITY, f64::INFINITY),
            (f64::MAX, f64::MAX),
        ] {
            assert!(
                origem
                    .gerar_passada_com(&[original(1)], 1, &opcoes_de_papel(largura, altura))
                    .is_err(),
                "{largura} {altura}"
            );
        }
    }
}
