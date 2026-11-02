//! Prazo para a interface responder a um pedido de fechamento da janela.
//!
//! Enquanto existe um ouvinte de fechamento na interface, o Tauri nunca fecha a janela sozinho:
//! quem fecha e a interface. Se ela travar, o pedido se perde. Aqui fica a decisao de quando
//! desistir de esperar; o tempo entra como argumento para que tudo possa ser testado sem espera.

use std::time::{Duration, Instant};

pub const PRAZO_DE_RESPOSTA: Duration = Duration::from_secs(5);

#[derive(Debug, PartialEq, Eq)]
pub enum DecisaoDoPrazo {
    /// A interface respondeu: nao ha mais o que vigiar.
    Encerrar,
    /// O pedido segue sem resposta e ainda ha prazo.
    Esperar(Duration),
    /// O prazo acabou sem resposta.
    Destruir,
}

#[derive(Debug, Default)]
pub struct EsperaDeResposta {
    pedido_em: Option<Instant>,
    vigia_ativa: bool,
}

impl EsperaDeResposta {
    /// Registra o pedido e diz se e preciso iniciar uma vigia; so existe uma por vez.
    /// Repetir o pedido nao adia o prazo que ja corre, por mais que o usuario insista.
    pub fn ao_pedir_fechamento(&mut self, agora: Instant) -> bool {
        if self.pedido_em.is_none() {
            self.pedido_em = Some(agora);
        }
        !std::mem::replace(&mut self.vigia_ativa, true)
    }

    /// A interface esta viva. O dialogo pode ficar aberto depois disso o tempo que for.
    pub fn ao_receber_resposta(&mut self) {
        self.pedido_em = None;
    }

    /// Sem vigia nao ha prazo; o pedido e esquecido para que o proximo tente de novo.
    pub fn ao_falhar_vigia(&mut self) {
        self.pedido_em = None;
        self.vigia_ativa = false;
    }

    /// O que a vigia faz ao acordar. Ao encerrar ou destruir, ela deixa de contar como ativa.
    pub fn decidir(&mut self, agora: Instant) -> DecisaoDoPrazo {
        let Some(pedido_em) = self.pedido_em else {
            self.vigia_ativa = false;
            return DecisaoDoPrazo::Encerrar;
        };
        let passado = agora.saturating_duration_since(pedido_em);
        match PRAZO_DE_RESPOSTA.checked_sub(passado) {
            Some(restante) if !restante.is_zero() => DecisaoDoPrazo::Esperar(restante),
            _ => {
                self.ao_falhar_vigia();
                DecisaoDoPrazo::Destruir
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const UM: Duration = Duration::from_secs(1);

    #[test]
    fn prazo_e_de_cinco_segundos() {
        assert_eq!(PRAZO_DE_RESPOSTA, Duration::from_secs(5));
    }

    #[test]
    fn sem_resposta_espera_o_que_falta_e_depois_destroi() {
        let inicio = Instant::now();
        let mut espera = EsperaDeResposta::default();
        assert!(espera.ao_pedir_fechamento(inicio));
        assert_eq!(espera.decidir(inicio), DecisaoDoPrazo::Esperar(5 * UM));
        assert_eq!(
            espera.decidir(inicio + 2 * UM),
            DecisaoDoPrazo::Esperar(3 * UM)
        );
        assert_eq!(espera.decidir(inicio + 5 * UM), DecisaoDoPrazo::Destruir);
    }

    #[test]
    fn um_atraso_grande_tambem_destroi() {
        let inicio = Instant::now();
        let mut espera = EsperaDeResposta::default();
        espera.ao_pedir_fechamento(inicio);
        assert_eq!(espera.decidir(inicio + 3600 * UM), DecisaoDoPrazo::Destruir);
    }

    #[test]
    fn resposta_encerra_a_vigia_e_o_dialogo_pode_ficar_aberto() {
        let inicio = Instant::now();
        let mut espera = EsperaDeResposta::default();
        espera.ao_pedir_fechamento(inicio);
        espera.ao_receber_resposta();
        assert_eq!(espera.decidir(inicio + 3600 * UM), DecisaoDoPrazo::Encerrar);
    }

    #[test]
    fn repetir_o_pedido_nao_adia_o_prazo_nem_cria_outra_vigia() {
        let inicio = Instant::now();
        let mut espera = EsperaDeResposta::default();
        assert!(espera.ao_pedir_fechamento(inicio));
        assert!(!espera.ao_pedir_fechamento(inicio + 4 * UM));
        assert_eq!(espera.decidir(inicio + 5 * UM), DecisaoDoPrazo::Destruir);
    }

    #[test]
    fn novo_pedido_apos_resposta_conta_do_zero_na_vigia_que_ja_existe() {
        let inicio = Instant::now();
        let mut espera = EsperaDeResposta::default();
        espera.ao_pedir_fechamento(inicio);
        espera.ao_receber_resposta();
        assert!(!espera.ao_pedir_fechamento(inicio + 3 * UM));
        assert_eq!(
            espera.decidir(inicio + 5 * UM),
            DecisaoDoPrazo::Esperar(3 * UM)
        );
        assert_eq!(espera.decidir(inicio + 8 * UM), DecisaoDoPrazo::Destruir);
    }

    #[test]
    fn depois_de_encerrar_um_novo_pedido_pede_outra_vigia() {
        let inicio = Instant::now();
        let mut espera = EsperaDeResposta::default();
        espera.ao_pedir_fechamento(inicio);
        espera.ao_receber_resposta();
        assert_eq!(espera.decidir(inicio + UM), DecisaoDoPrazo::Encerrar);
        assert!(espera.ao_pedir_fechamento(inicio + 2 * UM));
    }

    #[test]
    fn depois_de_destruir_sem_sucesso_um_novo_pedido_pede_outra_vigia() {
        let inicio = Instant::now();
        let mut espera = EsperaDeResposta::default();
        espera.ao_pedir_fechamento(inicio);
        assert_eq!(espera.decidir(inicio + 5 * UM), DecisaoDoPrazo::Destruir);
        assert!(espera.ao_pedir_fechamento(inicio + 6 * UM));
        assert_eq!(
            espera.decidir(inicio + 6 * UM),
            DecisaoDoPrazo::Esperar(5 * UM)
        );
    }

    #[test]
    fn falha_ao_iniciar_a_vigia_nao_impede_a_proxima_tentativa() {
        let inicio = Instant::now();
        let mut espera = EsperaDeResposta::default();
        assert!(espera.ao_pedir_fechamento(inicio));
        espera.ao_falhar_vigia();
        assert!(espera.ao_pedir_fechamento(inicio + UM));
        assert_eq!(espera.decidir(inicio + UM), DecisaoDoPrazo::Esperar(5 * UM));
    }

    #[test]
    fn relogio_anterior_ao_pedido_nao_encurta_o_prazo() {
        let inicio = Instant::now() + 10 * UM;
        let mut espera = EsperaDeResposta::default();
        espera.ao_pedir_fechamento(inicio);
        assert_eq!(
            espera.decidir(inicio - 5 * UM),
            DecisaoDoPrazo::Esperar(5 * UM)
        );
    }
}
