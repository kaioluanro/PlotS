//! Adaptador entre o programa conferido e o modelo físico. Só aqui os dois se encontram.
#![allow(dead_code)]

use plots_core::{
    plano_documento, BordaEntrada, GestoRecolocar, Lote, ModeloImpressora, OrdemPassadas, Pagina,
    PerfilImpressora, ViradaMaco,
};
use plots_print::fluxo::{self, Instrucao, Virada};
use plots_print::{Mecanica, Recolocacao};
use plotsim_physics::{
    conferir, ler_maco, BordaQueEntra, Entrada, Envio, FaceNaEntrada, Gesto, Giro, Impressora,
    Lado, Leitura, MacoLido, ModoImpressora, Motivo, Operacao, TopoDaImagem,
};

pub const PAGINAS: [usize; 8] = [1, 2, 3, 9, 10, 11, 24, 101];
/// Folhas por lote; o zero representa o documento inteiro.
pub const LOTES: [usize; 4] = [1, 5, 10, 0];

pub fn folhas_por_lote(lote: usize, total: usize) -> usize {
    if lote == 0 {
        total.div_ceil(2).max(1)
    } else {
        lote
    }
}

/// Entrada traseira, caminho reto: puxa a de cima, grava na face de cima e sai como entrou.
pub const JATO: ModoImpressora = ModoImpressora {
    entrada: Entrada::Traseira,
    puxa: Lado::Cima,
    imprime_na_face_para: Lado::Cima,
    sai_com_face_para: Lado::Cima,
    cai: Lado::Cima,
    topo_da_imagem: TopoDaImagem::NaArestaQueEntra,
};

/// Gaveta, curva em U: puxa a de cima, grava na face de cima e sai com ela para baixo.
pub const LASER: ModoImpressora = ModoImpressora {
    entrada: Entrada::Gaveta,
    puxa: Lado::Cima,
    imprime_na_face_para: Lado::Cima,
    sai_com_face_para: Lado::Baixo,
    cai: Lado::Cima,
    topo_da_imagem: TopoDaImagem::NaArestaQueEntra,
};

pub fn modo_do_modelo(modelo: ModeloImpressora) -> ModoImpressora {
    match modelo {
        ModeloImpressora::JatoDeTinta => JATO,
        ModeloImpressora::Laser => LASER,
    }
}

/// A mecânica que o programa conferido declara, escrita como modo do modelo. O programa não diz
/// qual é a entrada nem de que lado fica o topo da imagem: isso vem de quem chama.
pub fn modo_da_mecanica(m: Mecanica, entrada: Entrada, topo: TopoDaImagem) -> ModoImpressora {
    let lado = |cima: bool| if cima { Lado::Cima } else { Lado::Baixo };
    ModoImpressora {
        entrada,
        puxa: lado(m.entrada_pelo_topo),
        imprime_na_face_para: lado(m.imprime_face_para_cima),
        sai_com_face_para: lado(m.saida_face_impressa_para_cima),
        cai: lado(m.saida_no_topo),
        topo_da_imagem: topo,
    }
}

/// A virada da instrução, ao pé da letra: só as operações que ela nomeia, sem julgamento.
pub fn operacoes_da_virada(virada: Virada) -> Vec<Operacao> {
    match virada {
        Virada::PelaBordaDeCima => vec![Operacao::VirarPelaArestaCurta],
        Virada::PelaLateral => vec![Operacao::VirarPelaArestaLonga],
        Virada::NaoVirar => Vec::new(),
        Virada::Girar => vec![Operacao::Girar180],
    }
}

/// O gesto de quatro campos, como o programa o publica, sem nada dado por quem chama.
pub fn gesto_publicado(instrucao: Instrucao) -> Gesto {
    Gesto {
        entrada: match instrucao.entrada {
            fluxo::EntradaDoMaco::Traseira => Entrada::Traseira,
            fluxo::EntradaDoMaco::Gaveta => Entrada::Gaveta,
            fluxo::EntradaDoMaco::Manual => Entrada::Manual,
        },
        face_impressa: match instrucao.face_impressa {
            fluxo::FaceImpressa::ParaFrente => FaceNaEntrada::ParaFrente,
            fluxo::FaceImpressa::ParaTras => FaceNaEntrada::ParaTras,
            fluxo::FaceImpressa::ParaCima => FaceNaEntrada::ParaCima,
            fluxo::FaceImpressa::ParaBaixo => FaceNaEntrada::ParaBaixo,
        },
        borda_que_entra: match instrucao.borda_que_entra {
            fluxo::BordaQueEntra::Topo => BordaQueEntra::Topo,
            fluxo::BordaQueEntra::Base => BordaQueEntra::Base,
        },
        inverter_ordem: instrucao.inverter_ordem,
    }
}

/// O gesto de quatro campos com a entrada e a borda escolhidas por quem chama, para testar
/// hipóteses que o programa não afirma.
pub fn gesto_da_instrucao(
    instrucao: Instrucao,
    entrada: Entrada,
    borda_que_entra: BordaQueEntra,
    inverter_ordem: bool,
) -> Gesto {
    let cima = matches!(instrucao.lado, fluxo::Lado::Cima);
    let face_impressa = match (entrada, cima) {
        (Entrada::Traseira, true) => FaceNaEntrada::ParaFrente,
        (Entrada::Traseira, false) => FaceNaEntrada::ParaTras,
        (_, true) => FaceNaEntrada::ParaCima,
        (_, false) => FaceNaEntrada::ParaBaixo,
    };
    Gesto {
        entrada,
        face_impressa,
        borda_que_entra,
        inverter_ordem,
    }
}

pub fn envios(paginas: &[Pagina]) -> Vec<Envio> {
    paginas
        .iter()
        .map(|p| Envio {
            pagina: p
                .numero
                .map(|n| u32::try_from(n).expect("página cabe em u32")),
            giro: if p.girar_180 {
                Giro::Meia180
            } else {
                Giro::Nenhum
            },
        })
        .collect()
}

/// Como a pessoa recoloca.
#[derive(Clone, Debug)]
pub enum Recoloca {
    /// Só as operações nomeadas.
    AoPeDaLetra(Vec<Operacao>),
    /// O gesto de quatro campos, cumprido olhando a pilha.
    PeloGesto(Gesto),
}

/// Defeito injetado no maço entre as passadas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Defeito {
    Nenhum,
    /// Uma folha some de cima da pilha antes de recolocar.
    FolhaAMenos,
}

/// Imprime um lote nas duas passadas e devolve o maço lido.
pub fn executar_lote(
    lote: &Lote,
    modo: ModoImpressora,
    recoloca: &Recoloca,
    leitura: Leitura,
    defeito: Defeito,
) -> MacoLido {
    let frente = envios(&lote.passada1);
    let verso = envios(&lote.passada2);
    let mut impressora = Impressora::nova(modo, frente.len());
    let primeira = impressora.imprimir(&frente);
    assert!(primeira.sem_folha.is_empty(), "papel exato na entrada");
    if defeito == Defeito::FolhaAMenos {
        impressora.saida_mut().retirar_de_cima(0);
    }
    match recoloca {
        Recoloca::AoPeDaLetra(ops) => impressora.recolocar_com(ops),
        Recoloca::PeloGesto(gesto) => {
            impressora.recolocar(*gesto).expect("gesto cabe na máquina");
        }
    }
    impressora.imprimir(&verso);
    ler_maco(impressora.saida(), leitura)
}

/// Pacotes de cada lote, juntos na ordem do documento: a pessoa tira cada lote da bandeja e põe o
/// seguinte embaixo do anterior.
pub fn juntar(pacotes: impl IntoIterator<Item = MacoLido>) -> MacoLido {
    let mut maco = MacoLido::default();
    for pacote in pacotes {
        maco.folhas.extend(pacote.folhas);
    }
    maco
}

pub fn plano(total: usize, lote: usize, perfil: PerfilImpressora) -> Vec<Lote> {
    plano_documento(
        total,
        folhas_por_lote(lote, total),
        OrdemPassadas::ParesPrimeiro,
        perfil,
    )
    .expect("o plano aceita os números da matriz")
}

pub fn leitura_do_modelo(modelo: ModeloImpressora) -> Leitura {
    if fluxo::vira_para_ler(modelo) {
        Leitura::VirandoOMaco
    } else {
        Leitura::ComoSaiu
    }
}

/// O gesto que o programa dá ao modelo, com a borda dada por quem chama.
pub fn gesto_do_modelo(modelo: ModeloImpressora, borda: BordaQueEntra) -> Gesto {
    let modo = modo_do_modelo(modelo);
    gesto_da_instrucao(Instrucao::do_modelo(modelo), modo.entrada, borda, false)
}

/// A borda que fecha com o giro do plano: verso girado pede a base; sem giro, o topo. Nos dois
/// modelos o verso vai girado, e cumprir a instrução ao pé da letra já faz a base entrar primeiro.
pub fn borda_que_fecha(perfil: PerfilImpressora) -> BordaQueEntra {
    if perfil.verso_girar_180 {
        BordaQueEntra::Base
    } else {
        BordaQueEntra::Topo
    }
}

/// Documento inteiro em um modelo, com a borda que o modelo físico exige.
pub fn conferir_modelo(modelo: ModeloImpressora, total: usize, lote: usize) -> Vec<Motivo> {
    let perfil = modelo.perfil();
    let gesto = gesto_do_modelo(modelo, borda_que_fecha(perfil));
    let lotes = plano(total, lote, perfil);
    let maco = juntar(lotes.iter().map(|l| {
        executar_lote(
            l,
            modo_do_modelo(modelo),
            &Recoloca::PeloGesto(gesto),
            leitura_do_modelo(modelo),
            Defeito::Nenhum,
        )
    }));
    conferir(&maco, u32::try_from(total).expect("total cabe em u32"))
}

/// Todos os perfis possíveis: oito combinações de envio para cada um dos três gestos.
pub fn todos_os_perfis() -> Vec<PerfilImpressora> {
    let mut perfis = Vec::new();
    for virada in [
        ViradaMaco::NaoVirar,
        ViradaMaco::PelaLateral,
        ViradaMaco::PelaBordaDeCima,
    ] {
        for bits in 0u8..8 {
            perfis.push(PerfilImpressora {
                passada1_inversa: bits & 1 != 0,
                passada2_inversa: bits & 2 != 0,
                verso_girar_180: bits & 4 != 0,
                gesto_recolocar: GestoRecolocar {
                    virada,
                    borda_primeiro: BordaEntrada::Curta,
                },
            });
        }
    }
    perfis
}

/// As dezesseis mecânicas de saída, entrada e face, sem giro feito pela própria máquina.
pub fn todas_as_mecanicas() -> Vec<Mecanica> {
    (0u8..16)
        .map(|bits| Mecanica {
            saida_face_impressa_para_cima: bits & 1 != 0,
            saida_no_topo: bits & 2 != 0,
            imprime_face_para_cima: bits & 4 != 0,
            entrada_pelo_topo: bits & 8 != 0,
            giro_segunda_passada: false,
        })
        .collect()
}

pub fn nome_da_mecanica(m: Mecanica) -> String {
    let lado = |v: bool| if v { "cima" } else { "baixo" };
    format!(
        "puxa {}, grava {}, sai {}, cai {}",
        lado(m.entrada_pelo_topo),
        lado(m.imprime_face_para_cima),
        lado(m.saida_face_impressa_para_cima),
        lado(m.saida_no_topo),
    )
}

/// Recolocação do programa a partir de um gesto de perfil.
pub fn recolocacao(gesto: GestoRecolocar) -> Recolocacao {
    gesto.into()
}
