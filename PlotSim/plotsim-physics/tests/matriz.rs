//! A matriz: o programa conferido é a entrada, e o modelo físico é quem julga.

mod comum;

use comum::*;
use plots_core::distribuicao::{distribuir_lotes, ModoDistribuicao};
use plots_core::{
    plano_automatico_do_intervalo, DimensoesPagina, ModeloImpressora, OrdemPassadas, Pagina,
    PerfilImpressora, RepartoAutomatico,
};
use plots_print::fluxo::Instrucao;
use plots_print::{Mecanica, Simulador};
use plotsim_physics::{
    conferir, ler_maco, BordaQueEntra, Entrada, Envio, Impressora, Lado, Leitura, MacoLido,
    ModoImpressora, Motivo, TopoDaImagem, Uniao,
};

const MODELOS: [ModeloImpressora; 2] = [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser];

fn total_u32(total: usize) -> u32 {
    u32::try_from(total).expect("total cabe em u32")
}

#[test]
fn os_dois_modelos_passam_em_toda_a_matriz_com_a_borda_certa() {
    for modelo in MODELOS {
        for total in PAGINAS {
            for lote in LOTES {
                let motivos = conferir_modelo(modelo, total, lote);
                assert_eq!(motivos, vec![], "{modelo:?}, {total} páginas, lote {lote}");
            }
        }
    }
}

/// Cada lote do documento vai a uma fila; cada fila tem um modelo.
fn executar_distribuido(
    filas: &[ModeloImpressora],
    total: usize,
    lote: usize,
    modo: ModoDistribuicao,
) -> Vec<Vec<(usize, MacoLido)>> {
    let primeira = filas.first().expect("ao menos uma fila");
    let quantos = plano(total, lote, primeira.perfil()).len();
    let atribuicoes = distribuir_lotes(quantos, filas.len(), modo).expect("distribuição aceita");
    let mut por_fila = Vec::new();
    for atribuicao in atribuicoes {
        let modelo = filas[atribuicao.indice_impressora];
        let perfil = modelo.perfil();
        let gesto = gesto_do_modelo(modelo, borda_que_fecha(perfil));
        let lotes = plano(total, lote, perfil);
        let pacotes = atribuicao
            .indices_lotes()
            .map(|indice| {
                let pacote = executar_lote(
                    &lotes[indice],
                    modo_do_modelo(modelo),
                    &Recoloca::PeloGesto(gesto),
                    leitura_do_modelo(modelo),
                    Defeito::Nenhum,
                );
                (indice, pacote)
            })
            .collect();
        por_fila.push(pacotes);
    }
    por_fila
}

const PARES_DE_FILAS: [[ModeloImpressora; 2]; 3] = [
    [ModeloImpressora::JatoDeTinta, ModeloImpressora::JatoDeTinta],
    [ModeloImpressora::Laser, ModeloImpressora::Laser],
    [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser],
];

#[test]
fn no_espelho_cada_fila_entrega_o_documento_inteiro() {
    for filas in PARES_DE_FILAS {
        for total in PAGINAS {
            for lote in LOTES {
                let por_fila = executar_distribuido(&filas, total, lote, ModoDistribuicao::Espelho);
                assert_eq!(por_fila.len(), 2);
                for pacotes in por_fila {
                    let maco = juntar(pacotes.into_iter().map(|(_, p)| p));
                    assert_eq!(
                        conferir(&maco, total_u32(total)),
                        vec![],
                        "{filas:?}, {total} páginas, lote {lote}"
                    );
                }
            }
        }
    }
}

#[test]
fn na_divisao_os_pacotes_juntos_formam_o_documento() {
    for filas in PARES_DE_FILAS {
        for total in PAGINAS {
            for lote in LOTES {
                let mut pacotes: Vec<(usize, MacoLido)> =
                    executar_distribuido(&filas, total, lote, ModoDistribuicao::Divisao)
                        .into_iter()
                        .flatten()
                        .collect();
                pacotes.sort_by_key(|(i, _)| *i);
                let maco = juntar(pacotes.into_iter().map(|(_, p)| p));
                assert_eq!(
                    conferir(&maco, total_u32(total)),
                    vec![],
                    "{filas:?}, {total} páginas, lote {lote}"
                );
            }
        }
    }
}

/// Máquina de duas faces com caminho reto: a última face gravada sai para cima.
const MODO_DUAS_FACES: ModoImpressora = ModoImpressora {
    entrada: Entrada::Traseira,
    puxa: Lado::Cima,
    imprime_na_face_para: Lado::Cima,
    sai_com_face_para: Lado::Cima,
    cai: Lado::Cima,
    topo_da_imagem: TopoDaImagem::NaArestaQueEntra,
};

const A4: DimensoesPagina = DimensoesPagina::new(210.0, 297.0);

/// Os lotes automáticos em pares de frente e verso, na ordem em que o programa os manda.
fn lotes_automaticos(total: usize, lote: usize) -> Vec<Vec<(Envio, Envio)>> {
    let paginas = vec![A4; total];
    let reparto = if lote == 0 {
        RepartoAutomatico::PorTrechoDePapel
    } else {
        RepartoAutomatico::PorLote
    };
    let plano = plano_automatico_do_intervalo(
        &paginas,
        None,
        folhas_por_lote(lote, total),
        OrdemPassadas::ParesPrimeiro,
        ModeloImpressora::JatoDeTinta.perfil(),
        reparto,
    )
    .expect("o plano aceita os números da matriz");
    plano
        .iter()
        .map(|l| {
            assert!(l.duas_faces, "A4 segue pelo automático");
            assert!(l.passada2.is_empty());
            pares(&l.passada1)
        })
        .collect()
}

fn pares(paginas: &[Pagina]) -> Vec<(Envio, Envio)> {
    envios(paginas)
        .chunks(2)
        .map(|par| {
            let frente = par.first().copied().unwrap_or(Envio::branco());
            let verso = par.get(1).copied().unwrap_or(Envio::branco());
            (frente, verso)
        })
        .collect()
}

/// Cumpre o cartão: lê como saiu e, se a primeira página do lote não estiver em cima, vira o maço.
fn automatica(pares: &[(Envio, Envio)], modo: ModoImpressora) -> MacoLido {
    let mut impressora = Impressora::nova(modo, pares.len());
    impressora.imprimir_duas_faces(pares, Uniao::PelaArestaLonga);
    let primeira = pares.first().and_then(|(f, _)| f.pagina);
    let como_saiu = ler_maco(impressora.saida(), Leitura::ComoSaiu);
    let em_cima = como_saiu
        .folhas
        .first()
        .and_then(|f| f.frente.first())
        .copied();
    if em_cima == primeira {
        como_saiu
    } else {
        ler_maco(impressora.saida(), Leitura::VirandoOMaco)
    }
}

#[test]
fn o_automatico_passa_em_toda_a_matriz() {
    for total in PAGINAS {
        for lote in LOTES {
            let maco = juntar(
                lotes_automaticos(total, lote)
                    .iter()
                    .map(|pares| automatica(pares, MODO_DUAS_FACES)),
            );
            assert_eq!(
                conferir(&maco, total_u32(total)),
                vec![],
                "{total} páginas, lote {lote}"
            );
        }
    }
}

#[test]
fn o_mesmo_documento_da_o_mesmo_maco_em_todas_as_impressoras() {
    for total in PAGINAS {
        for lote in LOTES {
            let macos: Vec<MacoLido> = MODELOS
                .iter()
                .map(|modelo| {
                    let perfil = modelo.perfil();
                    let gesto = gesto_do_modelo(*modelo, borda_que_fecha(perfil));
                    juntar(plano(total, lote, perfil).iter().map(|l| {
                        executar_lote(
                            l,
                            modo_do_modelo(*modelo),
                            &Recoloca::PeloGesto(gesto),
                            leitura_do_modelo(*modelo),
                            Defeito::Nenhum,
                        )
                    }))
                })
                .collect();
            let automatico = juntar(
                lotes_automaticos(total, lote)
                    .iter()
                    .map(|pares| automatica(pares, MODO_DUAS_FACES)),
            );
            assert_eq!(macos[0], macos[1], "{total} páginas, lote {lote}");
            assert_eq!(macos[0], automatico, "{total} páginas, lote {lote}");
        }
    }
}

/// Uma mecânica com um perfil, cumprindo só o que a instrução nomeia, em toda a matriz.
fn mecanica_com_perfil(mecanica: Mecanica, perfil: PerfilImpressora) -> Vec<Motivo> {
    let instrucao = Instrucao::do_gesto(mecanica, recolocacao(perfil.gesto_recolocar));
    let ops = operacoes_da_virada(instrucao.virada);
    let leitura = if mecanica.saida_face_impressa_para_cima {
        Leitura::ComoSaiu
    } else {
        Leitura::VirandoOMaco
    };
    let modo = modo_da_mecanica(mecanica, Entrada::Manual, TopoDaImagem::NaArestaQueEntra);
    let mut motivos = Vec::new();
    for total in PAGINAS {
        for lote in LOTES {
            let maco = juntar(plano(total, lote, perfil).iter().map(|l| {
                executar_lote(
                    l,
                    modo,
                    &Recoloca::AoPeDaLetra(ops.clone()),
                    leitura,
                    Defeito::Nenhum,
                )
            }));
            motivos.extend(conferir(&maco, total_u32(total)));
        }
    }
    motivos
}

#[test]
fn cada_uma_das_dezesseis_mecanicas_tem_ao_menos_um_perfil_que_fecha() {
    let perfis = todos_os_perfis();
    let mut sem_saida = Vec::new();
    for mecanica in todas_as_mecanicas() {
        let validos: Vec<_> = perfis
            .iter()
            .filter(|p| mecanica_com_perfil(mecanica, **p).is_empty())
            .map(|p| {
                (
                    p.passada1_inversa,
                    p.passada2_inversa,
                    p.verso_girar_180,
                    p.gesto_recolocar.virada,
                )
            })
            .collect();
        println!(
            "{}: {} de {} perfis fecham: {validos:?}",
            nome_da_mecanica(mecanica),
            validos.len(),
            perfis.len(),
        );
        if validos.is_empty() {
            sem_saida.push(nome_da_mecanica(mecanica));
        }
    }
    assert_eq!(sem_saida, Vec::<String>::new());
}

/// O que o simulador do programa acusa nas seis páginas, só para comparar: quem julga é o modelo.
fn simulador_acusa(mecanica: Mecanica, perfil: PerfilImpressora) -> (bool, bool) {
    let lotes = plano(6, 0, perfil);
    let lote = lotes.first().expect("um lote");
    let mut sim = Simulador::novo(mecanica);
    sim.imprimir_frente(&lote.passada1).expect("frente");
    sim.recolocar(recolocacao(perfil.gesto_recolocar))
        .expect("recolocação");
    sim.imprimir_verso(&lote.passada2).expect("verso");
    let pilha = sim.folhas_para_leitura();
    let sobre = pilha.iter().any(|f| f.sobreimpressao.is_some());
    let de_cabeca = pilha.iter().any(|f| match (f.face_a, f.face_b) {
        (Some(a), Some(b)) => a.girada != b.girada,
        _ => false,
    });
    (sobre, de_cabeca)
}

/// O mesmo par de respostas, dado pelo modelo físico.
fn modelo_acusa(mecanica: Mecanica, perfil: PerfilImpressora) -> (bool, bool) {
    let instrucao = Instrucao::do_gesto(mecanica, recolocacao(perfil.gesto_recolocar));
    let ops = operacoes_da_virada(instrucao.virada);
    let leitura = if mecanica.saida_face_impressa_para_cima {
        Leitura::ComoSaiu
    } else {
        Leitura::VirandoOMaco
    };
    let modo = modo_da_mecanica(mecanica, Entrada::Manual, TopoDaImagem::NaArestaQueEntra);
    let maco = juntar(plano(6, 0, perfil).iter().map(|l| {
        executar_lote(
            l,
            modo,
            &Recoloca::AoPeDaLetra(ops.clone()),
            leitura,
            Defeito::Nenhum,
        )
    }));
    let motivos = conferir(&maco, 6);
    (
        motivos
            .iter()
            .any(|m| matches!(m, Motivo::ImpressaoSobreImpressao { .. })),
        motivos
            .iter()
            .any(|m| matches!(m, Motivo::VersoDeCabecaParaBaixo { .. })),
    )
}

#[test]
fn o_simulador_do_programa_concorda_com_o_modelo_em_cada_mecanica_e_perfil() {
    let mut divergencias = Vec::new();
    for mecanica in todas_as_mecanicas() {
        for perfil in todos_os_perfis() {
            let simulador = simulador_acusa(mecanica, perfil);
            let modelo = modelo_acusa(mecanica, perfil);
            if simulador != modelo {
                divergencias.push(format!(
                    "{}, {perfil:?}: simulador {simulador:?}, modelo {modelo:?}",
                    nome_da_mecanica(mecanica)
                ));
            }
        }
    }
    assert_eq!(divergencias, Vec::<String>::new());
}

#[test]
fn so_uma_borda_pode_fechar_e_o_giro_do_verso_decide_qual() {
    let modelo = ModeloImpressora::Laser;
    for (verso_girar_180, fecha, nao_fecha) in [
        (false, BordaQueEntra::Topo, BordaQueEntra::Base),
        (true, BordaQueEntra::Base, BordaQueEntra::Topo),
    ] {
        let perfil = PerfilImpressora {
            verso_girar_180,
            ..modelo.perfil()
        };
        let lotes = plano(6, 0, perfil);
        let lote = lotes.first().expect("um lote");
        let leitura = leitura_do_modelo(modelo);
        let com_borda = |borda| {
            executar_lote(
                lote,
                LASER,
                &Recoloca::PeloGesto(gesto_do_modelo(modelo, borda)),
                leitura,
                Defeito::Nenhum,
            )
        };
        assert_eq!(conferir(&com_borda(fecha), 6), vec![]);
        assert!(conferir(&com_borda(nao_fecha), 6)
            .contains(&Motivo::VersoDeCabecaParaBaixo { folha: 1 }));
    }
}

#[test]
fn duas_faces_com_o_verso_saindo_para_baixo_nao_fecham_na_ordem_do_documento() {
    // Frente, volta, verso: se a última face sai para baixo, a primeira folha fica no fundo com a
    // frente para cima. Nem como saiu nem virando o maço ele se lê em 1, 2, 3.
    let modo = ModoImpressora {
        sai_com_face_para: Lado::Baixo,
        ..MODO_DUAS_FACES
    };
    let lotes = lotes_automaticos(6, 0);
    let pares = lotes.first().expect("um lote");
    let mut impressora = Impressora::nova(modo, pares.len());
    impressora.imprimir_duas_faces(pares, Uniao::PelaArestaLonga);
    for leitura in [Leitura::ComoSaiu, Leitura::VirandoOMaco] {
        let motivos = conferir(&ler_maco(impressora.saida(), leitura), 6);
        assert!(!motivos.is_empty(), "{leitura:?}");
    }
}
