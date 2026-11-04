//! Calibração guiada: o que fazer com o perfil a partir do que a pessoa viu nas folhas de teste.
//!
//! O teste tem 2 folhas com as páginas 1 a 4: as pares saem na primeira passada, as ímpares na
//! segunda. Nada aqui sabe como a impressora é por dentro. Cada resposta só diz o que precisa mudar
//! em relação ao perfil que imprimiu aquele teste.

use crate::{ModeloImpressora, PerfilImpressora, ViradaMaco};

/// Quantas vezes o teste pode ser impresso antes de dizer que a impressora não se encaixou. Com as
/// respostas certas bastam 3; as outras dão margem para um engano de quem olha as folhas.
pub const RODADAS_MAXIMAS: u8 = 5;

/// O perfil usado em uma rodada, com a compensação do driver.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PerfilEmTeste {
    pub perfil: PerfilImpressora,
    pub driver_inverte: bool,
}

impl PerfilEmTeste {
    /// O ponto de partida: os valores do modelo, com o que já se sabe do driver.
    pub const fn do_modelo(modelo: ModeloImpressora, driver_inverte: bool) -> Self {
        Self {
            perfil: modelo.perfil(),
            driver_inverte,
        }
    }

    /// A página do verso que o PlotS manda primeiro neste perfil.
    pub const fn pagina_enviada_primeiro(self) -> usize {
        if self.perfil.passada2_inversa != self.driver_inverte {
            3
        } else {
            1
        }
    }
}

/// O segundo lado saiu na face em branco, ou sobre o que já estava impresso.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LadoDoVerso {
    EmBranco,
    PorCima,
}

/// Como o maço fica com o 1 por cima.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LeituraDoMaco {
    SemVirar,
    /// Virando o maço como página de livro.
    Virando,
    /// O 1 não fica por cima de nenhum dos dois jeitos.
    NaoFica,
}

/// O que a pessoa viu em uma rodada.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Respostas {
    pub verso: LadoDoVerso,
    /// No verso da folha com o 1 está o 2.
    pub par_certo: bool,
    /// Virando a folha como página de livro, as duas setas apontam para o mesmo lado.
    pub setas_iguais: bool,
    pub leitura: LeituraDoMaco,
    /// O número da primeira folha que saiu no verso, se a pessoa reparou. Só decide com o maço
    /// invertido; um número fora do teste vale como não ter reparado.
    pub primeira_a_sair: Option<usize>,
}

/// Um valor do perfil que mudou, já com o valor novo.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mudanca {
    Virada(ViradaMaco),
    FrenteInversa(bool),
    VersoInverso(bool),
    VersoGirado(bool),
    CompensaDriver(bool),
}

/// O que uma rodada decidiu.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Avaliacao {
    /// O perfil a usar daqui em diante; igual ao testado quando já está certo.
    pub proximo: PerfilEmTeste,
    pub mudancas: Vec<Mudanca>,
    /// As respostas dizem que o maço saiu certo com o perfil testado.
    pub certo: bool,
}

/// Virar troca a face impressa de lado, e não virar a mantém. Só resta trocar um pelo outro. Sem ter
/// por onde escolher, a virada nova é a pela borda de cima.
const fn virada_oposta(virada: ViradaMaco) -> ViradaMaco {
    match virada {
        ViradaMaco::NaoVirar => ViradaMaco::PelaBordaDeCima,
        ViradaMaco::PelaLateral | ViradaMaco::PelaBordaDeCima => ViradaMaco::NaoVirar,
    }
}

fn mudancas_entre(antes: PerfilEmTeste, depois: PerfilEmTeste) -> Vec<Mudanca> {
    let mut mudancas = Vec::new();
    if antes.perfil.gesto_recolocar.virada != depois.perfil.gesto_recolocar.virada {
        mudancas.push(Mudanca::Virada(depois.perfil.gesto_recolocar.virada));
    }
    if antes.perfil.passada1_inversa != depois.perfil.passada1_inversa {
        mudancas.push(Mudanca::FrenteInversa(depois.perfil.passada1_inversa));
    }
    if antes.perfil.passada2_inversa != depois.perfil.passada2_inversa {
        mudancas.push(Mudanca::VersoInverso(depois.perfil.passada2_inversa));
    }
    if antes.perfil.verso_girar_180 != depois.perfil.verso_girar_180 {
        mudancas.push(Mudanca::VersoGirado(depois.perfil.verso_girar_180));
    }
    if antes.driver_inverte != depois.driver_inverte {
        mudancas.push(Mudanca::CompensaDriver(depois.driver_inverte));
    }
    mudancas
}

/// O que a pessoa viu sobre o driver: `None` quando ela não reparou. A impressora não muda a ordem
/// das folhas de um trabalho: se a primeira a sair não é a primeira enviada, foi o driver.
fn driver_observado(testado: PerfilEmTeste, primeira_a_sair: Option<usize>) -> Option<bool> {
    let vista = primeira_a_sair.filter(|pagina| matches!(pagina, 1 | 3))?;
    Some(vista != testado.pagina_enviada_primeiro())
}

/// Decide o próximo perfil a partir das respostas de uma rodada.
///
/// Impressão por cima invalida as outras respostas: só a virada muda. Com o lado certo, a leitura
/// julga a ordem do verso, porque é ele que fica à vista, e o par julga a frente em relação a ele.
/// Maço invertido com o par certo troca as duas ordens, o mesmo que um driver que inverte: por isso
/// a pergunta de qual folha saiu primeiro é a única que separa os dois casos.
pub fn avaliar(testado: PerfilEmTeste, respostas: Respostas) -> Avaliacao {
    let mut proximo = testado;
    if respostas.verso == LadoDoVerso::PorCima {
        proximo.perfil.gesto_recolocar.virada =
            virada_oposta(testado.perfil.gesto_recolocar.virada);
        return Avaliacao {
            proximo,
            mudancas: mudancas_entre(testado, proximo),
            certo: false,
        };
    }
    let invertido = respostas.leitura == LeituraDoMaco::NaoFica;
    let mut trocar_as_duas = invertido;
    if invertido {
        if let Some(driver) = driver_observado(testado, respostas.primeira_a_sair) {
            // Quem passa a compensar, ou deixa de compensar, já troca as duas ordens no envio.
            trocar_as_duas ^= driver != testado.driver_inverte;
            proximo.driver_inverte = driver;
        }
    }
    proximo.perfil.passada1_inversa ^= trocar_as_duas != !respostas.par_certo;
    proximo.perfil.passada2_inversa ^= trocar_as_duas;
    proximo.perfil.verso_girar_180 ^= !respostas.setas_iguais;
    Avaliacao {
        proximo,
        mudancas: mudancas_entre(testado, proximo),
        certo: respostas.par_certo && respostas.setas_iguais && !invertido,
    }
}

/// O que conferir uma impressora que imprime as duas faces sozinha pode concluir. A ordem e a virada
/// são dela e do driver: nenhum valor do perfil corrige, então não há outra rodada.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConferenciaAutomatica {
    Certa,
    /// As setas divergem: o driver não está virando pela borda longa.
    NaoViraPelaBordaLonga,
    /// O maço não se lê em ordem nem virado, ou o par não fecha.
    OrdemNaoServe,
}

/// Julga as respostas da impressora automática.
pub const fn conferir_automatica(
    par_certo: bool,
    setas_iguais: bool,
    leitura: LeituraDoMaco,
) -> ConferenciaAutomatica {
    if !par_certo || matches!(leitura, LeituraDoMaco::NaoFica) {
        ConferenciaAutomatica::OrdemNaoServe
    } else if setas_iguais {
        ConferenciaAutomatica::Certa
    } else {
        ConferenciaAutomatica::NaoViraPelaBordaLonga
    }
}

/// A face impressa fica para cima ao entrar? Virar troca o lado com que o maço saiu.
pub const fn face_impressa_entra_para_cima(virada: ViradaMaco, saiu_para_cima: bool) -> bool {
    match virada {
        ViradaMaco::NaoVirar => saiu_para_cima,
        ViradaMaco::PelaLateral | ViradaMaco::PelaBordaDeCima => !saiu_para_cima,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const TUDO_CERTO: Respostas = Respostas {
        verso: LadoDoVerso::EmBranco,
        par_certo: true,
        setas_iguais: true,
        leitura: LeituraDoMaco::SemVirar,
        primeira_a_sair: None,
    };

    fn jato() -> PerfilEmTeste {
        PerfilEmTeste::do_modelo(ModeloImpressora::JatoDeTinta, false)
    }

    #[test]
    fn tudo_certo_nao_muda_nada_e_vale_com_o_maco_virado() {
        for leitura in [LeituraDoMaco::SemVirar, LeituraDoMaco::Virando] {
            let avaliacao = avaliar(
                jato(),
                Respostas {
                    leitura,
                    ..TUDO_CERTO
                },
            );
            assert!(avaliacao.certo);
            assert_eq!(avaliacao.proximo, jato());
            assert_eq!(avaliacao.mudancas, vec![]);
        }
    }

    #[test]
    fn impressao_por_cima_so_troca_a_virada_e_ignora_o_resto() {
        let avaliacao = avaliar(
            jato(),
            Respostas {
                verso: LadoDoVerso::PorCima,
                par_certo: false,
                setas_iguais: false,
                leitura: LeituraDoMaco::NaoFica,
                primeira_a_sair: Some(3),
            },
        );
        assert!(!avaliacao.certo);
        assert_eq!(
            avaliacao.mudancas,
            vec![Mudanca::Virada(ViradaMaco::NaoVirar)]
        );
    }

    #[test]
    fn par_errado_com_o_1_por_cima_muda_a_frente_e_nao_o_verso() {
        let avaliacao = avaliar(
            jato(),
            Respostas {
                par_certo: false,
                ..TUDO_CERTO
            },
        );
        assert_eq!(avaliacao.mudancas, vec![Mudanca::FrenteInversa(false)]);
    }

    #[test]
    fn maco_invertido_com_o_par_certo_troca_as_duas_ordens() {
        let avaliacao = avaliar(
            jato(),
            Respostas {
                leitura: LeituraDoMaco::NaoFica,
                ..TUDO_CERTO
            },
        );
        assert!(!avaliacao.certo);
        assert_eq!(
            avaliacao.mudancas,
            vec![Mudanca::FrenteInversa(false), Mudanca::VersoInverso(false)]
        );
    }

    #[test]
    fn par_errado_e_maco_invertido_so_mudam_o_verso() {
        let avaliacao = avaliar(
            jato(),
            Respostas {
                par_certo: false,
                leitura: LeituraDoMaco::NaoFica,
                ..TUDO_CERTO
            },
        );
        assert_eq!(avaliacao.mudancas, vec![Mudanca::VersoInverso(false)]);
    }

    #[test]
    fn setas_diferentes_so_trocam_o_giro() {
        let avaliacao = avaliar(
            jato(),
            Respostas {
                setas_iguais: false,
                ..TUDO_CERTO
            },
        );
        assert!(!avaliacao.certo);
        assert_eq!(avaliacao.mudancas, vec![Mudanca::VersoGirado(false)]);
    }

    #[test]
    fn folha_que_sai_fora_da_ordem_enviada_liga_a_compensacao_sem_trocar_a_ordem() {
        // O jato manda o verso 3, 1. Saiu o 1 primeiro: quem inverteu foi o driver.
        let avaliacao = avaliar(
            jato(),
            Respostas {
                leitura: LeituraDoMaco::NaoFica,
                primeira_a_sair: Some(1),
                ..TUDO_CERTO
            },
        );
        assert_eq!(avaliacao.mudancas, vec![Mudanca::CompensaDriver(true)]);
    }

    #[test]
    fn folha_que_sai_na_ordem_enviada_troca_a_ordem_e_mantem_o_driver() {
        let avaliacao = avaliar(
            jato(),
            Respostas {
                leitura: LeituraDoMaco::NaoFica,
                primeira_a_sair: Some(3),
                ..TUDO_CERTO
            },
        );
        assert_eq!(
            avaliacao.mudancas,
            vec![Mudanca::FrenteInversa(false), Mudanca::VersoInverso(false)]
        );
    }

    #[test]
    fn compensacao_ligada_sem_driver_que_inverte_e_desligada() {
        let testado = PerfilEmTeste::do_modelo(ModeloImpressora::JatoDeTinta, true);
        assert_eq!(testado.pagina_enviada_primeiro(), 1);
        let avaliacao = avaliar(
            testado,
            Respostas {
                leitura: LeituraDoMaco::NaoFica,
                primeira_a_sair: Some(1),
                ..TUDO_CERTO
            },
        );
        assert_eq!(avaliacao.mudancas, vec![Mudanca::CompensaDriver(false)]);
    }

    #[test]
    fn numero_fora_do_teste_vale_como_nao_ter_reparado() {
        let sem_ver = avaliar(
            jato(),
            Respostas {
                leitura: LeituraDoMaco::NaoFica,
                ..TUDO_CERTO
            },
        );
        for hostil in [0, 2, 4, usize::MAX] {
            let avaliacao = avaliar(
                jato(),
                Respostas {
                    leitura: LeituraDoMaco::NaoFica,
                    primeira_a_sair: Some(hostil),
                    ..TUDO_CERTO
                },
            );
            assert_eq!(avaliacao, sem_ver, "{hostil}");
        }
    }

    #[test]
    fn automatica_so_passa_com_par_setas_e_leitura() {
        use ConferenciaAutomatica as C;
        use LeituraDoMaco as L;
        assert_eq!(conferir_automatica(true, true, L::SemVirar), C::Certa);
        assert_eq!(conferir_automatica(true, true, L::Virando), C::Certa);
        assert_eq!(
            conferir_automatica(true, false, L::Virando),
            C::NaoViraPelaBordaLonga
        );
        assert_eq!(
            conferir_automatica(false, true, L::SemVirar),
            C::OrdemNaoServe
        );
        assert_eq!(
            conferir_automatica(true, true, L::NaoFica),
            C::OrdemNaoServe
        );
    }

    #[test]
    fn so_virar_troca_o_lado_da_face_impressa() {
        assert!(face_impressa_entra_para_cima(ViradaMaco::NaoVirar, true));
        assert!(!face_impressa_entra_para_cima(ViradaMaco::NaoVirar, false));
        assert!(!face_impressa_entra_para_cima(
            ViradaMaco::PelaLateral,
            true
        ));
        assert!(face_impressa_entra_para_cima(
            ViradaMaco::PelaBordaDeCima,
            false
        ));
    }
}
