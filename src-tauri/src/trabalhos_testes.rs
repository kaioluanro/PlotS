use super::*;
use plots_windows::plots_print::job::{EstadoEnvio, Job, Passada};
use std::collections::VecDeque;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Instant;

const PRAZO: Duration = Duration::from_secs(20);
const A4: DimensoesPagina = DimensoesPagina::new(210.0, 297.0);
const A5: DimensoesPagina = DimensoesPagina::new(148.0, 210.0);

fn entrada_de_modelo(modelo: ModeloImpressora) -> plots_perfis::EntradaPerfil {
    plots_perfis::EntradaPerfil {
        apelido: None,
        perfil: modelo.perfil(),
        gesto_texto: None,
        modelo: Some(modelo),
        automatico: false,
        driver_inverte: false,
    }
}

#[test]
fn sem_calibracao_o_envio_usa_o_gesto_e_a_leitura_do_modelo() {
    for modelo in [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser] {
        let (recolocar, le_virando) = gesto_e_leitura(&entrada_de_modelo(modelo), None);
        assert_eq!(
            recolocar,
            Recolocar::da_instrucao(Instrucao::do_modelo(modelo))
        );
        assert_eq!(le_virando, fluxo::vira_para_ler(modelo));
    }
}

#[test]
fn com_calibracao_o_envio_segue_o_que_as_respostas_disseram() {
    // Um modelo que diria o contrario nao pode ganhar da medida, nem para ler nem para recolocar.
    for modelo in [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser] {
        for le_virando in [true, false] {
            for entra_para_cima in [true, false] {
                let entrada = entrada_de_modelo(modelo);
                let (recolocar, lida) = gesto_e_leitura(
                    &entrada,
                    Some(plots_perfis::Calibracao {
                        face_impressa_entra_para_cima: entra_para_cima,
                        le_virando,
                    }),
                );
                assert_eq!(lida, le_virando);
                assert_eq!(
                    recolocar,
                    Recolocar::calibrado(entrada.perfil, entra_para_cima)
                );
            }
        }
    }
}

#[test]
fn ajustar_papel_planeja_documento_misto_em_um_formato() {
    let dimensoes = [A4, A5, A4, A5];
    let pedido = Pedido::novo("documento.pdf".into(), None);
    let sem_ajuste =
        planejar_pedido(&dimensoes, &pedido, PerfilImpressora::default(), false).unwrap();
    assert_eq!(sem_ajuste.len(), 4, "sem ajuste, um trecho por papel");
    let pedido = pedido.com_papel_de_ajuste(plots_core::ajuste::PapelDeAjuste::de_codigo("a4"));
    let lotes = planejar_pedido(&dimensoes, &pedido, PerfilImpressora::default(), false).unwrap();
    assert_eq!(lotes.len(), 1);
    assert_eq!(lotes[0].papel.formato, plots_core::FormatoPapel::A4);
}

#[test]
fn uma_copia_por_vez_separa_as_frentes_manuais_e_o_automatico() {
    let perfil = PerfilImpressora::default();
    let manual = Pedido::novo("documento.pdf".into(), None).com_copias(3);
    let junto = planejar_pedido(&[A4; 2], &manual, perfil, false).unwrap();
    assert_eq!(junto.len(), 1, "sem a opção, as frentes vão num maço só");
    let separado = planejar_pedido(
        &[A4; 2],
        &manual.clone().com_uma_copia_por_vez(true),
        perfil,
        false,
    )
    .unwrap();
    assert_eq!(separado.len(), 3, "com a opção, uma frente por cópia");
    for (indice, lote) in separado.iter().enumerate() {
        assert!(
            lote.lote
                .paginas
                .iter()
                .all(|pagina| pagina.copia == indice),
            "cópia {indice}"
        );
    }

    let automatico = manual.clone().com_alcance(Alcance::SoFrentes);
    let junto = planejar_pedido(&[A4; 2], &automatico, perfil, true).unwrap();
    assert_eq!(junto.len(), 1, "sem a opção, o automático junta as cópias");
    let separado = planejar_pedido(
        &[A4; 2],
        &automatico.com_uma_copia_por_vez(true),
        perfil,
        true,
    )
    .unwrap();
    assert_eq!(separado.len(), 3, "com a opção, uma cópia por trabalho");
}

impl ParaMotivo for String {
    fn para_motivo(&self) -> Motivo {
        Motivo::novo("sistema", self.clone())
    }
}

#[derive(Clone, Copy)]
enum Leitura {
    Estado(EstadoEnvio),
    Aviso(&'static str),
    /// Os bits do trabalho e da fila, traduzidos como no envio real.
    Spooler(u32, u32),
    /// Sem papel na fila, com o texto que o driver mandou.
    ComMensagem(&'static str),
    Erro,
    Quebra,
}

/// Backend que so registra: nenhum teste daqui alcanca uma fila.
struct Falso {
    enviados: Arc<Mutex<Vec<Passada>>>,
    cancelados: Arc<Mutex<u32>>,
    eventos: Arc<Mutex<Vec<&'static str>>>,
    leituras: VecDeque<Leitura>,
    motivo: Option<String>,
    condicao: Option<CondicaoDoAviso>,
    mensagem: Option<String>,
    recusa_ao_enviar: bool,
    recusa_ao_cancelar: bool,
}

impl Backend for Falso {
    type Erro = String;
    fn enviar(&mut self, passada: Passada, _: &[Pagina]) -> Result<(), String> {
        if self.recusa_ao_enviar {
            return Err("recusado".to_string());
        }
        self.eventos
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push("enviar");
        self.enviados
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(passada);
        Ok(())
    }
    fn consultar(&mut self) -> Result<EstadoEnvio, String> {
        self.eventos
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push("consultar");
        self.motivo = None;
        self.condicao = None;
        self.mensagem = None;
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
            Leitura::Spooler(trabalho, fila) => {
                let estado_da_fila = plots_windows::Estado::de_bits(fila);
                let situacao = plots_windows::traduzir_com_fila(trabalho, Some(&estado_da_fila));
                self.motivo = situacao.motivo;
                self.condicao = situacao.condicao;
                Ok(situacao.estado)
            }
            Leitura::ComMensagem(mensagem) => {
                self.motivo = Some("a impressora está sem papel".to_string());
                self.condicao = Some(CondicaoDoAviso::SemPapel);
                self.mensagem = Some(mensagem.to_string());
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
    fn condicao(&self) -> Option<CondicaoDoAviso> {
        self.condicao
    }
    fn mensagem_do_driver(&self) -> Option<String> {
        self.mensagem.clone()
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
    eventos: Arc<Mutex<Vec<&'static str>>>,
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
            eventos: Arc::default(),
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
        let lotes = planejar(dimensoes, None, 1, PerfilImpressora::default()).unwrap();
        self.gerente
            .iniciar(identidade(impressora), self.emissor(), move || {
                Ok(Preparo {
                    uma_copia_por_vez: false,
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
            eventos: Arc::clone(&self.eventos),
            leituras: leituras.iter().copied().collect(),
            motivo: None,
            condicao: None,
            mensagem: None,
            recusa_ao_enviar,
            recusa_ao_cancelar,
        }
    }

    /// Como `iniciar_com`, mas a preparacao espera o sinal de `liberar`: o teste manda recados antes.
    fn iniciar_retido(&self, impressora: &str, liberar: Receiver<()>) -> bool {
        let backend = self.backend(&[], false, false);
        let lotes = planejar(&[A4, A4], None, 1, PerfilImpressora::default()).unwrap();
        self.gerente
            .iniciar(identidade(impressora), self.emissor(), move || {
                let _ = liberar.recv();
                Ok(Preparo {
                    uma_copia_por_vez: false,
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

    /// Plano separado por cópia: a frente de cada uma sai em um trabalho e o próximo só depois
    /// da confirmação da fila.
    fn iniciar_uma_copia_por_vez(
        &self,
        impressora: &str,
        leituras: &[Leitura],
        dimensoes: &[DimensoesPagina],
        copias: usize,
    ) -> bool {
        let backend = self.backend(leituras, false, false);
        let plano = plots_core::plano_copias_do_intervalo_separadas(
            dimensoes,
            None,
            copias,
            OrdemPassadas::default(),
            PerfilImpressora::default(),
        )
        .unwrap();
        let lotes = fluxo::lotes_do_plano(&plano);
        self.gerente
            .iniciar(identidade(impressora), self.emissor(), move || {
                Ok(Preparo {
                    uma_copia_por_vez: true,
                    condutor: Condutor::novo(
                        backend,
                        lotes,
                        Espera {
                            intervalo: Duration::from_millis(2),
                            consultas: 1_000,
                        },
                    )
                    .map_err(|erro| motivo_do_job(&erro))?
                    .com_alcance(Alcance::SoFrentes),
                    recolocar: recolocar_do_perfil(Some(ModeloImpressora::Laser), None),
                    vira_para_ler: true,
                })
            })
    }

    // Trabalho que ja esta na fila e cuja fila recusa o cancelamento.
    fn iniciar_recusando_cancelamento(&self, impressora: &str, leituras: &[Leitura]) -> bool {
        let backend = self.backend(leituras, false, true);
        let lotes = planejar(&[A4, A4], None, 1, PerfilImpressora::default()).unwrap();
        self.gerente
            .iniciar(identidade(impressora), self.emissor(), move || {
                Ok(Preparo {
                    uma_copia_por_vez: false,
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

    fn eventos(&self) -> Vec<&'static str> {
        self.eventos
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
fn cancelar_um_trabalho_na_fila_mostra_cancelando_antes_de_tirar_da_fila() {
    let cena = Cena::nova();
    let mut leituras = vec![Leitura::Aviso("a impressora está sem papel"); 5_000];
    leituras.push(Leitura::Estado(EstadoEnvio::Concluido));
    cena.iniciar("a", &leituras, &[A4; 2]);
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Aviso { .. }));
    cena.comandar("a", Comando::Cancelar);
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Cancelado));
    esperar_fim(&cena, "a");
    let estados: Vec<EstadoCartao> = cena.cartoes_de("a").into_iter().map(|c| c.estado).collect();
    let cancelando = estados
        .iter()
        .position(|estado| matches!(estado, EstadoCartao::Cancelando))
        .expect("o cartão devia mostrar Cancelando");
    assert!(matches!(estados[cancelando + 1], EstadoCartao::Cancelado));
}

#[test]
fn aviso_que_se_resolve_sozinho_volta_a_imprimindo() {
    let cena = Cena::nova();
    let mut leituras = vec![Leitura::Aviso("a impressora não responde")];
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
fn trabalho_em_erro_nao_segura_a_impressora_e_o_novo_substitui_o_antigo() {
    let cena = Cena::nova();
    assert!(cena.iniciar_com(
        "a",
        &[Leitura::Estado(EstadoEnvio::Falhou)],
        &[A4; 2],
        false
    ));
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Erro { .. }));
    // O erro espera Reimprimir ou Cancelar, mas nao pode trancar o envio de um novo trabalho.
    assert!(
        cena.iniciar_com("a", &[], &[A4; 2], false),
        "a impressora com trabalho em erro ficou ocupada"
    );
    cena.esperar("a", aguardando);
    // O comando agora vai para o trabalho novo.
    cena.comandar("a", Comando::Cancelar);
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Cancelado));
}

#[test]
fn preparo_recusado_nao_segura_a_impressora() {
    let cena = Cena::nova();
    assert!(cena
        .gerente
        .iniciar::<Falso, _>(identidade("a"), cena.emissor(), || Err(Motivo::novo(
            "pdf",
            "o PDF foi recusado"
        )),));
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Erro { .. }));
    assert!(
        cena.iniciar_com("a", &[], &[A4; 2], false),
        "a impressora com o preparo recusado ficou ocupada"
    );
    cena.esperar("a", aguardando);
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
    // A frente isolada segue a primeira passada; o verso, a segunda, que muda com o modelo.
    let casos = [
        (ModeloImpressora::JatoDeTinta, [5, 3, 1], [6, 4, 2], false),
        (ModeloImpressora::Laser, [5, 3, 1], [2, 4, 6], true),
    ];
    for (modelo, frente, verso, vira) in casos {
        let cena = Cena::nova();
        let lotes = planejar(&[A4; 6], None, 10, modelo.perfil()).unwrap();
        let mut id = identidade("sim");
        id.simulada = true;
        cena.gerente.iniciar(id, cena.emissor(), move || {
            Ok(Preparo {
                uma_copia_por_vez: false,
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
        assert_eq!(numeros(&sequencia.frente), frente);
        assert_eq!(numeros(&sequencia.verso), verso);
        assert!(sequencia.verso.iter().all(|face| face.girada));
        assert!(sequencia.frente.iter().all(|face| !face.girada));
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
        mensagem_do_driver: Some("Bandeja 2".to_string()),
        respondido: false,
    });
    let texto = serde_json_de(&cartao);
    assert!(
        texto.contains(r#""mensagemDoDriver":"Bandeja 2""#),
        "{texto}"
    );
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
    for fila in [FILA_SEM_PAPEL, FILA_ATOLADA, FILA_OFFLINE] {
        let estado = plots_windows::Estado::de_bits(fila);
        let motivo = plots_windows::traduzir_com_fila(0x10, Some(&estado))
            .motivo
            .unwrap();
        assert!(cartoes.contains(motivo.as_str()), "sem traducao: {motivo}");
    }
}

/// Bits de estado da fila, como o spooler os informa.
const FILA_SEM_PAPEL: u32 = 0x10;
const FILA_ATOLADA: u32 = 0x8;
const FILA_OFFLINE: u32 = 0x80;

/// Cada condicao chega como aviso, com a frase dela, e o verso nao sai enquanto ela durar. Quando
/// passa, o cartao volta a imprimindo e o verso so libera com a confirmacao da fila.
#[test]
fn cada_condicao_do_spooler_vira_o_seu_aviso_some_ao_passar_e_nao_libera_o_verso() {
    for (fila, frase, codigo) in [
        (FILA_SEM_PAPEL, "a impressora está sem papel", "sem_papel"),
        (FILA_ATOLADA, "o papel atolou na impressora", "atolado"),
        (FILA_OFFLINE, "a impressora não responde", "sem_resposta"),
    ] {
        let cena = Cena::nova();
        let mut leituras = vec![Leitura::Spooler(0x10, fila); 100];
        leituras.extend([Leitura::Spooler(0x10, 0); 100]);
        leituras.push(Leitura::Spooler(0x1000, fila));
        cena.iniciar("a", &leituras, &[A4; 2]);
        let aviso = cena.esperar("a", |estado| matches!(estado, EstadoCartao::Aviso { .. }));
        let EstadoCartao::Aviso {
            motivo, passada, ..
        } = aviso.estado
        else {
            panic!("{aviso:?}");
        };
        assert_eq!(motivo.texto, frase);
        assert_eq!(motivo.codigo, codigo);
        assert_eq!(passada, PassadaDoCartao::Frente);
        cena.comandar("a", Comando::EnviarVerso);
        cena.esperar("a", aguardando);
        assert_eq!(cena.enviados(), [Passada::Frente], "{frase}");
        let estados: Vec<EstadoCartao> =
            cena.cartoes_de("a").into_iter().map(|c| c.estado).collect();
        let fim_do_aviso = estados
            .iter()
            .rposition(|estado| matches!(estado, EstadoCartao::Aviso { .. }))
            .unwrap();
        assert!(
            matches!(
                estados.get(fim_do_aviso + 1),
                Some(EstadoCartao::Imprimindo { .. })
            ),
            "{frase}: {estados:?}"
        );
        cena.comandar("a", Comando::Cancelar);
        esperar_fim(&cena, "a");
    }
}

/// A mensagem do driver chega em campo proprio, com o motivo intacto. Se so ela muda, o cartao
/// atualiza a linha e nao desfaz o "continuar esperando".
#[test]
fn mensagem_do_driver_vem_em_campo_proprio_e_mudar_nao_desfaz_a_resposta() {
    let cena = Cena::nova();
    let mut leituras = vec![Leitura::ComMensagem("Bandeja 2 vazia"); 300];
    leituras.extend([Leitura::ComMensagem("Bandeja 3 vazia"); 5_000]);
    cena.iniciar("a", &leituras, &[A4; 2]);
    let aviso = cena.esperar("a", |estado| matches!(estado, EstadoCartao::Aviso { .. }));
    let EstadoCartao::Aviso {
        motivo,
        mensagem_do_driver,
        ..
    } = aviso.estado
    else {
        panic!("{aviso:?}");
    };
    assert_eq!(motivo.texto, "a impressora está sem papel");
    assert_eq!(motivo.codigo, "sem_papel");
    assert_eq!(mensagem_do_driver.as_deref(), Some("Bandeja 2 vazia"));
    cena.comandar("a", Comando::ContinuarEsperando);
    cena.esperar("a", |estado| {
        matches!(
            estado,
            EstadoCartao::Aviso {
                mensagem_do_driver: Some(mensagem),
                respondido: true,
                ..
            } if mensagem == "Bandeja 3 vazia"
        )
    });
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
}

/// Sem texto do driver, o campo vai vazio: a interface nao mostra a linha.
#[test]
fn aviso_sem_mensagem_do_driver_vai_com_o_campo_vazio() {
    let cena = Cena::nova();
    cena.iniciar("a", &[Leitura::Aviso("sem papel"); 5_000], &[A4; 2]);
    let aviso = cena.esperar("a", |estado| matches!(estado, EstadoCartao::Aviso { .. }));
    assert!(matches!(
        aviso.estado,
        EstadoCartao::Aviso {
            mensagem_do_driver: None,
            ..
        }
    ));
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
}

/// Pedido de atencao nao tem orientacao propria: continua como frase da fila.
#[test]
fn aviso_sem_condicao_conhecida_segue_com_o_codigo_generico_da_fila() {
    let cena = Cena::nova();
    cena.iniciar("a", &[Leitura::Spooler(0x400, 0); 5_000], &[A4; 2]);
    let aviso = cena.esperar("a", |estado| matches!(estado, EstadoCartao::Aviso { .. }));
    let EstadoCartao::Aviso { motivo, .. } = aviso.estado else {
        panic!("{aviso:?}");
    };
    assert_eq!(motivo.codigo, "fila");
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
}

/// A fila com bit generico ou desconhecido nao mexe no cartao: ele segue imprimindo ate confirmar.
#[test]
fn estado_desconhecido_da_fila_nao_vira_aviso() {
    let cena = Cena::nova();
    let leituras: Vec<Leitura> = [0x2, 0x40, 0x0080_0000, 0x0800_0000]
        .into_iter()
        .map(|fila| Leitura::Spooler(0x10, fila))
        .collect();
    cena.iniciar("a", &leituras, &[A4; 2]);
    cena.esperar("a", aguardando);
    let houve_aviso = cena
        .cartoes_de("a")
        .iter()
        .any(|cartao| matches!(cartao.estado, EstadoCartao::Aviso { .. }));
    assert!(!houve_aviso);
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
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

const CAIXA_A4: &str = "[0 0 595 842]";
const CAIXA_A5: &str = "[0 0 420 595]";
const CAIXA_DEITADA: &str = "[0 0 842 595]";

/// Grava um PDF com uma pagina por caixa, em uma pasta so deste teste, apagada ao terminar.
struct PdfDeTeste {
    pasta: std::path::PathBuf,
    caminho: String,
}

impl PdfDeTeste {
    fn novo(rotulo: &str, caixas: &[&str]) -> Self {
        let pasta =
            std::env::temp_dir().join(format!("plots-trecho-{}-{rotulo}", std::process::id()));
        std::fs::create_dir_all(&pasta).unwrap();
        let filhos: Vec<String> = (0..caixas.len())
            .map(|n| format!("{} 0 R", n + 3))
            .collect();
        let mut objetos = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            format!(
                "<< /Type /Pages /Count {} /Kids [{}] >>",
                caixas.len(),
                filhos.join(" ")
            ),
        ];
        for caixa in caixas {
            objetos.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox {caixa} >>"));
        }
        let mut bytes = b"%PDF-1.5\n".to_vec();
        let mut posicoes = Vec::new();
        for (indice, objeto) in objetos.iter().enumerate() {
            posicoes.push(bytes.len());
            bytes.extend_from_slice(format!("{} 0 obj\n{objeto}\nendobj\n", indice + 1).as_bytes());
        }
        let tabela = bytes.len();
        let total = objetos.len() + 1;
        bytes.extend_from_slice(format!("xref\n0 {total}\n0000000000 65535 f \n").as_bytes());
        for posicao in posicoes {
            bytes.extend_from_slice(format!("{posicao:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size {total} /Root 1 0 R >>\nstartxref\n{tabela}\n%%EOF\n")
                .as_bytes(),
        );
        let caminho = pasta.join("documento.pdf");
        std::fs::write(&caminho, bytes).unwrap();
        Self {
            pasta,
            caminho: caminho.to_string_lossy().into_owned(),
        }
    }
}

impl Cena {
    /// O envio acontece logo depois de o cartao mudar: espera a contagem em vez de adivinhar.
    fn esperar_envios(&self, quantos: usize) {
        let inicio = Instant::now();
        while self.enviados().len() < quantos {
            assert!(inicio.elapsed() < PRAZO, "enviados: {:?}", self.enviados());
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

impl Drop for PdfDeTeste {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.pasta);
    }
}

fn trecho(inicio: usize, fim: usize) -> Option<IntervaloPaginas> {
    Some(IntervaloPaginas { inicio, fim })
}

fn numeros_da_face(lista: &[FaceDaSequencia]) -> Vec<Option<usize>> {
    lista.iter().map(|face| face.numero).collect()
}

#[test]
fn resumo_do_envio_conta_so_as_paginas_lotes_e_papeis_do_trecho() {
    let mut caixas = vec![CAIXA_A4; 4];
    caixas.extend([CAIXA_A5; 4]);
    caixas.extend([CAIXA_A4; 4]);
    let pdf = PdfDeTeste::novo("resumo", &caixas);

    let inteiro = resumo_do_envio(&pdf.caminho, None).unwrap();
    assert_eq!((inteiro.paginas, inteiro.lotes), (12, 3));

    let misto = resumo_do_envio(&pdf.caminho, trecho(3, 8)).unwrap();
    assert_eq!((misto.paginas, misto.lotes, misto.papeis.len()), (6, 2, 2));

    let so_a5 = resumo_do_envio(&pdf.caminho, trecho(5, 8)).unwrap();
    assert_eq!((so_a5.paginas, so_a5.lotes, so_a5.papeis.len()), (4, 1, 1));
    assert_eq!(so_a5.papeis[0].formato, "a5");

    assert_eq!(
        resumo_do_envio(&pdf.caminho, trecho(1, 12)).unwrap(),
        inteiro
    );
}

#[test]
fn resumo_com_papel_ajustado_informa_escala_e_unifica_os_lotes() {
    let pdf = PdfDeTeste::novo("resumo-ajustado", &[CAIXA_A4, CAIXA_A4, CAIXA_A5, CAIXA_A5]);
    let papel = plots_core::ajuste::PapelDeAjuste::de_codigo("a4").unwrap();
    let resumo = resumo_do_envio_com_papel(&pdf.caminho, None, Some(papel)).unwrap();
    assert_eq!((resumo.lotes, resumo.papeis.len()), (1, 1));
    assert_eq!(resumo.papeis[0].formato, "a4");
    let (menor, maior) = resumo.escalas.unwrap();
    assert!((menor - 1.0).abs() < 0.01);
    assert!((maior - 1.41).abs() < 0.02);
}

#[test]
fn resumo_do_envio_cita_a_orientacao_mista_com_o_numero_original() {
    let pdf = PdfDeTeste::novo(
        "misto",
        &[
            CAIXA_A4,
            CAIXA_A4,
            CAIXA_A4,
            CAIXA_DEITADA,
            CAIXA_A4,
            CAIXA_A4,
        ],
    );
    assert_eq!(
        resumo_do_envio(&pdf.caminho, trecho(3, 4))
            .unwrap()
            .pares_mistos,
        [(3, 4)]
    );
    assert!(resumo_do_envio(&pdf.caminho, trecho(5, 6))
        .unwrap()
        .pares_mistos
        .is_empty());
}

#[test]
fn resumo_do_envio_cita_o_par_de_tamanhos_diferentes_com_o_numero_original() {
    let pdf = PdfDeTeste::novo(
        "par-separado",
        &[CAIXA_A4, CAIXA_A4, CAIXA_A4, CAIXA_A5, CAIXA_A5, CAIXA_A5],
    );
    assert_eq!(
        resumo_do_envio(&pdf.caminho, None).unwrap().pares_separados,
        [(3, 4)]
    );
    assert!(resumo_do_envio(&pdf.caminho, trecho(1, 2))
        .unwrap()
        .pares_separados
        .is_empty());
    // Ajustado ao papel, todas as páginas ficam do mesmo tamanho: nada se separa.
    let papel = plots_core::ajuste::PapelDeAjuste::de_codigo("a4").unwrap();
    assert!(resumo_do_envio_com_papel(&pdf.caminho, None, Some(papel))
        .unwrap()
        .pares_separados
        .is_empty());
}

#[test]
fn resumo_do_envio_recusa_trecho_fora_do_documento() {
    let pdf = PdfDeTeste::novo("resumo-recusa", &[CAIXA_A4; 12]);
    for (inicio, fim) in [
        (8, 3),
        (0, 3),
        (3, 13),
        (usize::MAX, usize::MAX),
        (1, usize::MAX),
    ] {
        assert!(
            resumo_do_envio(&pdf.caminho, trecho(inicio, fim)).is_err(),
            "{inicio}..{fim}"
        );
    }
}

#[test]
fn pedido_guarda_o_trecho_e_a_preparacao_recusa_o_que_nao_cabe() {
    let pdf = PdfDeTeste::novo("preparar-recusa", &[CAIXA_A4; 12]);
    let pedido = Pedido::novo(pdf.caminho.clone(), trecho(3, 8));
    assert_eq!(pedido.intervalo, trecho(3, 8));
    for (inicio, fim) in [(8, 3), (0, 3), (3, 13), (usize::MAX, usize::MAX)] {
        let pedido = Pedido::novo(pdf.caminho.clone(), trecho(inicio, fim));
        let Err(motivo) = preparar_simulada(ModeloImpressora::Laser, &pedido) else {
            panic!("o trecho {inicio}..{fim} devia ser recusado");
        };
        assert_eq!(motivo.codigo, "pdf");
    }
}

/// Conduz o trecho na simulada ate o cartao esperar a recolocacao.
fn simular_trecho(modelo: ModeloImpressora, pedido: Pedido) -> (Cena, Cartao) {
    let cena = Cena::nova();
    let mut id = identidade("sim");
    id.simulada = true;
    assert!(cena.gerente.iniciar(id, cena.emissor(), move || {
        preparar_simulada(modelo, &pedido)
    }));
    let espera = cena.esperar("sim", aguardando);
    (cena, espera)
}

#[test]
fn simulada_envia_o_trecho_com_os_numeros_do_documento_nos_dois_modelos() {
    let pdf = PdfDeTeste::novo("simular", &[CAIXA_A4; 12]);
    let casos = [
        (
            ModeloImpressora::JatoDeTinta,
            [Some(7), Some(5), Some(3)],
            [Some(8), Some(6), Some(4)],
        ),
        (
            ModeloImpressora::Laser,
            [Some(7), Some(5), Some(3)],
            [Some(4), Some(6), Some(8)],
        ),
    ];
    for (modelo, frente, verso) in casos {
        let pedido = Pedido::novo(pdf.caminho.clone(), trecho(3, 8));
        let (cena, espera) = simular_trecho(modelo, pedido);
        let sequencia = espera.sequencia.clone().unwrap();
        assert_eq!(numeros_da_face(&sequencia.frente), frente);
        assert_eq!(numeros_da_face(&sequencia.verso), verso);
        assert_eq!(espera.total, 1);
        cena.comandar("sim", Comando::EnviarVerso);
        let fim = cena.esperar("sim", |estado| matches!(estado, EstadoCartao::Concluido));
        assert_eq!(
            fim.resultado,
            Some(Resultado {
                corretas: 3,
                folhas: 3
            })
        );
    }
}

#[test]
fn simulada_de_trecho_impar_leva_o_verso_em_branco_e_confere_todas_as_folhas() {
    let pdf = PdfDeTeste::novo("simular-impar", &[CAIXA_A4; 12]);
    for (modelo, frente, verso) in [
        (
            ModeloImpressora::JatoDeTinta,
            [Some(7), Some(5), Some(3)],
            [None, Some(6), Some(4)],
        ),
        (
            ModeloImpressora::Laser,
            [Some(7), Some(5), Some(3)],
            [Some(4), Some(6), None],
        ),
    ] {
        let pedido = Pedido::novo(pdf.caminho.clone(), trecho(3, 7));
        let (cena, espera) = simular_trecho(modelo, pedido);
        let sequencia = espera.sequencia.clone().unwrap();
        assert_eq!(numeros_da_face(&sequencia.frente), frente);
        assert_eq!(numeros_da_face(&sequencia.verso), verso);
        cena.comandar("sim", Comando::EnviarVerso);
        let fim = cena.esperar("sim", |estado| matches!(estado, EstadoCartao::Concluido));
        assert_eq!(
            fim.resultado,
            Some(Resultado {
                corretas: 3,
                folhas: 3
            })
        );
    }
}

/// Backend que guarda os numeros de cada passada; a passada citada em `falha` da erro na fila.
struct Gravador {
    passadas: Arc<Mutex<Vec<Vec<Option<usize>>>>>,
    falha: usize,
}

impl Backend for Gravador {
    type Erro = String;
    fn enviar(&mut self, _: Passada, paginas: &[Pagina]) -> Result<(), String> {
        self.passadas
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(paginas.iter().map(|pagina| pagina.numero).collect());
        Ok(())
    }
    fn consultar(&mut self) -> Result<EstadoEnvio, String> {
        let enviadas = self
            .passadas
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len();
        Ok(if enviadas == self.falha {
            EstadoEnvio::Falhou
        } else {
            EstadoEnvio::Concluido
        })
    }
    fn cancelar(&mut self) -> Result<(), String> {
        Ok(())
    }
}

impl DescreveFalha for Gravador {
    fn motivo(&self) -> Option<String> {
        None
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        false
    }
}

#[test]
fn reimprimir_refaz_o_mesmo_trecho_ja_planejado() {
    let cena = Cena::nova();
    let passadas: Arc<Mutex<Vec<Vec<Option<usize>>>>> = Arc::default();
    let backend = Gravador {
        passadas: Arc::clone(&passadas),
        falha: 2,
    };
    let modelo = ModeloImpressora::Laser;
    let lotes = planejar(&[A4; 12], trecho(3, 8), 10, modelo.perfil()).unwrap();
    assert!(cena
        .gerente
        .iniciar(identidade("a"), cena.emissor(), move || {
            Ok(Preparo {
                uma_copia_por_vez: false,
                condutor: Condutor::novo(
                    backend,
                    lotes,
                    Espera {
                        intervalo: Duration::from_millis(2),
                        consultas: 1_000,
                    },
                )
                .map_err(|erro| motivo_do_job(&erro))?,
                recolocar: recolocar_do_perfil(Some(modelo), None),
                vira_para_ler: false,
            })
        }));
    cena.esperar("a", aguardando);
    cena.comandar("a", Comando::EnviarVerso);
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Erro { .. }));
    cena.comandar("a", Comando::Reimprimir);
    cena.esperar("a", aguardando);
    let frente = vec![Some(8), Some(6), Some(4)];
    let verso = vec![Some(3), Some(5), Some(7)];
    assert_eq!(
        *passadas.lock().unwrap_or_else(PoisonError::into_inner),
        [frente.clone(), verso, frente]
    );
}

#[test]
fn envio_real_mede_a_face_em_branco_pelo_fim_do_trecho() {
    let medidas = vec![A4, A4, A4, A5];
    assert_eq!(dimensoes_do_envio(medidas.clone(), trecho(1, 3)), [A4; 3]);
    assert_eq!(dimensoes_do_envio(medidas.clone(), None), medidas);
    assert_eq!(dimensoes_do_envio(medidas.clone(), trecho(1, 4)), medidas);
}

#[test]
fn reimprimir_no_aviso_da_frente_tira_o_trabalho_da_fila_e_envia_a_frente_de_novo() {
    let cena = Cena::nova();
    cena.iniciar("a", &[Leitura::Aviso("sem papel"); 5_000], &[A4; 2]);
    cena.esperar("a", |estado| {
        matches!(
            estado,
            EstadoCartao::Aviso {
                passada: PassadaDoCartao::Frente,
                ..
            }
        )
    });
    cena.comandar("a", Comando::Reimprimir);
    cena.esperar_envios(2);
    assert_eq!(cena.cancelados(), 1);
    assert_eq!(cena.enviados(), [Passada::Frente, Passada::Frente]);
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
}

#[test]
fn reimprimir_no_aviso_do_verso_refaz_o_lote_desde_a_frente() {
    let cena = Cena::nova();
    let mut leituras = vec![Leitura::Estado(EstadoEnvio::Concluido)];
    leituras.extend([Leitura::Aviso("sem papel"); 5_000]);
    cena.iniciar("a", &leituras, &[A4; 2]);
    cena.esperar("a", aguardando);
    cena.comandar("a", Comando::EnviarVerso);
    cena.esperar("a", |estado| {
        matches!(
            estado,
            EstadoCartao::Aviso {
                passada: PassadaDoCartao::Verso,
                ..
            }
        )
    });
    cena.comandar("a", Comando::Reimprimir);
    cena.esperar_envios(3);
    assert_eq!(cena.cancelados(), 1);
    assert_eq!(
        cena.enviados(),
        [Passada::Frente, Passada::Verso, Passada::Frente]
    );
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
}

#[test]
fn reimprimir_no_aviso_com_cancelamento_recusado_nao_reenvia_a_frente() {
    let cena = Cena::nova();
    assert!(cena.iniciar_recusando_cancelamento("a", &[Leitura::Aviso("sem papel"); 5_000]));
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Aviso { .. }));
    cena.comandar("a", Comando::Reimprimir);
    cena.esperar(
        "a",
        |estado| matches!(estado, EstadoCartao::Erro { motivo } if motivo.codigo == "nao_cancelou"),
    );
    assert_eq!(cena.enviados(), [Passada::Frente]);
    assert_eq!(cena.cancelados(), 0);
}

#[test]
fn recusa_do_ajuste_aparece_no_cartao_e_continua_nos_estados_seguintes() {
    const FILA: &str = "Fila com ajuste que o driver recusou";
    let cena = Cena::nova();
    crate::preferencias::marcar_recusa(FILA);
    cena.iniciar(FILA, &[Leitura::Estado(EstadoEnvio::EmAndamento)], &[A4; 2]);
    let aguardando_a_pessoa = cena.esperar(FILA, aguardando);
    assert!(aguardando_a_pessoa.ajuste_recusado);
    cena.comandar(FILA, Comando::Cancelar);
    esperar_fim(&cena, FILA);
    assert!(!crate::preferencias::tomar_recusa(FILA), "conta uma vez só");
}

#[test]
fn sem_recusa_o_cartao_nao_fala_do_ajuste() {
    const FILA: &str = "Fila cujo driver aceitou o ajuste";
    let cena = Cena::nova();
    cena.iniciar(FILA, &[Leitura::Estado(EstadoEnvio::EmAndamento)], &[A4; 2]);
    let cartao = cena.esperar(FILA, aguardando);
    assert!(!cartao.ajuste_recusado);
    cena.comandar(FILA, Comando::Cancelar);
    esperar_fim(&cena, FILA);
}

impl Cena {
    /// Como `iniciar`, com o plano de uma impressora que imprime as duas faces sozinha.
    fn iniciar_automatica(
        &self,
        impressora: &str,
        leituras: &[Leitura],
        paginas: &[DimensoesPagina],
    ) {
        let backend = self.backend(leituras, false, false);
        let pedido = Pedido::novo("x.pdf".to_owned(), None);
        let perfil = ModeloImpressora::JatoDeTinta.perfil();
        let lotes = planejar_pedido(paginas, &pedido, perfil, true).unwrap();
        self.gerente
            .iniciar(identidade(impressora), self.emissor(), move || {
                Ok(Preparo {
                    uma_copia_por_vez: false,
                    condutor: Condutor::novo(
                        backend,
                        lotes,
                        Espera {
                            intervalo: Duration::from_millis(2),
                            consultas: 1_000,
                        },
                    )
                    .map_err(|erro| motivo_do_job(&erro))?,
                    recolocar: recolocar_do_perfil(Some(ModeloImpressora::JatoDeTinta), None),
                    vira_para_ler: false,
                })
            });
    }
}

fn simular_automatica(pedido: Pedido) -> Cena {
    let cena = Cena::nova();
    let mut id = identidade("auto");
    id.simulada = true;
    assert!(cena.gerente.iniciar(id, cena.emissor(), move || {
        preparar_simulada_automatica(ModeloImpressora::JatoDeTinta, &pedido)
    }));
    cena
}

fn concluido(estado: &EstadoCartao) -> bool {
    matches!(estado, EstadoCartao::Concluido)
}

fn proximo_lote(estado: &EstadoCartao) -> bool {
    matches!(estado, EstadoCartao::ProximoLote)
}

#[test]
fn simulada_automatica_termina_o_a4_sem_nenhum_comando_e_confere_as_folhas() {
    let pdf = PdfDeTeste::novo("automatica-a4", &[CAIXA_A4; 5]);
    let cena = simular_automatica(Pedido::novo(pdf.caminho.clone(), None));
    let fim = cena.esperar("auto", concluido);
    assert_eq!(
        fim.resultado,
        Some(Resultado {
            corretas: 3,
            folhas: 3
        })
    );
    assert_eq!(fim.total, 1, "folhas por lote não divide o trabalho");
    assert!(fim.tem_automatico);
    assert!(!fim.vira_para_ler, "como o maço sai, a pessoa confere");
    assert!(cena
        .cartoes_de("auto")
        .iter()
        .all(|cartao| !aguardando(&cartao.estado)));
}

#[test]
fn documento_misto_alterna_os_fluxos_e_so_o_papel_manual_pede_gesto() {
    let mut caixas = vec![CAIXA_A4; 4];
    caixas.extend([CAIXA_A5; 2]);
    caixas.extend([CAIXA_A4; 2]);
    let pdf = PdfDeTeste::novo("auto-misto", &caixas);
    let cena = simular_automatica(Pedido::novo(pdf.caminho.clone(), None));

    let antes_do_a5 = cena.esperar("auto", proximo_lote);
    assert!(antes_do_a5.anterior_automatico);
    assert!(!antes_do_a5.lote_automatico);
    assert_eq!(
        antes_do_a5.troca_de_papel.map(|papel| papel.formato),
        Some("a5")
    );

    cena.comandar("auto", Comando::EnviarFrente);
    let gesto = cena.esperar("auto", aguardando);
    assert!(!gesto.lote_automatico);
    assert!(gesto.recolocar.is_some());

    cena.comandar("auto", Comando::EnviarVerso);
    let antes_do_a4 = cena.esperar("auto", proximo_lote);
    assert!(antes_do_a4.lote_automatico);
    assert_eq!(
        antes_do_a4.troca_de_papel.map(|papel| papel.formato),
        Some("a4")
    );

    cena.comandar("auto", Comando::EnviarFrente);
    let fim = cena.esperar("auto", concluido);
    assert_eq!(
        fim.resultado,
        Some(Resultado {
            corretas: 4,
            folhas: 4
        })
    );
    assert_eq!(fim.total, 3);
}

#[test]
fn pagina_deitada_sai_no_mesmo_trabalho_e_a_simulada_nao_recusa() {
    let pdf = PdfDeTeste::novo(
        "auto-deitada",
        &[CAIXA_A4, CAIXA_A4, CAIXA_DEITADA, CAIXA_DEITADA],
    );
    let cena = simular_automatica(Pedido::novo(pdf.caminho.clone(), None));
    let fim = cena.esperar("auto", concluido);
    assert_eq!(fim.total, 1);
    assert_eq!(
        fim.resultado,
        Some(Resultado {
            corretas: 2,
            folhas: 2
        })
    );
}

#[test]
fn recusa_das_duas_faces_nao_envia_nada_e_libera_a_impressora() {
    let cena = Cena::nova();
    let mut id = identidade("recusa");
    id.simulada = true;
    let plano = {
        let pedido = Pedido::novo("x.pdf".to_owned(), None);
        planejar_pedido(
            &[A4; 2],
            &pedido,
            ModeloImpressora::JatoDeTinta.perfil(),
            true,
        )
        .unwrap()
    };
    // A simulada comum nao tem o recurso, como um driver que nao fica com as duas faces.
    let sem_recurso = SimGuiado::novo(
        Mecanica::do_modelo(ModeloImpressora::JatoDeTinta),
        ModeloImpressora::JatoDeTinta.gesto().into(),
    );
    cena.gerente.iniciar(id, cena.emissor(), move || {
        Ok(Preparo {
            uma_copia_por_vez: false,
            condutor: Condutor::novo(sem_recurso, plano, ESPERA_SIMULADA)
                .map_err(|erro| motivo_do_job(&erro))?,
            recolocar: Recolocar::da_instrucao(Instrucao::do_modelo(ModeloImpressora::JatoDeTinta)),
            vira_para_ler: false,
        })
    });
    let erro = cena.esperar("recusa", |estado| {
        matches!(estado, EstadoCartao::Erro { .. })
    });
    let EstadoCartao::Erro { motivo } = &erro.estado else {
        panic!("sem erro");
    };
    assert_eq!(motivo.codigo, MOTIVO_DUAS_FACES_RECUSADAS);
    assert_eq!(erro.resultado, None, "nada saiu");
    esperar_fim(&cena, "recusa");
    assert!(
        !ativo(&cena, "recusa"),
        "a pessoa troca o modelo e envia de novo"
    );
}

#[test]
fn recusa_depois_de_um_lote_ja_enviado_diz_em_que_lote_parou() {
    let cena = Cena::nova();
    let mut id = identidade("meio");
    id.simulada = true;
    let plano = {
        let pedido = Pedido::novo("x.pdf".to_owned(), None);
        planejar_pedido(
            &[A4, A4, A5, A5, A4, A4],
            &pedido,
            ModeloImpressora::JatoDeTinta.perfil(),
            true,
        )
        .unwrap()
    };
    assert_eq!(plano.len(), 3, "automatico, manual, automatico");
    // A simulada ve outro papel no fim, como um driver que so recusa ao receber aquele lote.
    let backend = SimGuiado::automatica(
        Mecanica::do_modelo(ModeloImpressora::JatoDeTinta),
        ModeloImpressora::JatoDeTinta.gesto().into(),
        vec![A4, A4, A5, A5, A5, A5],
    );
    cena.gerente.iniciar(id, cena.emissor(), move || {
        Ok(Preparo {
            uma_copia_por_vez: false,
            condutor: Condutor::novo(backend, plano, ESPERA_SIMULADA)
                .map_err(|erro| motivo_do_job(&erro))?,
            recolocar: Recolocar::da_instrucao(Instrucao::do_modelo(ModeloImpressora::JatoDeTinta)),
            vira_para_ler: false,
        })
    });
    cena.esperar("meio", proximo_lote);
    cena.comandar("meio", Comando::EnviarFrente);
    cena.esperar("meio", aguardando);
    cena.comandar("meio", Comando::EnviarVerso);
    cena.esperar("meio", proximo_lote);
    cena.comandar("meio", Comando::EnviarFrente);
    let erro = cena.esperar("meio", |estado| matches!(estado, EstadoCartao::Erro { .. }));
    let EstadoCartao::Erro { motivo } = &erro.estado else {
        panic!("sem erro");
    };
    assert_eq!(motivo.codigo, MOTIVO_DUAS_FACES_RECUSADAS);
    assert_eq!(
        (erro.lote, erro.total),
        (3, 3),
        "a interface sabe que os anteriores sairam"
    );
    esperar_fim(&cena, "meio");
}

#[test]
fn reimprimir_no_aviso_de_duas_faces_tira_da_fila_e_refaz_o_lote_inteiro() {
    let cena = Cena::nova();
    cena.iniciar_automatica("a", &[Leitura::Aviso("sem papel"); 5_000], &[A4; 4]);
    cena.esperar("a", |estado| {
        matches!(
            estado,
            EstadoCartao::Aviso {
                passada: PassadaDoCartao::DuasFaces,
                ..
            }
        )
    });
    cena.comandar("a", Comando::Reimprimir);
    cena.esperar_envios(2);
    assert_eq!(cena.cancelados(), 1);
    assert_eq!(cena.enviados(), [Passada::DuasFaces, Passada::DuasFaces]);
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
}

#[test]
fn pedir_verso_no_lote_de_duas_faces_nao_envia_nada() {
    let cena = Cena::nova();
    cena.iniciar_automatica(
        "a",
        &[Leitura::Estado(EstadoEnvio::EmAndamento); 5_000],
        &[A4; 4],
    );
    cena.esperar_envios(1);
    cena.comandar("a", Comando::EnviarVerso);
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
    assert_eq!(cena.enviados(), [Passada::DuasFaces]);
}

#[test]
fn papel_a5_na_automatica_continua_em_uma_face_por_passada() {
    let cena = Cena::nova();
    cena.iniciar_automatica("a", &[], &[A5; 2]);
    let espera = cena.esperar("a", aguardando);
    assert!(!espera.lote_automatico);
    assert!(!espera.tem_automatico);
    assert_eq!(cena.enviados(), [Passada::Frente]);
    cena.comandar("a", Comando::EnviarVerso);
    cena.esperar("a", concluido);
    assert_eq!(cena.enviados(), [Passada::Frente, Passada::Verso]);
}

const CARTA: DimensoesPagina = DimensoesPagina::new(215.9, 279.4);

fn com_papel(codigo: &str) -> Pedido {
    Pedido::novo("documento.pdf".into(), None)
        .com_papel_de_ajuste(plots_core::ajuste::PapelDeAjuste::de_codigo(codigo))
}

#[test]
fn ajustar_ao_papel_decide_a_rota_pelo_papel_de_destino() {
    let perfil = ModeloImpressora::JatoDeTinta.perfil();
    for (original, destino, duas_faces) in [
        (CARTA, "a4", true),
        (A5, "a4", true),
        (A5, "carta", true),
        (A4, "a5", false),
        (A4, "oficio", false),
    ] {
        let lotes = planejar_pedido(&[original; 4], &com_papel(destino), perfil, true).unwrap();
        assert!(
            lotes.iter().all(|item| item.lote.duas_faces == duas_faces),
            "{original:?} em {destino}"
        );
    }
}

#[test]
fn o_resumo_diz_a_rota_do_papel_de_destino_e_nao_a_do_original() {
    let pdf = PdfDeTeste::novo("rota-do-ajuste", &[CAIXA_A5; 4]);
    let sem_ajuste = resumo_do_envio(&pdf.caminho, None).unwrap();
    assert!(sem_ajuste.fluxos.iter().all(|fluxo| !fluxo.automatico));
    let a4 = plots_core::ajuste::PapelDeAjuste::de_codigo("a4");
    let ajustado = resumo_do_envio_com_papel(&pdf.caminho, None, a4).unwrap();
    assert_eq!(
        ajustado.fluxos,
        [TrechoDoFluxo {
            automatico: true,
            inicio: 1,
            fim: 4
        }]
    );
    assert_eq!(ajustado.lotes_automaticos, 1);
}

#[test]
fn simulada_automatica_aceita_a5_ajustado_a_a4_sem_nenhum_comando() {
    let pdf = PdfDeTeste::novo("automatica-a5-em-a4", &[CAIXA_A5; 4]);
    let pedido = Pedido::novo(pdf.caminho.clone(), None)
        .com_papel_de_ajuste(plots_core::ajuste::PapelDeAjuste::de_codigo("a4"));
    let cena = simular_automatica(pedido);
    let fim = cena.esperar("auto", concluido);
    assert_eq!(
        fim.resultado,
        Some(Resultado {
            corretas: 2,
            folhas: 2
        })
    );
}

#[test]
fn folha_de_controle_nao_toca_no_lote_de_duas_faces_e_marca_o_manual_da_mesma_fila() {
    let perfil = ModeloImpressora::JatoDeTinta.perfil();
    let pedido =
        Pedido::novo("documento.pdf".into(), None).com_controle(Some(IdiomaDaMarca::Portugues));
    let lotes = planejar_pedido(&[A4, A4, A5, A5], &pedido, perfil, true).unwrap();
    assert_eq!(lotes.len(), 2);
    assert!(lotes[0].lote.duas_faces);
    assert!(lotes[0].marcas.is_empty());
    assert!(lotes[0]
        .lote
        .passada1
        .iter()
        .all(|face| face.numero.is_some()));
    assert!(!lotes[1].lote.duas_faces);
    assert!(!lotes[1].marcas.is_empty());
}

fn sem_parar(alcance: Alcance) -> Pedido {
    Pedido::novo("documento.pdf".into(), None).com_alcance(alcance)
}

#[test]
fn o_plano_da_janela_e_um_lote_por_papel_sem_limite_de_folhas() {
    let perfil = ModeloImpressora::Laser.perfil();
    let so_a4 = planejar_pedido(&[A4; 24], &sem_parar(Alcance::SoFrentes), perfil, false).unwrap();
    assert_eq!(so_a4.len(), 1);
    assert_eq!(so_a4[0].lote.passada1.len(), 12);
    assert_eq!(so_a4[0].lote.passada2.len(), 12);
    let muitas = vec![A4; 500];
    let longo = planejar_pedido(&muitas, &sem_parar(Alcance::SoVersos), perfil, false).unwrap();
    assert_eq!(longo.len(), 1);
    assert_eq!(longo[0].lote.passada2.len(), 250);
    let misto = [[A4; 4].as_slice(), &[A5; 4], &[A4; 2]].concat();
    let lotes = planejar_pedido(&misto, &sem_parar(Alcance::SoFrentes), perfil, false).unwrap();
    let papeis: Vec<_> = lotes.iter().map(|item| item.papel.formato).collect();
    assert_eq!(
        papeis,
        [FormatoPapel::A4, FormatoPapel::A5, FormatoPapel::A4]
    );
}

#[test]
fn na_automatica_o_papel_aceito_e_o_feito_a_mao_saem_em_um_trabalho_por_trecho() {
    let perfil = PerfilImpressora::default();
    let pedido = sem_parar(Alcance::SoFrentes);
    let lotes = planejar_pedido(&[A5; 24], &pedido, perfil, true).unwrap();
    assert_eq!(lotes.len(), 1);
    assert!(!lotes[0].lote.duas_faces);
    let misto = [[A4; 4].as_slice(), &[A5; 4]].concat();
    let lotes = planejar_pedido(&misto, &pedido, perfil, true).unwrap();
    assert_eq!(lotes.len(), 2);
    assert!(lotes[0].lote.duas_faces);
    assert!(!lotes[1].lote.duas_faces);
}

#[test]
fn a_escolha_da_face_vale_com_trecho_ajuste_e_folha_de_controle() {
    let perfil = ModeloImpressora::JatoDeTinta.perfil();
    let pedido = Pedido::novo("documento.pdf".into(), trecho(3, 12))
        .com_alcance(Alcance::SoFrentes)
        .com_controle(Some(IdiomaDaMarca::Portugues))
        .com_papel_de_ajuste(plots_core::ajuste::PapelDeAjuste::de_codigo("a4"));
    let misto = [[A4; 6].as_slice(), &[A5; 8]].concat();
    let lotes = planejar_pedido(&misto, &pedido, perfil, false).unwrap();
    assert_eq!(lotes.len(), 1);
    let numeros: Vec<_> = lotes[0].lote.paginas.iter().map(|p| p.numero).collect();
    let esperado: Vec<_> = (3..=12).map(Some).collect();
    assert_eq!(numeros, esperado);
    assert_eq!(lotes[0].marcas.len(), 5);
    assert_eq!(lotes[0].marcas[&(0, Some(4))], "Folha 1");
    assert_eq!(lotes[0].marcas[&(0, Some(12))], "Folha 5");
}

/// Roda a simulada ate a thread acabar e devolve o ultimo cartao.
fn simular_face(cena: &Cena, modelo: ModeloImpressora, pedido: Pedido) -> Cartao {
    let mut id = identidade("sim");
    id.simulada = true;
    assert!(cena.gerente.iniciar(id, cena.emissor(), move || {
        preparar_simulada(modelo, &pedido)
    }));
    esperar_fim(cena, "sim");
    cena.cartoes_de("sim").pop().unwrap()
}

/// Backend que guarda as páginas de cada passada e o maço que veio de fora.
#[derive(Default)]
struct FacesGravadas {
    enviadas: Arc<Mutex<Vec<(Passada, Vec<Option<usize>>)>>>,
    de_fora: Arc<Mutex<Vec<Vec<Option<usize>>>>>,
}

impl Backend for FacesGravadas {
    type Erro = String;

    fn enviar(&mut self, passada: Passada, paginas: &[Pagina]) -> Result<(), String> {
        self.enviadas
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((
                passada,
                paginas.iter().map(|pagina| pagina.numero).collect(),
            ));
        Ok(())
    }

    fn consultar(&mut self) -> Result<EstadoEnvio, String> {
        Ok(EstadoEnvio::Concluido)
    }

    fn cancelar(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn maco_de_fora(&mut self, _: usize, frentes: &[Pagina]) -> Result<(), String> {
        self.de_fora
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(frentes.iter().map(|pagina| pagina.numero).collect());
        Ok(())
    }
}

/// Envia a face isolada de um documento e devolve o que foi para a fila.
fn paginas_da_face(
    total: usize,
    alcance: Alcance,
) -> (Vec<(Passada, Vec<Option<usize>>)>, Vec<Vec<Option<usize>>>) {
    let lote: Vec<plots_core::Lote> =
        planejar(&vec![A4; total], None, 10, PerfilImpressora::default())
            .unwrap()
            .into_iter()
            .map(|item| item.lote)
            .collect();
    let mut job = Job::novo(FacesGravadas::default(), lote)
        .unwrap()
        .com_alcance(alcance);
    match alcance {
        Alcance::SoFrentes => {
            job.enviar_frente().unwrap();
            assert_eq!(
                job.atualizar().unwrap(),
                EstadoJob::Concluido,
                "{total} página(s): a frente termina o lote sem esperar recolocação"
            );
        }
        Alcance::SoVersos => {
            job.enviar_verso().unwrap();
            assert_eq!(
                job.atualizar().unwrap(),
                EstadoJob::Concluido,
                "{total} página(s): o verso termina o lote"
            );
        }
        Alcance::FrenteEVerso => unreachable!(),
    }
    let enviadas = job
        .backend()
        .enviadas
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let de_fora = job
        .backend()
        .de_fora
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    (enviadas, de_fora)
}

#[test]
fn frente_isolada_sao_as_impares_e_verso_isolado_tem_as_pares() {
    for (total, frente) in [(1usize, vec![Some(1)]), (3, vec![Some(1), Some(3)])] {
        let (enviadas, sem_maco) = paginas_da_face(total, Alcance::SoFrentes);
        assert_eq!(enviadas, [(Passada::Frente, frente)], "{total}");
        assert!(sem_maco.is_empty());
    }

    let (enviadas, de_fora) = paginas_da_face(3, Alcance::SoVersos);
    assert_eq!(enviadas, [(Passada::Verso, vec![Some(2), None])]);
    assert_eq!(
        de_fora,
        [vec![Some(1), Some(3)]],
        "o maço do verso é o das frentes"
    );
}

#[test]
fn simulacao_de_uma_pagina_termina_em_frentes_prontas_com_a_pagina_1() {
    let pdf = PdfDeTeste::novo("uma-face", &[CAIXA_A4]);
    let cena = Cena::nova();
    let cartao = simular_face(
        &cena,
        ModeloImpressora::Laser,
        Pedido::novo(pdf.caminho.clone(), None).com_alcance(Alcance::SoFrentes),
    );
    assert_eq!(cartao.estado, EstadoCartao::FrentesProntas);
    assert!(!cartao.tem_verso, "uma página não tem verso");
    let sequencia = cartao.sequencia.unwrap();
    assert_eq!(numeros_da_face(&sequencia.frente), [Some(1)]);
    assert_eq!(numeros_da_face(&sequencia.verso), [None]);
}

#[test]
fn simulacao_de_tres_paginas_fecha_o_maco_com_as_faces_certas() {
    let pdf = PdfDeTeste::novo("tres-faces", &[CAIXA_A4; 3]);
    let frentes = simular_face(
        &Cena::nova(),
        ModeloImpressora::Laser,
        Pedido::novo(pdf.caminho.clone(), None).com_alcance(Alcance::SoFrentes),
    );
    assert_eq!(frentes.estado, EstadoCartao::FrentesProntas);
    assert!(frentes.tem_verso, "três páginas têm verso");
    let sequencia = frentes.sequencia.unwrap();
    // A laser inverte a primeira passada, que é a dos versos; as frentes seguem essa ordem.
    assert_eq!(numeros_da_face(&sequencia.frente), [Some(3), Some(1)]);
    assert_eq!(numeros_da_face(&sequencia.verso), [Some(2), None]);

    let versos = simular_face(
        &Cena::nova(),
        ModeloImpressora::Laser,
        Pedido::novo(pdf.caminho.clone(), None).com_alcance(Alcance::SoVersos),
    );
    assert_eq!(versos.estado, EstadoCartao::Concluido);
    let sequencia = versos.sequencia.unwrap();
    assert_eq!(numeros_da_face(&sequencia.verso), [Some(2), None]);
    assert_eq!(
        versos.resultado,
        Some(Resultado {
            corretas: 2,
            folhas: 2
        })
    );
}

#[test]
fn trecho_de_uma_pagina_tambem_nao_tem_verso() {
    let pdf = PdfDeTeste::novo("trecho-uma", &[CAIXA_A4; 12]);
    let cartao = simular_face(
        &Cena::nova(),
        ModeloImpressora::Laser,
        Pedido::novo(pdf.caminho.clone(), trecho(3, 3)).com_alcance(Alcance::SoFrentes),
    );
    assert_eq!(cartao.estado, EstadoCartao::FrentesProntas);
    assert!(!cartao.tem_verso);
    assert_eq!(
        numeros_da_face(&cartao.sequencia.unwrap().frente),
        [Some(3)]
    );
}

#[test]
fn trabalho_so_de_versos_sem_pagina_numerada_vira_erro_com_motivo() {
    let cena = Cena::nova();
    iniciar_face(&cena, &[], &[A4], Alcance::SoVersos);
    let cartao = cena.esperar(
        "a",
        |estado| matches!(estado, EstadoCartao::Erro { motivo } if motivo.codigo == "sem_verso"),
    );
    let EstadoCartao::Erro { motivo } = cartao.estado else {
        panic!("o cartao devia mostrar o motivo");
    };
    assert_eq!(motivo.texto, "Não há verso neste trabalho.");
    assert!(!cartao.tem_verso);
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
}

#[test]
fn frentes_num_trabalho_e_versos_em_outro_formam_o_maco_certo_nos_dois_modelos() {
    let pdf = PdfDeTeste::novo("so-faces", &[CAIXA_A4; 24]);
    for modelo in [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser] {
        for inverte in [false, true] {
            let cena = Cena::nova();
            let driver = DriverDaSimulada {
                inverte,
                compensa: inverte,
            };
            let pedido = |alcance| {
                Pedido::novo(pdf.caminho.clone(), None)
                    .com_alcance(alcance)
                    .com_driver_da_simulada(driver)
            };
            let frentes = simular_face(&cena, modelo, pedido(Alcance::SoFrentes));
            assert_eq!(frentes.estado, EstadoCartao::FrentesProntas);
            assert_eq!(frentes.alcance, "so_frentes");
            assert_eq!((frentes.lote, frentes.total), (1, 1));
            assert_eq!(frentes.folhas_do_lote, 12);
            assert_eq!(
                frentes.resultado, None,
                "so a frente nao fecha nenhuma folha"
            );
            assert!(!ativo(&cena, "sim"), "a impressora fica livre na espera");
            // Sem nada guardado desta sessao: e o que acontece ao voltar no dia seguinte.
            let versos = simular_face(&Cena::nova(), modelo, pedido(Alcance::SoVersos));
            assert_eq!(versos.estado, EstadoCartao::Concluido);
            assert_eq!(versos.alcance, "so_versos");
            assert_eq!(
                versos.resultado,
                Some(Resultado {
                    corretas: 12,
                    folhas: 12
                }),
                "{modelo:?}, driver que inverte: {inverte}",
            );
        }
    }
}

/// Inicia um trabalho falso com o plano da janela.
fn iniciar_face(
    cena: &Cena,
    leituras: &[Leitura],
    dimensoes: &[DimensoesPagina],
    alcance: Alcance,
) {
    let backend = cena.backend(leituras, false, false);
    let lotes = planejar_pedido(
        dimensoes,
        &sem_parar(alcance),
        PerfilImpressora::default(),
        false,
    )
    .unwrap();
    assert!(cena
        .gerente
        .iniciar(identidade("a"), cena.emissor(), move || {
            Ok(Preparo {
                uma_copia_por_vez: false,
                condutor: Condutor::novo(
                    backend,
                    lotes,
                    Espera {
                        intervalo: Duration::from_millis(2),
                        consultas: 1_000,
                    },
                )
                .map_err(|erro| motivo_do_job(&erro))?
                .com_alcance(alcance),
                recolocar: recolocar_do_perfil(Some(ModeloImpressora::Laser), None),
                vira_para_ler: true,
            })
        }));
}

const CONCLUIDO: Leitura = Leitura::Estado(EstadoEnvio::Concluido);

#[test]
fn so_frentes_envia_um_trabalho_sem_clique_e_a_thread_acaba_sem_verso() {
    let cena = Cena::nova();
    iniciar_face(
        &cena,
        &[Leitura::Estado(EstadoEnvio::EmAndamento), CONCLUIDO],
        &[A4; 24],
        Alcance::SoFrentes,
    );
    esperar_fim(&cena, "a");
    assert_eq!(cena.enviados(), [Passada::Frente]);
    let fim = cena.cartoes_de("a").pop().unwrap();
    assert_eq!(fim.estado, EstadoCartao::FrentesProntas);
    assert_eq!(cena.cancelados(), 0);
}

#[test]
fn so_frentes_para_na_troca_de_papel_e_diz_o_papel_antes() {
    let cena = Cena::nova();
    iniciar_face(
        &cena,
        &[CONCLUIDO; 2],
        &[[A4; 4].as_slice(), &[A5; 4]].concat(),
        Alcance::SoFrentes,
    );
    let troca = cena.esperar("a", |estado| matches!(estado, EstadoCartao::ProximoLote));
    assert_eq!(troca.troca_de_papel.map(|papel| papel.formato), Some("a5"));
    assert_eq!(troca.folhas_do_lote, 2);
    assert_eq!(
        troca.faixa_do_lote,
        Some(FaixaDoLote {
            lote: 2,
            inicio: 5,
            fim: 8
        })
    );
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(cena.enviados(), [Passada::Frente]);
    cena.comandar("a", Comando::EnviarFrente);
    esperar_fim(&cena, "a");
    assert_eq!(cena.enviados(), [Passada::Frente, Passada::Frente]);
}

#[test]
fn so_versos_comeca_sem_clique_e_para_na_troca_de_papel_dizendo_o_maco() {
    let cena = Cena::nova();
    iniciar_face(
        &cena,
        &[CONCLUIDO; 2],
        &[[A4; 4].as_slice(), &[A5; 4]].concat(),
        Alcance::SoVersos,
    );
    let espera = cena.esperar("a", aguardando);
    assert_eq!(cena.enviados(), [Passada::Verso]);
    assert_eq!(espera.troca_de_papel.map(|papel| papel.formato), Some("a5"));
    assert_eq!(espera.lote, 2);
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(cena.enviados(), [Passada::Verso]);
    cena.comandar("a", Comando::EnviarVerso);
    cena.esperar("a", |estado| matches!(estado, EstadoCartao::Concluido));
    assert_eq!(cena.enviados(), [Passada::Verso, Passada::Verso]);
}

#[test]
fn reimprimir_refaz_so_a_face_escolhida_e_tira_antes_o_trabalho_da_fila() {
    for (alcance, passada) in [
        (Alcance::SoFrentes, Passada::Frente),
        (Alcance::SoVersos, Passada::Verso),
    ] {
        let cena = Cena::nova();
        iniciar_face(
            &cena,
            &[Leitura::Aviso("sem papel"); 5_000],
            &[A4; 24],
            alcance,
        );
        let aviso = cena.esperar("a", |estado| matches!(estado, EstadoCartao::Aviso { .. }));
        assert_eq!(aviso.folhas_do_lote, 12);
        cena.comandar("a", Comando::Reimprimir);
        cena.esperar_envios(2);
        assert_eq!(cena.enviados(), [passada, passada]);
        assert_eq!(cena.cancelados(), 1);
        cena.comandar("a", Comando::Cancelar);
        esperar_fim(&cena, "a");
    }
}

#[test]
fn com_trabalho_so_de_frentes_ou_so_de_versos_nenhum_comando_envia_a_outra_face() {
    let cena = Cena::nova();
    iniciar_face(
        &cena,
        &[Leitura::Estado(EstadoEnvio::EmAndamento); 5],
        &[A4; 4],
        Alcance::SoFrentes,
    );
    cena.esperar_envios(1);
    cena.comandar("a", Comando::EnviarVerso);
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(cena.enviados(), [Passada::Frente]);
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");

    let cena = Cena::nova();
    iniciar_face(
        &cena,
        &[CONCLUIDO; 2],
        &[[A4; 4].as_slice(), &[A5; 4]].concat(),
        Alcance::SoVersos,
    );
    cena.esperar("a", aguardando);
    cena.comandar("a", Comando::EnviarFrente);
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(cena.enviados(), [Passada::Verso]);
    cena.comandar("a", Comando::Cancelar);
    esperar_fim(&cena, "a");
}

#[test]
fn so_versos_sem_lote_manual_acaba_sem_enviar_nada() {
    let pdf = PdfDeTeste::novo("so-versos-auto", &[CAIXA_A4; 6]);
    let cena = Cena::nova();
    let pedido = Pedido::novo(pdf.caminho.clone(), None).com_alcance(Alcance::SoVersos);
    let mut id = identidade("auto");
    id.simulada = true;
    assert!(cena.gerente.iniciar(id, cena.emissor(), move || {
        preparar_simulada_automatica(ModeloImpressora::JatoDeTinta, &pedido)
    }));
    esperar_fim(&cena, "auto");
    let fim = cena.cartoes_de("auto").pop().unwrap();
    assert_eq!(fim.estado, EstadoCartao::Concluido);
    assert_eq!(fim.resultado, None);
}

#[test]
fn resumo_conta_as_folhas_e_o_maior_maco_por_papel() {
    let misto = [[CAIXA_A4; 70].as_slice(), &[CAIXA_A5; 7]].concat();
    let pdf = PdfDeTeste::novo("resumo-folhas", &misto);
    let resumo = resumo_do_envio(&pdf.caminho, None).unwrap();
    assert_eq!(resumo.lotes, 2);
    assert_eq!(resumo.folhas, 39);
    assert_eq!(resumo.maior_maco, 35);
}

#[test]
fn resumo_com_copias_conta_as_folhas_de_cada_copia() {
    let pdf = PdfDeTeste::novo("resumo-copias", &[CAIXA_A4; 3]);
    let uma = resumo_do_envio_com_copias(&pdf.caminho, None, None, 1).unwrap();
    assert_eq!((uma.folhas, uma.lotes), (2, 1));
    let duas = resumo_do_envio_com_copias(&pdf.caminho, None, None, 2).unwrap();
    assert_eq!((duas.folhas, duas.maior_maco, duas.lotes), (4, 4, 1));
}

#[test]
fn copias_do_pedido_formam_um_lote_so_e_o_numero_fora_do_limite_e_recusado() {
    let dimensoes = [A4; 3];
    let perfil = PerfilImpressora::default();
    let duas = Pedido::novo("x.pdf".to_owned(), None).com_copias(2);
    let lotes = planejar_pedido(&dimensoes, &duas, perfil, false).unwrap();
    assert_eq!(lotes.len(), 1);
    assert_eq!(lotes[0].lote.paginas.len(), 8);
    for numero in [0, 1000] {
        let pedido = Pedido::novo("x.pdf".to_owned(), None).com_copias(numero);
        assert!(
            planejar_pedido(&dimensoes, &pedido, perfil, false).is_err(),
            "{numero}"
        );
    }
}

#[test]
fn automatico_com_copias_sai_em_um_trabalho_por_copia_e_o_resumo_conta_todos() {
    let pedido = Pedido::novo("x.pdf".to_owned(), None)
        .com_copias(3)
        .com_uma_copia_por_vez(true);
    let lotes = planejar_pedido(&[A4; 3], &pedido, PerfilImpressora::default(), true).unwrap();
    assert_eq!(lotes.len(), 3);
    assert!(lotes.iter().all(|item| item.lote.duas_faces));
    let pdf = PdfDeTeste::novo("automatico-copias", &[CAIXA_A4; 3]);
    let resumo = resumo_do_envio_com_copias(&pdf.caminho, None, None, 3).unwrap();
    assert_eq!((resumo.lotes_automaticos, resumo.folhas), (3, 6));
}

#[test]
fn troca_do_pdf_entre_frente_e_verso_recusa_o_verso_antes_de_enviar() {
    let pdf = PdfDeTeste::novo("documento-mudou-troca", &[CAIXA_A4; 2]);
    let antes = impressao_do_arquivo(&pdf.caminho).unwrap();
    let outro = PdfDeTeste::novo("documento-mudou-troca-b", &[CAIXA_A4; 3]);
    std::fs::copy(&outro.caminho, &pdf.caminho).unwrap();
    let pedido = Pedido::novo(pdf.caminho.clone(), None).com_impressao_esperada(Some(antes));
    let erro = preparar_simulada(ModeloImpressora::Laser, &pedido)
        .err()
        .expect("o verso e recusado");
    assert_eq!(erro.codigo, "documento_mudou");
}

#[test]
fn mudar_so_a_medida_de_uma_pagina_recusa_o_verso() {
    let pdf = PdfDeTeste::novo("documento-mudou-medida", &[CAIXA_A4; 2]);
    let antes = impressao_do_arquivo(&pdf.caminho).unwrap();
    let outro = PdfDeTeste::novo("documento-mudou-medida-b", &[CAIXA_A4, CAIXA_A5]);
    std::fs::copy(&outro.caminho, &pdf.caminho).unwrap();
    let pedido = Pedido::novo(pdf.caminho.clone(), None).com_impressao_esperada(Some(antes));
    let erro = preparar_simulada(ModeloImpressora::Laser, &pedido)
        .err()
        .expect("o verso e recusado");
    assert_eq!(erro.codigo, "documento_mudou");
}

#[test]
fn arquivo_igual_depois_das_frentes_segue_para_o_verso() {
    let pdf = PdfDeTeste::novo("documento-igual", &[CAIXA_A4; 2]);
    let antes = impressao_do_arquivo(&pdf.caminho).unwrap();
    let pedido = Pedido::novo(pdf.caminho.clone(), None).com_impressao_esperada(Some(antes));
    assert!(preparar_simulada(ModeloImpressora::Laser, &pedido).is_ok());
}

#[test]
fn aba_que_nao_imprimiu_frente_segue_livre_mesmo_com_o_arquivo_trocado() {
    let pdf = PdfDeTeste::novo("documento-livre", &[CAIXA_A4; 2]);
    let outro = PdfDeTeste::novo("documento-livre-b", &[CAIXA_A4; 3]);
    std::fs::copy(&outro.caminho, &pdf.caminho).unwrap();
    let pedido = Pedido::novo(pdf.caminho.clone(), None);
    assert!(preparar_simulada(ModeloImpressora::Laser, &pedido).is_ok());
}

#[test]
fn simulada_automatica_com_gesto_de_laser_nao_pede_virar_o_maco() {
    let pdf = PdfDeTeste::novo("automatica-sem-virar", &[CAIXA_A4; 4]);
    let pedido = Pedido::novo(pdf.caminho.clone(), None);
    let preparo = preparar_simulada_automatica(ModeloImpressora::Laser, &pedido)
        .expect("a simulada automatica prepara");
    assert!(!preparo.vira_para_ler);
}

#[test]
fn ultima_frente_aceita_prevalece_na_impressao_guardada() {
    let aba = 910_002;
    let a = PdfDeTeste::novo("frente-aceita", &[CAIXA_A4; 2]);
    let pedido_a = Pedido::novo(a.caminho.clone(), None)
        .com_alcance(Alcance::SoFrentes)
        .com_aba(aba);
    assert!(preparar_simulada(ModeloImpressora::Laser, &pedido_a).is_ok());
    let impressao_a = impressao_do_arquivo(&a.caminho).unwrap();
    assert_eq!(impressao_guardada(aba), Some(impressao_a));
    let outro = PdfDeTeste::novo("frente-aceita-b", &[CAIXA_A4; 3]);
    std::fs::copy(&outro.caminho, &a.caminho).unwrap();
    assert!(preparar_simulada(ModeloImpressora::Laser, &pedido_a).is_ok());
    let impressao_b = impressao_do_arquivo(&a.caminho).unwrap();
    assert_eq!(impressao_guardada(aba), Some(impressao_b));
    esquecer_impressao(aba);
}

#[test]
fn frente_guarda_o_que_leu_mesmo_com_o_arquivo_trocado_depois_do_pedido() {
    let aba = 920_001;
    let pdf = PdfDeTeste::novo("frente-le-depois", &[CAIXA_A4; 2]);
    let pedido = Pedido::novo(pdf.caminho.clone(), None)
        .com_alcance(Alcance::SoFrentes)
        .com_aba(aba);
    let outro = PdfDeTeste::novo("frente-le-depois-b", &[CAIXA_A4; 3]);
    std::fs::copy(&outro.caminho, &pdf.caminho).unwrap();
    let lido_pela_frente = impressao_do_arquivo(&pdf.caminho).unwrap();
    assert!(preparar_simulada(ModeloImpressora::Laser, &pedido).is_ok());
    assert_eq!(impressao_guardada(aba), Some(lido_pela_frente.clone()));
    let verso = Pedido::novo(pdf.caminho.clone(), None)
        .com_alcance(Alcance::SoVersos)
        .com_impressao_esperada(impressao_guardada(aba));
    assert!(preparar_simulada(ModeloImpressora::Laser, &verso).is_ok());
    esquecer_impressao(aba);
}

#[test]
fn frente_cujo_preparo_falha_nao_guarda_impressao_nenhuma() {
    let aba = 920_002;
    let pdf = PdfDeTeste::novo("frente-falha-preparo", &[CAIXA_A4; 2]);
    let pedido = Pedido::novo(pdf.caminho.clone(), None)
        .com_alcance(Alcance::SoFrentes)
        .com_aba(aba)
        .com_copias(0);
    assert!(preparar_simulada(ModeloImpressora::Laser, &pedido).is_err());
    assert_eq!(impressao_guardada(aba), None);
    let sumido = Pedido::novo(format!("{}-nao-existe.pdf", pdf.caminho), None)
        .com_alcance(Alcance::SoFrentes)
        .com_aba(aba);
    assert!(preparar_simulada(ModeloImpressora::Laser, &sumido).is_err());
    assert_eq!(impressao_guardada(aba), None);
}

#[test]
fn fechar_a_aba_esquece_a_impressao_da_frente() {
    let aba = 910_003;
    guardar_impressao_da_frente(aba, "0000000000000000:0".to_owned());
    esquecer_impressao(aba);
    assert_eq!(impressao_guardada(aba), None);
}

#[test]
fn impressao_do_pdf_lido_e_a_mesma_do_arquivo_em_disco() {
    let pdf = PdfDeTeste::novo("impressao-lida", &[CAIXA_A4, CAIXA_A5]);
    let lido = leitura_pdf::ler_pdf(&pdf.caminho).unwrap();
    assert_eq!(lido.impressao, impressao_do_arquivo(&pdf.caminho).unwrap());
}

#[test]
fn verso_recusa_quando_o_pdf_lido_nao_bate_com_a_frente() {
    let pdf = PdfDeTeste::novo("verso-confere-bytes", &[CAIXA_A4; 2]);
    let outro = PdfDeTeste::novo("verso-confere-bytes-b", &[CAIXA_A4; 3]);
    let lido_outro = leitura_pdf::ler_pdf(&outro.caminho).unwrap();
    let pedido = Pedido::novo(pdf.caminho.clone(), None)
        .com_impressao_esperada(Some(lido_outro.impressao.clone()));
    let erro = preparar_simulada(ModeloImpressora::Laser, &pedido)
        .err()
        .expect("o verso e recusado");
    assert_eq!(erro.codigo, "documento_mudou");
}

#[test]
fn uma_copia_por_vez_envia_a_proxima_so_depois_da_confirmacao() {
    let cena = Cena::nova();
    let leituras = [
        Leitura::Estado(EstadoEnvio::EmAndamento),
        Leitura::Estado(EstadoEnvio::Concluido),
        Leitura::Estado(EstadoEnvio::EmAndamento),
        Leitura::Estado(EstadoEnvio::Concluido),
        Leitura::Estado(EstadoEnvio::EmAndamento),
        Leitura::Estado(EstadoEnvio::Concluido),
    ];
    assert!(cena.iniciar_uma_copia_por_vez("a", &leituras, &[A4; 2], 3));
    cena.esperar_envios(3);
    let fim = cena.esperar("a", |estado| matches!(estado, EstadoCartao::FrentesProntas));
    assert_eq!(fim.copias, 3);
    assert!(fim.por_copia);
    assert_eq!(
        cena.enviados(),
        vec![Passada::Frente, Passada::Frente, Passada::Frente]
    );
    assert_eq!(
        cena.eventos(),
        vec![
            "enviar",
            "consultar",
            "consultar",
            "enviar",
            "consultar",
            "consultar",
            "enviar",
            "consultar",
            "consultar",
        ],
        "a próxima cópia só sai depois do segundo consultar, que confirma a anterior"
    );
}

#[test]
fn copias_com_inversao_e_giro_do_perfil_valem_para_o_maco_inteiro() {
    let dimensoes = [A4; 3];
    let ajustado = PerfilImpressora {
        passada1_inversa: true,
        passada2_inversa: true,
        verso_girar_180: true,
        ..PerfilImpressora::default()
    };
    let neutro = PerfilImpressora {
        passada1_inversa: false,
        passada2_inversa: false,
        verso_girar_180: false,
        ..PerfilImpressora::default()
    };
    for copias in [2usize, 3] {
        let pedido = Pedido::novo("x.pdf".to_owned(), None).com_copias(copias);
        let com = planejar_pedido(&dimensoes, &pedido, ajustado, false).unwrap();
        let sem = planejar_pedido(&dimensoes, &pedido, neutro, false).unwrap();
        assert_eq!(
            com.len(),
            1,
            "{copias} cópias formam um trabalho de frentes"
        );
        let numeros = |passada: &[plots_core::Pagina]| -> Vec<Option<usize>> {
            passada.iter().map(|pagina| pagina.numero).collect()
        };
        let mut esperada1 = numeros(&sem[0].lote.passada1);
        esperada1.reverse();
        let mut esperada2 = numeros(&sem[0].lote.passada2);
        esperada2.reverse();
        assert_eq!(numeros(&com[0].lote.passada1), esperada1, "{copias} cópias");
        assert_eq!(numeros(&com[0].lote.passada2), esperada2, "{copias} cópias");
        assert!(com[0].lote.passada2.iter().all(|pagina| pagina.girar_180));
        assert!(sem[0].lote.passada2.iter().all(|pagina| !pagina.girar_180));
    }
}
