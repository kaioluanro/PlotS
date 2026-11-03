//! Erros injetados de propósito: o modelo tem de acusar, com o motivo certo.

mod comum;

use comum::*;
use plots_core::{ModeloImpressora, Pagina};
use plotsim_physics::{conferir, BordaQueEntra, Gesto, Leitura, MacoLido, Motivo, Operacao};

const MODELOS: [ModeloImpressora; 2] = [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser];

fn com_operacoes(modelo: ModeloImpressora, total: usize, ops: &[Operacao]) -> MacoLido {
    let lotes = plano(total, 0, modelo.perfil());
    juntar(lotes.iter().map(|l| {
        executar_lote(
            l,
            modo_do_modelo(modelo),
            &Recoloca::AoPeDaLetra(ops.to_vec()),
            leitura_do_modelo(modelo),
            Defeito::Nenhum,
        )
    }))
}

fn gesto_certo(modelo: ModeloImpressora) -> Gesto {
    gesto_do_modelo(modelo, borda_que_fecha(modelo.perfil()))
}

fn sobre_impressao(motivos: &[Motivo]) -> bool {
    motivos
        .iter()
        .any(|m| matches!(m, Motivo::ImpressaoSobreImpressao { .. }))
}

#[test]
fn gesto_errado_e_acusado_nos_dois_modelos() {
    for total in [2usize, 3, 9, 10, 24] {
        let jato_sem_virar = com_operacoes(ModeloImpressora::JatoDeTinta, total, &[]);
        let motivos = conferir(&jato_sem_virar, u32::try_from(total).unwrap());
        assert!(sobre_impressao(&motivos), "{total}: {motivos:?}");

        let laser_virando = com_operacoes(
            ModeloImpressora::Laser,
            total,
            &[Operacao::VirarPelaArestaLonga],
        );
        let motivos = conferir(&laser_virando, u32::try_from(total).unwrap());
        assert!(sobre_impressao(&motivos), "{total}: {motivos:?}");
    }
}

#[test]
fn virada_pela_outra_aresta_deixa_versos_de_cabeca_para_baixo() {
    let maco = com_operacoes(
        ModeloImpressora::JatoDeTinta,
        6,
        &[Operacao::VirarPelaArestaLonga],
    );
    let motivos = conferir(&maco, 6);
    for folha in 1..=3 {
        assert!(motivos.contains(&Motivo::VersoDeCabecaParaBaixo { folha }));
    }
    let primeiro = motivos.first().expect("há motivo").to_string();
    assert_eq!(primeiro, "verso de cabeça para baixo na folha 1");
}

#[test]
fn borda_trocada_e_acusada_nos_dois_modelos() {
    for modelo in MODELOS {
        let trocada = match borda_que_fecha(modelo.perfil()) {
            BordaQueEntra::Topo => BordaQueEntra::Base,
            BordaQueEntra::Base => BordaQueEntra::Topo,
        };
        let lotes = plano(10, 0, modelo.perfil());
        let maco = juntar(lotes.iter().map(|l| {
            executar_lote(
                l,
                modo_do_modelo(modelo),
                &Recoloca::PeloGesto(gesto_do_modelo(modelo, trocada)),
                leitura_do_modelo(modelo),
                Defeito::Nenhum,
            )
        }));
        assert!(conferir(&maco, 10).contains(&Motivo::VersoDeCabecaParaBaixo { folha: 1 }));
    }
}

#[test]
fn folha_a_menos_e_acusada() {
    for modelo in MODELOS {
        for total in [2usize, 3, 10, 11, 24] {
            let lotes = plano(total, 0, modelo.perfil());
            let maco = juntar(lotes.iter().map(|l| {
                executar_lote(
                    l,
                    modo_do_modelo(modelo),
                    &Recoloca::PeloGesto(gesto_certo(modelo)),
                    leitura_do_modelo(modelo),
                    Defeito::FolhaAMenos,
                )
            }));
            let motivos = conferir(&maco, u32::try_from(total).unwrap());
            assert!(
                matches!(motivos.first(), Some(Motivo::FolhaAMenos { .. })),
                "{modelo:?}, {total}: {motivos:?}"
            );
            assert_eq!(motivos[0].to_string(), "folha a menos");
        }
    }
}

/// Executa o lote com o giro da página pedida invertido, como página de cabeça para baixo no PDF.
fn com_pagina_girada(modelo: ModeloImpressora, total: usize, alvo: usize) -> Vec<Motivo> {
    let mut lotes = plano(total, 0, modelo.perfil());
    let inverter = |paginas: &mut Vec<Pagina>| {
        for p in paginas.iter_mut() {
            if p.numero == Some(alvo) {
                p.girar_180 = !p.girar_180;
            }
        }
    };
    for lote in &mut lotes {
        inverter(&mut lote.passada1);
        inverter(&mut lote.passada2);
    }
    let maco = juntar(lotes.iter().map(|l| {
        executar_lote(
            l,
            modo_do_modelo(modelo),
            &Recoloca::PeloGesto(gesto_certo(modelo)),
            leitura_do_modelo(modelo),
            Defeito::Nenhum,
        )
    }));
    conferir(&maco, u32::try_from(total).unwrap())
}

#[test]
fn pagina_girada_e_acusada_em_qualquer_posicao() {
    for modelo in MODELOS {
        for alvo in 1..=6 {
            let motivos = com_pagina_girada(modelo, 6, alvo);
            let folha = u32::try_from(alvo).unwrap().div_ceil(2);
            assert!(
                motivos.contains(&Motivo::VersoDeCabecaParaBaixo { folha }),
                "{modelo:?}, página {alvo}: {motivos:?}"
            );
        }
    }
}

#[test]
fn ordem_invertida_no_verso_diz_com_qual_folha_trocou() {
    let modelo = ModeloImpressora::Laser;
    let maco = juntar(plano(6, 0, modelo.perfil()).iter().map(|l| {
        let mut lote = l.clone();
        lote.passada2.reverse();
        executar_lote(
            &lote,
            LASER,
            &Recoloca::PeloGesto(gesto_certo(modelo)),
            leitura_do_modelo(modelo),
            Defeito::Nenhum,
        )
    }));
    let textos: Vec<String> = conferir(&maco, 6).iter().map(ToString::to_string).collect();
    assert!(
        textos.iter().any(|t| t.ends_with("com a folha 3")),
        "{textos:?}"
    );
}

#[test]
fn ler_sem_virar_o_que_saiu_para_baixo_e_acusado() {
    let modelo = ModeloImpressora::Laser;
    let maco = juntar(plano(6, 0, modelo.perfil()).iter().map(|l| {
        executar_lote(
            l,
            LASER,
            &Recoloca::PeloGesto(gesto_certo(modelo)),
            Leitura::ComoSaiu,
            Defeito::Nenhum,
        )
    }));
    assert!(!conferir(&maco, 6).is_empty());
}
