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
/// O que um trabalho imprime dos lotes que pedem o gesto. O lote em duas faces sai inteiro na primeira
/// etapa, e um trabalho só de versos passa por cima dele.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Alcance {
    #[default]
    FrenteEVerso,
    SoFrentes,
    /// O maço já saiu antes, talvez em outro dia: quem garante que é o certo é a pessoa.
    SoVersos,
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
    /// Um verso vai sair sem a frente neste trabalho: o maço do lote `indice`, que vem de fora, tem
    /// estas frentes.
    fn maco_de_fora(&mut self, _indice: usize, _frentes: &[Pagina]) -> Result<(), Self::Erro> {
        Ok(())
    }
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
    alcance: Alcance,
    terminado: Option<usize>,
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
            alcance: Alcance::default(),
            terminado: None,
        })
    }
    /// Vale antes do primeiro envio. Só de versos, o trabalho já começa esperando o maço do primeiro
    /// lote que pede o gesto; sem nenhum, não há o que imprimir.
    pub fn com_alcance(mut self, alcance: Alcance) -> Self {
        self.alcance = alcance;
        if alcance == Alcance::SoVersos {
            self.ir_ao_proximo_verso(0);
        }
        self
    }
    pub fn alcance(&self) -> Alcance {
        self.alcance
    }
    /// Índice do último lote que terminou neste trabalho.
    pub fn lote_terminado(&self) -> Option<usize> {
        self.terminado
    }
    fn ir_ao_proximo_verso(&mut self, desde: usize) {
        let proximo = self
            .lotes
            .iter()
            .enumerate()
            .skip(desde)
            .find(|(_, lote)| !lote.duas_faces)
            .map(|(indice, _)| indice);
        match proximo {
            Some(indice) => {
                self.atual = indice;
                self.estado = EstadoJob::AguardandoRecolocacao;
            }
            None => {
                self.atual = self.lotes.len();
                self.estado = EstadoJob::Concluido;
            }
        }
    }
    /// De onde o mesmo lote recomeça: só de versos, a frente não é deste trabalho.
    fn inicio_do_lote(&self) -> EstadoJob {
        if self.alcance == Alcance::SoVersos {
            EstadoJob::AguardandoRecolocacao
        } else {
            EstadoJob::Pronto
        }
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
        self.estado = self.inicio_do_lote();
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
        self.estado = self.inicio_do_lote();
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
        if self.alcance == Alcance::SoVersos {
            if let Err(e) = self.backend.maco_de_fora(self.atual, &lote.passada1) {
                self.estado = EstadoJob::Erro;
                return Err(ErroJob::Backend(e));
            }
        }
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
            EstadoEnvio::Concluido
                if self.estado == EstadoJob::FrenteEnviada
                    && self.alcance != Alcance::SoFrentes =>
            {
                self.estado = EstadoJob::AguardandoRecolocacao
            }
            EstadoEnvio::Concluido if self.alcance == Alcance::SoVersos => {
                self.terminado = Some(self.atual);
                self.ir_ao_proximo_verso(self.atual.saturating_add(1));
            }
            EstadoEnvio::Concluido => {
                self.terminado = Some(self.atual);
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

    /// Registra as passadas e o que veio de fora, sem nenhuma fila.
    #[derive(Default)]
    struct Registro {
        passadas: Vec<Passada>,
        de_fora: Vec<usize>,
        cancelados: usize,
    }
    impl Backend for Registro {
        type Erro = ();
        fn enviar(&mut self, passada: Passada, _: &[Pagina]) -> Result<(), ()> {
            self.passadas.push(passada);
            Ok(())
        }
        fn consultar(&mut self) -> Result<EstadoEnvio, ()> {
            Ok(EstadoEnvio::Concluido)
        }
        fn cancelar(&mut self) -> Result<(), ()> {
            self.cancelados += 1;
            Ok(())
        }
        fn maco_de_fora(&mut self, indice: usize, _: &[Pagina]) -> Result<(), ()> {
            self.de_fora.push(indice);
            Ok(())
        }
    }

    #[test]
    fn so_frentes_passa_pelos_lotes_sem_liberar_verso_e_acaba_na_ultima_frente() {
        let mut job = Job::novo(Registro::default(), lotes())
            .unwrap()
            .com_alcance(Alcance::SoFrentes);
        job.enviar_frente().unwrap();
        assert!(job.enviar_verso().is_err());
        assert_eq!(job.atualizar().unwrap(), EstadoJob::Pronto);
        assert_eq!(job.lote_terminado(), Some(0));
        assert!(job.enviar_verso().is_err());
        job.enviar_frente().unwrap();
        assert_eq!(job.atualizar().unwrap(), EstadoJob::Concluido);
        assert_eq!(job.backend().passadas, [Passada::Frente, Passada::Frente]);
    }

    #[test]
    fn so_versos_comeca_esperando_o_maco_e_nunca_envia_frente() {
        let mut job = Job::novo(Registro::default(), lotes())
            .unwrap()
            .com_alcance(Alcance::SoVersos);
        assert_eq!(job.estado(), EstadoJob::AguardandoRecolocacao);
        assert!(job.enviar_frente().is_err());
        job.enviar_verso().unwrap();
        assert_eq!(job.atualizar().unwrap(), EstadoJob::AguardandoRecolocacao);
        assert_eq!(job.lote_atual(), 1);
        assert_eq!(job.lote_terminado(), Some(0));
        job.enviar_verso().unwrap();
        assert_eq!(job.atualizar().unwrap(), EstadoJob::Concluido);
        assert_eq!(job.backend().passadas, [Passada::Verso, Passada::Verso]);
        assert_eq!(job.backend().de_fora, [0, 1]);
    }

    #[test]
    fn so_versos_passa_por_cima_do_lote_em_duas_faces() {
        let mut misto = lotes();
        misto[0].duas_faces = true;
        let job = Job::novo(Registro::default(), misto)
            .unwrap()
            .com_alcance(Alcance::SoVersos);
        assert_eq!(job.lote_atual(), 1);
        let job = Job::novo(Registro::default(), lotes_em_duas_faces(4, 1))
            .unwrap()
            .com_alcance(Alcance::SoVersos);
        assert_eq!(job.estado(), EstadoJob::Concluido);
    }

    #[test]
    fn refazer_so_versos_volta_ao_verso_do_mesmo_lote_e_nao_a_frente() {
        let mut job = Job::novo(Registro::default(), lotes())
            .unwrap()
            .com_alcance(Alcance::SoVersos);
        job.enviar_verso().unwrap();
        job.cancelar_envio().unwrap();
        assert_eq!(job.estado(), EstadoJob::AguardandoRecolocacao);
        assert_eq!(job.lote_atual(), 0);
        job.enviar_verso().unwrap();
        assert_eq!(job.backend().cancelados, 1);
        assert_eq!(job.backend().passadas, [Passada::Verso, Passada::Verso]);
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
