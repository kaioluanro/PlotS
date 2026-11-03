use super::*;
use plots_windows::plots_print::job::{EstadoEnvio, Passada};
use std::collections::VecDeque;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Instant;

const PRAZO: Duration = Duration::from_secs(20);
const A4: DimensoesPagina = DimensoesPagina::new(210.0, 297.0);
const A5: DimensoesPagina = DimensoesPagina::new(148.0, 210.0);

impl ParaMotivo for String {
    fn para_motivo(&self) -> Motivo {
        Motivo::novo("sistema", self.clone())
    }
}

#[derive(Clone, Copy)]
enum Leitura {
    Estado(EstadoEnvio),
    Aviso(&'static str),
    Erro,
    Quebra,
}

/// Backend que so registra: nenhum teste daqui alcanca uma fila.
struct Falso {
    enviados: Arc<Mutex<Vec<Passada>>>,
    cancelados: Arc<Mutex<u32>>,
    leituras: VecDeque<Leitura>,
    motivo: Option<String>,
    recusa_ao_enviar: bool,
    recusa_ao_cancelar: bool,
}

impl Backend for Falso {
    type Erro = String;
    fn enviar(&mut self, passada: Passada, _: &[Pagina]) -> Result<(), String> {
        if self.recusa_ao_enviar {
            return Err("recusado".to_string());
        }
        self.enviados
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(passada);
        Ok(())
    }
    fn consultar(&mut self) -> Result<EstadoEnvio, String> {
        self.motivo = None;
        match self
            .leituras
            .pop_front()
            .unwrap_or(Leitura::Estado(EstadoEnvio::Concluido))
        {
            Leitura::Estado(EstadoEnvio::Falhou) => {
                self.motivo = Some("o trabalho deu erro".to_string());
                Ok(EstadoEnvio::Falhou)
            }
            Leitura::Estado(estado) => Ok(estado),
            Leitura::Aviso(motivo) => {
                self.motivo = Some(motivo.to_string());
                Ok(EstadoEnvio::EmAndamento)
            }
            Leitura::Erro => Err("a fila não respondeu".to_string()),
            Leitura::Quebra => panic!("thread interrompida de proposito"),
        }
    }
    fn cancelar(&mut self) -> Result<(), String> {
        if self.recusa_ao_cancelar {
            return Err("a fila não aceitou o cancelamento".to_string());
        }
        *self
            .cancelados
            .lock()
            .unwrap_or_else(PoisonError::into_inner) += 1;
        Ok(())
    }
}

impl DescreveFalha for Falso {
    fn motivo(&self) -> Option<String> {
        self.motivo.clone()
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        false
    }
}

struct Cena {
    gerente: Gerente,
    cartoes: Arc<Mutex<Vec<Cartao>>>,
    enviados: Arc<Mutex<Vec<Passada>>>,
    cancelados: Arc<Mutex<u32>>,
}

fn identidade(impressora: &str) -> Identidade {
    Identidade {
        aba: 7,
        impressora: impressora.to_string(),
        nome: impressora.to_string(),
        simulada: false,
    }
}

impl Cena {
    fn nova() -> Self {
        Self {
            gerente: Gerente::default(),
            cartoes: Arc::default(),
            enviados: Arc::default(),
            cancelados: Arc::default(),
        }
    }

    fn emissor(&self) -> Emissor {
        let cartoes = Arc::clone(&self.cartoes);
        Arc::new(move |cartao| {
            cartoes
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(cartao);
        })
    }

    fn iniciar(&self, impressora: &str, leituras: &[Leitura], dimensoes: &[DimensoesPagina]) {
        self.iniciar_com(impressora, leituras, dimensoes, false);
    }

    fn iniciar_com(
        &self,
        impressora: &str,
        leituras: &[Leitura],
        dimensoes: &[DimensoesPagina],
        recusa_ao_enviar: bool,
    ) -> bool {
        let backend = self.backend(leituras, recusa_ao_enviar, false);
        let lotes = planejar(dimensoes, 1, PerfilImpressora::default()).unwrap();
        self.gerente
            .iniciar(identidade(impressora), self.emissor(), move || {
                Ok(Preparo {
                    condutor: Condutor::novo(
                        backend,
                        lotes,
                        Espera {
                            intervalo: Duration::from_millis(2),
                            consultas: 1_000,
                        },
                    )
                    .map_err(|erro| motivo_do_job(&erro))?,
                    recolocar: recolocar_do_perfil(Some(ModeloImpressora::Laser), None),
                    vira_para_ler: true,
                })
            })
    }

    fn backend(
        &self,
        leituras: &[Leitura],
        recusa_ao_enviar: bool,
        recusa_ao_cancelar: bool,
    ) -> Falso {
        Falso {
            enviados: Arc::clone(&self.enviados),
            cancelados: Arc::clone(&self.cancelados),
            leituras: leituras.iter().copied().collect(),
            motivo: None,
            recusa_ao_enviar,
            recusa_ao_cancelar,
        }
    }

    /// Como `iniciar_com`, mas a preparacao espera o sinal de `liberar`: o teste manda recados antes.
    fn iniciar_retido(&self, impressora: &str, liberar: Receiver<()>) -> bool {
        let backend = self.backend(&[], false, false);
        let lotes = planejar(&[A4, A4], 1, PerfilImpressora::default()).unwrap();
        self.gerente
            .iniciar(identidade(impressora), self.emissor(), move || {
                let _ = liberar.recv();
                Ok(Preparo {
                    condutor: Condutor::novo(
                        backend,
                        lotes,
                        Espera {
                            intervalo: Duration::from_millis(2),
                            consultas: 1_000,
                        },
                    )
                    .map_err(|erro| motivo_do_job(&erro))?,
                    recolocar: recolocar_do_perfil(Some(ModeloImpressora::Laser), None),
                    vira_para_ler: true,
                })
            })
    }

    // Trabalho que ja esta na fila e cuja fila recusa o cancelamento.
    fn iniciar_recusando_cancelamento(&self, impressora: &str, leituras: &[Leitura]) -> bool {
        let backend = self.backend(leituras, false, true);
        let lotes = planejar(&[A4, A4], 1, PerfilImpressora::default()).unwrap();
        self.gerente
            .iniciar(identidade(impressora), self.emissor(), move || {
                Ok(Preparo {
                    condutor: Condutor::novo(
                        backend,
                        lotes,
                        Espera {
                            intervalo: Duration::from_millis(2),
                            consultas: 1_000,
                        },
                    )
                    .map_err(|erro| motivo_do_job(&erro))?,
                    recolocar: recolocar_do_perfil(Some(ModeloImpressora::Laser), None),
                    vira_para_ler: true,
                })
            })
    }
    fn cartoes_de(&self, impressora: &str) -> Vec<Cartao> {
        self.cartoes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|cartao| cartao.impressora == impressora)
            .cloned()
            .collect()
    }

    /// Espera o cartao chegar a um estado, olhando a partir do fim da lista.
    fn esperar(&self, impressora: &str, aceita: impl Fn(&EstadoCartao) -> bool) -> Cartao {
        let inicio = Instant::now();
        loop {
            if let Some(ultimo) = self.cartoes_de(impressora).pop() {
                if aceita(&ultimo.estado) {
                    return ultimo;
                }
            }
            assert!(
                inicio.elapsed() < PRAZO,
                "sem o estado esperado: {:?}",
                self.cartoes_de(impressora)
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn enviados(&self) -> Vec<Passada> {
        self.enviados
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn cancelados(&self) -> u32 {
        *self
            .cancelados
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn comandar(&self, impressora: &str, comando: Comando) {
        self.gerente.comandar(impressora, comando).unwrap();
    }
}

fn aguardando(estado: &EstadoCartao) -> bool {
    matches!(estado, EstadoCartao::Aguardando)
}

fn ativo(cena: &Cena, impressora: &str) -> bool {
    cena.gerente
        .trabalhos
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(impressora)
        .is_some_and(|entrada| entrada.ativo.load(Ordering::SeqCst))
}

fn esperar_fim(cena: &Cena, impressora: &str) {
    let inicio = Instant::now();
    while ativo(cena, impressora) {
        assert!(inicio.elapsed() < PRAZO, "a thread nao terminou");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn verso_so_libera_com_a_confirmacao_e_o_documento_termina() {
    let cena = Cena::nova();
    cena.iniciar(
        "a",
        &[
            Leitura::Estado(EstadoEnvio::EmAndamento),
            Leitura::Estado(EstadoEnvio::EmAndamento),
        ],
        &[A4; 4],
    );
    let primeiro = cena.esperar("a", aguardando);
    assert_eq!((primeiro.lote, primeiro.total), (1, 2));
    assert_eq!(cena.enviados(), [Passada::Frente]);
    assert!(cena.cartoes_de("a")[0].estado == EstadoCartao::Preparando);
    cena.comandar("a", Comando::EnviarVerso);
    let proximo = cena.esperar("a", |estado| matches!(estado, EstadoCartao::ProximoLote));
    assert_eq!(proximo.lote, 2);
    cena.comandar("a", Comando::EnviarFrente);
    cena.esperar("a", aguardando);
    cena.comandar("a", Comando::EnviarVerso);
    let fim = cena.esperar("a", |estado| matches!(estado, EstadoCartao::Concluido));
    assert!(fim.vira_para_ler);
    esperar_fim(&cena, "a");
    assert_eq!(
        cena.enviados(),
        [
            Passada::Frente,
            Passada::Verso,
            Passada::Frente,
            Passada::Verso
        ]
    );
}

#[test]
fn pedir_o_verso_antes_da_confirmacao_nao_envia_nada() {
    let cena = Cena::nova();
    let leituras = [Leitura::Estado(EstadoEnvio::EmAndamento); 40];
    cena.iniciar("a", &leituras, &[A4; 2]);
    cena.esperar("a", |estado| {
        matches!(estado, EstadoCartao::Imprimindo { .. })
    });
    cena.comandar("a", Comando::EnviarVerso);
    cena.esperar("a", aguardando);
    assert_eq!(cena.enviados(), [Passada::Frente]);
}

#[test]
fn aviso_mostra_o_motivo_continuar_nao_toca_na_fila_e_cancelar_tira_de_la() {
    let cena = Cena::nova();
    let mut leituras = vec![Leitura::Aviso("a impressora está sem papel"); 5_000];
    leituras.push(Leitura::Estado(EstadoEnvio::Concluido));
    cena.iniciar("a", &leituras, &[A4; 2]);
    let aviso = cena.esperar("a", |estado| {
        matches!(
            estado,
            EstadoCartao::Aviso {
                respondido: false,
                ..
            }
        )
    });
    let EstadoCartao::Aviso { motivo, .. } = aviso.estado else {
        panic!("{aviso:?}");
    };
    assert_eq!(motivo.texto, "a impressora está sem papel");
    cena.comandar("a", Comando::ContinuarEsperando);
    cena.esperar("a", |estado| {
        matches!(
            estado,
            EstadoCartao::Aviso {
                respondido: true,
                ..
            }
        )
    });
    assert_eq!(cena.cancelados(), 0);
    cena.comandar("a", Comando::Cancelar);
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Cancelado));
    esperar_fim(&cena, "a");
    assert_eq!(cena.cancelados(), 1);
    assert_eq!(cena.enviados(), [Passada::Frente]);
}

#[test]
fn aviso_que_se_resolve_sozinho_volta_a_imprimindo() {
    let cena = Cena::nova();
    let mut leituras = vec![Leitura::Aviso("a impressora está offline")];
    leituras.extend([Leitura::Estado(EstadoEnvio::EmAndamento); 3]);
    cena.iniciar("a", &leituras, &[A4; 2]);
    cena.esperar("a", aguardando);
    let estados: Vec<EstadoCartao> = cena.cartoes_de("a").into_iter().map(|c| c.estado).collect();
    let posicao = estados
        .iter()
        .position(|estado| matches!(estado, EstadoCartao::Aviso { .. }))
        .unwrap();
    assert!(matches!(
        estados[posicao + 1],
        EstadoCartao::Imprimindo { .. }
    ));
}

#[test]
fn erro_na_fila_mostra_o_motivo_e_reimprimir_refaz_o_mesmo_lote() {
    let cena = Cena::nova();
    cena.iniciar(
        "a",
        &[
            Leitura::Estado(EstadoEnvio::Concluido),
            Leitura::Estado(EstadoEnvio::Falhou),
        ],
        &[A4; 2],
    );
    cena.esperar("a", aguardando);
    cena.comandar("a", Comando::EnviarVerso);
    let erro = cena.esperar("a", |estado| matches!(estado, EstadoCartao::Erro { .. }));
    assert_eq!(erro.lote, 1);
    let EstadoCartao::Erro { motivo } = erro.estado else {
        panic!("{erro:?}");
    };
    assert_eq!(motivo.texto, "o trabalho deu erro");
    cena.comandar("a", Comando::Reimprimir);
    cena.esperar("a", aguardando);
    assert_eq!(cena.cancelados(), 1);
    assert_eq!(
        cena.enviados(),
        [Passada::Frente, Passada::Verso, Passada::Frente]
    );
}

#[test]
fn erro_ao_enviar_vira_erro_no_cartao_e_cancelar_encerra() {
    let cena = Cena::nova();
    cena.iniciar_com("a", &[], &[A4; 2], true);
    let erro = cena.esperar("a", |estado| matches!(estado, EstadoCartao::Erro { .. }));
    assert_eq!(erro.lote, 1);
    cena.comandar("a", Comando::Cancelar);
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Cancelado));
    esperar_fim(&cena, "a");
}

#[test]
fn consulta_que_erra_seguidas_vezes_para_sem_apagar_o_trabalho() {
    let cena = Cena::nova();
    let leituras = [Leitura::Erro; fluxo::ERROS_DE_CONSULTA_SEGUIDOS as usize];
    cena.iniciar("a", &leituras, &[A4; 2]);
    let erro = cena.esperar("a", |estado| matches!(estado, EstadoCartao::Erro { .. }));
    let EstadoCartao::Erro { motivo } = erro.estado else {
        panic!("{erro:?}");
    };
    assert_eq!(motivo.codigo, "sem_acompanhamento");
    assert_eq!(cena.cancelados(), 0);
    assert!(ativo(&cena, "a"));
}

#[test]
fn soltar_a_aba_para_de_acompanhar_sem_apagar_nada_da_fila() {
    let cena = Cena::nova();
    cena.iniciar("a", &[], &[A4; 2]);
    cena.esperar("a", aguardando);
    cena.gerente.soltar_aba(7);
    esperar_fim(&cena, "a");
    assert_eq!(cena.cancelados(), 0);
    assert_eq!(cena.enviados(), [Passada::Frente]);
}

#[test]
fn duas_impressoras_avancam_cada_uma_no_seu_ritmo() {
    let cena = Cena::nova();
    cena.iniciar("a", &[], &[A4; 2]);
    cena.iniciar(
        "b",
        &[Leitura::Aviso("a impressora está sem papel"); 5_000],
        &[A4; 2],
    );
    cena.esperar("a", aguardando);
    cena.esperar("b", |estado| matches!(estado, EstadoCartao::Aviso { .. }));
    cena.comandar("a", Comando::EnviarVerso);
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Concluido));
    assert!(matches!(
        cena.cartoes_de("b").pop().map(|c| c.estado),
        Some(EstadoCartao::Aviso { .. })
    ));
    cena.comandar("b", Comando::Cancelar);
    cena.esperar("b", |estado| matches!(estado, EstadoCartao::Cancelado));
}

#[test]
fn a_mesma_impressora_nao_recebe_dois_trabalhos_e_volta_a_receber_depois_do_fim() {
    let cena = Cena::nova();
    assert!(cena.iniciar_com("a", &[], &[A4; 2], false));
    assert!(!cena.iniciar_com("a", &[], &[A4; 2], false));
    cena.esperar("a", aguardando);
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
    assert!(cena.iniciar_com("a", &[], &[A4; 2], false));
}

#[test]
fn thread_interrompida_vira_erro_no_cartao_e_comando_devolve_erro() {
    let cena = Cena::nova();
    cena.iniciar("a", &[Leitura::Quebra], &[A4; 2]);
    let erro = cena.esperar("a", |estado| matches!(estado, EstadoCartao::Erro { .. }));
    let EstadoCartao::Erro { motivo } = erro.estado else {
        panic!("{erro:?}");
    };
    assert_eq!(motivo.codigo, "interrompido");
    esperar_fim(&cena, "a");
    let inicio = Instant::now();
    while cena.gerente.comandar("a", Comando::Cancelar).is_ok() {
        assert!(inicio.elapsed() < PRAZO);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(cena.gerente.comandar("outra", Comando::Cancelar).is_err());
}

#[test]
fn preparacao_recusada_mostra_o_motivo_sem_enviar_nada() {
    let cena = Cena::nova();
    cena.gerente
        .iniciar::<Falso, _>(identidade("a"), cena.emissor(), || {
            Err(Motivo::novo("fila_pede_arquivo", "pede arquivo"))
        });
    let erro = cena.esperar("a", |estado| matches!(estado, EstadoCartao::Erro { .. }));
    let EstadoCartao::Erro { motivo } = erro.estado else {
        panic!("{erro:?}");
    };
    assert_eq!(motivo.codigo, "fila_pede_arquivo");
    assert!(cena.enviados().is_empty());
    cena.comandar("a", Comando::Reimprimir);
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
}

#[test]
fn troca_de_papel_chega_no_cartao_do_proximo_lote() {
    let cena = Cena::nova();
    cena.iniciar("a", &[], &[A4, A4, A5, A5]);
    cena.esperar("a", aguardando);
    cena.comandar("a", Comando::EnviarVerso);
    let proximo = cena.esperar("a", |estado| matches!(estado, EstadoCartao::ProximoLote));
    assert_eq!(
        proximo.troca_de_papel.map(|papel| papel.formato),
        Some("a5")
    );
    assert_eq!(proximo.papel.map(|papel| papel.formato), Some("a5"));
}

#[test]
fn simulacao_pela_thread_entrega_a_sequencia_do_modelo_e_o_resultado() {
    let casos = [
        (ModeloImpressora::JatoDeTinta, [5, 3, 1], true, false),
        (ModeloImpressora::Laser, [1, 3, 5], false, true),
    ];
    for (modelo, verso, girada, vira) in casos {
        let cena = Cena::nova();
        let lotes = planejar(&[A4; 6], 10, modelo.perfil()).unwrap();
        let mut id = identidade("sim");
        id.simulada = true;
        cena.gerente.iniciar(id, cena.emissor(), move || {
            Ok(Preparo {
                condutor: Condutor::novo(
                    SimGuiado::novo(Mecanica::do_modelo(modelo), modelo.gesto().into()),
                    lotes,
                    Espera {
                        intervalo: Duration::ZERO,
                        consultas: 10,
                    },
                )
                .map_err(|erro| motivo_do_job(&erro))?,
                recolocar: Recolocar::da_instrucao(Instrucao::do_modelo(modelo)),
                vira_para_ler: fluxo::vira_para_ler(modelo),
            })
        });
        let espera = cena.esperar("sim", aguardando);
        let sequencia = espera.sequencia.clone().unwrap();
        let numeros = |lista: &[FaceDaSequencia]| -> Vec<usize> {
            lista.iter().filter_map(|face| face.numero).collect()
        };
        assert_eq!(numeros(&sequencia.frente), [6, 4, 2]);
        assert_eq!(numeros(&sequencia.verso), verso);
        assert!(sequencia.verso.iter().all(|face| face.girada == girada));
        assert_eq!(espera.recolocar.as_ref().unwrap().lado, Some("baixo"));
        cena.comandar("sim", Comando::EnviarVerso);
        let fim = cena.esperar("sim", |estado| matches!(estado, EstadoCartao::Concluido));
        assert_eq!(
            fim.resultado,
            Some(Resultado {
                corretas: 3,
                folhas: 3
            })
        );
        assert_eq!(fim.vira_para_ler, vira);
    }
}

#[test]
fn lote_vindo_de_fora_fica_dentro_do_limite() {
    assert!(Pedido::novo("a.pdf".into(), 0).is_err());
    assert_eq!(
        Pedido::novo("a.pdf".into(), 9_999).unwrap().folhas_por_lote,
        50
    );
    assert_eq!(Pedido::novo("a.pdf".into(), 7).unwrap().folhas_por_lote, 7);
}

#[test]
fn perfil_sem_modelo_nao_afirma_lado_nem_virada() {
    let recolocar = recolocar_do_perfil(None, Some("Vire pela lateral.".into()));
    assert_eq!(recolocar.lado, None);
    assert_eq!(recolocar.virada, None);
    assert_eq!(
        recolocar.texto_do_perfil.as_deref(),
        Some("Vire pela lateral.")
    );
}

#[test]
fn estado_do_cartao_chega_a_interface_com_os_nomes_que_ela_espera() {
    let cartao = identidade("a").cartao(EstadoCartao::Aviso {
        passada: PassadaDoCartao::Verso,
        motivo: Motivo::novo("fila", "x"),
        respondido: false,
    });
    let texto = serde_json_de(&cartao);
    assert!(texto.contains(r#""tipo":"aviso""#), "{texto}");
    assert!(texto.contains(r#""passada":"verso""#), "{texto}");
    assert!(texto.contains(r#""viraParaLer":false"#), "{texto}");
    assert!(texto.contains(r#""trocaDePapel":null"#), "{texto}");
}

fn serde_json_de(cartao: &Cartao) -> String {
    match tauri::ipc::IpcResponse::body(cartao.clone()).unwrap() {
        tauri::ipc::InvokeResponseBody::Json(texto) => texto,
        tauri::ipc::InvokeResponseBody::Raw(_) => panic!("resposta binaria"),
    }
}

/// As frases da janela em portugues sao as do terminal, palavra por palavra.
#[test]
fn frases_da_janela_em_portugues_sao_as_do_fluxo_compartilhado() {
    let i18n = include_str!("../../ui/src/i18n.ts");
    let (jato_virada, jato_lado) =
        fluxo::frases_de_recolocacao_do_modelo(ModeloImpressora::JatoDeTinta);
    let (laser_virada, _) = fluxo::frases_de_recolocacao_do_modelo(ModeloImpressora::Laser);
    for frase in [
        jato_lado.as_str(),
        jato_virada,
        laser_virada,
        fluxo::FRASE_ESPERAR_FOLHA,
        fluxo::FRASE_DA_CALIBRACAO,
        fluxo::FRASE_VIRAR_PARA_LER,
    ] {
        assert!(i18n.contains(frase), "falta na interface: {frase}");
    }
}

/// O motivo vindo da fila so tem traducao porque a interface conhece cada frase.
#[test]
fn motivos_da_fila_tem_traducao_na_interface() {
    let cartoes = include_str!("../../ui/src/cartoes.ts");
    for status in [0x40u32, 0x20, 0x400, 0x200, 0x2, 0x4] {
        let motivo = plots_windows::traduzir_status(status).motivo.unwrap();
        for parte in motivo.split("; ") {
            assert!(cartoes.contains(parte), "sem traducao: {parte}");
        }
    }
}

fn ler_comando(texto: &str) -> Result<Comando, serde::de::value::Error> {
    Comando::deserialize(serde::de::value::StrDeserializer::new(texto))
}

#[test]
fn comando_desconhecido_e_recusado_ao_ler() {
    assert!(ler_comando("apagar_tudo").is_err());
    assert_eq!(ler_comando("enviar_verso").unwrap(), Comando::EnviarVerso);
}

#[test]
fn espera_com_canal_fechado_nao_trava() {
    let (envia, recebe) = channel::<Recado>();
    drop(envia);
    assert!(matches!(
        recebe.recv_timeout(Duration::ZERO),
        Err(RecvTimeoutError::Disconnected)
    ));
}

#[test]
fn soltar_durante_a_preparacao_nao_envia_a_frente() {
    let cena = Cena::nova();
    let (liberar, preparo_liberado) = channel::<()>();
    assert!(cena.iniciar_retido("a", preparo_liberado));
    cena.gerente.soltar_aba(7);
    liberar.send(()).unwrap();
    esperar_fim(&cena, "a");
    assert!(cena.enviados().is_empty(), "{:?}", cena.enviados());
}

#[test]
fn cancelar_durante_a_preparacao_mostra_cancelado_sem_enviar_nada() {
    let cena = Cena::nova();
    let (liberar, preparo_liberado) = channel::<()>();
    assert!(cena.iniciar_retido("a", preparo_liberado));
    cena.comandar("a", Comando::Cancelar);
    liberar.send(()).unwrap();
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Cancelado));
    esperar_fim(&cena, "a");
    assert!(cena.enviados().is_empty(), "{:?}", cena.enviados());
    assert_eq!(cena.cancelados(), 0);
}

#[test]
fn cancelamento_recusado_pela_fila_pede_conferir_a_fila_e_nao_esquece_o_trabalho() {
    let cena = Cena::nova();
    assert!(cena.iniciar_recusando_cancelamento("a", &[Leitura::Erro; 30]));
    cena.esperar("a", |estado| {
        matches!(estado, EstadoCartao::Erro { motivo } if motivo.codigo == "sem_acompanhamento")
    });
    cena.comandar("a", Comando::Cancelar);
    let cartao = cena.esperar(
        "a",
        |estado| matches!(estado, EstadoCartao::Erro { motivo } if motivo.codigo == "nao_cancelou"),
    );
    let EstadoCartao::Erro { motivo } = cartao.estado else {
        panic!("o cartao devia mostrar o erro do cancelamento");
    };
    assert!(
        motivo.texto.contains("Confira a fila do Windows"),
        "{}",
        motivo.texto
    );
    assert_eq!(cena.cancelados(), 0);
}

#[test]
fn reimprimir_com_cancelamento_recusado_nao_envia_a_frente_de_novo() {
    let cena = Cena::nova();
    assert!(cena.iniciar_recusando_cancelamento("a", &[Leitura::Erro; 30]));
    cena.esperar("a", |estado| {
        matches!(estado, EstadoCartao::Erro { motivo } if motivo.codigo == "sem_acompanhamento")
    });
    let antes = cena.enviados();
    cena.comandar("a", Comando::Reimprimir);
    let cartao = cena.esperar(
        "a",
        |estado| matches!(estado, EstadoCartao::Erro { motivo } if motivo.codigo == "nao_cancelou"),
    );
    assert_eq!(
        cena.enviados(),
        antes,
        "a frente nao pode ser enviada de novo"
    );
    assert_eq!(cena.cancelados(), 0);
    let EstadoCartao::Erro { motivo } = cartao.estado else {
        panic!("o cartao devia mostrar o erro do cancelamento");
    };
    assert!(
        motivo.texto.contains("Confira a fila do Windows"),
        "{}",
        motivo.texto
    );
}
