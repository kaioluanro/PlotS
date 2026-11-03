//! Em que condições cada borda deixa os versos de cabeça para baixo.

mod comum;

use comum::*;
use plots_core::ModeloImpressora;
use plots_print::fluxo::Instrucao;
use plotsim_physics::{
    conferir, ler_maco, BordaQueEntra, Envio, Face, Giro, Impressora, Lado, Leitura,
    ModoImpressora, Motivo, Operacao, TopoDaImagem,
};

const OPERACOES: [(&str, &[Operacao]); 6] = [
    ("nada", &[]),
    ("girar", &[Operacao::Girar180]),
    ("livro", &[Operacao::VirarPelaArestaLonga]),
    ("bloco", &[Operacao::VirarPelaArestaCurta]),
    (
        "livro e girar",
        &[Operacao::VirarPelaArestaLonga, Operacao::Girar180],
    ),
    (
        "bloco e girar",
        &[Operacao::VirarPelaArestaCurta, Operacao::Girar180],
    ),
];

/// Qual borda da página impressa está na frente da folha de cima da entrada.
fn borda_na_entrada(i: &Impressora) -> Option<BordaQueEntra> {
    let folha = i.entrada().folhas().last()?;
    [Face::A, Face::B].iter().find_map(|face| {
        let impressao = folha.impressoes(*face).first()?;
        Some(if folha.topo_no_mundo(*impressao).y > 0 {
            BordaQueEntra::Topo
        } else {
            BordaQueEntra::Base
        })
    })
}

fn modos() -> [(&'static str, ModoImpressora); 4] {
    [
        ("caminho reto, topo na aresta que entra", JATO),
        ("curva em U, topo na aresta que entra", LASER),
        (
            "caminho reto, topo na aresta que sai",
            ModoImpressora {
                topo_da_imagem: TopoDaImagem::NaArestaQueSai,
                ..JATO
            },
        ),
        (
            "curva em U, topo na aresta que sai",
            ModoImpressora {
                topo_da_imagem: TopoDaImagem::NaArestaQueSai,
                ..LASER
            },
        ),
    ]
}

#[test]
fn a_borda_e_o_giro_decidem_sozinhos_se_o_verso_fica_em_pe() {
    println!("modo | operações | borda que entra | giro do verso | resultado");
    for (nome, modo) in modos() {
        let leitura = if modo.sai_com_face_para == Lado::Cima {
            Leitura::ComoSaiu
        } else {
            Leitura::VirandoOMaco
        };
        for (nome_ops, ops) in OPERACOES {
            for giro in [Giro::Nenhum, Giro::Meia180] {
                let mut i = Impressora::nova(modo, 1);
                i.imprimir(&[Envio::pagina(2)]);
                i.recolocar_com(ops);
                let borda = borda_na_entrada(&i).expect("há impressão");
                i.imprimir(&[Envio::girada(1, giro)]);
                let motivos = conferir(&ler_maco(i.saida(), leitura), 2);

                let sobre = motivos
                    .iter()
                    .any(|m| matches!(m, Motivo::ImpressaoSobreImpressao { .. }));
                let de_cabeca = motivos.contains(&Motivo::VersoDeCabecaParaBaixo { folha: 1 });
                println!(
                    "{nome} | {nome_ops} | {borda:?} | {giro:?} | {}",
                    if sobre {
                        "imprime sobre a face impressa"
                    } else if de_cabeca {
                        "verso de cabeça para baixo"
                    } else {
                        "em pé"
                    }
                );
                if sobre {
                    continue;
                }
                // A imagem nova entra com o topo onde a máquina o põe, girado ou não. Ela fecha
                // com a impressa só quando as duas apontam para a mesma aresta da folha.
                let topo_na_frente_sem_giro = modo.topo_da_imagem == TopoDaImagem::NaArestaQueEntra;
                let topo_novo_na_frente = topo_na_frente_sem_giro == (giro == Giro::Nenhum);
                let topo_velho_na_frente = borda == BordaQueEntra::Topo;
                assert_eq!(
                    !de_cabeca,
                    topo_novo_na_frente == topo_velho_na_frente,
                    "{nome}, {nome_ops}, {giro:?}"
                );
            }
        }
    }
}

#[test]
fn a_instrucao_de_cada_modelo_faz_a_base_entrar_primeiro() {
    let jato = Instrucao::do_modelo(ModeloImpressora::JatoDeTinta);
    let mut i = Impressora::nova(JATO, 1);
    i.imprimir(&[Envio::pagina(2)]);
    i.recolocar_com(&operacoes_da_virada(jato.virada));
    assert_eq!(borda_na_entrada(&i), Some(BordaQueEntra::Base));

    let laser = Instrucao::do_modelo(ModeloImpressora::Laser);
    let mut i = Impressora::nova(LASER, 1);
    i.imprimir(&[Envio::pagina(2)]);
    i.recolocar_com(&operacoes_da_virada(laser.virada));
    assert_eq!(borda_na_entrada(&i), Some(BordaQueEntra::Base));
}
