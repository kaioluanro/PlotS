//! Controle de trabalhos, independente da mecânica da impressora.
use crate::{ErroSimulacao, Estado, Folha, Mecanica, Recolocacao, Simulador};
use plots_core::{Lote, Pagina};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Passada {
    Frente,
    Verso,
    /// Um trabalho só, em que a impressora imprime as duas faces de cada folha sozinha.
    DuasFaces,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EstadoEnvio {
    EmAndamento,
    Concluido,
    Falhou,
}

/// Cada instância representa uma fila independente. O estado de conclusão deve
/// refletir o backend: submissão ao spooler, por si só, não é conclusão.
pub trait Backend {
    type Erro;
    fn enviar(&mut self, passada: Passada, paginas: &[Pagina]) -> Result<(), Self::Erro>;
    fn consultar(&mut self) -> Result<EstadoEnvio, Self::Erro>;
    /// Retorna apenas quando o envio anterior não pode continuar imprimindo.
    fn cancelar(&mut self) -> Result<(), Self::Erro>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EstadoJob {
    Pronto,
    FrenteEnviada,
    AguardandoRecolocacao,
    VersoEnviado,
    /// O lote inteiro foi em um trabalho só: quando a fila confirmar, ele acabou.
    DuasFacesEnviadas,
    Concluido,
    Erro,
}
#[derive(Debug)]
pub enum ErroJob<E> {
    EstadoInvalido(EstadoJob),
    SemLotes,
    Backend(E),
    EnvioFalhou,
}

/// Controla uma cópia do documento para uma impressora. Cada lote só avança
/// depois de confirmação explícita do backend para ambas as passadas.
pub struct Job<B: Backend> {
    backend: B,
    lotes: Vec<Lote>,
    atual: usize,
    estado: EstadoJob,
}
impl<B: Backend> Job<B> {
    pub fn novo(backend: B, lotes: Vec<Lote>) -> Result<Self, ErroJob<B::Erro>> {
        if lotes.is_empty() {
            return Err(ErroJob::SemLotes);
        }
        Ok(Self {
            backend,
            lotes,
            atual: 0,
            estado: EstadoJob::Pronto,
        })
    }
    pub fn estado(&self) -> EstadoJob {
        self.estado
    }
    /// Índice de lote começando em zero.
    pub fn lote_atual(&self) -> usize {
        self.atual
    }
    /// Permite operações próprias do backend, como recolocação simulada.
    pub fn backend(&self) -> &B {
        &self.backend
    }
    /// Returns to the same batch only after the backend cancels the previous job.
    pub fn reiniciar_lote(&mut self) -> Result<(), ErroJob<B::Erro>> {
        if self.estado != EstadoJob::Erro {
            return Err(ErroJob::EstadoInvalido(self.estado));
        }
        self.backend.cancelar().map_err(ErroJob::Backend)?;
        self.estado = EstadoJob::Pronto;
        Ok(())
    }
    /// Cancela o envio em andamento, ou o que falhou, e volta ao início do mesmo lote. Quem
    /// decide cancelar é o chamador: nada aqui apaga um trabalho sozinho.
    pub fn cancelar_envio(&mut self) -> Result<(), ErroJob<B::Erro>> {
        if !matches!(
            self.estado,
            EstadoJob::FrenteEnviada
                | EstadoJob::VersoEnviado
                | EstadoJob::DuasFacesEnviadas
                | EstadoJob::Erro
        ) {
            return Err(ErroJob::EstadoInvalido(self.estado));
        }
        self.backend.cancelar().map_err(ErroJob::Backend)?;
        self.estado = EstadoJob::Pronto;
        Ok(())
    }
    /// Envia a primeira etapa do lote atual. Um lote em duas faces só tem esta etapa: o lote
    /// inteiro vai em um trabalho, sem recolocação nem verso.
    pub fn enviar_frente(&mut self) -> Result<(), ErroJob<B::Erro>> {
        if self.estado != EstadoJob::Pronto {
            return Err(ErroJob::EstadoInvalido(self.estado));
        }
        let Some(lote) = self.lotes.get(self.atual) else {
            return Err(ErroJob::EstadoInvalido(self.estado));
        };
        let (passada, depois) = if lote.duas_faces {
            (Passada::DuasFaces, EstadoJob::DuasFacesEnviadas)
        } else {
            (Passada::Frente, EstadoJob::FrenteEnviada)
        };
        if let Err(e) = self.backend.enviar(passada, &lote.passada1) {
            self.estado = EstadoJob::Erro;
            return Err(ErroJob::Backend(e));
        }
        self.estado = depois;
        Ok(())
    }
    pub fn enviar_verso(&mut self) -> Result<(), ErroJob<B::Erro>> {
        if self.estado != EstadoJob::AguardandoRecolocacao {
            return Err(ErroJob::EstadoInvalido(self.estado));
        }
        let Some(lote) = self.lotes.get(self.atual) else {
            return Err(ErroJob::EstadoInvalido(self.estado));
        };
        if let Err(e) = self.backend.enviar(Passada::Verso, &lote.passada2) {
            self.estado = EstadoJob::Erro;
            return Err(ErroJob::Backend(e));
        }
        self.estado = EstadoJob::VersoEnviado;
        Ok(())
    }
    /// Consulta o backend. Um erro de consulta não diz nada sobre o trabalho, então o estado
    /// fica como estava e a consulta pode ser repetida.
    pub fn atualizar(&mut self) -> Result<EstadoJob, ErroJob<B::Erro>> {
        if !matches!(
            self.estado,
            EstadoJob::FrenteEnviada | EstadoJob::VersoEnviado | EstadoJob::DuasFacesEnviadas
        ) {
            return Err(ErroJob::EstadoInvalido(self.estado));
        }
        let estado = self.backend.consultar().map_err(ErroJob::Backend)?;
        match estado {
            EstadoEnvio::EmAndamento => (),
            EstadoEnvio::Falhou => {
                self.estado = EstadoJob::Erro;
                return Err(ErroJob::EnvioFalhou);
            }
            EstadoEnvio::Concluido if self.estado == EstadoJob::FrenteEnviada => {
                self.estado = EstadoJob::AguardandoRecolocacao
            }
            EstadoEnvio::Concluido => {
                self.atual += 1;
                self.estado = if self.atual == self.lotes.len() {
                    EstadoJob::Concluido
                } else {
                    EstadoJob::Pronto
                };
            }
        }
        Ok(self.estado)
    }
}

impl Job<SimBackend> {
    pub fn recolocar(&mut self, gesto: Recolocacao) -> Result<(), ErroJob<ErroSimulacao>> {
        if self.estado != EstadoJob::AguardandoRecolocacao {
            return Err(ErroJob::EstadoInvalido(self.estado));
        }
        self.backend.recolocar(gesto).map_err(ErroJob::Backend)
    }
}

/// Backend em memória: as operações físicas são imediatas, mas o controlador
/// ainda exige uma consulta antes de liberar a próxima etapa.
pub struct SimBackend {
    simulador: Simulador,
    ultimo_lote: Option<Vec<Folha>>,
}
impl SimBackend {
    pub fn novo(mecanica: Mecanica) -> Self {
        Self {
            simulador: Simulador::novo(mecanica),
            ultimo_lote: None,
        }
    }
    pub fn recolocar(&mut self, gesto: Recolocacao) -> Result<(), ErroSimulacao> {
        self.simulador.recolocar(gesto)
    }
    pub fn simulador(&self) -> &Simulador {
        &self.simulador
    }
    pub fn ultimo_lote(&self) -> Option<&[Folha]> {
        self.ultimo_lote.as_deref()
    }
}
impl Backend for SimBackend {
    type Erro = ErroSimulacao;
    fn enviar(&mut self, passada: Passada, paginas: &[Pagina]) -> Result<(), Self::Erro> {
        if passada != Passada::Verso && self.simulador.estado() == Estado::Concluido {
            self.ultimo_lote = Some(self.simulador.retirar_lote()?);
        }
        match passada {
            Passada::Frente => self.simulador.imprimir_frente(paginas),
            Passada::Verso => self.simulador.imprimir_verso(paginas),
            Passada::DuasFaces => self.simulador.imprimir_duas_faces(paginas),
        }
    }
    fn consultar(&mut self) -> Result<EstadoEnvio, Self::Erro> {
        Ok(EstadoEnvio::Concluido)
    }
    fn cancelar(&mut self) -> Result<(), Self::Erro> {
        match self.simulador.estado() {
            Estado::Ocioso => (),
            Estado::Concluido => {
                self.simulador.retirar_lote()?;
            }
            _ => {
                self.simulador.cancelar_lote()?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plots_core::{plano_documento, OrdemPassadas, PerfilImpressora};
    fn lotes() -> Vec<Lote> {
        plano_documento(4, 1, OrdemPassadas::default(), PerfilImpressora::default()).unwrap()
    }
    fn lotes_em_duas_faces(paginas: usize, folhas: usize) -> Vec<Lote> {
        plano_documento(
            paginas,
            folhas,
            OrdemPassadas::default(),
            PerfilImpressora::default(),
        )
        .unwrap()
        .into_iter()
        .map(|lote| {
            let passada1 = lote
                .paginas
                .iter()
                .copied()
                .filter(|pagina| pagina.numero.is_some())
                .collect();
            Lote {
                passada1,
                passada2: Vec::new(),
                duas_faces: true,
                ..lote
            }
        })
        .collect()
    }

    #[test]
    fn lote_em_duas_faces_vai_em_um_trabalho_e_acaba_sem_verso() {
        let mut job = Job::novo(
            SimBackend::novo(Mecanica::traseira()),
            lotes_em_duas_faces(4, 1),
        )
        .unwrap();
        for indice in 0..2 {
            job.enviar_frente().unwrap();
            assert_eq!(job.estado(), EstadoJob::DuasFacesEnviadas);
            assert!(job.enviar_verso().is_err(), "não há verso a enviar");
            let esperado = if indice == 0 {
                EstadoJob::Pronto
            } else {
                EstadoJob::Concluido
            };
            assert_eq!(job.atualizar().unwrap(), esperado);
        }
        assert_eq!(job.backend().simulador().pilha().len(), 1);
    }

    #[test]
    fn lote_em_duas_faces_pode_ser_cancelado_e_volta_ao_mesmo_lote() {
        let mut job = Job::novo(
            SimBackend::novo(Mecanica::traseira()),
            lotes_em_duas_faces(4, 1),
        )
        .unwrap();
        job.enviar_frente().unwrap();
        job.cancelar_envio().unwrap();
        assert_eq!(job.estado(), EstadoJob::Pronto);
        assert_eq!(job.lote_atual(), 0);
        job.enviar_frente().unwrap();
        assert_eq!(job.estado(), EstadoJob::DuasFacesEnviadas);
    }

    #[test]
    fn exige_confirmacao_e_permite_lotes_sequenciais() {
        let mut job = Job::novo(SimBackend::novo(Mecanica::gaveta()), lotes()).unwrap();
        for indice in 0..2 {
            assert_eq!(job.lote_atual(), indice);
            job.enviar_frente().unwrap();
            assert!(matches!(
                job.enviar_verso(),
                Err(ErroJob::EstadoInvalido(EstadoJob::FrenteEnviada))
            ));
            assert_eq!(job.atualizar().unwrap(), EstadoJob::AguardandoRecolocacao);
            job.recolocar(Recolocacao {
                virar_maco: true,
                ..Default::default()
            })
            .unwrap();
            job.enviar_verso().unwrap();
            assert_eq!(
                job.atualizar().unwrap(),
                if indice == 0 {
                    EstadoJob::Pronto
                } else {
                    EstadoJob::Concluido
                }
            );
        }
        assert!(matches!(
            job.enviar_frente(),
            Err(ErroJob::EstadoInvalido(EstadoJob::Concluido))
        ));
    }
    struct Pendente {
        consultas: usize,
    }
    impl Backend for Pendente {
        type Erro = ();
        fn enviar(&mut self, _: Passada, _: &[Pagina]) -> Result<(), ()> {
            Ok(())
        }
        fn consultar(&mut self) -> Result<EstadoEnvio, ()> {
            self.consultas += 1;
            Ok(if self.consultas > 1 {
                EstadoEnvio::Falhou
            } else {
                EstadoEnvio::EmAndamento
            })
        }
        fn cancelar(&mut self) -> Result<(), ()> {
            Ok(())
        }
    }
    #[test]
    fn pendencia_e_falha_nao_liberam_verso() {
        let mut job = Job::novo(Pendente { consultas: 0 }, lotes()).unwrap();
        job.enviar_frente().unwrap();
        assert_eq!(job.atualizar().unwrap(), EstadoJob::FrenteEnviada);
        assert!(job.enviar_verso().is_err());
        assert!(matches!(job.atualizar(), Err(ErroJob::EnvioFalhou)));
        assert_eq!(job.estado(), EstadoJob::Erro);
        assert!(job.enviar_verso().is_err());
    }
    /// Consulta que falha na primeira vez e depois confirma.
    struct ConsultaInstavel {
        consultas: usize,
    }
    impl Backend for ConsultaInstavel {
        type Erro = ();
        fn enviar(&mut self, _: Passada, _: &[Pagina]) -> Result<(), ()> {
            Ok(())
        }
        fn consultar(&mut self) -> Result<EstadoEnvio, ()> {
            self.consultas += 1;
            if self.consultas == 1 {
                Err(())
            } else {
                Ok(EstadoEnvio::Concluido)
            }
        }
        fn cancelar(&mut self) -> Result<(), ()> {
            Ok(())
        }
    }
    #[test]
    fn erro_ao_consultar_nao_muda_o_estado_e_a_consulta_pode_repetir() {
        let mut job = Job::novo(ConsultaInstavel { consultas: 0 }, lotes()).unwrap();
        job.enviar_frente().unwrap();
        assert!(matches!(job.atualizar(), Err(ErroJob::Backend(()))));
        assert_eq!(job.estado(), EstadoJob::FrenteEnviada);
        assert!(job.enviar_verso().is_err());
        assert_eq!(job.atualizar().unwrap(), EstadoJob::AguardandoRecolocacao);
    }
    #[test]
    fn cancelar_envio_vale_com_trabalho_enviado_ou_com_erro_e_volta_ao_mesmo_lote() {
        let mut job = Job::novo(Pendente { consultas: 0 }, lotes()).unwrap();
        assert!(matches!(
            job.cancelar_envio(),
            Err(ErroJob::EstadoInvalido(EstadoJob::Pronto))
        ));
        job.enviar_frente().unwrap();
        job.cancelar_envio().unwrap();
        assert_eq!(job.estado(), EstadoJob::Pronto);
        assert_eq!(job.lote_atual(), 0);
        job.enviar_frente().unwrap();
        job.atualizar().unwrap();
        assert!(job.atualizar().is_err());
        assert_eq!(job.estado(), EstadoJob::Erro);
        job.cancelar_envio().unwrap();
        assert_eq!(job.estado(), EstadoJob::Pronto);
    }
    #[test]
    fn erro_do_backend_impede_progressao() {
        let mut job = Job::novo(SimBackend::novo(Mecanica::traseira()), lotes()).unwrap();
        job.enviar_frente().unwrap();
        job.atualizar().unwrap();
        assert!(matches!(job.enviar_verso(), Err(ErroJob::Backend(_))));
        assert_eq!(job.estado(), EstadoJob::Erro);
        job.reiniciar_lote().unwrap();
        assert_eq!(job.lote_atual(), 0);
        job.enviar_frente().unwrap();
        job.atualizar().unwrap();
        job.recolocar(Recolocacao {
            virar_maco: true,
            ..Default::default()
        })
        .unwrap();
        job.enviar_verso().unwrap();
        assert_eq!(job.atualizar().unwrap(), EstadoJob::Pronto);
        assert_eq!(job.lote_atual(), 1);
    }
}
