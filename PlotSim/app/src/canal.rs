//! Liga o canal local à janela. O PlotS em modo de teste publica o gesto de cada lote; quando a fila
//! é desta janela, o gesto é cumprido na recolocação, sem escolha da pessoa. Se outro programa já tiver
//! o canal, ou nenhum PlotS mandar nada, a janela segue só com a escolha da pessoa.

use crate::bandeja::RecusaDoGesto;
use crate::estado::Estado;
use plotsim_core::canal::{LeituraPedida, Mensagem};
use plotsim_core::NomeVirtual;
use plotsim_physics::Gesto;
use std::sync::Arc;
use std::time::Duration;

/// Quantas vezes o gesto é tentado antes de desistir. Ele pode chegar antes de a frente aparecer na
/// bandeja, porque o PlotS só sabe que a frente saiu quando o spooler a dá como concluída.
const TENTATIVAS: u32 = 600;
/// Espaço entre as tentativas: no máximo cerca de 30 segundos de espera.
const PASSO: Duration = Duration::from_millis(50);

const SEM_TRABALHO: &str = "nenhum trabalho espera o gesto nesta fila";
const ENTRADA_DIFERENTE: &str = "a entrada pedida não é a desta impressora";
const FACE_SEM_SENTIDO: &str = "a face pedida não existe nesta entrada";

/// O motivo, em texto, de um gesto que não foi cumprido. A linha vai para o PlotS e para o terminal.
fn motivo_do_gesto(recusa: plotsim_physics::ErroGesto) -> &'static str {
    match recusa {
        plotsim_physics::ErroGesto::EntradaDiferente { .. } => ENTRADA_DIFERENTE,
        plotsim_physics::ErroGesto::FaceSemSentido { .. } => FACE_SEM_SENTIDO,
    }
}

/// Tenta cumprir o gesto até a frente chegar à bandeja. Cada tentativa que não encontra trabalho
/// espera um passo, sem travar a fila: o tempo de espera é limitado e não depende do relógio.
pub(crate) fn cumprir_com_espera(
    mut tentar: impl FnMut() -> Result<(), RecusaDoGesto>,
    mut esperar: impl FnMut(Duration),
) -> Result<(), String> {
    for tentativa in 0..TENTATIVAS {
        match tentar() {
            Ok(()) => return Ok(()),
            Err(RecusaDoGesto::NadaParaRecolocar) => {
                if tentativa.saturating_add(1) < TENTATIVAS {
                    esperar(PASSO);
                }
            }
            Err(RecusaDoGesto::Fisica(erro)) => return Err(motivo_do_gesto(erro).to_string()),
        }
    }
    Err(SEM_TRABALHO.to_string())
}

/// A resposta a uma mensagem do PlotS. Início e fim só confirmam; o gesto é cumprido na fila pedida.
pub(crate) fn responder_a(
    mensagem: Mensagem,
    mut cumprir: impl FnMut(&str, Gesto) -> Result<(), RecusaDoGesto>,
    mut encerrar: impl FnMut(&str, LeituraPedida),
    esperar: impl FnMut(Duration),
) -> Result<(), String> {
    match mensagem {
        Mensagem::Inicio { .. } => Ok(()),
        Mensagem::Fim { fila, leitura, .. } => {
            let curto = curto_da_fila(&fila)?;
            encerrar(&curto, leitura);
            Ok(())
        }
        Mensagem::Gesto { fila, gesto, .. } => {
            let curto = curto_da_fila(&fila)?;
            cumprir_com_espera(|| cumprir(&curto, gesto), esperar)
        }
    }
}

/// O nome curto da fila do simulador; outra fila nunca chega à bandeja.
fn curto_da_fila(fila: &str) -> Result<String, String> {
    Ok(NomeVirtual::da_fila(fila)
        .map_err(|erro| erro.to_string())?
        .curto()
        .to_string())
}

/// Abre o canal e atende em uma thread própria. Sem o canal, a janela segue só com a escolha da pessoa.
#[cfg(windows)]
pub fn iniciar(estado: Arc<Estado>) -> Result<(), String> {
    use plotsim_core::canal::{Pedido, ServidorDoCanal, CAMINHO_DO_CANAL};
    use std::sync::mpsc;
    use std::thread;

    let (destino, chegada) = mpsc::channel::<Pedido>();
    let servidor =
        ServidorDoCanal::iniciar(CAMINHO_DO_CANAL, destino).map_err(|erro| erro.to_string())?;
    thread::Builder::new()
        .name("plotsim-canal".to_string())
        .spawn(move || {
            // Enquanto esta thread viver, o canal atende; quando ela acabar, o canal fecha.
            let _servidor = servidor;
            for pedido in chegada {
                let resposta = responder_a(
                    pedido.mensagem.clone(),
                    |curto, gesto| estado.cumprir_gesto_do_canal(curto, gesto),
                    |curto, leitura| estado.encerrar_fila(curto, leitura),
                    thread::sleep,
                );
                pedido.responder(resposta);
            }
        })
        .map(|_| ())
        .map_err(|erro| erro.to_string())
}

#[cfg(not(windows))]
pub fn iniciar(_: Arc<Estado>) -> Result<(), String> {
    Err("o canal só existe no Windows".to_string())
}

#[cfg(test)]
mod testes {
    use super::*;
    use plotsim_physics::{BordaQueEntra, Entrada, FaceNaEntrada};
    use std::cell::Cell;

    const GESTO: Gesto = Gesto {
        entrada: Entrada::Traseira,
        face_impressa: FaceNaEntrada::ParaTras,
        borda_que_entra: BordaQueEntra::Base,
        inverter_ordem: false,
    };

    fn gesto_do_plots(fila: &str) -> Mensagem {
        Mensagem::Gesto {
            fila: fila.to_string(),
            lote: 1,
            gesto: GESTO,
        }
    }

    #[test]
    fn gesto_aceito_na_primeira_tentativa_nao_espera() {
        let esperas = Cell::new(0u32);
        let resultado = responder_a(
            gesto_do_plots("PlotSim — Jato"),
            |curto, _| {
                assert_eq!(curto, "Jato");
                Ok(())
            },
            |_, _| {},
            |_| esperas.set(esperas.get() + 1),
        );
        assert_eq!(resultado, Ok(()));
        assert_eq!(esperas.get(), 0);
    }

    #[test]
    fn gesto_que_chega_antes_da_frente_espera_e_cumpre_quando_ela_chega() {
        let tentativas = Cell::new(0u32);
        let esperas = Cell::new(0u32);
        let resultado = responder_a(
            gesto_do_plots("PlotSim — Jato"),
            |_, _| {
                tentativas.set(tentativas.get() + 1);
                if tentativas.get() < 3 {
                    Err(RecusaDoGesto::NadaParaRecolocar)
                } else {
                    Ok(())
                }
            },
            |_, _| {},
            |_| esperas.set(esperas.get() + 1),
        );
        assert_eq!(resultado, Ok(()));
        assert_eq!(tentativas.get(), 3);
        assert_eq!(esperas.get(), 2);
    }

    #[test]
    fn sem_frente_nenhuma_o_gesto_desiste_com_motivo_e_espera_limitada() {
        let esperas = Cell::new(0u32);
        let resultado = responder_a(
            gesto_do_plots("PlotSim — Jato"),
            |_, _| Err(RecusaDoGesto::NadaParaRecolocar),
            |_, _| {},
            |_| esperas.set(esperas.get() + 1),
        );
        assert_eq!(resultado, Err(SEM_TRABALHO.to_string()));
        assert_eq!(esperas.get(), TENTATIVAS - 1);
    }

    #[test]
    fn gesto_que_a_maquina_nao_faz_volta_sem_esperar() {
        let esperas = Cell::new(0u32);
        let resultado = cumprir_com_espera(
            || {
                Err(RecusaDoGesto::Fisica(
                    plotsim_physics::ErroGesto::EntradaDiferente {
                        pedida: Entrada::Gaveta,
                        da_maquina: Entrada::Traseira,
                    },
                ))
            },
            |_| esperas.set(esperas.get() + 1),
        );
        assert_eq!(resultado, Err(ENTRADA_DIFERENTE.to_string()));
        assert_eq!(esperas.get(), 0);
    }

    #[test]
    fn inicio_so_confirma_sem_mexer_na_bandeja() {
        let mut chamadas = 0;
        let mut cumprir = |_: &str, _: Gesto| {
            chamadas += 1;
            Ok(())
        };
        let inicio = Mensagem::Inicio {
            fila: "PlotSim — Jato".to_string(),
            lotes: 2,
        };
        assert_eq!(responder_a(inicio, &mut cumprir, |_, _| {}, |_| {}), Ok(()));
        assert_eq!(chamadas, 0);
    }

    #[test]
    fn fim_entrega_a_leitura_a_fila_e_nao_cumpre_gesto() {
        let mut chamadas = 0;
        let mut cumprir = |_: &str, _: Gesto| {
            chamadas += 1;
            Ok(())
        };
        let mut lidas: Vec<(String, LeituraPedida)> = Vec::new();
        let fim = Mensagem::Fim {
            fila: "PlotSim — Jato".to_string(),
            lotes: 2,
            leitura: LeituraPedida::Virando,
        };
        let resultado = responder_a(
            fim,
            &mut cumprir,
            |curto, leitura| lidas.push((curto.to_string(), leitura)),
            |_| {},
        );
        assert_eq!(resultado, Ok(()));
        assert_eq!(chamadas, 0);
        assert_eq!(lidas, vec![("Jato".to_string(), LeituraPedida::Virando)]);
    }

    #[test]
    fn fim_de_fila_que_nao_e_do_simulador_nao_chega_a_bandeja() {
        let mut lidas = 0;
        let fim = Mensagem::Fim {
            fila: "Impressora real".to_string(),
            lotes: 1,
            leitura: LeituraPedida::ComoSaiu,
        };
        let resultado = responder_a(fim, |_, _| Ok(()), |_, _| lidas += 1, |_| {});
        assert!(resultado.is_err());
        assert_eq!(lidas, 0);
    }

    #[test]
    fn fila_que_nao_e_do_simulador_nao_chega_a_bandeja() {
        let mut chamadas = 0;
        let resultado = responder_a(
            gesto_do_plots("Impressora real"),
            |_, _| {
                chamadas += 1;
                Ok(())
            },
            |_, _| {},
            |_| {},
        );
        assert!(resultado.is_err());
        assert_eq!(chamadas, 0);
    }
}
