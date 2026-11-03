//! Toda página do documento sai exatamente uma vez quando os lotes são repartidos entre impressoras
//! de perfis diferentes.

use plots_core::distribuicao::{distribuir_lotes, ModoDistribuicao};
use plots_core::{
    plano_lotes_por_papel, BordaEntrada, DimensoesPagina, GestoRecolocar, OrdemPassadas,
    PerfilImpressora, ViradaMaco,
};
use proptest::prelude::*;

fn perfil() -> impl Strategy<Value = PerfilImpressora> {
    (any::<bool>(), any::<bool>(), any::<bool>()).prop_map(|(frente, verso, girar)| {
        PerfilImpressora {
            passada1_inversa: frente,
            passada2_inversa: verso,
            verso_girar_180: girar,
            gesto_recolocar: GestoRecolocar {
                virada: ViradaMaco::NaoVirar,
                borda_primeiro: BordaEntrada::Curta,
            },
        }
    })
}

fn papel() -> impl Strategy<Value = DimensoesPagina> {
    prop_oneof![
        Just(DimensoesPagina::new(210.0, 297.0)),
        Just(DimensoesPagina::new(148.0, 210.0)),
        Just(DimensoesPagina::new(297.0, 210.0)),
    ]
}

proptest! {
    #[test]
    fn toda_pagina_sai_exatamente_uma_vez(
        papeis_das_folhas in prop::collection::vec(papel(), 1..60),
        impar: bool,
        folhas in 1usize..12,
        perfis in prop::collection::vec(perfil(), 1..7),
    ) {
        // As duas faces de uma folha têm o mesmo papel; a última pode ficar sem verso.
        let mut paginas: Vec<DimensoesPagina> =
            papeis_das_folhas.iter().flat_map(|papel| [*papel, *papel]).collect();
        if impar {
            paginas.pop();
        }
        let total_lotes = plano_lotes_por_papel(
            &paginas,
            folhas,
            OrdemPassadas::default(),
            PerfilImpressora::default(),
        )
        .unwrap()
        .len();
        let divisao =
            distribuir_lotes(total_lotes, perfis.len(), ModoDistribuicao::Divisao).unwrap();
        let mut saidas = Vec::new();
        for (fila, perfil) in divisao.iter().zip(&perfis) {
            let plano =
                plano_lotes_por_papel(&paginas, folhas, OrdemPassadas::default(), *perfil).unwrap();
            prop_assert_eq!(plano.len(), total_lotes);
            for lote in fila.selecionar(plano) {
                saidas.extend(
                    lote.passada1
                        .iter()
                        .chain(&lote.passada2)
                        .filter_map(|pagina| pagina.numero),
                );
            }
        }
        saidas.sort_unstable();
        prop_assert_eq!(saidas, (1..=paginas.len()).collect::<Vec<_>>());
    }
}
