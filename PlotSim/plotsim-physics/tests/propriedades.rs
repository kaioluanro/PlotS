//! Propriedades que valem para qualquer tamanho de documento e qualquer sequência de gestos.

mod comum;

use comum::*;
use plots_core::ModeloImpressora;
use plotsim_physics::{
    conferir, ler_maco, BordaQueEntra, Entrada, Envio, FaceNaEntrada, Gesto, Giro, Impressora,
    Lado, Leitura, ModoImpressora, Motivo, Operacao, Pilha, TopoDaImagem,
};
use proptest::prelude::*;

fn operacao() -> impl Strategy<Value = Operacao> {
    prop_oneof![
        Just(Operacao::VirarPelaArestaCurta),
        Just(Operacao::VirarPelaArestaLonga),
        Just(Operacao::Girar180),
        Just(Operacao::InverterFolhaAFolha),
    ]
}

fn lado() -> impl Strategy<Value = Lado> {
    prop_oneof![Just(Lado::Cima), Just(Lado::Baixo)]
}

fn entrada() -> impl Strategy<Value = Entrada> {
    prop_oneof![
        Just(Entrada::Traseira),
        Just(Entrada::Gaveta),
        Just(Entrada::Manual)
    ]
}

fn modo() -> impl Strategy<Value = ModoImpressora> {
    (
        lado(),
        lado(),
        lado(),
        lado(),
        prop_oneof![
            Just(TopoDaImagem::NaArestaQueEntra),
            Just(TopoDaImagem::NaArestaQueSai)
        ],
        entrada(),
    )
        .prop_map(
            |(puxa, imprime_na_face_para, sai_com_face_para, cai, topo_da_imagem, entrada)| {
                ModoImpressora {
                    entrada,
                    puxa,
                    imprime_na_face_para,
                    sai_com_face_para,
                    cai,
                    topo_da_imagem,
                }
            },
        )
}

/// Primeira passada que dá um maço certo: as folhas do fim para o começo, com a página par.
fn frentes_certas(total: u32) -> Vec<Envio> {
    (1..=total)
        .filter(|p| p % 2 == 1)
        .rev()
        .map(|impar| {
            if impar + 1 <= total {
                Envio::pagina(impar + 1)
            } else {
                Envio::branco()
            }
        })
        .collect()
}

/// Segunda passada que dá um maço certo: as ímpares do fim para o começo.
fn versos_certos(total: u32) -> Vec<Envio> {
    (1..=total)
        .filter(|p| p % 2 == 1)
        .rev()
        .map(Envio::pagina)
        .collect()
}

/// Maço certo para `total` páginas, feito pelo modelo sem o programa conferido.
fn pilha_certa(total: u32) -> Pilha {
    let folhas = usize::try_from(total.div_ceil(2)).unwrap();
    let mut i = Impressora::nova(JATO, folhas);
    i.imprimir(&frentes_certas(total));
    i.recolocar_com(&[Operacao::VirarPelaArestaLonga]);
    i.imprimir(&versos_certos(total));
    i.retirar_saida()
}

proptest! {
    #[test]
    fn cada_operacao_feita_duas_vezes_devolve_a_pilha(
        paginas in 1usize..40,
        op in operacao(),
    ) {
        let mut i = Impressora::nova(JATO, paginas);
        let envios: Vec<Envio> = (1..=paginas)
            .map(|p| Envio::pagina(u32::try_from(p).unwrap()))
            .collect();
        i.imprimir(&envios);
        let inicial = i.saida().clone();
        let mut pilha = inicial.clone();
        pilha.aplicar(op);
        pilha.aplicar(op);
        prop_assert_eq!(pilha, inicial);
    }

    #[test]
    fn sequencias_arbitrarias_de_gestos_e_modos_nunca_travam(
        m in modo(),
        papel in 0usize..30,
        total in 0u32..60,
        ops in proptest::collection::vec(operacao(), 0..8),
        giro in prop_oneof![
            Just(Giro::Nenhum), Just(Giro::Direita90), Just(Giro::Meia180), Just(Giro::Esquerda270)
        ],
    ) {
        let mut i = Impressora::nova(m, papel);
        let envios: Vec<Envio> = (1..=total).map(|p| Envio::girada(p, giro)).collect();
        i.imprimir(&envios);
        i.recolocar_com(&ops);
        i.imprimir(&envios);
        let _ = conferir(&ler_maco(i.saida(), Leitura::ComoSaiu), total);
        let _ = conferir(&ler_maco(i.saida(), Leitura::VirandoOMaco), total);
    }

    #[test]
    fn os_gestos_nunca_travam_com_entrada_e_face_quaisquer(
        m in modo(),
        papel in 0usize..10,
        entrada in entrada(),
        face_impressa in prop_oneof![
            Just(FaceNaEntrada::ParaFrente),
            Just(FaceNaEntrada::ParaTras),
            Just(FaceNaEntrada::ParaCima),
            Just(FaceNaEntrada::ParaBaixo)
        ],
        borda_que_entra in prop_oneof![Just(BordaQueEntra::Topo), Just(BordaQueEntra::Base)],
        inverter_ordem in any::<bool>(),
    ) {
        let mut i = Impressora::nova(m, papel);
        let envios: Vec<Envio> = (1..=papel)
            .map(|p| Envio::pagina(u32::try_from(p).unwrap()))
            .collect();
        i.imprimir(&envios);
        let _ = i.recolocar(Gesto { entrada, face_impressa, borda_que_entra, inverter_ordem });
    }

    #[test]
    fn os_dois_modelos_fecham_com_a_borda_certa_em_qualquer_tamanho(
        total in 1usize..300,
        lote in 1usize..40,
    ) {
        for modelo in [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser] {
            prop_assert_eq!(conferir_modelo(modelo, total, lote), vec![]);
        }
    }

    #[test]
    fn o_maco_certo_passa_e_qualquer_folha_a_menos_e_acusada(
        total in 2u32..200,
        posicao in 0usize..100,
    ) {
        let certa = pilha_certa(total);
        prop_assert_eq!(conferir(&ler_maco(&certa, Leitura::ComoSaiu), total), vec![]);
        let mut pilha = certa;
        pilha.retirar_de_cima(posicao % pilha.len());
        let motivos = conferir(&ler_maco(&pilha, Leitura::ComoSaiu), total);
        let a_menos = matches!(motivos.first(), Some(Motivo::FolhaAMenos { .. }));
        prop_assert!(a_menos);
    }

    #[test]
    fn qualquer_troca_de_folhas_vizinhas_e_acusada(total in 3u32..200, posicao in 0usize..100) {
        let mut lido = ler_maco(&pilha_certa(total), Leitura::ComoSaiu);
        let posicao = posicao % (lido.folhas.len() - 1);
        lido.folhas.swap(posicao, posicao + 1);
        prop_assert!(!conferir(&lido, total).is_empty());
    }

    #[test]
    fn qualquer_operacao_a_mais_antes_do_verso_e_acusada(
        total in 2u32..100,
        extra in operacao(),
    ) {
        // Um documento de duas ou mais páginas: a mão a mais muda a borda, a face ou a ordem.
        // A inversão não acusa com uma folha só, porque não há ordem a mudar.
        prop_assume!(total > 2 || extra != Operacao::InverterFolhaAFolha);
        let folhas = usize::try_from(total.div_ceil(2)).unwrap();
        let mut i = Impressora::nova(JATO, folhas);
        i.imprimir(&frentes_certas(total));
        i.recolocar_com(&[Operacao::VirarPelaArestaLonga, extra]);
        i.imprimir(&versos_certos(total));
        let motivos = conferir(&ler_maco(i.saida(), Leitura::ComoSaiu), total);
        prop_assert!(!motivos.is_empty());
    }
}
