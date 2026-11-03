//! Pergunta aos drivers se a fila imprime as duas faces, sem deixar um driver lento segurar a lista.
//!
//! `DeviceCapabilitiesW` pode demorar dezenas de segundos em fila de rede. Cada pergunta corre na sua
//! propria thread, a leitura espera ate um limite somado para todas, e a resposta que chega depois
//! fica guardada para a proxima leitura. Sem resposta, a fila apenas nao oferece o modelo automatico.

use plots_windows::{Condicao, Fila};
use std::collections::BTreeMap;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// O quanto uma leitura da lista espera, somando todas as filas.
const ESPERA_POR_LEITURA: Duration = Duration::from_millis(1500);
/// Por quanto tempo uma resposta vale, inclusive a falta dela.
const VALIDADE_DA_RESPOSTA: Duration = Duration::from_secs(60);
/// Mais que isso, as respostas guardadas recomecam: a memoria nao cresce sem limite.
const FILAS_LEMBRADAS: usize = 256;
/// Um driver travado prende a sua thread: o numero delas tem teto. Quem passa dele fica para depois.
const PERGUNTAS_AO_MESMO_TEMPO: usize = 32;

/// O driver e a porta identificam a resposta: trocar o driver da fila pergunta de novo.
type ChaveDaFila = (String, String, String);

fn chave_da(fila: &Fila) -> ChaveDaFila {
    (fila.nome.clone(), fila.driver.clone(), fila.porta.clone())
}

enum Lembrada {
    /// Ha uma thread perguntando; ninguem pergunta de novo nem espera por ela outra vez.
    Perguntando,
    Respondida {
        resposta: Option<bool>,
        vale_ate: Instant,
    },
}

struct Memoria {
    filas: BTreeMap<ChaveDaFila, Lembrada>,
    perguntando: usize,
}

pub struct ConsultaDeDuasFaces {
    memoria: Mutex<Memoria>,
    respondeu: Condvar,
    espera: Duration,
    validade: Duration,
}

/// Fila parada nao responde, ou responde so depois de esgotar o tempo da rede: nem se pergunta.
fn fila_sem_contato(fila: &Fila) -> bool {
    fila.estado.condicoes().iter().any(|condicao| {
        matches!(
            condicao,
            Condicao::Offline
                | Condicao::ServidorOffline
                | Condicao::ServidorDesconhecido
                | Condicao::Indisponivel
                | Condicao::Erro
        )
    })
}

impl ConsultaDeDuasFaces {
    pub const fn padrao() -> Self {
        Self::nova(ESPERA_POR_LEITURA, VALIDADE_DA_RESPOSTA)
    }

    const fn nova(espera: Duration, validade: Duration) -> Self {
        Self {
            memoria: Mutex::new(Memoria {
                filas: BTreeMap::new(),
                perguntando: 0,
            }),
            respondeu: Condvar::new(),
            espera,
            validade,
        }
    }

    fn travar(&self) -> MutexGuard<'_, Memoria> {
        self.memoria.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn guardar(&self, chave: ChaveDaFila, resposta: Option<bool>) {
        let agora = Instant::now();
        let mut memoria = self.travar();
        memoria.perguntando = memoria.perguntando.saturating_sub(1);
        memoria.filas.insert(
            chave,
            Lembrada::Respondida {
                resposta,
                vale_ate: agora.checked_add(self.validade).unwrap_or(agora),
            },
        );
        self.respondeu.notify_all();
    }

    /// Pergunta o que falta saber e espera, no maximo, o limite de uma leitura. Quem ja foi perguntado
    /// em outra leitura nao atrasa esta: so se espera pelas perguntas que comecaram aqui.
    pub fn perguntar_as_que_faltam<P>(&'static self, filas: &[Fila], perguntar: P)
    where
        P: Fn(&Fila) -> Option<bool> + Clone + Send + 'static,
    {
        let inicio = Instant::now();
        let mut memoria = self.travar();
        let mut novas = Vec::new();
        for fila in filas {
            if memoria.perguntando >= PERGUNTAS_AO_MESMO_TEMPO {
                break;
            }
            let chave = chave_da(fila);
            let ja_sabida = match memoria.filas.get(&chave) {
                Some(Lembrada::Perguntando) => true,
                Some(Lembrada::Respondida { vale_ate, .. }) => *vale_ate > inicio,
                None => false,
            };
            if ja_sabida || fila_sem_contato(fila) {
                continue;
            }
            if memoria.filas.len() >= FILAS_LEMBRADAS {
                memoria
                    .filas
                    .retain(|_, lembrada| matches!(lembrada, Lembrada::Perguntando));
            }
            memoria.filas.insert(chave.clone(), Lembrada::Perguntando);
            memoria.perguntando = memoria.perguntando.saturating_add(1);
            let (fila_da_thread, chave_da_thread, perguntar) =
                (fila.clone(), chave.clone(), perguntar.clone());
            let iniciada = std::thread::Builder::new()
                .name("duas-faces-do-driver".into())
                .spawn(move || {
                    let resposta = perguntar(&fila_da_thread);
                    self.guardar(chave_da_thread, resposta);
                });
            if iniciada.is_err() {
                // Sem thread nao ha pergunta: a fila fica sem o modelo e tenta de novo depois do prazo.
                memoria.perguntando = memoria.perguntando.saturating_sub(1);
                memoria.filas.insert(
                    chave,
                    Lembrada::Respondida {
                        resposta: None,
                        vale_ate: inicio.checked_add(self.validade).unwrap_or(inicio),
                    },
                );
                continue;
            }
            novas.push(chave);
        }
        let fim = inicio.checked_add(self.espera).unwrap_or(inicio);
        while novas
            .iter()
            .any(|chave| matches!(memoria.filas.get(chave), Some(Lembrada::Perguntando)))
        {
            let Some(resta) = fim.checked_duration_since(Instant::now()) else {
                return;
            };
            if resta.is_zero() {
                return;
            }
            memoria = self
                .respondeu
                .wait_timeout(memoria, resta)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }

    /// O que o driver desta fila declarou e ainda vale. Sem resposta valida, nao oferece o modelo.
    pub fn declara(&self, fila: &Fila) -> bool {
        let agora = Instant::now();
        matches!(
            self.travar().filas.get(&chave_da(fila)),
            Some(Lembrada::Respondida { resposta: Some(true), vale_ate }) if *vale_ate > agora
        )
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use plots_windows::Estado;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{channel, Receiver};
    use std::sync::Arc;

    const OFFLINE: u32 = 0x0000_0080;
    const ERRO: u32 = 0x0000_0002;
    const SEM_PAPEL: u32 = 0x0000_0010;
    const UMA_HORA: Duration = Duration::from_secs(3600);
    /// Passou disto, a pergunta falsa desiste: um erro na espera falha o teste em vez de travar.
    const TETO_DA_PERGUNTA_FALSA: Duration = Duration::from_secs(20);

    fn fila(nome: &str, bits: u32) -> Fila {
        Fila {
            nome: nome.to_owned(),
            driver: "Driver".to_owned(),
            porta: "USB001".to_owned(),
            padrao: false,
            estado: Estado::de_bits(bits),
        }
    }

    fn consulta(espera: Duration, validade: Duration) -> &'static ConsultaDeDuasFaces {
        Box::leak(Box::new(ConsultaDeDuasFaces::nova(espera, validade)))
    }

    /// Pergunta que conta as chamadas e responde sempre o mesmo.
    fn contada(
        resposta: Option<bool>,
    ) -> (
        Arc<AtomicUsize>,
        impl Fn(&Fila) -> Option<bool> + Clone + Send + 'static,
    ) {
        let perguntas = Arc::new(AtomicUsize::new(0));
        let contador = Arc::clone(&perguntas);
        (perguntas, move |_: &Fila| {
            contador.fetch_add(1, Ordering::SeqCst);
            resposta
        })
    }

    /// Pergunta que so responde quando o teste soltar.
    fn presa(
        resposta: Option<bool>,
    ) -> (
        Arc<AtomicUsize>,
        std::sync::mpsc::Sender<()>,
        impl Fn(&Fila) -> Option<bool> + Clone + Send + 'static,
    ) {
        let (solta, espera): (_, Receiver<()>) = channel();
        let espera = Arc::new(Mutex::new(espera));
        let perguntas = Arc::new(AtomicUsize::new(0));
        let contador = Arc::clone(&perguntas);
        (perguntas, solta, move |_: &Fila| {
            contador.fetch_add(1, Ordering::SeqCst);
            let _ = espera.lock().unwrap().recv_timeout(TETO_DA_PERGUNTA_FALSA);
            resposta
        })
    }

    fn esperar_ate(condicao: impl Fn() -> bool) {
        let inicio = Instant::now();
        while !condicao() {
            assert!(inicio.elapsed() < Duration::from_secs(10), "nao respondeu");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn fila_offline_ou_com_erro_nao_e_perguntada() {
        let consulta = consulta(Duration::from_secs(2), UMA_HORA);
        let (perguntas, perguntar) = contada(Some(true));
        let filas = [fila("Longe", OFFLINE), fila("Quebrada", ERRO)];
        consulta.perguntar_as_que_faltam(&filas, perguntar);
        assert_eq!(perguntas.load(Ordering::SeqCst), 0);
        assert!(!consulta.declara(&filas[0]));
        assert!(!consulta.declara(&filas[1]));
    }

    #[test]
    fn fila_sem_papel_continua_sendo_perguntada() {
        let consulta = consulta(Duration::from_secs(5), UMA_HORA);
        let (perguntas, perguntar) = contada(Some(true));
        let filas = [fila("Bandeja", SEM_PAPEL)];
        consulta.perguntar_as_que_faltam(&filas, perguntar);
        assert_eq!(perguntas.load(Ordering::SeqCst), 1);
        assert!(consulta.declara(&filas[0]));
    }

    #[test]
    fn resposta_que_chega_a_tempo_vale_nesta_leitura_e_nao_pergunta_de_novo() {
        let consulta = consulta(Duration::from_secs(5), UMA_HORA);
        let (perguntas, perguntar) = contada(Some(true));
        let filas = [fila("Sala", 0)];
        consulta.perguntar_as_que_faltam(&filas, perguntar.clone());
        assert!(consulta.declara(&filas[0]));
        consulta.perguntar_as_que_faltam(&filas, perguntar);
        assert_eq!(perguntas.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn pergunta_lenta_nao_segura_a_leitura_e_a_resposta_fica_para_a_proxima() {
        let consulta = consulta(Duration::from_millis(50), UMA_HORA);
        let (perguntas, solta, perguntar) = presa(Some(true));
        let filas = [fila("Rede", 0)];
        let inicio = Instant::now();
        consulta.perguntar_as_que_faltam(&filas, perguntar);
        assert!(inicio.elapsed() < TETO_DA_PERGUNTA_FALSA / 2);
        assert!(!consulta.declara(&filas[0]), "sem resposta, sem o modelo");
        solta.send(()).unwrap();
        esperar_ate(|| consulta.declara(&filas[0]));
        assert_eq!(perguntas.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn atraso_nao_se_repete_enquanto_a_pergunta_esta_presa() {
        let consulta = consulta(Duration::from_millis(300), UMA_HORA);
        let (perguntas, solta, perguntar) = presa(None);
        let filas = [fila("Rede", 0)];
        consulta.perguntar_as_que_faltam(&filas, perguntar.clone());
        let segunda = Instant::now();
        consulta.perguntar_as_que_faltam(&filas, perguntar);
        assert!(
            segunda.elapsed() < Duration::from_millis(250),
            "a segunda leitura esperou de novo: {:?}",
            segunda.elapsed()
        );
        assert_eq!(perguntas.load(Ordering::SeqCst), 1);
        solta.send(()).unwrap();
    }

    #[test]
    fn falta_de_resposta_fica_guardada_ate_o_prazo_e_depois_pergunta_de_novo() {
        let consulta = consulta(Duration::from_secs(5), Duration::from_millis(150));
        let (perguntas, perguntar) = contada(None);
        let filas = [fila("Rede", 0)];
        consulta.perguntar_as_que_faltam(&filas, perguntar.clone());
        consulta.perguntar_as_que_faltam(&filas, perguntar.clone());
        assert_eq!(perguntas.load(Ordering::SeqCst), 1, "nao repete no prazo");
        std::thread::sleep(Duration::from_millis(300));
        consulta.perguntar_as_que_faltam(&filas, perguntar);
        assert_eq!(
            perguntas.load(Ordering::SeqCst),
            2,
            "repete depois do prazo"
        );
    }

    #[test]
    fn resposta_vencida_nao_oferece_o_modelo() {
        let consulta = consulta(Duration::from_secs(5), Duration::from_millis(100));
        let (_, perguntar) = contada(Some(true));
        let filas = [fila("Sala", 0)];
        consulta.perguntar_as_que_faltam(&filas, perguntar);
        std::thread::sleep(Duration::from_millis(250));
        assert!(!consulta.declara(&filas[0]));
    }

    #[test]
    fn varias_filas_lentas_nao_somam_a_espera() {
        let espera = Duration::from_millis(300);
        let consulta = consulta(espera, UMA_HORA);
        let (perguntas, solta, perguntar) = presa(None);
        let filas = [fila("A", 0), fila("B", 0), fila("C", 0), fila("D", 0)];
        let inicio = Instant::now();
        consulta.perguntar_as_que_faltam(&filas, perguntar);
        assert!(inicio.elapsed() < espera * 3, "{:?}", inicio.elapsed());
        esperar_ate(|| perguntas.load(Ordering::SeqCst) >= 1);
        drop(solta);
    }

    #[test]
    fn driver_travado_nao_cria_threads_sem_limite() {
        let consulta = consulta(Duration::from_millis(10), UMA_HORA);
        let (perguntas, solta, perguntar) = presa(None);
        let filas: Vec<Fila> = (0..(PERGUNTAS_AO_MESMO_TEMPO * 2))
            .map(|indice| fila(&format!("F{indice}"), 0))
            .collect();
        consulta.perguntar_as_que_faltam(&filas, perguntar);
        assert_eq!(consulta.travar().perguntando, PERGUNTAS_AO_MESMO_TEMPO);
        assert!(perguntas.load(Ordering::SeqCst) <= PERGUNTAS_AO_MESMO_TEMPO);
        drop(solta);
    }

    #[test]
    fn trocar_o_driver_da_fila_pergunta_de_novo() {
        let consulta = consulta(Duration::from_secs(5), UMA_HORA);
        let (perguntas, perguntar) = contada(Some(true));
        let antes = fila("Sala", 0);
        consulta.perguntar_as_que_faltam(std::slice::from_ref(&antes), perguntar.clone());
        let depois = Fila {
            driver: "Outro".to_owned(),
            ..antes
        };
        assert!(!consulta.declara(&depois));
        consulta.perguntar_as_que_faltam(std::slice::from_ref(&depois), perguntar);
        assert_eq!(perguntas.load(Ordering::SeqCst), 2);
        assert!(consulta.declara(&depois));
    }

    #[test]
    fn a_memoria_de_respostas_nao_cresce_sem_limite() {
        let consulta = consulta(Duration::from_secs(5), UMA_HORA);
        let (_, perguntar) = contada(Some(true));
        for indice in 0..(FILAS_LEMBRADAS * 3) {
            let filas = [fila(&format!("F{indice}"), 0)];
            consulta.perguntar_as_que_faltam(&filas, perguntar.clone());
        }
        assert!(consulta.travar().filas.len() <= FILAS_LEMBRADAS);
    }
}
