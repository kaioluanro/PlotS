//! A calibração guiada, julgada pelo modelo físico: quem responde às perguntas é a própria pilha de
//! folhas, olhada como uma pessoa olharia, e quem diz se o maço saiu certo é a conferência daqui.

mod comum;

use comum::{envios, juntar, plano, LOTES, PAGINAS};
use plots_core::calibracao::{
    avaliar, conferir_automatica, face_impressa_entra_para_cima, ConferenciaAutomatica,
    LadoDoVerso, LeituraDoMaco, PerfilEmTeste, Respostas, RODADAS_MAXIMAS,
};
use plots_core::{
    BordaEntrada, GestoRecolocar, Lote, ModeloImpressora, Pagina, PerfilImpressora, ViradaMaco,
};
use plotsim_physics::{
    conferir, ler_maco, Entrada, Envio, Face, Folha, Impressora, Lado, Leitura, MacoLido,
    ModoImpressora, Operacao, Pilha, TopoDaImagem, Uniao,
};
use std::collections::{BTreeMap, HashSet};

const LADOS: [Lado; 2] = [Lado::Cima, Lado::Baixo];
/// Com todas as respostas certas, nenhuma máquina pode precisar de mais que isto. O resto do limite
/// é a folga para um engano.
const RODADAS_COM_RESPOSTAS_CERTAS: u8 = 3;

/// Uma impressora real: a mecânica e o que o driver faz com cada trabalho.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Maquina {
    modo: ModoImpressora,
    driver_inverte: bool,
}

fn todas_as_maquinas() -> Vec<Maquina> {
    let mut maquinas = Vec::new();
    for entrada in [Entrada::Traseira, Entrada::Gaveta, Entrada::Manual] {
        for puxa in LADOS {
            for imprime_na_face_para in LADOS {
                for sai_com_face_para in LADOS {
                    for cai in LADOS {
                        for topo_da_imagem in
                            [TopoDaImagem::NaArestaQueEntra, TopoDaImagem::NaArestaQueSai]
                        {
                            for driver_inverte in [false, true] {
                                maquinas.push(Maquina {
                                    modo: ModoImpressora {
                                        entrada,
                                        puxa,
                                        imprime_na_face_para,
                                        sai_com_face_para,
                                        cai,
                                        topo_da_imagem,
                                    },
                                    driver_inverte,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    maquinas
}

fn todos_os_inicios() -> Vec<PerfilEmTeste> {
    let mut inicios = Vec::new();
    for virada in [
        ViradaMaco::NaoVirar,
        ViradaMaco::PelaLateral,
        ViradaMaco::PelaBordaDeCima,
    ] {
        for borda_primeiro in [BordaEntrada::Curta, BordaEntrada::Longa] {
            for bits in 0u8..16 {
                inicios.push(PerfilEmTeste {
                    perfil: PerfilImpressora {
                        passada1_inversa: bits & 1 != 0,
                        passada2_inversa: bits & 2 != 0,
                        verso_girar_180: bits & 4 != 0,
                        gesto_recolocar: GestoRecolocar {
                            virada,
                            borda_primeiro,
                        },
                    },
                    driver_inverte: bits & 8 != 0,
                });
            }
        }
    }
    inicios
}

/// O que chega à máquina em uma passada: a compensação do programa e, depois, o driver.
fn no_papel(paginas: &[Pagina], compensa: bool, maquina: Maquina) -> Vec<Envio> {
    let mut lista = envios(paginas);
    if compensa != maquina.driver_inverte {
        lista.reverse();
    }
    lista
}

/// A pessoa só faz o que a frase nomeia.
fn operacoes(virada: ViradaMaco) -> &'static [Operacao] {
    match virada {
        ViradaMaco::NaoVirar => &[],
        ViradaMaco::PelaLateral => &[Operacao::VirarPelaArestaLonga],
        ViradaMaco::PelaBordaDeCima => &[Operacao::VirarPelaArestaCurta],
    }
}

/// O que sobrou de uma rodada: o maço final, e o que a pessoa pôde ver no caminho.
struct Rodada {
    saida: Pilha,
    saiu_para_cima: bool,
    entrou_para_cima: bool,
    primeira_a_sair: Option<u32>,
}

/// A face impressa da folha de cima está à vista?
fn impressa_a_vista(pilha: &Pilha) -> bool {
    pilha
        .folhas()
        .last()
        .is_some_and(|folha| !folha.impressoes(folha.face_para(Lado::Cima)).is_empty())
}

fn imprimir_lote(lote: &Lote, maquina: Maquina, testado: PerfilEmTeste) -> Rodada {
    let frente = no_papel(&lote.passada1, testado.driver_inverte, maquina);
    let verso = no_papel(&lote.passada2, testado.driver_inverte, maquina);
    let mut impressora = Impressora::nova(maquina.modo, frente.len());
    impressora.imprimir(&frente);
    let saiu_para_cima = impressa_a_vista(impressora.saida());
    impressora.recolocar_com(operacoes(testado.perfil.gesto_recolocar.virada));
    let entrou_para_cima = impressa_a_vista(impressora.entrada());
    impressora.imprimir(&verso);
    Rodada {
        saida: impressora.retirar_saida(),
        saiu_para_cima,
        entrou_para_cima,
        primeira_a_sair: verso.first().and_then(|envio| envio.pagina),
    }
}

/// A página à vista na folha de cima de uma pilha.
fn na_folha_de_cima(pilha: &Pilha) -> Option<u32> {
    let folha = pilha.folhas().last()?;
    folha
        .impressoes(folha.face_para(Lado::Cima))
        .first()
        .map(|impressao| impressao.pagina)
}

fn outra(face: Face) -> Face {
    match face {
        Face::A => Face::B,
        Face::B => Face::A,
    }
}

fn folha_com(pilha: &Pilha, pagina: u32) -> Option<(&Folha, Face)> {
    pilha.folhas().iter().find_map(|folha| {
        [Face::A, Face::B]
            .into_iter()
            .find(|face| folha.impressoes(*face).iter().any(|i| i.pagina == pagina))
            .map(|face| (folha, face))
    })
}

/// Olha a pilha como quem tem as folhas na mão, sem saber nada da máquina nem do perfil.
fn responder(saida: &Pilha, primeira_a_sair: Option<u32>) -> Respostas {
    let por_cima = saida
        .folhas()
        .iter()
        .any(|folha| folha.impressoes(Face::A).len() > 1 || folha.impressoes(Face::B).len() > 1);
    let verso = if por_cima {
        LadoDoVerso::PorCima
    } else {
        LadoDoVerso::EmBranco
    };
    let do_par = folha_com(saida, 1).and_then(|(folha, face)| {
        let atras = folha.impressoes(outra(face)).first()?;
        let aqui = folha.impressoes(face).first()?;
        Some((
            atras.pagina,
            folha.topo_no_mundo(*aqui),
            folha.topo_no_mundo(*atras),
        ))
    });
    let par_certo = do_par.is_some_and(|(atras, _, _)| atras == 2);
    // Virar como página de livro mantém o sentido de cima a baixo: basta comparar os topos.
    let setas_iguais = do_par.is_some_and(|(_, aqui, atras)| aqui == atras);
    let mut virado = saida.clone();
    virado.aplicar(Operacao::VirarPelaArestaLonga);
    let leitura = if na_folha_de_cima(saida) == Some(1) {
        LeituraDoMaco::SemVirar
    } else if na_folha_de_cima(&virado) == Some(1) {
        LeituraDoMaco::Virando
    } else {
        LeituraDoMaco::NaoFica
    };
    Respostas {
        verso,
        par_certo,
        setas_iguais,
        leitura,
        primeira_a_sair: primeira_a_sair.and_then(|pagina| usize::try_from(pagina).ok()),
    }
}

/// O fim de uma calibração: o perfil gravado e o que ela aprendeu do maço.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Calibrada {
    perfil: PerfilEmTeste,
    leitura: Leitura,
    rodadas: u8,
}

fn calibrar(maquina: Maquina, inicio: PerfilEmTeste, reparou: bool) -> Option<Calibrada> {
    let mut testado = inicio;
    for rodada in 1..=RODADAS_MAXIMAS {
        let lote = plano(4, 0, testado.perfil).into_iter().next()?;
        let saida = imprimir_lote(&lote, maquina, testado);
        // A frase de colocação tem de dizer o lado em que o maço realmente entra.
        assert_eq!(
            face_impressa_entra_para_cima(
                testado.perfil.gesto_recolocar.virada,
                saida.saiu_para_cima
            ),
            saida.entrou_para_cima,
            "{maquina:?} {testado:?}"
        );
        let vista = saida.primeira_a_sair.filter(|_| reparou);
        let respostas = responder(&saida.saida, vista);
        let avaliacao = avaliar(testado, respostas);
        if avaliacao.certo {
            assert_eq!(avaliacao.mudancas, vec![]);
            let leitura = match respostas.leitura {
                LeituraDoMaco::SemVirar => Leitura::ComoSaiu,
                _ => Leitura::VirandoOMaco,
            };
            // Quem lê sem virar é quem saiu com a face à vista.
            assert_eq!(leitura == Leitura::ComoSaiu, saida.saiu_para_cima);
            return Some(Calibrada {
                perfil: testado,
                leitura,
                rodadas: rodada,
            });
        }
        testado = avaliacao.proximo;
    }
    None
}

fn imprimir_documento(
    maquina: Maquina,
    calibrada: PerfilEmTeste,
    leitura: Leitura,
    total: usize,
    lote: usize,
) -> MacoLido {
    juntar(
        plano(total, lote, calibrada.perfil)
            .iter()
            .map(|l| ler_maco(&imprimir_lote(l, maquina, calibrada).saida, leitura)),
    )
}

fn nome_do_lado(lado: Lado) -> &'static str {
    match lado {
        Lado::Cima => "cima",
        Lado::Baixo => "baixo",
    }
}

#[test]
fn toda_maquina_converge_de_todo_perfil_inicial_e_o_maco_final_sai_certo() {
    let maquinas = todas_as_maquinas();
    let inicios = todos_os_inicios();
    let mut sem_convergir = Vec::new();
    let mut pior_por_maquina: BTreeMap<String, (u8, usize)> = BTreeMap::new();
    let mut por_rodadas = [0usize; 4];
    let mut vistos: HashSet<String> = HashSet::new();
    let mut julgados: Vec<(Maquina, PerfilEmTeste, Leitura)> = Vec::new();
    for maquina in &maquinas {
        let m = maquina.modo;
        let chave = format!(
            "puxa {}, grava {}, sai {}, cai {}, driver {}",
            nome_do_lado(m.puxa),
            nome_do_lado(m.imprime_na_face_para),
            nome_do_lado(m.sai_com_face_para),
            nome_do_lado(m.cai),
            if maquina.driver_inverte {
                "inverte"
            } else {
                "normal"
            },
        );
        for inicio in &inicios {
            for reparou in [false, true] {
                let Some(fim) = calibrar(*maquina, *inicio, reparou) else {
                    sem_convergir.push((*maquina, *inicio, reparou));
                    continue;
                };
                let entrada = pior_por_maquina.entry(chave.clone()).or_insert((0, 0));
                entrada.0 = entrada.0.max(fim.rodadas);
                entrada.1 += 1;
                assert!(
                    fim.rodadas <= RODADAS_COM_RESPOSTAS_CERTAS,
                    "{maquina:?} {inicio:?} levou {}",
                    fim.rodadas
                );
                if let Some(contagem) = por_rodadas.get_mut(usize::from(fim.rodadas)) {
                    *contagem += 1;
                }
                // Só muda a compensação quem viu a primeira folha, e muda para o que o driver faz.
                if fim.perfil.driver_inverte != inicio.driver_inverte {
                    assert!(reparou, "{maquina:?} {inicio:?}");
                    assert_eq!(fim.perfil.driver_inverte, maquina.driver_inverte);
                }
                if vistos.insert(format!("{maquina:?} {:?} {:?}", fim.perfil, fim.leitura)) {
                    julgados.push((*maquina, fim.perfil, fim.leitura));
                }
            }
        }
    }
    println!("mecânica | pior rodada | calibrações");
    for (chave, (pior, quantas)) in &pior_por_maquina {
        println!("{chave} | {pior} | {quantas}");
    }
    println!(
        "uma rodada: {}; duas: {}; três: {}",
        por_rodadas[1], por_rodadas[2], por_rodadas[3]
    );
    assert_eq!(sem_convergir, vec![], "não convergiram");

    // O teste de 4 páginas fechar não basta: o perfil gravado tem de servir a qualquer documento.
    println!("perfis finais julgados: {}", julgados.len());
    let mut errados = Vec::new();
    for (maquina, perfil, leitura) in &julgados {
        for total in PAGINAS {
            for lote in LOTES {
                let maco = imprimir_documento(*maquina, *perfil, *leitura, total, lote);
                let motivos = conferir(&maco, u32::try_from(total).unwrap_or(u32::MAX));
                if !motivos.is_empty() {
                    errados.push((*maquina, *perfil, total, lote, motivos));
                }
            }
        }
    }
    assert_eq!(errados.len(), 0, "{:?}", errados.first());
}

/// Uma resposta dada errado, sem que a pessoa perceba.
#[derive(Clone, Copy, Debug)]
enum Engano {
    Lado,
    Par,
    Setas,
    Leitura(LeituraDoMaco),
    Primeira(Option<usize>),
}

const ENGANOS: [Engano; 8] = [
    Engano::Lado,
    Engano::Par,
    Engano::Setas,
    Engano::Leitura(LeituraDoMaco::SemVirar),
    Engano::Leitura(LeituraDoMaco::Virando),
    Engano::Leitura(LeituraDoMaco::NaoFica),
    Engano::Primeira(Some(1)),
    Engano::Primeira(Some(3)),
];

/// As respostas com o engano. `None` quando ele não existe: a pergunta não foi feita nesta rodada,
/// ou a resposta errada é igual à certa.
fn com_engano(certas: Respostas, engano: Engano) -> Option<Respostas> {
    let por_cima = certas.verso == LadoDoVerso::PorCima;
    let mudada = match engano {
        Engano::Lado => Respostas {
            verso: if por_cima {
                LadoDoVerso::EmBranco
            } else {
                LadoDoVerso::PorCima
            },
            ..certas
        },
        _ if por_cima => return None,
        Engano::Par => Respostas {
            par_certo: !certas.par_certo,
            ..certas
        },
        Engano::Setas => Respostas {
            setas_iguais: !certas.setas_iguais,
            ..certas
        },
        Engano::Leitura(leitura) => Respostas { leitura, ..certas },
        Engano::Primeira(_) if certas.leitura != LeituraDoMaco::NaoFica => return None,
        Engano::Primeira(primeira_a_sair) => Respostas {
            primeira_a_sair,
            ..certas
        },
    };
    (mudada != certas).then_some(mudada)
}

/// Onde termina uma calibração com um engano.
#[derive(Debug)]
enum FimComEngano {
    /// A rodada do engano não chegou a acontecer, ou nela essa pergunta não existia.
    NaoHouve,
    /// O engano foi a última resposta que faltava para dizer que está certo. Nenhuma pergunta
    /// seguinte existe para desmenti-lo.
    AceitoSemVolta,
    Certa(Calibrada),
    NaoConvergiu,
}

fn calibrar_com_engano(
    maquina: Maquina,
    inicio: PerfilEmTeste,
    reparou: bool,
    rodada_do_engano: u8,
    engano: Engano,
) -> FimComEngano {
    let mut testado = inicio;
    for rodada in 1..=RODADAS_MAXIMAS {
        let Some(lote) = plano(4, 0, testado.perfil).into_iter().next() else {
            return FimComEngano::NaoConvergiu;
        };
        let saida = imprimir_lote(&lote, maquina, testado);
        let vista = saida.primeira_a_sair.filter(|_| reparou);
        let certas = responder(&saida.saida, vista);
        let dadas = if rodada == rodada_do_engano {
            let Some(mudada) = com_engano(certas, engano) else {
                return FimComEngano::NaoHouve;
            };
            mudada
        } else {
            certas
        };
        let avaliacao = avaliar(testado, dadas);
        if avaliacao.certo {
            if rodada < rodada_do_engano {
                return FimComEngano::NaoHouve;
            }
            let leitura_certa = dadas.leitura == certas.leitura;
            if !avaliar(testado, certas).certo || !leitura_certa {
                return FimComEngano::AceitoSemVolta;
            }
            return FimComEngano::Certa(Calibrada {
                perfil: testado,
                leitura: if certas.leitura == LeituraDoMaco::SemVirar {
                    Leitura::ComoSaiu
                } else {
                    Leitura::VirandoOMaco
                },
                rodadas: rodada,
            });
        }
        testado = avaliacao.proximo;
    }
    FimComEngano::NaoConvergiu
}

#[test]
fn um_engano_em_uma_rodada_ainda_converge_dentro_do_limite() {
    let mut sem_convergir = Vec::new();
    let mut por_rodadas = [0usize; 6];
    let mut aceitos: BTreeMap<String, usize> = BTreeMap::new();
    let mut vistos: HashSet<String> = HashSet::new();
    let mut julgados: Vec<(Maquina, PerfilEmTeste, Leitura)> = Vec::new();
    for maquina in todas_as_maquinas() {
        for inicio in todos_os_inicios() {
            for reparou in [false, true] {
                for rodada in 1..=RODADAS_COM_RESPOSTAS_CERTAS {
                    for engano in ENGANOS {
                        match calibrar_com_engano(maquina, inicio, reparou, rodada, engano) {
                            FimComEngano::NaoHouve => {}
                            FimComEngano::AceitoSemVolta => {
                                *aceitos.entry(format!("{engano:?}")).or_insert(0) += 1;
                            }
                            FimComEngano::NaoConvergiu => {
                                sem_convergir.push((maquina, inicio, reparou, rodada, engano));
                            }
                            FimComEngano::Certa(fim) => {
                                if let Some(contagem) =
                                    por_rodadas.get_mut(usize::from(fim.rodadas))
                                {
                                    *contagem += 1;
                                }
                                let chave =
                                    format!("{maquina:?} {:?} {:?}", fim.perfil, fim.leitura);
                                if vistos.insert(chave) {
                                    julgados.push((maquina, fim.perfil, fim.leitura));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    println!(
        "com um engano, rodadas até dar certo: {:?}",
        &por_rodadas[1..]
    );
    println!("engano aceito como certo, sem pergunta que o desminta: {aceitos:?}");
    assert_eq!(
        sem_convergir.len(),
        0,
        "não convergiram: {:?}",
        sem_convergir.first()
    );
    let mut errados = Vec::new();
    for (maquina, perfil, leitura) in &julgados {
        for total in PAGINAS {
            for lote in LOTES {
                let maco = imprimir_documento(*maquina, *perfil, *leitura, total, lote);
                let motivos = conferir(&maco, u32::try_from(total).unwrap_or(u32::MAX));
                if !motivos.is_empty() {
                    errados.push((*maquina, *perfil, total, lote, motivos));
                }
            }
        }
    }
    assert_eq!(errados.len(), 0, "{:?}", errados.first());
}

#[test]
fn perfil_do_jato_em_maquina_de_gaveta_comeca_imprimindo_por_cima() {
    let gaveta = Maquina {
        modo: comum::LASER,
        driver_inverte: false,
    };
    let inicio = PerfilEmTeste::do_modelo(ModeloImpressora::JatoDeTinta, false);
    let lote = plano(4, 0, inicio.perfil)
        .into_iter()
        .next()
        .expect("um lote");
    let rodada = imprimir_lote(&lote, gaveta, inicio);
    assert_eq!(responder(&rodada.saida, None).verso, LadoDoVerso::PorCima);
}

#[test]
fn driver_que_inverte_so_e_reconhecido_por_quem_viu_a_primeira_folha() {
    let maquina = Maquina {
        modo: comum::JATO,
        driver_inverte: true,
    };
    let inicio = PerfilEmTeste::do_modelo(ModeloImpressora::JatoDeTinta, false);
    let viu = calibrar(maquina, inicio, true).expect("converge");
    assert!(viu.perfil.driver_inverte);
    assert_eq!(viu.perfil.perfil, inicio.perfil, "a ordem do modelo fica");
    let nao_viu = calibrar(maquina, inicio, false).expect("converge");
    assert!(!nao_viu.perfil.driver_inverte);
    assert_ne!(
        nao_viu.perfil.perfil, inicio.perfil,
        "sem saber do driver, a ordem é que muda"
    );
}

#[test]
fn entrada_e_lado_do_topo_da_imagem_nao_mudam_o_perfil_final() {
    // O que as respostas não podem distinguir não pode ser preciso distinguir.
    let inicio = PerfilEmTeste::do_modelo(ModeloImpressora::JatoDeTinta, false);
    let mut por_mecanica = std::collections::HashMap::new();
    for maquina in todas_as_maquinas() {
        let m = maquina.modo;
        let chave = (
            m.puxa,
            m.imprime_na_face_para,
            m.sai_com_face_para,
            m.cai,
            maquina.driver_inverte,
        );
        let fim = calibrar(maquina, inicio, false).expect("converge").perfil;
        assert_eq!(
            *por_mecanica.entry(chave).or_insert(fim),
            fim,
            "{maquina:?}"
        );
    }
}

/// Máquina de duas faces, sob o mesmo olhar: quem olha só vê as folhas.
fn conferencia_automatica(modo: ModoImpressora, uniao: Uniao) -> (ConferenciaAutomatica, bool) {
    let pares = [
        (Envio::pagina(1), Envio::pagina(2)),
        (Envio::pagina(3), Envio::pagina(4)),
    ];
    let mut impressora = Impressora::nova(modo, pares.len());
    impressora.imprimir_duas_faces(&pares, uniao);
    let saida = impressora.retirar_saida();
    let r = responder(&saida, None);
    let julgou = conferir_automatica(r.par_certo, r.setas_iguais, r.leitura);
    let algum_jeito_serve = [Leitura::ComoSaiu, Leitura::VirandoOMaco]
        .into_iter()
        .any(|leitura| conferir(&ler_maco(&saida, leitura), 4).is_empty());
    (julgou, algum_jeito_serve)
}

#[test]
fn automatica_so_passa_quando_um_jeito_de_ler_fecha_na_conferencia() {
    let mut linhas = Vec::new();
    for puxa in LADOS {
        for imprime_na_face_para in LADOS {
            for sai_com_face_para in LADOS {
                for cai in LADOS {
                    for uniao in [Uniao::PelaArestaLonga, Uniao::PelaArestaCurta] {
                        let modo = ModoImpressora {
                            entrada: Entrada::Traseira,
                            puxa,
                            imprime_na_face_para,
                            sai_com_face_para,
                            cai,
                            topo_da_imagem: TopoDaImagem::NaArestaQueEntra,
                        };
                        let (julgou, serve) = conferencia_automatica(modo, uniao);
                        linhas.push(format!("{modo:?} {uniao:?} {julgou:?} {serve}"));
                        assert_eq!(
                            julgou == ConferenciaAutomatica::Certa,
                            serve,
                            "{modo:?} {uniao:?}"
                        );
                        if uniao == Uniao::PelaArestaCurta {
                            assert_ne!(julgou, ConferenciaAutomatica::Certa, "{modo:?}");
                        }
                    }
                }
            }
        }
    }
    println!("{}", linhas.join("\n"));
}
