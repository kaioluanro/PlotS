//! O modelo físico sozinho, sem o programa conferido.

use plotsim_physics::{
    conferir, ler_maco, BordaQueEntra, Entrada, Envio, ErroGesto, FaceNaEntrada, Gesto, Giro,
    Idioma, Impressora, Lado, Leitura, MacoLido, ModoImpressora, Motivo, Operacao, Pilha,
    TopoDaImagem, Uniao,
};

const RETO: ModoImpressora = ModoImpressora {
    entrada: Entrada::Traseira,
    puxa: Lado::Cima,
    imprime_na_face_para: Lado::Cima,
    sai_com_face_para: Lado::Cima,
    cai: Lado::Cima,
    topo_da_imagem: TopoDaImagem::NaArestaQueEntra,
};

const EM_U: ModoImpressora = ModoImpressora {
    entrada: Entrada::Gaveta,
    puxa: Lado::Cima,
    imprime_na_face_para: Lado::Cima,
    sai_com_face_para: Lado::Baixo,
    cai: Lado::Cima,
    topo_da_imagem: TopoDaImagem::NaArestaQueEntra,
};

fn impressa(modo: ModoImpressora, paginas: &[u32]) -> Impressora {
    let mut i = Impressora::nova(modo, paginas.len());
    let envios: Vec<Envio> = paginas.iter().map(|p| Envio::pagina(*p)).collect();
    i.imprimir(&envios);
    i
}

#[test]
fn caminho_reto_mantem_a_face_impressa_para_cima() {
    let i = impressa(RETO, &[1]);
    let lida = ler_maco(i.saida(), Leitura::ComoSaiu);
    assert_eq!(lida.folhas[0].frente, vec![1]);
}

#[test]
fn curva_em_u_deixa_a_face_impressa_para_baixo() {
    let i = impressa(EM_U, &[1]);
    let lida = ler_maco(i.saida(), Leitura::ComoSaiu);
    assert!(lida.folhas[0].frente.is_empty());
    assert_eq!(lida.folhas[0].verso, vec![1]);
}

#[test]
fn virar_a_pilha_inverte_a_ordem_e_girar_nao() {
    let i = impressa(RETO, &[6, 4, 2]);
    let ordem = |p: &Pilha| -> Vec<u32> {
        ler_maco(p, Leitura::ComoSaiu)
            .folhas
            .iter()
            .flat_map(|f| f.frente.iter().chain(f.verso.iter()).copied())
            .collect()
    };
    let inicial = i.saida().clone();
    assert_eq!(ordem(&inicial), vec![2, 4, 6]);

    let mut girada = inicial.clone();
    girada.aplicar(Operacao::Girar180);
    assert_eq!(ordem(&girada), vec![2, 4, 6]);

    for virada in [
        Operacao::VirarPelaArestaCurta,
        Operacao::VirarPelaArestaLonga,
    ] {
        let mut virada_pilha = inicial.clone();
        virada_pilha.aplicar(virada);
        assert_eq!(ordem(&virada_pilha), vec![6, 4, 2]);
    }

    let mut folha_a_folha = inicial.clone();
    folha_a_folha.aplicar(Operacao::InverterFolhaAFolha);
    assert_eq!(ordem(&folha_a_folha), vec![6, 4, 2]);
}

#[test]
fn bloco_de_notas_equivale_a_virar_como_livro_e_girar() {
    let i = impressa(RETO, &[6, 4, 2]);
    let mut bloco = i.saida().clone();
    bloco.aplicar(Operacao::VirarPelaArestaCurta);
    let mut composta = i.saida().clone();
    composta.aplicar(Operacao::VirarPelaArestaLonga);
    composta.aplicar(Operacao::Girar180);
    assert_eq!(bloco, composta);
}

/// Duas passadas num mesmo modo, com as operações dadas e giro opcional no verso.
fn duas_passadas(
    modo: ModoImpressora,
    frente: &[u32],
    ops: &[Operacao],
    verso: &[u32],
    giro: Giro,
) -> Impressora {
    let mut i = impressa(modo, frente);
    i.recolocar_com(ops);
    let envios: Vec<Envio> = verso.iter().map(|p| Envio::girada(*p, giro)).collect();
    i.imprimir(&envios);
    i
}

#[test]
fn virada_de_livro_sem_giro_fecha_na_maquina_de_caminho_reto() {
    let i = duas_passadas(
        RETO,
        &[6, 4, 2],
        &[Operacao::VirarPelaArestaLonga],
        &[5, 3, 1],
        Giro::Nenhum,
    );
    let maco = ler_maco(i.saida(), Leitura::ComoSaiu);
    assert_eq!(conferir(&maco, 6), vec![]);
}

#[test]
fn virada_de_bloco_de_notas_pede_o_verso_girado() {
    let certa = duas_passadas(
        RETO,
        &[6, 4, 2],
        &[Operacao::VirarPelaArestaCurta],
        &[5, 3, 1],
        Giro::Meia180,
    );
    assert_eq!(
        conferir(&ler_maco(certa.saida(), Leitura::ComoSaiu), 6),
        vec![]
    );

    let errada = duas_passadas(
        RETO,
        &[6, 4, 2],
        &[Operacao::VirarPelaArestaCurta],
        &[5, 3, 1],
        Giro::Nenhum,
    );
    let motivos = conferir(&ler_maco(errada.saida(), Leitura::ComoSaiu), 6);
    assert!(motivos.contains(&Motivo::VersoDeCabecaParaBaixo { folha: 1 }));
}

#[test]
fn curva_em_u_sem_virar_exige_girar_o_maco_ou_o_verso() {
    let parada = duas_passadas(EM_U, &[6, 4, 2], &[], &[1, 3, 5], Giro::Nenhum);
    let motivos = conferir(&ler_maco(parada.saida(), Leitura::VirandoOMaco), 6);
    assert!(motivos.contains(&Motivo::VersoDeCabecaParaBaixo { folha: 1 }));

    let maco_girado = duas_passadas(
        EM_U,
        &[6, 4, 2],
        &[Operacao::Girar180],
        &[1, 3, 5],
        Giro::Nenhum,
    );
    assert_eq!(
        conferir(&ler_maco(maco_girado.saida(), Leitura::VirandoOMaco), 6),
        vec![]
    );

    let verso_girado = duas_passadas(EM_U, &[6, 4, 2], &[], &[1, 3, 5], Giro::Meia180);
    assert_eq!(
        conferir(&ler_maco(verso_girado.saida(), Leitura::VirandoOMaco), 6),
        vec![]
    );
}

#[test]
fn o_defeito_da_curva_em_u_nao_depende_de_onde_a_maquina_poe_o_topo_da_imagem() {
    for topo_da_imagem in [TopoDaImagem::NaArestaQueEntra, TopoDaImagem::NaArestaQueSai] {
        let modo = ModoImpressora {
            topo_da_imagem,
            ..EM_U
        };
        let i = duas_passadas(modo, &[6, 4, 2], &[], &[1, 3, 5], Giro::Nenhum);
        let motivos = conferir(&ler_maco(i.saida(), Leitura::VirandoOMaco), 6);
        assert!(motivos.contains(&Motivo::VersoDeCabecaParaBaixo { folha: 1 }));
    }
}

#[test]
fn gesto_de_quatro_campos_escolhe_as_operacoes_pela_posicao_da_pilha() {
    let topo = Gesto {
        entrada: Entrada::Gaveta,
        face_impressa: FaceNaEntrada::ParaBaixo,
        borda_que_entra: BordaQueEntra::Topo,
        inverter_ordem: false,
    };
    let mut gaveta = impressa(EM_U, &[6, 4, 2]);
    assert_eq!(gaveta.recolocar(topo), Ok(vec![Operacao::Girar180]));

    let base = Gesto {
        borda_que_entra: BordaQueEntra::Base,
        ..topo
    };
    let mut gaveta = impressa(EM_U, &[6, 4, 2]);
    assert_eq!(gaveta.recolocar(base), Ok(vec![]));

    let traseira_base = Gesto {
        entrada: Entrada::Traseira,
        face_impressa: FaceNaEntrada::ParaTras,
        borda_que_entra: BordaQueEntra::Base,
        inverter_ordem: false,
    };
    let mut traseira = impressa(RETO, &[6, 4, 2]);
    assert_eq!(
        traseira.recolocar(traseira_base),
        Ok(vec![Operacao::VirarPelaArestaCurta])
    );
}

#[test]
fn gesto_impossivel_vira_erro_e_nao_mexe_na_pilha() {
    let mut i = impressa(EM_U, &[2]);
    let antes = i.saida().clone();
    let entrada_errada = Gesto {
        entrada: Entrada::Traseira,
        face_impressa: FaceNaEntrada::ParaTras,
        borda_que_entra: BordaQueEntra::Topo,
        inverter_ordem: false,
    };
    assert_eq!(
        i.recolocar(entrada_errada),
        Err(ErroGesto::EntradaDiferente {
            pedida: Entrada::Traseira,
            da_maquina: Entrada::Gaveta,
        })
    );
    let face_errada = Gesto {
        entrada: Entrada::Gaveta,
        face_impressa: FaceNaEntrada::ParaFrente,
        borda_que_entra: BordaQueEntra::Topo,
        inverter_ordem: false,
    };
    assert_eq!(
        i.recolocar(face_errada),
        Err(ErroGesto::FaceSemSentido {
            entrada: Entrada::Gaveta,
            face: FaceNaEntrada::ParaFrente,
        })
    );
    assert_eq!(i.saida(), &antes);
}

#[test]
fn papel_faltando_na_entrada_nao_trava_e_devolve_o_que_sobrou() {
    let mut i = Impressora::nova(RETO, 1);
    let resumo = i.imprimir(&[Envio::pagina(1), Envio::pagina(2)]);
    assert_eq!(resumo.sem_folha, vec![Envio::pagina(2)]);
    assert_eq!(i.saida().len(), 1);
}

#[test]
fn imprimir_sobre_a_face_impressa_e_acusado() {
    let i = duas_passadas(RETO, &[2], &[], &[1], Giro::Nenhum);
    let motivos = conferir(&ler_maco(i.saida(), Leitura::ComoSaiu), 2);
    assert!(motivos.contains(&Motivo::ImpressaoSobreImpressao { folha: 1 }));
}

#[test]
fn duas_faces_automaticas_cumprem_a_uniao_pela_geometria() {
    let pares = [
        (Envio::pagina(1), Envio::pagina(2)),
        (Envio::pagina(3), Envio::pagina(4)),
    ];
    let mut longa = Impressora::nova(RETO, 2);
    longa.imprimir_duas_faces(&pares, Uniao::PelaArestaLonga);
    assert_eq!(
        conferir(&ler_maco(longa.saida(), Leitura::VirandoOMaco), 4),
        vec![]
    );

    let mut curta = Impressora::nova(RETO, 2);
    curta.imprimir_duas_faces(&pares, Uniao::PelaArestaCurta);
    let motivos = conferir(&ler_maco(curta.saida(), Leitura::VirandoOMaco), 4);
    assert!(motivos.contains(&Motivo::VersoDeCabecaParaBaixo { folha: 1 }));
}

#[test]
fn os_motivos_dizem_a_causa_nos_dois_idiomas() {
    let casos = [
        (
            Motivo::VersoDeCabecaParaBaixo { folha: 2 },
            "verso de cabeça para baixo na folha 2",
            "back upside down on sheet 2",
        ),
        (
            Motivo::VersoTrocado {
                folha: 1,
                com_folha: 3,
                pagina: 6,
            },
            "verso trocado com a folha 3",
            "back swapped with sheet 3",
        ),
        (
            Motivo::FolhaAMenos {
                esperadas: 3,
                encontradas: 2,
            },
            "folha a menos",
            "one sheet short",
        ),
    ];
    for (motivo, pt, en) in casos {
        assert_eq!(motivo.texto(Idioma::PtBr), pt);
        assert_eq!(motivo.to_string(), pt);
        assert_eq!(motivo.texto(Idioma::En), en);
    }
}

#[test]
fn maco_vazio_de_documento_vazio_passa_e_de_documento_com_paginas_nao() {
    assert_eq!(conferir(&MacoLido::default(), 0), vec![]);
    assert_eq!(
        conferir(&MacoLido::default(), 3),
        vec![Motivo::FolhaAMenos {
            esperadas: 2,
            encontradas: 0
        }]
    );
    assert_eq!(
        conferir(&MacoLido::default(), u32::MAX),
        vec![Motivo::FolhaAMenos {
            esperadas: 2_147_483_648,
            encontradas: 0
        }]
    );
}

#[test]
fn pedido_de_papel_absurdo_para_no_teto() {
    assert_eq!(
        Pilha::de_papel(usize::MAX).len(),
        plotsim_physics::LIMITE_FOLHAS
    );
}
