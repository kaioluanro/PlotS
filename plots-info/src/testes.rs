use super::*;
use plots_core::{DimensoesPagina, LIMITE_PAGINAS};

const A4: DimensoesPagina = DimensoesPagina::new(210.0, 297.0);
const A3: DimensoesPagina = DimensoesPagina::new(297.0, 420.0);

fn perto(obtido: f64, esperado: f64) {
    assert!(
        (obtido - esperado).abs() < 1e-9,
        "esperado {esperado}, obtido {obtido}"
    );
}

fn paginas(valores: &[usize]) -> Vec<usize> {
    valores.to_vec()
}

fn perfil_de_tanque() -> PerfilDeTinta {
    PerfisDeTinta::padrao()
        .buscar(PERFIL_PADRAO)
        .expect("o perfil padrão existe")
        .clone()
}

fn perfil_valido() -> PerfilDeTinta {
    PerfilDeTinta {
        id: "teste".to_owned(),
        nome: "Perfil de teste".to_owned(),
        moeda: "BRL".to_owned(),
        volumes_ml: [50.0, 50.0, 50.0, 50.0],
        rendimentos_paginas: [5000, 5000, 5000, 5000],
        precos_ml: [1.0, 2.0, 3.0, 4.0],
        fatores_qualidade: [0.6, 1.0, 1.3],
    }
}

// --- seleção ---

#[test]
fn texto_vazio_ou_so_espacos_escolhe_o_documento_inteiro() {
    for texto in ["", "   ", "\t\n"] {
        let selecao = selecao_de_texto(texto, 10).unwrap();
        assert!(selecao.documento_inteiro());
        assert_eq!(selecao.quantidade(), 10);
        assert_eq!(selecao.numeros(), paginas(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
        assert_eq!(selecao.total(), 10);
    }
}

#[test]
fn um_numero_escolhe_uma_pagina() {
    let selecao = selecao_de_texto("1", 1).unwrap();
    assert!(!selecao.documento_inteiro());
    assert_eq!(selecao.numeros(), [1]);
}

#[test]
fn lista_separada_por_virgula_escolhe_as_paginas_indicadas() {
    let selecao = selecao_de_texto("2,5,7,9", 10).unwrap();
    assert_eq!(selecao.numeros(), [2, 5, 7, 9]);
    assert_eq!(selecao.quantidade(), 4);
}

#[test]
fn faixa_inclusiva_escolhe_todas_as_paginas_do_trecho() {
    let selecao = selecao_de_texto("5-16", 16).unwrap();
    assert_eq!(selecao.numeros(), paginas(&(5..=16).collect::<Vec<_>>()));
    assert_eq!(selecao.quantidade(), 12);
}

#[test]
fn lista_e_faixa_se_misturam() {
    let selecao = selecao_de_texto("1,3,5-9,12", 12).unwrap();
    assert_eq!(selecao.numeros(), [1, 3, 5, 6, 7, 8, 9, 12]);
}

#[test]
fn espacos_em_volta_dos_itens_e_das_faixas_sao_ignorados() {
    let selecao = selecao_de_texto(" 2 , 5 - 7 ", 10).unwrap();
    assert_eq!(selecao.numeros(), [2, 5, 6, 7]);
}

#[test]
fn repeticoes_saem_em_ordem_crescente() {
    let selecao = selecao_de_texto("3,1,2,2,1,3-4,4", 4).unwrap();
    assert_eq!(selecao.numeros(), [1, 2, 3, 4]);
    assert_eq!(selecao.quantidade(), 4);
}

#[test]
fn pagina_fora_do_documento_e_recusada() {
    assert_eq!(
        selecao_de_texto("11", 10),
        Err(ErroSelecao::ForaDoDocumento {
            pagina: 11,
            total: 10
        })
    );
    assert_eq!(
        selecao_de_texto("0", 10),
        Err(ErroSelecao::ForaDoDocumento {
            pagina: 0,
            total: 10
        })
    );
    assert_eq!(
        selecao_de_texto("1", 0),
        Err(ErroSelecao::ForaDoDocumento {
            pagina: 1,
            total: 0
        })
    );
}

#[test]
fn faixa_fora_do_documento_e_recusada() {
    assert_eq!(
        selecao_de_texto("5-16", 10),
        Err(ErroSelecao::FaixaForaDoDocumento {
            inicio: 5,
            fim: 16,
            total: 10
        })
    );
    assert_eq!(
        selecao_de_texto("0-3", 10),
        Err(ErroSelecao::FaixaForaDoDocumento {
            inicio: 0,
            fim: 3,
            total: 10
        })
    );
}

#[test]
fn faixa_invertida_e_recusada() {
    assert_eq!(
        selecao_de_texto("16-5", 20),
        Err(ErroSelecao::FaixaInvertida { inicio: 16, fim: 5 })
    );
}

#[test]
fn item_vazio_e_recusado() {
    for texto in ["1,,2", "1,", ",1", ","] {
        assert_eq!(selecao_de_texto(texto, 10), Err(ErroSelecao::ItemVazio));
    }
}

#[test]
fn item_nao_numerico_e_recusado() {
    for texto in ["abc", "1,abc", "5-", "-5", "1-2-3", "1.5", "1 2"] {
        assert!(
            matches!(
                selecao_de_texto(texto, 10),
                Err(ErroSelecao::ItemNaoNumerico { .. })
            ),
            "{texto}"
        );
    }
    assert!(matches!(
        selecao_de_texto("999999999999999999999999999", 10),
        Err(ErroSelecao::ItemNaoNumerico { .. })
    ));
}

#[test]
fn texto_grande_demais_e_recusado_antes_de_ler() {
    let texto = "1,".repeat(LIMITE_TEXTO_BYTES);
    assert_eq!(
        selecao_de_texto(&texto, 10),
        Err(ErroSelecao::TextoGrande { bytes: texto.len() })
    );
}

#[test]
fn selecao_com_mais_de_dez_mil_numeros_e_recusada() {
    assert_eq!(
        selecao_de_texto("1-10001", LIMITE_NUMEROS + 1),
        Err(ErroSelecao::NumerosDemais)
    );
}

#[test]
fn documento_acima_do_limite_de_paginas_e_recusado() {
    assert_eq!(
        selecao_de_texto("", LIMITE_PAGINAS + 1),
        Err(ErroSelecao::DocumentoGrande {
            total: LIMITE_PAGINAS + 1,
            limite: LIMITE_PAGINAS
        })
    );
}

// --- cobertura ---

fn bgra_de(pixels: &[(u8, u8, u8)]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(pixels.len() * 4);
    for (r, g, b) in pixels {
        bytes.extend([*b, *g, *r, 255]);
    }
    bytes
}

#[test]
fn pagina_branca_tem_cobertura_zero() {
    let pixels = bgra_de(&[(255, 255, 255); 4]);
    let cobertura = cobertura_de_bgra(&pixels, 2, 2, 8, false).unwrap();
    for valor in cobertura.valores() {
        perto(valor, 0.0);
    }
}

#[test]
fn pagina_preta_tem_so_preto_em_cem_por_cento() {
    let pixels = bgra_de(&[(0, 0, 0); 4]);
    let cobertura = cobertura_de_bgra(&pixels, 2, 2, 8, false).unwrap();
    perto(cobertura.c, 0.0);
    perto(cobertura.m, 0.0);
    perto(cobertura.y, 0.0);
    perto(cobertura.k, 100.0);
}

#[test]
fn quadrado_preto_em_um_quarto_da_pagina_da_vinte_e_cinco_por_cento_de_preto() {
    let (largura, altura) = (10usize, 10usize);
    let brancos = vec![(255, 255, 255); largura * altura];
    let mut pixels = bgra_de(&brancos);
    for y in 0..5 {
        for x in 0..5 {
            let inicio = (y * largura + x) * 4;
            pixels[inicio] = 0;
            pixels[inicio + 1] = 0;
            pixels[inicio + 2] = 0;
        }
    }
    let cobertura = cobertura_de_bgra(&pixels, largura, altura, largura * 4, false).unwrap();
    perto(cobertura.k, 25.0);
    perto(cobertura.c, 0.0);
}

#[test]
fn ciano_puro_tem_ciano_em_cem_por_cento() {
    let pixels = bgra_de(&[(0, 255, 255)]);
    let cobertura = cobertura_de_bgra(&pixels, 1, 1, 4, false).unwrap();
    perto(cobertura.c, 100.0);
    perto(cobertura.m, 0.0);
    perto(cobertura.y, 0.0);
    perto(cobertura.k, 0.0);
}

#[test]
fn cinza_meio_usa_so_o_canal_preto_com_e_sem_a_regra() {
    let pixels = bgra_de(&[(128, 128, 128); 4]);
    let sem_regra = cobertura_de_bgra(&pixels, 2, 2, 8, false).unwrap();
    let com_regra = cobertura_de_bgra(&pixels, 2, 2, 8, true).unwrap();
    let esperado = (1.0 - 128.0 / 255.0) * 100.0;
    perto(sem_regra.k, esperado);
    perto(com_regra.k, esperado);
    for cobertura in [sem_regra, com_regra] {
        perto(cobertura.c, 0.0);
        perto(cobertura.m, 0.0);
        perto(cobertura.y, 0.0);
    }
}

#[test]
fn passo_maior_que_a_linha_ignora_o_preenchimento() {
    let (largura, altura, passo) = (2, 2, 12);
    let mut pixels = vec![255u8; passo * altura];
    // Primeiro pixel vermelho: M e Y em 100%, C e K em zero.
    pixels[0] = 0;
    pixels[1] = 0;
    pixels[2] = 255;
    // O preenchimento depois dos pixels da linha é preto e não deve contar.
    for byte in &mut pixels[8..12] {
        *byte = 0;
    }
    let cobertura = cobertura_de_bgra(&pixels, largura, altura, passo, false).unwrap();
    perto(cobertura.m, 25.0);
    perto(cobertura.y, 25.0);
    perto(cobertura.c, 0.0);
    perto(cobertura.k, 0.0);
}

#[test]
fn pixels_curtos_sao_recusados() {
    assert_eq!(
        cobertura_de_bgra(&[0; 3], 1, 1, 4, false),
        Err(ErroCobertura::DadosCurtos {
            esperado: 4,
            recebido: 3
        })
    );
}

#[test]
fn passo_menor_que_a_linha_e_recusado() {
    assert_eq!(
        cobertura_de_bgra(&[0; 8], 2, 1, 7, false),
        Err(ErroCobertura::PassoInvalido {
            passo: 7,
            minimo: 8
        })
    );
}

#[test]
fn bgra_sem_dimensao_e_recusado() {
    assert_eq!(
        cobertura_de_bgra(&[], 0, 1, 4, false),
        Err(ErroCobertura::DimensaoInvalida)
    );
    assert_eq!(
        cobertura_de_bgra(&[], 1, 0, 4, false),
        Err(ErroCobertura::DimensaoInvalida)
    );
}

fn pam(largura: usize, altura: usize, profundidade: u32, maxval: u32, dados: &[u8]) -> Vec<u8> {
    let mut bytes = format!(
        "P7\nWIDTH {largura}\nHEIGHT {altura}\nDEPTH {profundidade}\nMAXVAL {maxval}\nTUPLTYPE CMYK\nENDHDR\n"
    )
    .into_bytes();
    bytes.extend_from_slice(dados);
    bytes
}

#[test]
fn pam_cmyk_traz_a_media_de_cada_canal() {
    let bytes = pam(1, 1, 4, 255, &[102, 0, 0, 0]);
    let cobertura = cobertura_de_pam(&bytes).unwrap();
    perto(cobertura.c, 40.0);
    perto(cobertura.m, 0.0);
    perto(cobertura.y, 0.0);
    perto(cobertura.k, 0.0);
}

#[test]
fn pam_soma_os_pixels_de_cada_canal() {
    let bytes = pam(2, 1, 4, 255, &[255, 0, 0, 0, 0, 255, 0, 0]);
    let cobertura = cobertura_de_pam(&bytes).unwrap();
    perto(cobertura.c, 50.0);
    perto(cobertura.m, 50.0);
    perto(cobertura.y, 0.0);
    perto(cobertura.k, 0.0);
}

#[test]
fn pam_de_profundidade_diferente_e_recusado() {
    let bytes = pam(1, 1, 3, 255, &[0, 0, 0]);
    assert_eq!(
        cobertura_de_pam(&bytes),
        Err(ErroCobertura::ProfundidadeNaoSuportada { profundidade: 3 })
    );
}

#[test]
fn pam_de_maxval_diferente_e_recusado() {
    let bytes = pam(1, 1, 4, 65535, &[0; 8]);
    assert_eq!(
        cobertura_de_pam(&bytes),
        Err(ErroCobertura::MaxvalNaoSuportado { maxval: 65535 })
    );
}

#[test]
fn pam_com_dados_de_tamanho_errado_e_recusado() {
    let bytes = pam(2, 2, 4, 255, &[0; 15]);
    assert_eq!(
        cobertura_de_pam(&bytes),
        Err(ErroCobertura::TamanhoInvalido {
            esperado: 16,
            recebido: 15
        })
    );
}

#[test]
fn pam_sem_o_marcador_p7_e_recusado() {
    assert_eq!(
        cobertura_de_pam(b"P6\nWIDTH 1\nHEIGHT 1\nDEPTH 4\nMAXVAL 255\nENDHDR\n"),
        Err(ErroCobertura::MagicInvalido)
    );
}

#[test]
fn pam_sem_endhdr_e_recusado() {
    assert_eq!(
        cobertura_de_pam(b"P7\nWIDTH 1\nHEIGHT 1\nDEPTH 4\nMAXVAL 255\n"),
        Err(ErroCobertura::CabecalhoInvalido)
    );
}

#[test]
fn pam_sem_dimensao_e_recusado() {
    let bytes = pam(0, 1, 4, 255, &[]);
    assert_eq!(
        cobertura_de_pam(&bytes),
        Err(ErroCobertura::DimensaoInvalida)
    );
}

// --- perfis ---

#[test]
fn os_cinco_perfis_padrao_sao_validos_e_tem_ids_distintos() {
    let perfis = PerfisDeTinta::padrao();
    assert_eq!(perfis.perfis().len(), 5);
    for perfil in perfis.perfis() {
        perfil.validar().unwrap();
    }
    let mut vistos = std::collections::HashSet::new();
    for perfil in perfis.perfis() {
        assert!(
            vistos.insert(perfil.id.as_str()),
            "o identificador {} se repete",
            perfil.id
        );
    }
    for id in [
        "tanque-generico",
        "cartucho-generico",
        "hp-smart-tank-110-gt51",
        "epson-l3150-544",
        "epson-l6270-504",
    ] {
        assert!(perfis.buscar(id).is_some(), "o perfil {id} existe");
    }
}

#[test]
fn busca_por_identificador_encontra_o_perfil() {
    let perfis = PerfisDeTinta::padrao();
    assert_eq!(
        perfis
            .buscar("cartucho-generico")
            .map(|perfil| &perfil.nome),
        Some(&"Cartucho de jato de tinta genérico".to_owned())
    );
    for (id, nome) in [
        ("tanque-generico", "Tanque de tinta genérico"),
        ("hp-smart-tank-110-gt51", "HP Smart Tank 110 (GT51)"),
        ("epson-l3150-544", "Epson EcoTank L3150 (544)"),
        ("epson-l6270-504", "Epson EcoTank L6270 (504)"),
    ] {
        assert_eq!(
            perfis.buscar(id).map(|perfil| perfil.nome.as_str()),
            Some(nome)
        );
    }
    assert!(perfis.buscar("inexistente").is_none());
}

#[test]
fn os_perfis_de_impressora_tem_o_volume_e_o_preco_do_frasco() {
    let perfis = PerfisDeTinta::padrao();
    let casos = [
        (
            "hp-smart-tank-110-gt51",
            "HP Smart Tank 110 (GT51)",
            90.0,
            30.0 / 90.0,
        ),
        (
            "epson-l3150-544",
            "Epson EcoTank L3150 (544)",
            65.0,
            33.0 / 65.0,
        ),
        (
            "epson-l6270-504",
            "Epson EcoTank L6270 (504)",
            127.0,
            55.0 / 127.0,
        ),
    ];
    for (id, nome, volume, preco_ml) in casos {
        let perfil = perfis
            .buscar(id)
            .unwrap_or_else(|| panic!("o perfil {id} existe"));
        assert_eq!(perfil.nome, nome);
        assert_eq!(perfil.moeda, "BRL");
        for canal in 0..perfil.volumes_ml.len() {
            perto(perfil.volumes_ml[canal], volume);
            perto(perfil.precos_ml[canal], preco_ml);
        }
        assert_eq!(perfil.rendimentos_paginas, [6500, 6500, 6500, 4500]);
        assert_eq!(perfil.fatores_qualidade, [0.6, 1.0, 1.3]);
    }
}

#[test]
fn guardar_o_mesmo_identificador_substitui_no_lugar() {
    let mut perfis = PerfisDeTinta::padrao();
    let mut perfil = perfil_valido();
    perfis.substituir(perfil.clone()).unwrap();
    assert_eq!(perfis.perfis().len(), 6);
    perfil.nome = "Outro nome".to_owned();
    assert_eq!(perfis.substituir(perfil.clone()), Ok(true));
    assert_eq!(perfis.perfis().len(), 6);
    assert_eq!(perfis.buscar("teste").map(|p| &p.nome), Some(&perfil.nome));
}

#[test]
fn remover_devolve_se_o_perfil_existia() {
    let mut perfis = PerfisDeTinta::padrao();
    assert!(perfis.remover("tanque-generico"));
    assert!(!perfis.remover("tanque-generico"));
    assert!(perfis.buscar("tanque-generico").is_none());
}

#[test]
fn perfil_com_campo_vazio_ou_fora_da_faixa_e_recusado() {
    let casos: Vec<Box<dyn Fn(&mut PerfilDeTinta)>> = vec![
        Box::new(|p| p.id = " ".to_owned()),
        Box::new(|p| p.id = "x".repeat(LIMITE_ID_BYTES + 1)),
        Box::new(|p| p.nome = "".to_owned()),
        Box::new(|p| p.nome = "x".repeat(LIMITE_NOME_BYTES + 1)),
        Box::new(|p| p.moeda = "brl".to_owned()),
        Box::new(|p| p.moeda = "BR".to_owned()),
        Box::new(|p| p.moeda = "BR1".to_owned()),
        Box::new(|p| p.volumes_ml[0] = 0.0),
        Box::new(|p| p.volumes_ml[1] = -1.0),
        Box::new(|p| p.volumes_ml[2] = f64::NAN),
        Box::new(|p| p.volumes_ml[3] = LIMITE_VOLUME_ML + 1.0),
        Box::new(|p| p.rendimentos_paginas[0] = 0),
        Box::new(|p| p.rendimentos_paginas[1] = LIMITE_RENDIMENTO_PAGINAS + 1),
        Box::new(|p| p.precos_ml[0] = -0.01),
        Box::new(|p| p.precos_ml[1] = f64::INFINITY),
        Box::new(|p| p.precos_ml[2] = LIMITE_PRECO_ML + 1.0),
        Box::new(|p| p.fatores_qualidade[0] = 0.0),
        Box::new(|p| p.fatores_qualidade[1] = -1.0),
        Box::new(|p| p.fatores_qualidade[2] = f64::NAN),
    ];
    for (indice, mudar) in casos.iter().enumerate() {
        let mut perfil = perfil_valido();
        mudar(&mut perfil);
        assert!(perfil.validar().is_err(), "caso {indice}");
    }
    perfil_valido().validar().unwrap();
}

#[test]
fn preco_zero_e_aceito_e_volume_ou_rendimento_zero_nao() {
    let mut perfil = perfil_valido();
    perfil.precos_ml = [0.0; 4];
    perfil.validar().unwrap();
    perfil.volumes_ml[3] = 0.0;
    assert!(perfil.validar().is_err());
}

// --- configuração ---

#[test]
fn configuracao_padrao_aponta_para_um_perfil_padrao() {
    let configuracao = ConfiguracaoDeTinta::default();
    configuracao.validar().unwrap();
    assert_eq!(configuracao.perfil, PERFIL_PADRAO);
    assert!(PerfisDeTinta::padrao()
        .buscar(&configuracao.perfil)
        .is_some());
}

#[test]
fn resolucao_fora_de_cento_e_cinquenta_ou_trezentos_e_recusada() {
    let mut configuracao = ConfiguracaoDeTinta::default();
    for dpi in [DPIS_SUPORTADOS[0], DPIS_SUPORTADOS[1]] {
        configuracao.dpi = dpi;
        configuracao.validar().unwrap();
    }
    for dpi in [0, 72, 200, 600] {
        configuracao.dpi = dpi;
        assert_eq!(
            configuracao.validar(),
            Err(ErroConfiguracao::DpiNaoSuportado { dpi })
        );
    }
}

#[test]
fn configuracao_sem_perfil_e_recusada() {
    let mut configuracao = ConfiguracaoDeTinta::default();
    configuracao.perfil = "  ".to_owned();
    assert_eq!(configuracao.validar(), Err(ErroConfiguracao::PerfilVazio));
    configuracao.perfil = "x".repeat(LIMITE_ID_BYTES + 1);
    assert!(matches!(
        configuracao.validar(),
        Err(ErroConfiguracao::PerfilGrande { .. })
    ));
}

// --- custo ---

#[test]
fn cinco_por_cento_em_a4_gasta_o_volume_do_rendimento() {
    let perfil = perfil_de_tanque();
    let cobertura = Cobertura::new(5.0, 5.0, 5.0, 5.0);
    let consumo = ml_da_pagina(cobertura, A4, &perfil, Qualidade::Normal).unwrap();
    perto(
        consumo.c,
        perfil.volumes_ml[0] / f64::from(perfil.rendimentos_paginas[0]),
    );
    perto(
        consumo.m,
        perfil.volumes_ml[1] / f64::from(perfil.rendimentos_paginas[1]),
    );
    perto(
        consumo.y,
        perfil.volumes_ml[2] / f64::from(perfil.rendimentos_paginas[2]),
    );
    perto(
        consumo.k,
        perfil.volumes_ml[3] / f64::from(perfil.rendimentos_paginas[3]),
    );
}

#[test]
fn a3_gasta_o_dobro_de_a4() {
    let perfil = perfil_de_tanque();
    let cobertura = Cobertura::new(20.0, 10.0, 5.0, 30.0);
    let a4 = ml_da_pagina(cobertura, A4, &perfil, Qualidade::Normal).unwrap();
    let a3 = ml_da_pagina(cobertura, A3, &perfil, Qualidade::Normal).unwrap();
    for (menor, maior) in a4.valores().iter().zip(a3.valores()) {
        perto(maior, menor * 2.0);
    }
}

#[test]
fn qualidade_aplica_os_fatores_do_perfil() {
    let perfil = perfil_de_tanque();
    let cobertura = Cobertura::new(25.0, 25.0, 25.0, 25.0);
    let normal = ml_da_pagina(cobertura, A4, &perfil, Qualidade::Normal).unwrap();
    let rascunho = ml_da_pagina(cobertura, A4, &perfil, Qualidade::Rascunho).unwrap();
    let alta = ml_da_pagina(cobertura, A4, &perfil, Qualidade::Alta).unwrap();
    perto(rascunho.c, normal.c * 0.6);
    perto(alta.c, normal.c * 1.3);
    let base = ml_da_pagina(cobertura, A4, &perfil, Qualidade::Normal).unwrap();
    perto(normal.c, base.c);
}

#[test]
fn o_custo_usa_o_preco_por_mililitro_de_cada_canal() {
    let perfil = perfil_valido();
    let custo = custo_do_consumo(Consumo::new(2.0, 3.0, 4.0, 5.0), &perfil).unwrap();
    perto(custo.c, 2.0);
    perto(custo.m, 6.0);
    perto(custo.y, 12.0);
    perto(custo.k, 20.0);
    perto(custo.total(), 40.0);
}

#[test]
fn a_selecao_multiplica_pelas_copias_e_o_documento_usa_uma() {
    let perfil = perfil_de_tanque();
    let pagina = ml_da_pagina(
        Cobertura::new(10.0, 10.0, 10.0, 10.0),
        A4,
        &perfil,
        Qualidade::Normal,
    )
    .unwrap();
    let duas = [pagina, pagina];
    let selecao = estimativa_da_selecao(&duas, 3, &perfil).unwrap();
    perto(selecao.tinta.c, pagina.c * 2.0 * 3.0);
    perto(selecao.custo.c, pagina.c * 2.0 * 3.0 * perfil.precos_ml[0]);
    let documento = estimativa_do_documento(&duas, &perfil).unwrap();
    perto(documento.tinta.c, pagina.c * 2.0);
    assert_eq!(
        documento.custo,
        custo_do_consumo(documento.tinta, &perfil).unwrap()
    );
}

#[test]
fn copias_fora_de_um_a_novecentos_e_noventa_e_nove_sao_recusadas() {
    let perfil = perfil_de_tanque();
    let paginas = [Consumo::new(1.0, 1.0, 1.0, 1.0)];
    for copias in [0, 1000] {
        assert_eq!(
            estimativa_da_selecao(&paginas, copias, &perfil),
            Err(ErroCusto::CopiasInvalidas { copias })
        );
        assert_eq!(
            cartucho(paginas[0], copias, &perfil),
            Err(ErroCusto::CopiasInvalidas { copias })
        );
    }
    assert!(estimativa_da_selecao(&paginas, 1, &perfil).is_ok());
    assert!(estimativa_da_selecao(&paginas, 999, &perfil).is_ok());
}

#[test]
fn perfil_invalido_e_recusado_no_calculo() {
    let mut perfil = perfil_de_tanque();
    perfil.id.clear();
    assert!(matches!(
        ml_da_pagina(
            Cobertura::new(1.0, 1.0, 1.0, 1.0),
            A4,
            &perfil,
            Qualidade::Normal
        ),
        Err(ErroCusto::Perfil(_))
    ));
}

#[test]
fn dimensao_e_cobertura_invalidas_sao_recusadas() {
    let perfil = perfil_de_tanque();
    let cobertura = Cobertura::new(1.0, 1.0, 1.0, 1.0);
    for dimensoes in [
        DimensoesPagina::new(0.0, 297.0),
        DimensoesPagina::new(-1.0, 297.0),
        DimensoesPagina::new(f64::NAN, 297.0),
        DimensoesPagina::new(210.0, f64::INFINITY),
    ] {
        assert_eq!(
            ml_da_pagina(cobertura, dimensoes, &perfil, Qualidade::Normal),
            Err(ErroCusto::DimensaoInvalida)
        );
    }
    for valor in [-1.0, 100.1, f64::NAN] {
        let cobertura = Cobertura::new(valor, 0.0, 0.0, 0.0);
        assert!(matches!(
            ml_da_pagina(cobertura, A4, &perfil, Qualidade::Normal),
            Err(ErroCusto::CoberturaInvalida { canal: 0 })
        ));
    }
}

#[test]
fn consumo_negativo_e_recusado() {
    let perfil = perfil_de_tanque();
    assert!(matches!(
        custo_do_consumo(Consumo::new(-1.0, 0.0, 0.0, 0.0), &perfil),
        Err(ErroCusto::ConsumoInvalido { canal: 0 })
    ));
    assert!(matches!(
        estimativa_da_selecao(&[Consumo::new(f64::NAN, 0.0, 0.0, 0.0)], 1, &perfil),
        Err(ErroCusto::ConsumoInvalido { canal: 0 })
    ));
}

#[test]
fn cartucho_mostra_o_consumo_e_a_cor_que_acaba_primeiro() {
    let perfil = perfil_de_tanque();
    let pagina = ml_da_pagina(
        Cobertura::new(5.0, 5.0, 5.0, 5.0),
        A4,
        &perfil,
        Qualidade::Normal,
    )
    .unwrap();
    let estado = cartucho(pagina, 1, &perfil).unwrap();
    assert_eq!(estado.canal_que_acaba_primeiro, 3, "o preto acaba antes");
    assert_eq!(estado.copias_ate_esvaziar, 4500);
    perto(
        estado.percentual_consumido,
        pagina.k / perfil.volumes_ml[3] * 100.0,
    );

    let dez = cartucho(
        Consumo::new(
            pagina.c * 10.0,
            pagina.m * 10.0,
            pagina.y * 10.0,
            pagina.k * 10.0,
        ),
        10,
        &perfil,
    )
    .unwrap();
    assert_eq!(dez.copias_ate_esvaziar, 4500);
}

#[test]
fn cartucho_sem_consumo_nunca_esvazia() {
    let perfil = perfil_de_tanque();
    let estado = cartucho(Consumo::default(), 1, &perfil).unwrap();
    assert_eq!(estado.copias_ate_esvaziar, u64::MAX);
    perto(estado.percentual_consumido, 0.0);
}

#[test]
fn pagina_mais_cara_soma_os_canais_e_ignora_valor_invalido() {
    let paginas = [
        Custo::new(1.0, 0.0, 0.0, 0.0),
        Custo::new(1.0, 1.0, 0.0, 0.0),
        Custo::new(f64::NAN, 10.0, 10.0, 10.0),
    ];
    assert_eq!(pagina_mais_cara(&paginas), Some((1, 2.0)));
    assert_eq!(pagina_mais_cara(&[]), None);
}
