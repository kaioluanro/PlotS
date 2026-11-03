//! Quanto tempo a impressora trabalhou: do envio de cada passada até a fila confirmar. A espera da
//! pessoa na recolocação fica de fora porque nenhuma passada corre nela; aviso e fila parada
//! pausam a conta. O tempo vem de fora, e por isso os testes não dependem de esperar.

use std::time::Instant;

/// De onde vem o tempo, em milissegundos. Só a diferença entre duas leituras tem sentido.
pub trait Relogio: Send + Sync {
    fn agora_ms(&self) -> u64;
}

/// O relógio de verdade: não anda para trás, mesmo se a hora do sistema mudar.
pub struct RelogioDoSistema(Instant);

impl Default for RelogioDoSistema {
    fn default() -> Self {
        Self(Instant::now())
    }
}

impl Relogio for RelogioDoSistema {
    fn agora_ms(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// O que ficou medido: só lotes inteiros contam, porque as folhas só ficam prontas no fim do lote.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Medida {
    pub milissegundos: u64,
    pub folhas: u64,
}

#[derive(Debug, Default)]
pub struct Cronometro {
    medida: Medida,
    medida_automatica: Medida,
    do_lote: u64,
    em_passada: bool,
    desde: Option<u64>,
}

impl Cronometro {
    fn somar_ate(&mut self, agora: u64) {
        if let Some(desde) = self.desde.take() {
            self.do_lote = self.do_lote.saturating_add(agora.saturating_sub(desde));
        }
    }

    /// Uma passada foi para a fila.
    pub fn iniciar_passada(&mut self, agora: u64) {
        self.somar_ate(agora);
        self.em_passada = true;
        self.desde = Some(agora);
    }

    /// A impressora parou por um aviso, ou a fila deixou de responder.
    pub fn pausar(&mut self, agora: u64) {
        self.somar_ate(agora);
    }

    /// A fila voltou a andar. Fora de uma passada, não há o que retomar.
    pub fn retomar(&mut self, agora: u64) {
        if self.em_passada && self.desde.is_none() {
            self.desde = Some(agora);
        }
    }

    /// A fila confirmou a passada.
    pub fn fechar_passada(&mut self, agora: u64) {
        self.somar_ate(agora);
        self.em_passada = false;
    }

    /// O lote terminou: seu tempo e suas folhas entram na medida da rota por que ele saiu.
    pub fn fechar_lote(&mut self, agora: u64, folhas: u64, automatico: bool) {
        self.fechar_passada(agora);
        let medida = if automatico {
            &mut self.medida_automatica
        } else {
            &mut self.medida
        };
        medida.milissegundos = medida.milissegundos.saturating_add(self.do_lote);
        medida.folhas = medida.folhas.saturating_add(folhas);
        self.do_lote = 0;
    }

    /// O lote vai ser refeito: o que correu nele não virou folha pronta.
    pub fn descartar_lote(&mut self) {
        self.do_lote = 0;
        self.em_passada = false;
        self.desde = None;
    }

    pub fn medida(&self) -> Medida {
        self.medida
    }

    pub fn medida_automatica(&self) -> Medida {
        self.medida_automatica
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn so_conta_as_passadas_e_nao_a_recolocacao() {
        let mut tempo = Cronometro::default();
        tempo.iniciar_passada(1_000);
        tempo.fechar_passada(4_000);
        // A pessoa levou uma hora para recolocar o maço.
        tempo.iniciar_passada(3_604_000);
        tempo.fechar_lote(3_606_000, 5, false);
        assert_eq!(
            tempo.medida(),
            Medida {
                milissegundos: 5_000,
                folhas: 5
            }
        );
    }

    #[test]
    fn aviso_de_papel_e_fila_parada_nao_contam() {
        let mut tempo = Cronometro::default();
        tempo.iniciar_passada(0);
        tempo.pausar(1_000);
        tempo.retomar(61_000);
        tempo.pausar(62_000);
        tempo.pausar(70_000);
        tempo.retomar(90_000);
        tempo.fechar_lote(91_000, 1, false);
        assert_eq!(tempo.medida().milissegundos, 3_000);
    }

    #[test]
    fn terminar_ainda_em_pausa_nao_soma_a_pausa() {
        let mut tempo = Cronometro::default();
        tempo.iniciar_passada(0);
        tempo.pausar(1_000);
        tempo.fechar_lote(50_000, 2, false);
        assert_eq!(tempo.medida().milissegundos, 1_000);
    }

    #[test]
    fn retomar_fora_de_uma_passada_nao_liga_a_conta() {
        let mut tempo = Cronometro::default();
        tempo.retomar(10);
        tempo.iniciar_passada(100);
        tempo.fechar_passada(200);
        tempo.retomar(300);
        tempo.fechar_lote(900, 1, false);
        assert_eq!(tempo.medida().milissegundos, 100);
    }

    #[test]
    fn lote_refeito_conta_so_a_tentativa_que_saiu() {
        let mut tempo = Cronometro::default();
        tempo.iniciar_passada(0);
        tempo.pausar(500);
        tempo.descartar_lote();
        tempo.iniciar_passada(1_000);
        tempo.fechar_lote(1_200, 3, false);
        assert_eq!(
            tempo.medida(),
            Medida {
                milissegundos: 200,
                folhas: 3
            }
        );
    }

    #[test]
    fn lote_sem_terminar_nao_entra_na_medida() {
        let mut tempo = Cronometro::default();
        tempo.iniciar_passada(0);
        tempo.fechar_passada(1_000);
        assert_eq!(tempo.medida(), Medida::default());
    }

    #[test]
    fn numeros_hostis_nao_estouram() {
        let mut tempo = Cronometro::default();
        tempo.iniciar_passada(u64::MAX);
        tempo.fechar_lote(0, u64::MAX, false);
        tempo.iniciar_passada(0);
        tempo.fechar_lote(u64::MAX, u64::MAX, false);
        tempo.iniciar_passada(0);
        tempo.fechar_lote(u64::MAX, 1, false);
        assert_eq!(
            tempo.medida(),
            Medida {
                milissegundos: u64::MAX,
                folhas: u64::MAX
            }
        );
    }

    #[test]
    fn duas_faces_e_manual_nao_se_misturam_na_mesma_impressora() {
        let mut tempo = Cronometro::default();
        tempo.iniciar_passada(0);
        tempo.fechar_lote(1_000, 10, true);
        tempo.iniciar_passada(5_000);
        tempo.fechar_lote(9_000, 3, false);
        assert_eq!(
            tempo.medida_automatica(),
            Medida {
                milissegundos: 1_000,
                folhas: 10
            }
        );
        assert_eq!(
            tempo.medida(),
            Medida {
                milissegundos: 4_000,
                folhas: 3
            }
        );
    }
}
