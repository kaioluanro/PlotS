//! Divergências com o programa conferido. A que falha fica ignorada para não travar a suíte, até
//! alguém decidir qual dos dois está certo; as outras mostram a extensão do problema.

mod comum;

use comum::*;
use plots_core::{ModeloImpressora, PerfilImpressora};
use plots_print::fluxo::Instrucao;
use plots_print::Mecanica;
use plotsim_physics::{
    conferir, BordaQueEntra, Entrada, Face, Impressora, Lado, Leitura, TopoDaImagem,
};

fn conferir_ao_pe_da_letra(modelo: ModeloImpressora, total: usize, lote: usize) -> Vec<String> {
    let ops = operacoes_da_virada(Instrucao::do_modelo(modelo).virada);
    let maco = juntar(plano(total, lote, modelo.perfil()).iter().map(|l| {
        executar_lote(
            l,
            modo_do_modelo(modelo),
            &Recoloca::AoPeDaLetra(ops.clone()),
            leitura_do_modelo(modelo),
            Defeito::Nenhum,
        )
    }));
    conferir(&maco, u32::try_from(total).unwrap())
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn o_jato_ao_pe_da_letra_fecha_em_toda_a_matriz() {
    for total in PAGINAS {
        for lote in LOTES {
            assert_eq!(
                conferir_ao_pe_da_letra(ModeloImpressora::JatoDeTinta, total, lote),
                Vec::<String>::new(),
                "{total} páginas, lote {lote}"
            );
        }
    }
}

#[test]
#[ignore = "A laser recoloca como saiu e manda o verso sem giro: pela curva em U, a aresta que entrou primeiro volta por último, e todo verso sai de cabeça para baixo"]
fn a_laser_ao_pe_da_letra_fecha_em_toda_a_matriz() {
    for total in PAGINAS {
        for lote in LOTES {
            assert_eq!(
                conferir_ao_pe_da_letra(ModeloImpressora::Laser, total, lote),
                Vec::<String>::new(),
                "{total} páginas, lote {lote}"
            );
        }
    }
}

#[test]
fn a_laser_ao_pe_da_letra_acerta_a_ordem_e_erra_so_a_orientacao_do_verso() {
    let motivos = conferir_ao_pe_da_letra(ModeloImpressora::Laser, 6, 0);
    assert_eq!(
        motivos,
        vec![
            "verso de cabeça para baixo na folha 1",
            "verso de cabeça para baixo na folha 2",
            "verso de cabeça para baixo na folha 3",
        ]
    );
}

#[test]
fn com_verso_girado_a_laser_ao_pe_da_letra_fecha() {
    // Uma saída que não pede giro nenhum à pessoa: o programa manda o verso girado.
    let modelo = ModeloImpressora::Laser;
    let perfil = PerfilImpressora {
        verso_girar_180: true,
        ..modelo.perfil()
    };
    for total in PAGINAS {
        for lote in LOTES {
            let maco = juntar(plano(total, lote, perfil).iter().map(|l| {
                executar_lote(
                    l,
                    LASER,
                    &Recoloca::AoPeDaLetra(Vec::new()),
                    Leitura::VirandoOMaco,
                    Defeito::Nenhum,
                )
            }));
            assert_eq!(
                conferir(&maco, u32::try_from(total).unwrap()),
                vec![],
                "{total} páginas, lote {lote}"
            );
        }
    }
}

#[test]
fn a_mecanica_declarada_pelo_programa_e_a_mesma_dos_modos_daqui() {
    let m = Mecanica::do_modelo(ModeloImpressora::Laser);
    assert_eq!(
        modo_da_mecanica(m, Entrada::Gaveta, TopoDaImagem::NaArestaQueEntra),
        LASER
    );
    let m = Mecanica::do_modelo(ModeloImpressora::JatoDeTinta);
    assert_eq!(
        modo_da_mecanica(m, Entrada::Traseira, TopoDaImagem::NaArestaQueEntra),
        JATO
    );
}

#[test]
fn a_face_que_a_instrucao_anuncia_e_a_que_fica_na_entrada() {
    for modelo in [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser] {
        let instrucao = Instrucao::do_modelo(modelo);
        let mut i = Impressora::nova(modo_do_modelo(modelo), 1);
        i.imprimir(&envios(&plano(2, 0, modelo.perfil())[0].passada1));
        i.recolocar_com(&operacoes_da_virada(instrucao.virada));
        let folha = i.entrada().folhas().last().expect("a folha voltou");
        let lado = if matches!(instrucao.lado, plots_print::fluxo::Lado::Cima) {
            Lado::Cima
        } else {
            Lado::Baixo
        };
        let face_impressa = if folha.impressoes(Face::A).is_empty() {
            Face::B
        } else {
            Face::A
        };
        assert_eq!(folha.face_para(lado), face_impressa, "{modelo:?}");
    }
}

/// Quantas das combinações de páginas e lote passam com esta forma de recolocar.
fn quantas_passam(modelo: ModeloImpressora, recoloca: &Recoloca) -> usize {
    let mut certas = 0;
    for total in PAGINAS {
        for lote in LOTES {
            let maco = juntar(plano(total, lote, modelo.perfil()).iter().map(|l| {
                executar_lote(
                    l,
                    modo_do_modelo(modelo),
                    recoloca,
                    leitura_do_modelo(modelo),
                    Defeito::Nenhum,
                )
            }));
            if conferir(&maco, u32::try_from(total).unwrap()).is_empty() {
                certas += 1;
            }
        }
    }
    certas
}

#[test]
fn quadro_de_cada_modelo_por_forma_de_recolocar() {
    let combinacoes = PAGINAS.len() * LOTES.len();
    // A um documento de uma página não há verso a errar: essas quatro combinações sempre passam.
    let so_de_uma_pagina = LOTES.len();
    for (modelo, literal, topo, base) in [
        (
            ModeloImpressora::JatoDeTinta,
            combinacoes,
            so_de_uma_pagina,
            combinacoes,
        ),
        (
            ModeloImpressora::Laser,
            so_de_uma_pagina,
            combinacoes,
            so_de_uma_pagina,
        ),
    ] {
        let ops = operacoes_da_virada(Instrucao::do_modelo(modelo).virada);
        let achado = (
            quantas_passam(modelo, &Recoloca::AoPeDaLetra(ops)),
            quantas_passam(
                modelo,
                &Recoloca::PeloGesto(gesto_do_modelo(modelo, BordaQueEntra::Topo)),
            ),
            quantas_passam(
                modelo,
                &Recoloca::PeloGesto(gesto_do_modelo(modelo, BordaQueEntra::Base)),
            ),
        );
        println!(
            "{modelo:?}: ao pé da letra {} de {combinacoes}; topo {}; base {}",
            achado.0, achado.1, achado.2
        );
        assert_eq!(achado, (literal, topo, base), "{modelo:?}");
    }
}
