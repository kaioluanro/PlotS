use super::*;
use crate::tempo_de_impressao::{Medida, Relogio};
use crate::trabalhos::{
    self, Cartao, Comando, DriverDaSimulada, Emissor, EstadoCartao, Identidade, Pedido, Preparo,
};
use plots_core::Pagina;
use plots_core::{ModeloImpressora, PerfilImpressora};
use plots_windows::plots_print::fluxo::{Condutor, DescreveFalha, Espera};
use plots_windows::plots_print::job::{Backend, EstadoEnvio, Passada};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const PRAZO: Duration = Duration::from_secs(20);

type Registros = Arc<Mutex<Vec<RegistroDeEnvio>>>;

fn cabecalho(divisao: bool, lotes: usize) -> CabecalhoDoEnvio {
    CabecalhoDoEnvio {
        arquivo: "C:/pasta secreta/contrato.pdf".to_owned(),
        intervalo: Some((3, 8)),
        divisao,
        lotes_confirmados: lotes,
    }
}

fn diario(divisao: bool, lotes: usize, nomes: &[&str]) -> (Arc<Diario>, Registros) {
    let registros: Registros = Arc::default();
    let copia = Arc::clone(&registros);
    let gravar: Gravar = Box::new(move |registro| copia.lock().unwrap().push(registro));
    let diario = Diario::novo(
        cabecalho(divisao, lotes),
        nomes.iter().map(|nome| (*nome).to_owned()).collect(),
        1_700_000_000,
        gravar,
    );
    (Arc::new(diario), registros)
}

fn desfecho(
    resultado: ResultadoDoEnvio,
    lotes: u64,
    feitos: u64,
    ms: u64,
    folhas: u64,
) -> Desfecho {
    Desfecho {
        resultado,
        lotes,
        lotes_feitos: feitos,
        paginas: 6,
        medida: Medida {
            milissegundos: ms,
            folhas,
        },
        medida_automatica: Medida::default(),
    }
}

#[test]
fn o_registro_so_sai_quando_a_ultima_impressora_termina_e_uma_vez_so() {
    let (diario, registros) = diario(false, 0, &["fila:A", "fila:B"]);
    Diario::fim_da_parte(&diario, "fila:A", false)(desfecho(
        ResultadoDoEnvio::Concluido,
        2,
        2,
        60_000,
        12,
    ));
    assert!(registros.lock().unwrap().is_empty());
    Diario::fim_da_parte(&diario, "fila:B", false)(desfecho(
        ResultadoDoEnvio::Concluido,
        2,
        2,
        30_000,
        12,
    ));
    let lista = registros.lock().unwrap().clone();
    assert_eq!(lista.len(), 1);
    let registro = &lista[0];
    assert_eq!(registro.resultado, ResultadoDoEnvio::Concluido);
    assert_eq!((registro.lotes, registro.lotes_faltando), (2, 0));
    assert_eq!(registro.intervalo, Some((3, 8)));
    assert_eq!(registro.impressoras, ["fila:A", "fila:B"]);
    assert_eq!(registro.tempos.len(), 2);
    assert_eq!(registro.tempos[0].milissegundos, 60_000);
    diario.gravar_se_pronto(true);
    assert_eq!(registros.lock().unwrap().len(), 1, "nao grava duas vezes");
}

#[test]
fn com_o_programa_fechando_quem_faltou_entra_como_incompleto() {
    let (diario, registros) = diario(false, 0, &["fila:A", "fila:B"]);
    Diario::fim_da_parte(&diario, "fila:A", false)(desfecho(
        ResultadoDoEnvio::Concluido,
        3,
        3,
        1_000,
        3,
    ));
    let diarios = Diarios::default();
    diarios.0.lock().unwrap().push(Arc::clone(&diario));
    diarios.descarregar();
    let lista = registros.lock().unwrap().clone();
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].resultado, ResultadoDoEnvio::Incompleto);
    assert_eq!(lista[0].lotes_faltando, 3);
}

#[test]
fn erro_pesa_mais_que_cancelado_e_cancelado_mais_que_incompleto() {
    for (a, b, esperado) in [
        (
            ResultadoDoEnvio::Concluido,
            ResultadoDoEnvio::ComErro,
            ResultadoDoEnvio::ComErro,
        ),
        (
            ResultadoDoEnvio::Cancelado,
            ResultadoDoEnvio::ComErro,
            ResultadoDoEnvio::ComErro,
        ),
        (
            ResultadoDoEnvio::Incompleto,
            ResultadoDoEnvio::Cancelado,
            ResultadoDoEnvio::Cancelado,
        ),
        (
            ResultadoDoEnvio::Concluido,
            ResultadoDoEnvio::Incompleto,
            ResultadoDoEnvio::Incompleto,
        ),
    ] {
        let (diario, registros) = diario(false, 0, &["x", "y"]);
        Diario::fim_da_parte(&diario, "x", false)(desfecho(a, 1, 1, 1, 1));
        Diario::fim_da_parte(&diario, "y", false)(desfecho(b, 1, 0, 1, 1));
        assert_eq!(registros.lock().unwrap()[0].resultado, esperado);
    }
}

#[test]
fn na_divisao_os_lotes_se_somam_e_o_que_falta_conta_o_documento_todo() {
    let (diario, registros) = diario(true, 5, &["x", "y"]);
    Diario::fim_da_parte(&diario, "x", false)(desfecho(ResultadoDoEnvio::Concluido, 3, 3, 1, 1));
    Diario::fim_da_parte(&diario, "y", false)(desfecho(ResultadoDoEnvio::Cancelado, 2, 1, 1, 1));
    let registro = registros.lock().unwrap()[0].clone();
    assert_eq!((registro.lotes, registro.lotes_faltando), (5, 1));
    assert_eq!(registro.paginas, 12);
    assert_eq!(registro.resultado, ResultadoDoEnvio::Cancelado);
}

#[test]
fn impressora_ocupada_nao_entra_no_registro_e_sem_nenhuma_nada_e_gravado() {
    let (diario_um, registros) = diario(false, 0, &["x", "y"]);
    diario_um.desistir("y");
    Diario::fim_da_parte(&diario_um, "x", false)(desfecho(ResultadoDoEnvio::Concluido, 1, 1, 1, 1));
    assert_eq!(registros.lock().unwrap()[0].impressoras, ["x"]);

    let (diario_dois, registros) = diario(false, 0, &["x"]);
    diario_dois.desistir("x");
    assert!(registros.lock().unwrap().is_empty());
}

#[test]
fn a_simulada_entra_no_historico_mas_nao_na_medida_de_tempo() {
    let (diario, registros) = diario(false, 0, &["simulada:gaveta"]);
    Diario::fim_da_parte(&diario, "simulada:gaveta", true)(desfecho(
        ResultadoDoEnvio::Concluido,
        1,
        1,
        900,
        30,
    ));
    let registro = registros.lock().unwrap()[0].clone();
    assert_eq!(registro.impressoras, ["simulada:gaveta"]);
    assert!(registro.tempos.is_empty());
}

#[test]
fn gravar_de_verdade_tira_a_pasta_do_nome_e_falha_de_gravacao_nao_trava() {
    let pasta = std::env::temp_dir().join(format!("plots-diario-{}", std::process::id()));
    std::fs::create_dir_all(&pasta).unwrap();
    let arquivo = pasta.join("historico.toml");
    let destino = arquivo.clone();
    let gravar: Gravar = Box::new(move |registro| {
        historico::acrescentar_ao_historico(&destino, registro).unwrap();
    });
    let diario = Arc::new(Diario::novo(
        cabecalho(false, 0),
        vec!["x".to_owned()],
        5,
        gravar,
    ));
    Diario::fim_da_parte(&diario, "x", false)(desfecho(ResultadoDoEnvio::Concluido, 1, 1, 10, 10));
    let lido = historico::carregar_historico(&arquivo).unwrap().registros();
    assert_eq!(lido[0].arquivo, "contrato.pdf");
    assert!(!std::fs::read_to_string(&arquivo)
        .unwrap()
        .contains("pasta secreta"));
    std::fs::remove_dir_all(&pasta).unwrap();

    // Caminho impossível: a falha fica no diário.
    let gravar: Gravar = Box::new(|registro| {
        let _ = historico::acrescentar_ao_historico(
            std::path::Path::new("Z:/\0/nao/existe.toml"),
            registro,
        );
    });
    let diario = Arc::new(Diario::novo(
        cabecalho(false, 0),
        vec!["x".to_owned()],
        5,
        gravar,
    ));
    Diario::fim_da_parte(&diario, "x", false)(desfecho(ResultadoDoEnvio::Concluido, 1, 1, 10, 10));
    assert!(diario.gravado());
}

fn medido(impressora: &str, ms: u64, folhas: u64, resultado: ResultadoDoEnvio) -> RegistroDeEnvio {
    RegistroDeEnvio {
        arquivo: "a.pdf".to_owned(),
        data: 1,
        impressoras: vec![impressora.to_owned()],
        intervalo: None,
        paginas: 1,
        lotes: 1,
        resultado,
        lotes_faltando: 0,
        tempos: vec![TempoDaImpressora {
            impressora: impressora.to_owned(),
            milissegundos: ms,
            folhas,
            resultado,
            automatico: false,
        }],
    }
}

fn ids(nomes: &[&str]) -> Vec<String> {
    nomes.iter().map(|nome| (*nome).to_owned()).collect()
}

#[test]
fn a_comparacao_precisa_de_dados_de_todas_e_de_diferenca_de_um_quarto() {
    use ResultadoDoEnvio::{Cancelado, Concluido};
    let registros = [
        medido("fila:A", 60_000, 20, Concluido),
        medido("fila:B", 60_000, 10, Concluido),
        medido("fila:C", 60_000, 9, Concluido),
        medido("fila:D", 60_000, 12, Concluido),
        medido("fila:E", 1_000, 500, Cancelado),
    ];
    let achada = comparar_pelos_registros(&registros, &ids(&["fila:B", "fila:A"]), false).unwrap();
    assert_eq!(
        (achada.rapida.as_str(), achada.lenta.as_str()),
        ("fila:A", "fila:B")
    );
    assert!((achada.razao - 2.0).abs() < 1e-9);
    assert!(
        comparar_pelos_registros(&registros, &ids(&["fila:A", "fila:C"]), false).is_none(),
        "com menos de dez folhas nao ha numero"
    );
    assert!(
        comparar_pelos_registros(&registros, &ids(&["fila:A", "fila:E"]), false).is_none(),
        "cancelado nao entra na media"
    );
    assert!(
        comparar_pelos_registros(&registros, &ids(&["fila:B", "fila:D"]), false).is_none(),
        "vinte por cento nao avisa"
    );
    assert!(comparar_pelos_registros(&registros, &ids(&["fila:A"]), false).is_none());
    assert!(comparar_pelos_registros(&[], &ids(&["fila:A", "fila:B"]), false).is_none());
}

#[test]
fn a_lista_so_traz_velocidade_de_quem_tem_dez_folhas() {
    use ResultadoDoEnvio::Concluido;
    let registros = [
        medido("fila:A", 60_000, 30, Concluido),
        medido("fila:B", 60_000, 9, Concluido),
    ];
    let lista = velocidades_de(&registros);
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].impressora, "fila:A");
    assert!((lista[0].folhas_por_minuto - 30.0).abs() < 1e-9);
}

// A partir daqui, a thread de verdade, com um relógio que só anda quando o teste manda.

/// Anda de acordo com o que cada consulta à fila fingida levou.
#[derive(Default)]
struct RelogioFalso(AtomicU64);

impl RelogioFalso {
    fn andar(&self, ms: u64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}

impl Relogio for RelogioFalso {
    fn agora_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Clone, Copy)]
enum Cena {
    /// A fila trabalha por este tempo e continua.
    Anda(u64),
    /// A impressora parou com um aviso, por este tempo.
    Parada(u64),
    Confirma(u64),
    Falha(u64),
}

struct FilaFingida {
    relogio: Arc<RelogioFalso>,
    cena: VecDeque<Cena>,
    motivo: Option<String>,
}

impl Backend for FilaFingida {
    type Erro = String;
    fn enviar(&mut self, _: Passada, _: &[Pagina]) -> Result<(), String> {
        Ok(())
    }
    fn consultar(&mut self) -> Result<EstadoEnvio, String> {
        self.motivo = None;
        match self.cena.pop_front() {
            Some(Cena::Anda(ms)) => {
                self.relogio.andar(ms);
                Ok(EstadoEnvio::EmAndamento)
            }
            Some(Cena::Parada(ms)) => {
                self.relogio.andar(ms);
                self.motivo = Some("sem papel".to_owned());
                Ok(EstadoEnvio::EmAndamento)
            }
            Some(Cena::Confirma(ms)) => {
                self.relogio.andar(ms);
                Ok(EstadoEnvio::Concluido)
            }
            Some(Cena::Falha(ms)) => {
                self.relogio.andar(ms);
                Ok(EstadoEnvio::Falhou)
            }
            None => Ok(EstadoEnvio::EmAndamento),
        }
    }
    fn cancelar(&mut self) -> Result<(), String> {
        Ok(())
    }
}

impl DescreveFalha for FilaFingida {
    fn motivo(&self) -> Option<String> {
        self.motivo.clone()
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        false
    }
}

struct Palco {
    gerente: Gerente,
    cartoes: Arc<Mutex<Vec<Cartao>>>,
    desfechos: Arc<Mutex<Vec<Desfecho>>>,
    relogio: Arc<RelogioFalso>,
}

impl Palco {
    /// Um documento de 4 folhas em dois lotes, na fila fingida.
    fn iniciar(cena: &[Cena]) -> Self {
        let palco = Self {
            gerente: Gerente::default(),
            cartoes: Arc::default(),
            desfechos: Arc::default(),
            relogio: Arc::default(),
        };
        let copia = Arc::clone(&palco.cartoes);
        let emitir: Emissor = Arc::new(move |cartao| copia.lock().unwrap().push(cartao));
        let destino = Arc::clone(&palco.desfechos);
        let fim: FimDoTrabalho = Box::new(move |desfecho| destino.lock().unwrap().push(desfecho));
        let backend = FilaFingida {
            relogio: Arc::clone(&palco.relogio),
            cena: cena.iter().copied().collect(),
            motivo: None,
        };
        let identidade = Identidade {
            aba: 1,
            impressora: "fila:Medida".to_owned(),
            nome: "Medida".to_owned(),
            simulada: false,
        };
        let pedido = Pedido::novo("x.pdf".to_owned(), 2, None).unwrap();
        let dimensoes = [plots_core::DimensoesPagina::new(210.0, 297.0); 8];
        let lotes =
            trabalhos::planejar_pedido(&dimensoes, &pedido, PerfilImpressora::default(), false)
                .unwrap();
        let relogio: Arc<dyn Relogio> = palco.relogio.clone();
        assert!(palco
            .gerente
            .iniciar_medido(identidade, emitir, relogio, fim, move || {
                Ok(Preparo {
                    condutor: Condutor::novo(
                        backend,
                        lotes,
                        Espera {
                            intervalo: Duration::from_millis(1),
                            consultas: 100_000,
                        },
                    )
                    .map_err(|_| trabalhos::fila_indisponivel())?,
                    recolocar: trabalhos::Recolocar {
                        lado: None,
                        virada: None,
                        texto_do_perfil: None,
                    },
                    vira_para_ler: false,
                })
            }));
        palco
    }

    fn esperar(&self, aceita: impl Fn(&EstadoCartao) -> bool) {
        let inicio = Instant::now();
        loop {
            let ultimo = self
                .cartoes
                .lock()
                .unwrap()
                .last()
                .map(|c| c.estado.clone());
            if ultimo.as_ref().is_some_and(&aceita) {
                return;
            }
            assert!(inicio.elapsed() < PRAZO, "sem o estado: {ultimo:?}");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn comandar(&self, comando: Comando) {
        self.gerente.comandar("fila:Medida", comando).unwrap();
    }

    fn desfecho(&self) -> Desfecho {
        let inicio = Instant::now();
        loop {
            if let Some(desfecho) = self.desfechos.lock().unwrap().first().copied() {
                return desfecho;
            }
            assert!(inicio.elapsed() < PRAZO, "o trabalho nao terminou");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

fn aguardando(estado: &EstadoCartao) -> bool {
    matches!(estado, EstadoCartao::Aguardando)
}

fn proximo(estado: &EstadoCartao) -> bool {
    matches!(estado, EstadoCartao::ProximoLote)
}

#[test]
fn o_tempo_e_so_o_da_impressora_sem_recolocacao_nem_aviso() {
    let palco = Palco::iniciar(&[
        Cena::Anda(1_000),
        Cena::Parada(0),
        Cena::Anda(600_000),
        Cena::Anda(500),
        Cena::Confirma(500),
        Cena::Confirma(2_000),
        Cena::Confirma(1_000),
        Cena::Confirma(1_000),
    ]);
    palco.esperar(aguardando);
    // A pessoa demora uma hora: o relógio anda, e nada conta.
    palco.relogio.andar(3_600_000);
    palco.comandar(Comando::EnviarVerso);
    palco.esperar(proximo);
    palco.relogio.andar(120_000);
    palco.comandar(Comando::EnviarFrente);
    palco.esperar(aguardando);
    palco.comandar(Comando::EnviarVerso);
    palco.esperar(|estado| matches!(estado, EstadoCartao::Concluido));
    let desfecho = palco.desfecho();
    assert_eq!(desfecho.resultado, ResultadoDoEnvio::Concluido);
    // 1000 antes do aviso; depois do aviso, o que correu até a confirmação; 2000 do verso; 2000 do outro lote.
    assert_eq!(desfecho.medida.milissegundos, 1_000 + 1_000 + 2_000 + 2_000);
    assert_eq!(desfecho.medida.folhas, 4);
    assert_eq!((desfecho.lotes, desfecho.lotes_feitos), (2, 2));
    assert_eq!(desfecho.paginas, 8);
}

#[test]
fn cancelar_diz_cancelado_e_guarda_so_os_lotes_inteiros() {
    let palco = Palco::iniciar(&[Cena::Confirma(1_000), Cena::Confirma(1_000), Cena::Anda(50)]);
    palco.esperar(aguardando);
    palco.comandar(Comando::EnviarVerso);
    palco.esperar(proximo);
    palco.comandar(Comando::EnviarFrente);
    palco.comandar(Comando::Cancelar);
    let desfecho = palco.desfecho();
    assert_eq!(desfecho.resultado, ResultadoDoEnvio::Cancelado);
    assert_eq!(desfecho.medida.milissegundos, 2_000);
    assert_eq!(desfecho.medida.folhas, 2);
    assert_eq!(desfecho.lotes_feitos, 1);
}

#[test]
fn erro_na_fila_seguido_do_fechamento_diz_com_erro() {
    let palco = Palco::iniciar(&[Cena::Falha(10)]);
    palco.esperar(|estado| matches!(estado, EstadoCartao::Erro { .. }));
    palco.gerente.soltar_todos_e_esperar(PRAZO);
    assert_eq!(palco.desfecho().resultado, ResultadoDoEnvio::ComErro);
}

#[test]
fn reimprimir_depois_do_erro_conta_so_a_tentativa_que_saiu_e_o_fim_vale() {
    let palco = Palco::iniciar(&[
        Cena::Falha(70_000),
        Cena::Confirma(1_000),
        Cena::Confirma(1_000),
        Cena::Confirma(1_000),
        Cena::Confirma(1_000),
    ]);
    palco.esperar(|estado| matches!(estado, EstadoCartao::Erro { .. }));
    palco.comandar(Comando::Reimprimir);
    palco.esperar(aguardando);
    palco.comandar(Comando::EnviarVerso);
    palco.esperar(proximo);
    palco.comandar(Comando::EnviarFrente);
    palco.esperar(aguardando);
    palco.comandar(Comando::EnviarVerso);
    palco.esperar(|estado| matches!(estado, EstadoCartao::Concluido));
    let desfecho = palco.desfecho();
    assert_eq!(desfecho.resultado, ResultadoDoEnvio::Concluido);
    assert_eq!(desfecho.medida.milissegundos, 4_000);
}

#[test]
fn soltar_com_o_programa_fechando_conta_como_incompleto_e_nao_espera_alem_do_prazo() {
    let palco = Palco::iniciar(&[Cena::Confirma(1_000)]);
    palco.esperar(aguardando);
    let inicio = Instant::now();
    palco.gerente.soltar_todos_e_esperar(PRAZO);
    assert!(inicio.elapsed() < PRAZO);
    let desfecho = palco.desfecho();
    assert_eq!(desfecho.resultado, ResultadoDoEnvio::Incompleto);
    assert_eq!(desfecho.lotes_feitos, 0);
}

#[test]
fn preparacao_recusada_conta_como_erro_sem_nada_impresso() {
    let gerente = Gerente::default();
    let desfechos: Arc<Mutex<Vec<Desfecho>>> = Arc::default();
    let destino = Arc::clone(&desfechos);
    let emitir: Emissor = Arc::new(|_| {});
    let fim: FimDoTrabalho = Box::new(move |desfecho| destino.lock().unwrap().push(desfecho));
    let identidade = Identidade {
        aba: 1,
        impressora: "fila:X".to_owned(),
        nome: "X".to_owned(),
        simulada: false,
    };
    let relogio: Arc<dyn Relogio> = Arc::new(RelogioFalso::default());
    gerente.iniciar_medido::<FilaFingida, _>(identidade, emitir, relogio, fim, || {
        Err(trabalhos::Motivo::novo(
            trabalhos::MOTIVO_DOCUMENTO_MUDOU,
            "mudou",
        ))
    });
    let inicio = Instant::now();
    while desfechos.lock().unwrap().is_empty() {
        assert!(inicio.elapsed() < PRAZO);
        std::thread::sleep(Duration::from_millis(1));
    }
    let desfecho = desfechos.lock().unwrap()[0];
    assert_eq!(desfecho.resultado, ResultadoDoEnvio::ComErro);
    assert_eq!(desfecho.lotes, 0);
}

#[test]
fn a_simulada_com_controle_mostra_a_marca_e_sem_controle_nao() {
    use plots_windows::plots_print::opcoes::IdiomaDaMarca;
    let pdf = std::env::temp_dir().join(format!("plots-controle-{}.pdf", std::process::id()));
    std::fs::write(&pdf, pdf_a4(4)).unwrap();
    let caminho = pdf.to_string_lossy().into_owned();
    let marcas = |controle: Option<IdiomaDaMarca>| {
        let pedido = Pedido::novo(caminho.clone(), 2, None)
            .unwrap()
            .com_controle(controle)
            .com_driver_da_simulada(DriverDaSimulada::default());
        let preparo = trabalhos::preparar_simulada(ModeloImpressora::Laser, &pedido)
            .map_err(|motivo| motivo.texto)
            .unwrap();
        preparo.condutor.lotes()[0].marcas.clone()
    };
    assert!(marcas(None).is_empty());
    let com = marcas(Some(IdiomaDaMarca::Portugues));
    assert_eq!(com[&Some(2)], "Lote 1 · folha 1");
    assert_eq!(com[&Some(4)], "Lote 1 · folha 2");
    std::fs::remove_file(&pdf).unwrap();
}

#[test]
fn na_divisao_a_marca_mantem_o_numero_do_lote_no_documento() {
    use plots_windows::plots_print::opcoes::IdiomaDaMarca;
    let dimensoes = [plots_core::DimensoesPagina::new(210.0, 297.0); 8];
    let pedido = Pedido::novo("x.pdf".to_owned(), 1, None)
        .unwrap()
        .com_controle(Some(IdiomaDaMarca::Portugues))
        .com_parte(Some(trabalhos::ParteDoEnvio {
            indice: 1,
            impressoras: 2,
            lotes: 4,
            paginas: 8,
        }));
    let lotes = trabalhos::planejar_pedido(&dimensoes, &pedido, PerfilImpressora::default(), false)
        .unwrap();
    assert_eq!(lotes.len(), 2);
    assert_eq!(lotes[0].marcas[&Some(4)], "Lote 2 · folha 1");
    assert_eq!(lotes[1].marcas[&Some(8)], "Lote 4 · folha 1");
}

fn pdf_a4(paginas: usize) -> Vec<u8> {
    let mut objetos = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {paginas} >>",
            (0..paginas)
                .map(|indice| format!("{} 0 R", indice + 3))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    ];
    for _ in 0..paginas {
        objetos.push("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>".to_owned());
    }
    let mut texto = String::from("%PDF-1.4\n");
    let mut posicoes = Vec::new();
    for (indice, objeto) in objetos.iter().enumerate() {
        posicoes.push(texto.len());
        texto.push_str(&format!("{} 0 obj\n{objeto}\nendobj\n", indice + 1));
    }
    let tabela = texto.len();
    texto.push_str(&format!(
        "xref\n0 {}\n0000000000 65535 f \n",
        objetos.len() + 1
    ));
    for posicao in posicoes {
        texto.push_str(&format!("{posicao:010} 00000 n \n"));
    }
    texto.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{tabela}\n%%EOF\n",
        objetos.len() + 1
    ));
    texto.into_bytes()
}

static PASTA_DA_ESCOLHA: AtomicU64 = AtomicU64::new(0);

/// Uma pasta só deste teste, apagada no fim, com os dois arquivos: perfis e histórico.
struct PastaDaEscolha(PathBuf);

impl PastaDaEscolha {
    fn nova() -> Self {
        let numero = PASTA_DA_ESCOLHA.fetch_add(1, Ordering::Relaxed);
        let pasta =
            std::env::temp_dir().join(format!("plots-escolha-{}-{numero}", std::process::id()));
        std::fs::create_dir_all(&pasta).unwrap();
        Self(pasta)
    }
    fn perfis(&self) -> PathBuf {
        self.0.join("perfis.toml")
    }
    fn historico(&self) -> PathBuf {
        self.0.join("historico.toml")
    }
    /// Um envio inteiro, do jeito que o comando faz: a escolha é lida quando ele começa.
    fn enviar(&self, ao_comecar: impl FnOnce()) {
        let guardar = ler_escolha(Ok(self.perfis())).unwrap_or(false);
        let diarios = Diarios::default();
        let diario = diarios.abrir_com_escolha(
            cabecalho(false, 1),
            vec!["x".to_owned()],
            guardar,
            Ok(self.historico()),
        );
        ao_comecar();
        Diario::fim_da_parte(&diario, "x", false)(desfecho(
            ResultadoDoEnvio::Concluido,
            1,
            1,
            10,
            10,
        ));
    }
    fn registros(&self) -> usize {
        historico::carregar_historico(&self.historico())
            .unwrap()
            .registros()
            .len()
    }
}

impl Drop for PastaDaEscolha {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn com_a_escolha_ligada_o_envio_entra_na_lista() {
    let pasta = PastaDaEscolha::nova();
    gravar_escolha(Ok(pasta.perfis()), true).unwrap();
    pasta.enviar(|| ());
    assert_eq!(pasta.registros(), 1);
}

#[test]
fn com_a_escolha_desligada_nada_novo_entra_e_a_lista_continua_igual() {
    let pasta = PastaDaEscolha::nova();
    pasta.enviar(|| ());
    let antes = std::fs::read(pasta.historico()).unwrap();
    gravar_escolha(Ok(pasta.perfis()), false).unwrap();
    assert!(!ler_escolha(Ok(pasta.perfis())).unwrap());
    pasta.enviar(|| ());
    assert_eq!(std::fs::read(pasta.historico()).unwrap(), antes);
}

#[test]
fn desligada_sem_lista_nem_chega_a_criar_o_arquivo() {
    let pasta = PastaDaEscolha::nova();
    gravar_escolha(Ok(pasta.perfis()), false).unwrap();
    pasta.enviar(|| ());
    assert!(!pasta.historico().exists());
}

#[test]
fn arquivo_antigo_ou_ausente_vale_como_ligada() {
    let pasta = PastaDaEscolha::nova();
    assert!(ler_escolha(Ok(pasta.perfis())).unwrap());
    std::fs::write(
        pasta.perfis(),
        "versao = 1\n\n[filas.\"x\"]\npassada1_inversa = true\npassada2_inversa = false\nverso_girar_180 = false\nvirar_maco = true\nborda_primeiro = \"curta\"\n",
    )
    .unwrap();
    assert!(ler_escolha(Ok(pasta.perfis())).unwrap());
    pasta.enviar(|| ());
    assert_eq!(pasta.registros(), 1);
}

#[test]
fn o_envio_em_andamento_segue_a_escolha_de_quando_comecou() {
    let pasta = PastaDaEscolha::nova();
    pasta.enviar(|| gravar_escolha(Ok(pasta.perfis()), false).unwrap());
    assert_eq!(pasta.registros(), 1);

    pasta.enviar(|| gravar_escolha(Ok(pasta.perfis()), true).unwrap());
    assert_eq!(pasta.registros(), 1);
}

#[test]
fn com_o_programa_fechando_a_escolha_desligada_tambem_vale() {
    let pasta = PastaDaEscolha::nova();
    let diarios = Diarios::default();
    let diario = diarios.abrir_com_escolha(
        cabecalho(false, 1),
        vec!["x".to_owned()],
        false,
        Ok(pasta.historico()),
    );
    diarios.descarregar();
    assert!(diario.gravado());
    assert!(!pasta.historico().exists());
}

#[test]
fn falha_ao_gravar_a_escolha_nao_muda_o_arquivo_e_avisa() {
    let pasta = PastaDaEscolha::nova();
    std::fs::write(pasta.perfis(), "isto nao e toml [").unwrap();
    assert!(matches!(
        gravar_escolha(Ok(pasta.perfis()), false),
        Err(FalhaHistorico::Gravacao)
    ));
    assert_eq!(
        std::fs::read_to_string(pasta.perfis()).unwrap(),
        "isto nao e toml ["
    );

    // A pasta do arquivo não pode ser criada: um arquivo está no lugar dela.
    let bloqueio = pasta.0.join("ocupado");
    std::fs::write(&bloqueio, "x").unwrap();
    assert!(matches!(
        gravar_escolha(Ok(bloqueio.join("perfis.toml")), false),
        Err(FalhaHistorico::Gravacao)
    ));
    assert!(matches!(
        gravar_escolha(Err(ErroPerfis::PastaUsuarioIndisponivel), false),
        Err(FalhaHistorico::Gravacao)
    ));
}

#[test]
fn sem_conseguir_ler_a_escolha_nenhum_envio_novo_e_registrado() {
    let pasta = PastaDaEscolha::nova();
    std::fs::write(pasta.perfis(), "isto nao e toml [").unwrap();
    assert!(matches!(
        ler_escolha(Ok(pasta.perfis())),
        Err(FalhaHistorico::Leitura)
    ));
    pasta.enviar(|| ());
    assert!(!pasta.historico().exists());
}

#[test]
fn a_comparacao_e_a_lista_olham_so_a_rota_pedida() {
    let mut rapida_em_duas_faces = medido("fila:A", 60_000, 40, ResultadoDoEnvio::Concluido);
    rapida_em_duas_faces.tempos[0].automatico = true;
    let mut outra_em_duas_faces = medido("fila:B", 60_000, 20, ResultadoDoEnvio::Concluido);
    outra_em_duas_faces.tempos[0].automatico = true;
    let manual_igual = medido("fila:A", 60_000, 10, ResultadoDoEnvio::Concluido);
    let manual_da_outra = medido("fila:B", 60_000, 10, ResultadoDoEnvio::Concluido);
    let registros = [
        rapida_em_duas_faces,
        outra_em_duas_faces,
        manual_igual,
        manual_da_outra,
    ];
    let nomes = ids(&["fila:A", "fila:B"]);
    let automatica = comparar_pelos_registros(&registros, &nomes, true).unwrap();
    assert_eq!(automatica.rapida, "fila:A");
    assert!((automatica.razao - 2.0).abs() < 1e-9);
    assert!(comparar_pelos_registros(&registros, &nomes, false).is_none());

    let lista = velocidades_de(&registros);
    assert_eq!(lista.len(), 4);
    assert!(lista
        .iter()
        .any(|item| item.impressora == "fila:A" && item.automatico));
}

#[test]
fn o_registro_guarda_cada_rota_em_uma_medida_e_a_simulada_em_nenhuma() {
    let (diario, registros) = diario(false, 2, &["x", "s"]);
    let medida = |milissegundos, folhas| Medida {
        milissegundos,
        folhas,
    };
    let mut misto = desfecho(ResultadoDoEnvio::Concluido, 2, 2, 4_000, 3);
    misto.medida = medida(4_000, 3);
    misto.medida_automatica = medida(1_000, 10);
    Diario::fim_da_parte(&diario, "x", false)(misto);
    Diario::fim_da_parte(&diario, "s", true)(misto);
    let lido = registros.lock().unwrap();
    let tempos: Vec<_> = lido[0]
        .tempos
        .iter()
        .map(|tempo| (tempo.impressora.as_str(), tempo.automatico, tempo.folhas))
        .collect();
    assert_eq!(tempos, [("x", false, 3), ("x", true, 10)]);
}
