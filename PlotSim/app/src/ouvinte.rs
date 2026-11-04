//! Uma linha de execução por fila, acompanhando o spooler. A tela nunca espera por ela: mesmo um
//! spooler travado só prende a própria linha.

use plotsim_core::recepcao::{Evento, Recepcao};
use plotsim_core::spooler::Spooler;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// O mesmo ritmo da recepção do núcleo: a folga antes de o próximo trabalho apagar o arquivo é maior
/// que isso.
const INTERVALO: Duration = Duration::from_millis(100);

/// O que a linha conta para quem a iniciou.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Novidade {
    Evento(Evento),
    Ocupada(bool),
    /// A leitura da fila falhou, ou voltou ao normal com `None`.
    Falha(Option<String>),
}

/// Para a linha ao ser solto. Não espera por ela, porque ela pode estar presa no sistema.
pub struct Ouvinte {
    parar: Arc<AtomicBool>,
    /// Desconecta quando a linha termina: é o sinal de que a parada chegou.
    fim: mpsc::Receiver<()>,
}

impl Drop for Ouvinte {
    fn drop(&mut self) {
        self.parar.store(true, Ordering::Relaxed);
    }
}

impl Ouvinte {
    /// Pede a parada e espera a linha terminar, no máximo até o prazo. Devolve se ela terminou.
    pub fn parar(self, prazo: Duration) -> bool {
        self.parar.store(true, Ordering::Relaxed);
        match self.fim.recv_timeout(prazo) {
            Err(RecvTimeoutError::Disconnected) => true,
            Ok(()) | Err(RecvTimeoutError::Timeout) => false,
        }
    }
}

/// Lembra o que já foi contado, para só avisar o que mudou.
#[derive(Default)]
struct Memoria {
    ocupada: bool,
    falha: Option<String>,
}

impl Memoria {
    fn passo<S: Spooler>(&mut self, recepcao: &mut Recepcao<S>, contar: &mut impl FnMut(Novidade)) {
        match recepcao.passo() {
            Ok(eventos) => {
                if self.falha.take().is_some() {
                    contar(Novidade::Falha(None));
                }
                // Avisa antes de ler o PDF, que pode demorar.
                for evento in eventos {
                    contar(Novidade::Evento(evento));
                }
            }
            Err(erro) => {
                let texto = erro.to_string();
                if self.falha.as_ref() != Some(&texto) {
                    self.falha = Some(texto.clone());
                    contar(Novidade::Falha(Some(texto)));
                }
            }
        }
        let ocupada = recepcao.ocupada();
        if ocupada != self.ocupada {
            self.ocupada = ocupada;
            contar(Novidade::Ocupada(ocupada));
        }
    }
}

pub fn iniciar<S: Spooler + Send + 'static>(
    mut recepcao: Recepcao<S>,
    mut contar: impl FnMut(Novidade) + Send + 'static,
) -> io::Result<Ouvinte> {
    let parar = Arc::new(AtomicBool::new(false));
    let parar_na_linha = Arc::clone(&parar);
    let (fim_da_linha, fim) = mpsc::channel::<()>();
    thread::Builder::new()
        .name("plotsim-ouvinte".to_string())
        .spawn(move || {
            // Ao sair, a linha solta o emissor, e quem pediu a parada vê a desconexão.
            let _fim_da_linha = fim_da_linha;
            let mut memoria = Memoria::default();
            while !parar_na_linha.load(Ordering::Relaxed) {
                memoria.passo(&mut recepcao, &mut contar);
                thread::sleep(INTERVALO);
            }
        })?;
    Ok(Ouvinte { parar, fim })
}

#[cfg(test)]
mod testes {
    use super::*;
    use plotsim_core::spooler::{ErroSpooler, TrabalhoNaFila};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;

    #[derive(Clone)]
    struct SpoolerDeTeste(Arc<Mutex<Result<Vec<TrabalhoNaFila>, ErroSpooler>>>);

    impl Spooler for SpoolerDeTeste {
        fn trabalhos(&self, _: &str) -> Result<Vec<TrabalhoNaFila>, ErroSpooler> {
            self.0.lock().unwrap().clone()
        }
    }

    fn trabalho() -> TrabalhoNaFila {
        TrabalhoNaFila {
            id: 1,
            documento: "doc".to_string(),
            status: 0x10,
            paginas_totais: 1,
            paginas_impressas: 0,
        }
    }

    fn pasta(nome: &str) -> PathBuf {
        let caminho =
            std::env::temp_dir().join(format!("plotsim-ouvinte-{}-{nome}", std::process::id()));
        let _ = fs::remove_dir_all(&caminho);
        fs::create_dir_all(&caminho).unwrap();
        caminho
    }

    #[test]
    fn conta_quando_fica_ocupada_e_quando_o_trabalho_some_e_uma_vez_cada() {
        let raiz = pasta("ocupada");
        let spooler = SpoolerDeTeste(Arc::new(Mutex::new(Ok(vec![trabalho()]))));
        let mut recepcao = Recepcao::nova(
            spooler.clone(),
            "fila".to_string(),
            raiz.join("saida.pdf"),
            raiz.join("historico"),
        )
        .unwrap();
        let mut memoria = Memoria::default();
        let mut novidades = Vec::new();
        let mut contar = |novidade| novidades.push(novidade);
        memoria.passo(&mut recepcao, &mut contar);
        memoria.passo(&mut recepcao, &mut contar);
        *spooler.0.lock().unwrap() = Ok(Vec::new());
        memoria.passo(&mut recepcao, &mut contar);
        assert_eq!(novidades.first(), Some(&Novidade::Ocupada(true)));
        assert_eq!(novidades.last(), Some(&Novidade::Ocupada(false)));
        assert_eq!(
            novidades
                .iter()
                .filter(|n| matches!(n, Novidade::Ocupada(_)))
                .count(),
            2
        );
        let _ = fs::remove_dir_all(raiz);
    }

    #[test]
    fn a_mesma_falha_so_avisa_uma_vez_e_a_volta_ao_normal_tambem() {
        let raiz = pasta("falha");
        let spooler = SpoolerDeTeste(Arc::new(Mutex::new(Err(ErroSpooler::Sistema {
            chamada: "EnumJobsW",
            codigo: 5,
        }))));
        let mut recepcao = Recepcao::nova(
            spooler.clone(),
            "fila".to_string(),
            raiz.join("saida.pdf"),
            raiz.join("historico"),
        )
        .unwrap();
        let mut memoria = Memoria::default();
        let mut novidades = Vec::new();
        let mut contar = |novidade| novidades.push(novidade);
        memoria.passo(&mut recepcao, &mut contar);
        memoria.passo(&mut recepcao, &mut contar);
        *spooler.0.lock().unwrap() = Ok(Vec::new());
        memoria.passo(&mut recepcao, &mut contar);
        assert_eq!(novidades.len(), 2, "{novidades:?}");
        assert!(matches!(novidades[0], Novidade::Falha(Some(_))));
        assert_eq!(novidades[1], Novidade::Falha(None));
        let _ = fs::remove_dir_all(raiz);
    }

    #[test]
    fn soltar_o_ouvinte_para_a_linha() {
        let raiz = pasta("parar");
        let spooler = SpoolerDeTeste(Arc::new(Mutex::new(Ok(Vec::new()))));
        let recepcao = Recepcao::nova(
            spooler,
            "fila".to_string(),
            raiz.join("saida.pdf"),
            raiz.join("historico"),
        )
        .unwrap();
        let ouvinte = iniciar(recepcao, |_| {}).unwrap();
        let parar = Arc::clone(&ouvinte.parar);
        drop(ouvinte);
        assert!(parar.load(Ordering::Relaxed));
        let _ = fs::remove_dir_all(raiz);
    }

    #[test]
    fn parar_espera_a_linha_terminar_dentro_do_prazo() {
        let raiz = pasta("parar-espera");
        let spooler = SpoolerDeTeste(Arc::new(Mutex::new(Ok(Vec::new()))));
        let recepcao = Recepcao::nova(
            spooler,
            "fila".to_string(),
            raiz.join("saida.pdf"),
            raiz.join("historico"),
        )
        .unwrap();
        let ouvinte = iniciar(recepcao, |_| {}).unwrap();
        assert!(ouvinte.parar(Duration::from_secs(5)));
        let _ = fs::remove_dir_all(raiz);
    }

    /// Um spooler que demora: a parada não pode prender a tela além do prazo.
    #[derive(Clone)]
    struct SpoolerPreso;

    impl Spooler for SpoolerPreso {
        fn trabalhos(&self, _: &str) -> Result<Vec<TrabalhoNaFila>, ErroSpooler> {
            thread::sleep(Duration::from_millis(1_500));
            Ok(Vec::new())
        }
    }

    #[test]
    fn parar_nao_espera_uma_linha_presa_alem_do_prazo() {
        let raiz = pasta("parar-preso");
        let recepcao = Recepcao::nova(
            SpoolerPreso,
            "fila".to_string(),
            raiz.join("saida.pdf"),
            raiz.join("historico"),
        )
        .unwrap();
        let ouvinte = iniciar(recepcao, |_| {}).unwrap();
        thread::sleep(Duration::from_millis(50));
        let inicio = std::time::Instant::now();
        assert!(!ouvinte.parar(Duration::from_millis(100)));
        assert!(inicio.elapsed() < Duration::from_secs(1));
        let _ = fs::remove_dir_all(raiz);
    }
}
