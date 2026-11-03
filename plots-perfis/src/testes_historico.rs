use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::armazenamento::FALHA_NA_GRAVACAO;
use crate::historico::*;

static PROXIMO: AtomicU64 = AtomicU64::new(0);

/// Uma pasta só deste teste, apagada no fim. Nunca o arquivo real do usuário.
struct Pasta(PathBuf);

impl Pasta {
    fn nova() -> Self {
        let numero = PROXIMO.fetch_add(1, Ordering::Relaxed);
        let caminho =
            std::env::temp_dir().join(format!("plots-historico-{}-{numero}", std::process::id()));
        std::fs::create_dir_all(&caminho).unwrap();
        Self(caminho)
    }
    fn arquivo(&self) -> PathBuf {
        self.0.join("historico.toml")
    }
}

impl Drop for Pasta {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn registro(arquivo: &str, data: u64) -> RegistroDeEnvio {
    RegistroDeEnvio {
        arquivo: arquivo.to_owned(),
        data,
        impressoras: vec!["simulada:gaveta".to_owned(), "fila:Sala 2".to_owned()],
        intervalo: Some((3, 8)),
        paginas: 6,
        lotes: 2,
        resultado: ResultadoDoEnvio::Cancelado,
        lotes_faltando: 1,
        tempos: vec![TempoDaImpressora {
            impressora: "fila:Sala 2".to_owned(),
            milissegundos: 90_000,
            folhas: 3,
            resultado: ResultadoDoEnvio::Cancelado,
            automatico: false,
        }],
    }
}

#[test]
fn ida_e_volta_mantem_tudo_e_na_pasta_que_ainda_nao_existe() {
    let pasta = Pasta::nova();
    let destino = pasta.0.join("sub").join("historico.toml");
    let original = registro("contrato.pdf", 1_700_000_000);
    acrescentar_ao_historico(&destino, original.clone()).unwrap();
    let lido = carregar_historico(&destino).unwrap();
    assert_eq!(lido.registros(), [original]);
    assert_eq!(lido.ignorados(), 0);
}

#[test]
fn historico_guarda_somente_nome_sem_pasta() {
    let pasta = Pasta::nova();
    acrescentar_ao_historico(
        &pasta.arquivo(),
        registro("C:\\privado\\pasta/documento.pdf", 1),
    )
    .unwrap();
    let texto = std::fs::read_to_string(pasta.arquivo()).unwrap();
    assert!(!texto.contains("privado"));
    assert_eq!(
        carregar_historico(&pasta.arquivo()).unwrap().registros()[0].arquivo,
        "documento.pdf"
    );
}

#[test]
fn envios_concorrentes_nao_perdem_registros() {
    let pasta = Pasta::nova();
    let caminho = pasta.arquivo();
    std::thread::scope(|escopo| {
        for numero in 0..16 {
            let destino = &caminho;
            escopo.spawn(move || {
                acrescentar_ao_historico(destino, registro("documento.pdf", numero)).unwrap();
            });
        }
    });
    assert_eq!(carregar_historico(&caminho).unwrap().total(), 16);
}

#[test]
fn arquivo_ausente_e_historico_vazio_e_nao_e_criado_ao_ler() {
    let pasta = Pasta::nova();
    assert_eq!(carregar_historico(&pasta.arquivo()).unwrap().total(), 0);
    assert!(!pasta.arquivo().exists());
}

#[test]
fn o_teto_tira_os_mais_antigos() {
    let pasta = Pasta::nova();
    let texto = (0..LIMITE_REGISTROS as u64)
        .map(|data| {
            format!(
                "[[registro]]\narquivo = \"a.pdf\"\ndata = {data}\nimpressoras = []\npaginas = 1\nlotes = 1\nresultado = \"concluido\"\nlotes_faltando = 0\n"
            )
        })
        .collect::<String>();
    std::fs::write(pasta.arquivo(), format!("versao = 1\n{texto}")).unwrap();
    acrescentar_ao_historico(&pasta.arquivo(), registro("novo.pdf", 9_999)).unwrap();
    let lidos = carregar_historico(&pasta.arquivo()).unwrap().registros();
    assert_eq!(lidos.len(), LIMITE_REGISTROS);
    assert_eq!(lidos.first().unwrap().data, 1, "o mais antigo saiu");
    assert_eq!(lidos.last().unwrap().arquivo, "novo.pdf");
}

#[test]
fn arquivo_corrompido_fica_igual_e_a_gravacao_e_recusada() {
    for conteudo in [
        &b"isto = nao e [ toml"[..],
        &b"\xff\xfe\x00"[..],
        &b"versao = 99\n"[..],
        &b"versao = 1\nregistro = 3\n"[..],
    ] {
        let pasta = Pasta::nova();
        std::fs::write(pasta.arquivo(), conteudo).unwrap();
        assert!(matches!(
            carregar_historico(&pasta.arquivo()),
            Err(ErroHistorico::Corrompido)
        ));
        assert!(matches!(
            acrescentar_ao_historico(&pasta.arquivo(), registro("a.pdf", 1)),
            Err(ErroHistorico::Corrompido)
        ));
        assert_eq!(std::fs::read(pasta.arquivo()).unwrap(), conteudo);
    }
}

#[test]
fn registro_ilegivel_fica_de_fora_da_lista_e_volta_ao_gravar() {
    let pasta = Pasta::nova();
    std::fs::write(
        pasta.arquivo(),
        "versao = 1\n[[registro]]\narquivo = \"estranho\"\ndata = -5\nvindo_do_futuro = true\n",
    )
    .unwrap();
    let lido = carregar_historico(&pasta.arquivo()).unwrap();
    assert_eq!((lido.registros().len(), lido.ignorados()), (0, 1));
    acrescentar_ao_historico(&pasta.arquivo(), registro("bom.pdf", 2)).unwrap();
    let depois = carregar_historico(&pasta.arquivo()).unwrap();
    assert_eq!((depois.registros().len(), depois.ignorados()), (1, 1));
    let texto = std::fs::read_to_string(pasta.arquivo()).unwrap();
    assert!(texto.contains("vindo_do_futuro"), "{texto}");
}

#[test]
fn gravacao_que_falha_devolve_erro_e_o_arquivo_fica_como_estava() {
    let pasta = Pasta::nova();
    acrescentar_ao_historico(&pasta.arquivo(), registro("antes.pdf", 1)).unwrap();
    let antes = std::fs::read(pasta.arquivo()).unwrap();
    FALHA_NA_GRAVACAO.with(|falha| falha.set(true));
    assert!(matches!(
        acrescentar_ao_historico(&pasta.arquivo(), registro("depois.pdf", 2)),
        Err(ErroHistorico::Gravacao(_))
    ));
    assert_eq!(std::fs::read(pasta.arquivo()).unwrap(), antes);
    let sobras = std::fs::read_dir(&pasta.0).unwrap().count();
    assert_eq!(sobras, 1, "o temporário não fica para trás");
}

#[test]
fn caminho_que_nao_e_arquivo_devolve_erro_sem_travar() {
    let pasta = Pasta::nova();
    assert!(matches!(
        carregar_historico(&pasta.0),
        Err(ErroHistorico::Leitura(_))
    ));
    assert!(acrescentar_ao_historico(&pasta.0, registro("a.pdf", 1)).is_err());
}

#[test]
fn nome_hostil_vira_texto_limpo_e_limitado() {
    let pasta = Pasta::nova();
    let hostil = format!(
        "<img src=x onerror=alert(1)>\u{202E}\u{200B}\"\n[[registro]]\u{1b}[31m{}",
        "é".repeat(100_000)
    );
    acrescentar_ao_historico(&pasta.arquivo(), registro(&hostil, 1)).unwrap();
    let lidos = carregar_historico(&pasta.arquivo()).unwrap().registros();
    assert_eq!(lidos.len(), 1, "o texto não vira outro registro");
    let nome = &lidos[0].arquivo;
    assert!(
        nome.starts_with("<img src=x onerror=alert(1)>\"[[registro]]"),
        "{nome}"
    );
    assert!(!nome.chars().any(char::is_control));
    assert!(!nome.contains('\u{202E}'));
    assert_eq!(nome.chars().count(), 255);
}

#[test]
fn numeros_grandes_nao_impedem_de_gravar() {
    let pasta = Pasta::nova();
    let mut enorme = registro("a.pdf", u64::MAX);
    enorme.paginas = u64::MAX;
    enorme.tempos[0].milissegundos = u64::MAX;
    acrescentar_ao_historico(&pasta.arquivo(), enorme).unwrap();
    let lidos = carregar_historico(&pasta.arquivo()).unwrap().registros();
    assert_eq!(lidos[0].paginas, i64::MAX.unsigned_abs());
}

#[test]
fn apagar_tira_o_arquivo_e_apagar_de_novo_nao_e_erro() {
    let pasta = Pasta::nova();
    acrescentar_ao_historico(&pasta.arquivo(), registro("a.pdf", 1)).unwrap();
    apagar_historico(&pasta.arquivo()).unwrap();
    assert!(!pasta.arquivo().exists());
    apagar_historico(&pasta.arquivo()).unwrap();
}

#[test]
fn apagar_leva_os_temporarios_do_historico_e_deixa_os_outros_arquivos() {
    let pasta = Pasta::nova();
    acrescentar_ao_historico(&pasta.arquivo(), registro("a.pdf", 1)).unwrap();
    let sobra = pasta.0.join(".historico.toml.77-3.tmp");
    std::fs::write(&sobra, "arquivo = \"segredo.pdf\"").unwrap();
    let vizinhos = [
        pasta.0.join(".perfis.toml.77-3.tmp"),
        pasta.0.join("perfis.toml"),
        pasta.0.join("historico.toml.tmp"),
    ];
    for vizinho in &vizinhos {
        std::fs::write(vizinho, "outro").unwrap();
    }
    apagar_historico(&pasta.arquivo()).unwrap();
    assert!(!pasta.arquivo().exists());
    assert!(!sobra.exists());
    assert!(vizinhos.iter().all(|vizinho| vizinho.exists()));
}

#[test]
fn apagar_sem_a_lista_ainda_leva_o_temporario_que_ficou() {
    let pasta = Pasta::nova();
    let sobra = pasta.0.join(".historico.toml.77-4.tmp");
    std::fs::write(&sobra, "resto").unwrap();
    apagar_historico(&pasta.arquivo()).unwrap();
    assert!(!sobra.exists());
}

#[test]
fn caminho_com_outro_nome_so_e_apagado_se_for_mesmo_um_historico() {
    let pasta = Pasta::nova();
    let alheio = pasta.0.join("relatorio.toml");
    std::fs::write(&alheio, "isto nao e historico [").unwrap();
    assert!(apagar_historico(&alheio).is_err());
    assert_eq!(
        std::fs::read_to_string(&alheio).unwrap(),
        "isto nao e historico ["
    );

    let outro_formato = pasta.0.join("config.toml");
    std::fs::write(
        &outro_formato,
        "nome = \"qualquer\"
",
    )
    .unwrap();
    assert!(apagar_historico(&outro_formato).is_err());
    assert!(outro_formato.exists());

    let de_teste = pasta.0.join("lista-de-teste.toml");
    acrescentar_ao_historico(&de_teste, registro("a.pdf", 1)).unwrap();
    apagar_historico(&de_teste).unwrap();
    assert!(!de_teste.exists());
}

#[test]
fn com_o_nome_certo_um_arquivo_danificado_ainda_pode_ser_apagado() {
    let pasta = Pasta::nova();
    std::fs::write(pasta.arquivo(), "isto nao e toml [").unwrap();
    apagar_historico(&pasta.arquivo()).unwrap();
    assert!(!pasta.arquivo().exists());
}

fn medido(impressora: &str, ms: u64, folhas: u64, resultado: ResultadoDoEnvio) -> RegistroDeEnvio {
    let mut item = registro("a.pdf", 1);
    item.tempos = vec![TempoDaImpressora {
        impressora: impressora.to_owned(),
        milissegundos: ms,
        folhas,
        resultado,
        automatico: false,
    }];
    item
}

#[test]
fn com_menos_de_dez_folhas_nao_ha_numero() {
    use ResultadoDoEnvio::Concluido;
    assert_eq!(velocidade_da_impressora(&[], "a", false), None);
    let nove = [medido("a", 60_000, 9, Concluido)];
    assert_eq!(velocidade_da_impressora(&nove, "a", false), None);
    let dez = [
        medido("a", 30_000, 5, Concluido),
        medido("a", 30_000, 5, Concluido),
    ];
    assert_eq!(velocidade_da_impressora(&dez, "a", false), Some(10.0));
}

#[test]
fn cancelado_com_erro_e_incompleto_nao_entram_na_conta() {
    use ResultadoDoEnvio::{Cancelado, ComErro, Concluido, Incompleto};
    let lista = [
        medido("a", 60_000, 20, Concluido),
        medido("a", 999_999, 50, Cancelado),
        medido("a", 999_999, 50, ComErro),
        medido("a", 999_999, 50, Incompleto),
    ];
    assert_eq!(velocidade_da_impressora(&lista, "a", false), Some(20.0));
    let so_cancelado = [medido("a", 60_000, 200, Cancelado)];
    assert_eq!(velocidade_da_impressora(&so_cancelado, "a", false), None);
}

#[test]
fn a_velocidade_e_de_cada_impressora_e_valor_hostil_nao_estoura() {
    use ResultadoDoEnvio::Concluido;
    let lista = [
        medido("a", 60_000, 20, Concluido),
        medido("b", 60_000, 40, Concluido),
        medido("c", 0, 40, Concluido),
        medido("d", u64::MAX, u64::MAX, Concluido),
        medido("d", u64::MAX, u64::MAX, Concluido),
    ];
    assert_eq!(velocidade_da_impressora(&lista, "a", false), Some(20.0));
    assert_eq!(velocidade_da_impressora(&lista, "b", false), Some(40.0));
    assert_eq!(
        velocidade_da_impressora(&lista, "c", false),
        None,
        "tempo zero"
    );
    assert!(velocidade_da_impressora(&lista, "d", false).is_some());
}

fn duas(a: Option<f64>, b: Option<f64>) -> Option<ComparacaoDeVelocidade> {
    comparar_velocidades(&[("a".to_owned(), a), ("b".to_owned(), b)])
}

#[test]
fn vinte_e_cinco_por_cento_e_o_limite_do_aviso() {
    assert_eq!(duas(Some(100.0), Some(100.0)), None);
    assert_eq!(duas(Some(124.9), Some(100.0)), None);
    let no_limite = duas(Some(125.0), Some(100.0)).unwrap();
    assert_eq!(
        (no_limite.rapida.as_str(), no_limite.lenta.as_str()),
        ("a", "b")
    );
    assert_eq!(no_limite.razao, 1.25);
    let invertido = duas(Some(10.0), Some(20.0)).unwrap();
    assert_eq!(
        (
            invertido.rapida.as_str(),
            invertido.lenta.as_str(),
            invertido.razao
        ),
        ("b", "a", 2.0)
    );
}

#[test]
fn sem_medida_de_todas_nao_ha_comparacao() {
    assert_eq!(duas(Some(100.0), None), None);
    assert_eq!(duas(None, Some(100.0)), None);
    assert_eq!(comparar_velocidades(&[("a".to_owned(), Some(100.0))]), None);
    assert_eq!(comparar_velocidades(&[]), None);
    assert_eq!(duas(Some(f64::NAN), Some(100.0)), None);
    assert_eq!(duas(Some(f64::INFINITY), Some(100.0)), None);
    assert_eq!(duas(Some(0.0), Some(100.0)), None);
    let tres = comparar_velocidades(&[
        ("a".to_owned(), Some(100.0)),
        ("b".to_owned(), Some(300.0)),
        ("c".to_owned(), None),
    ]);
    assert_eq!(tres, None);
}

#[test]
fn com_tres_compara_a_mais_rapida_e_a_mais_lenta() {
    let tres = comparar_velocidades(&[
        ("a".to_owned(), Some(100.0)),
        ("b".to_owned(), Some(300.0)),
        ("c".to_owned(), Some(50.0)),
    ])
    .unwrap();
    assert_eq!(
        (tres.rapida.as_str(), tres.lenta.as_str(), tres.razao),
        ("b", "c", 6.0)
    );
}

#[test]
fn velocidade_nao_mistura_as_rotas_da_mesma_impressora() {
    use ResultadoDoEnvio::Concluido;
    let mut automatico = medido("a", 60_000, 40, Concluido);
    automatico.tempos[0].automatico = true;
    let manual = medido("a", 60_000, 10, Concluido);
    let lista = [automatico, manual];
    assert_eq!(velocidade_da_impressora(&lista, "a", true), Some(40.0));
    assert_eq!(velocidade_da_impressora(&lista, "a", false), Some(10.0));
}

#[test]
fn registro_antigo_sem_a_rota_conta_como_manual() {
    let pasta = Pasta::nova();
    std::fs::write(
        pasta.arquivo(),
        "versao = 1\n\n[[registro]]\narquivo = \"a.pdf\"\ndata = 1\nimpressoras = [\"a\"]\npaginas = 20\nlotes = 1\nresultado = \"concluido\"\nlotes_faltando = 0\n[[registro.tempos]]\nimpressora = \"a\"\nmilissegundos = 60000\nfolhas = 10\nresultado = \"concluido\"\n",
    )
    .unwrap();
    let lidos = carregar_historico(&pasta.arquivo()).unwrap().registros();
    assert_eq!(velocidade_da_impressora(&lidos, "a", false), Some(10.0));
    assert_eq!(velocidade_da_impressora(&lidos, "a", true), None);
}
