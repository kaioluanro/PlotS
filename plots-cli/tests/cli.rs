use plots_cli::{executar, executar_com_io};
use std::path::{Path, PathBuf};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).into()).collect()
}

/// Pasta temporária removida ao sair de escopo, mesmo quando o teste falha.
struct Pasta(PathBuf);

impl Pasta {
    fn nova(nome: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static CONTADOR: AtomicU64 = AtomicU64::new(0);
        let caminho = std::env::temp_dir().join(format!(
            "plots-cli-{nome}-{}-{}",
            std::process::id(),
            CONTADOR.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&caminho).unwrap();
        Self(caminho)
    }

    fn caminho(&self) -> &Path {
        &self.0
    }
}

impl Drop for Pasta {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn argumentos_gerar(entrada: &Path, saida: &Path, lote: &str) -> Vec<String> {
    vec![
        "gerar-pdf".into(),
        entrada.to_string_lossy().into_owned(),
        "--lote".into(),
        lote.into(),
        "--impressora".into(),
        "gaveta".into(),
        "--saida".into(),
        saida.to_string_lossy().into_owned(),
    ]
}

fn executar_binario(argumentos: &[&str]) -> std::process::Output {
    use std::process::{Command, Stdio};
    Command::new(env!("CARGO_BIN_EXE_plots-cli"))
        .args(argumentos)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn simula_duas_impressoras_virtuais_e_exibe_ordem_e_folhas() {
    for (printer, front_order) in [("traseira", "4, 2"), ("gaveta", "2, 4")] {
        let output = executar(&args(&[
            "simular",
            "--paginas",
            "4",
            "--lote",
            "2",
            "--impressora",
            printer,
        ]))
        .unwrap();
        assert!(output.contains("SIMULAÇÃO"));
        assert!(output.contains(&format!("Passada 1: {front_order}")));
        assert!(output.contains("Passada 2:"));
        assert!(output.contains("Folhas finais"));
        assert!(output.contains("1|2"));
        assert!(output.contains("3|4"));
    }
}

#[test]
fn trata_pagina_impar_e_divisao_em_lotes() {
    let output = executar(&args(&[
        "simular",
        "--paginas",
        "5",
        "--lote",
        "2",
        "--impressora",
        "gaveta",
    ]))
    .unwrap();
    assert!(output.contains("2 lote(s)"));
    assert!(output.contains("branca"));
}

#[test]
fn rejeita_pdf_com_mensagem_explicita() {
    let esperado = "'simular' não lê PDF: informe --paginas N, ou use 'gerar-pdf' para gerar as passadas de um arquivo";
    assert_eq!(
        executar(&args(&[
            "simular",
            "arquivo.pdf",
            "--impressora",
            "traseira",
        ]))
        .unwrap_err(),
        esperado
    );
    assert_eq!(
        executar(&args(&[
            "simular",
            "--pdf",
            "--paginas",
            "2",
            "--lote",
            "1",
            "--impressora",
            "gaveta",
        ]))
        .unwrap_err(),
        esperado
    );
    assert_eq!(
        executar(&args(
            &["simular", "ARQUIVO.PDF", "--impressora", "gaveta",]
        ))
        .unwrap_err(),
        esperado
    );
}

#[test]
fn erros_de_argumentos_sao_claros() {
    assert!(executar(&args(&[
        "simular",
        "--paginas",
        "x",
        "--lote",
        "2",
        "--impressora",
        "gaveta"
    ]))
    .unwrap_err()
    .contains("inteiro"));
    assert!(executar(&args(&[
        "simular",
        "--paginas",
        "2",
        "--lote",
        "0",
        "--impressora",
        "gaveta"
    ]))
    .unwrap_err()
    .contains("lote"));
    assert!(executar(&args(&[
        "simular",
        "--paginas",
        "2",
        "--lote",
        "1",
        "--impressora",
        "laser"
    ]))
    .unwrap_err()
    .contains("desconhecida"));
}

#[test]
fn guiado_espera_confirmacao_por_lote_nas_duas_impressoras() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    for printer in ["traseira", "gaveta"] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_plots-cli"))
            .args([
                "simular",
                "--paginas",
                "4",
                "--lote",
                "1",
                "--impressora",
                printer,
                "--guiado",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(b"\n\n").unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert_eq!(text.matches("pressione Enter").count(), 2);
        assert!(text.contains("1|2"));
        assert!(text.contains("3|4"));
    }
}

#[test]
fn guiado_cancela_sem_enviar_verso_quando_entrada_termina() {
    use std::process::{Command, Stdio};
    let output = Command::new(env!("CARGO_BIN_EXE_plots-cli"))
        .args([
            "simular",
            "--paginas",
            "4",
            "--lote",
            "1",
            "--impressora",
            "gaveta",
            "--guiado",
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Passada 1:"));
    assert!(!text.contains("Passada 2:"));
    assert!(!text.contains("Lote 2:"));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("cancelada"));
}

#[test]
fn guiado_cancela_com_entrada_parcial_sem_enter() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_plots-cli"))
        .args([
            "simular",
            "--paginas",
            "2",
            "--lote",
            "1",
            "--impressora",
            "gaveta",
            "--guiado",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"parcial").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Passada 1:"));
    assert!(!stdout.contains("Passada 2:"));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("cancelada"));
}

fn pdf_de_paginas(tamanhos: &[(i64, i64)]) -> Vec<u8> {
    use lopdf::{dictionary, Document, Object};
    let mut doc = Document::with_version("1.5");
    let raiz = doc.new_object_id();
    let mut filhos = Vec::new();
    for (i, (largura, altura)) in tamanhos.iter().enumerate() {
        let pagina = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => raiz, "MediaBox" => vec![0.into(), 0.into(), (*largura).into(), (*altura).into()],
            "Resources" => dictionary! {}, "Marker" => (i + 1) as i64
        });
        filhos.push(Object::Reference(pagina));
    }
    doc.objects.insert(
        raiz,
        Object::Dictionary(
            dictionary! {"Type" => "Pages", "Count" => filhos.len() as i64, "Kids" => filhos},
        ),
    );
    let catalogo = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => raiz});
    doc.trailer.set("Root", catalogo);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

fn pdf_criptografado() -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(&pdf_de_paginas(&[(595, 842)])).unwrap();
    doc.trailer.set(
        "ID",
        vec![
            lopdf::Object::string_literal("id-de-teste"),
            lopdf::Object::string_literal("id-de-teste"),
        ],
    );
    let versao = lopdf::EncryptionVersion::V1 {
        document: &doc,
        owner_password: "dono",
        user_password: "segredo",
        permissions: lopdf::Permissions::default(),
    };
    let estado = lopdf::EncryptionState::try_from(versao).unwrap();
    doc.encrypt(&estado).unwrap();
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn gera_passadas_com_mudanca_de_papel_e_nao_sobrescreve() {
    let pasta = Pasta::nova("papel");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(
        &entrada,
        pdf_de_paginas(&[(595, 842), (595, 842), (420, 595), (420, 595), (420, 595)]),
    )
    .unwrap();
    let argumentos = argumentos_gerar(&entrada, pasta.caminho(), "10");
    let resultado = executar(&argumentos).unwrap();
    assert!(resultado.contains("4 PDFs"));
    let primeira = lopdf::Document::load(pasta.caminho().join("lote-0001-passada-1.pdf")).unwrap();
    let pagina = *primeira.get_pages().values().next().unwrap();
    assert_eq!(
        primeira
            .get_object(pagina)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Marker")
            .unwrap()
            .as_i64()
            .unwrap(),
        2
    );
    let ultima = lopdf::Document::load(pasta.caminho().join("lote-0002-passada-1.pdf")).unwrap();
    assert_eq!(ultima.get_pages().len(), 2);
    let conteudo_original = std::fs::read(pasta.caminho().join("lote-0001-passada-1.pdf")).unwrap();
    assert!(executar(&argumentos).is_err());
    assert_eq!(
        std::fs::read(pasta.caminho().join("lote-0001-passada-1.pdf")).unwrap(),
        conteudo_original
    );
}

#[test]
fn rejeita_paginas_de_tamanhos_incompativeis_na_mesma_folha() {
    let pasta = Pasta::nova("incompativel");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_de_paginas(&[(595, 842), (420, 595)])).unwrap();
    let argumentos = argumentos_gerar(&entrada, pasta.caminho(), "10");
    assert!(executar(&argumentos)
        .unwrap_err()
        .contains("tamanhos incompatíveis"));
    assert!(!pasta.caminho().join("lote-0001-passada-1.pdf").exists());
}

const AVISO_FOLHA_1_2: &str = "Aviso: a folha das páginas 1 e 2 mistura retrato e paisagem; confira o resultado antes de imprimir.";

#[test]
fn gerar_pdf_avisa_folha_com_retrato_e_paisagem_e_continua() {
    let pasta = Pasta::nova("mista");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_de_paginas(&[(595, 842), (842, 595)])).unwrap();
    let saida = executar(&argumentos_gerar(&entrada, pasta.caminho(), "10")).unwrap();
    let posicao_aviso = saida.find(AVISO_FOLHA_1_2).expect("aviso ausente");
    let posicao_resumo = saida.find("Gerados 2 PDFs").expect("resumo ausente");
    assert!(posicao_aviso < posicao_resumo);
    assert_eq!(saida.matches("Aviso:").count(), 1);
    assert!(pasta.caminho().join("lote-0001-passada-1.pdf").is_file());
    assert!(pasta.caminho().join("lote-0001-passada-2.pdf").is_file());
}

#[test]
fn gerar_pdf_avisa_somente_a_folha_misturada() {
    let pasta = Pasta::nova("varias");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(
        &entrada,
        pdf_de_paginas(&[(595, 842), (595, 842), (595, 842), (842, 595)]),
    )
    .unwrap();
    let saida = executar(&argumentos_gerar(&entrada, pasta.caminho(), "10")).unwrap();
    assert_eq!(saida.matches("Aviso:").count(), 1);
    assert!(saida.contains(
        "Aviso: a folha das páginas 3 e 4 mistura retrato e paisagem; confira o resultado antes de imprimir."
    ));
}

#[test]
fn gerar_pdf_sem_mistura_de_orientacao_nao_mostra_aviso() {
    let pasta = Pasta::nova("sem-mistura");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(
        &entrada,
        pdf_de_paginas(&[(595, 842), (595, 842), (842, 595), (842, 595)]),
    )
    .unwrap();
    let saida = executar(&argumentos_gerar(&entrada, pasta.caminho(), "10")).unwrap();
    assert!(!saida.contains("Aviso"));
    assert!(saida.contains("Gerados 4 PDFs"));
}

#[test]
fn gerar_pdf_nao_avisa_pagina_quadrada_nem_pagina_sem_verso() {
    let quadrada_pasta = Pasta::nova("quadrada");
    let quadrada = quadrada_pasta.caminho().join("quadrada.pdf");
    std::fs::write(&quadrada, pdf_de_paginas(&[(300, 300), (300, 300)])).unwrap();
    let saida = executar(&argumentos_gerar(&quadrada, quadrada_pasta.caminho(), "10")).unwrap();
    assert!(!saida.contains("Aviso"));

    let unica_pasta = Pasta::nova("unica");
    let unica = unica_pasta.caminho().join("unica.pdf");
    std::fs::write(&unica, pdf_de_paginas(&[(595, 842)])).unwrap();
    let saida = executar(&argumentos_gerar(&unica, unica_pasta.caminho(), "10")).unwrap();
    assert!(!saida.contains("Aviso"));
    assert!(saida.contains("Gerados 2 PDFs"));
}

#[test]
fn comandos_ainda_nao_disponiveis_dizem_o_que_usar() {
    for comando in ["imprimir", "calibrar"] {
        let esperado = format!(
            "o comando '{comando}' ainda não está disponível; use 'simular' ou 'gerar-pdf'"
        );
        assert_eq!(executar(&args(&[comando])).unwrap_err(), esperado);
        assert_eq!(
            executar(&args(&[comando, "arquivo.pdf", "--impressora", "traseira"])).unwrap_err(),
            esperado
        );
    }
}

#[test]
fn comando_ausente_ou_desconhecido_lista_os_comandos_validos() {
    let esperado = "comando ausente ou desconhecido; use 'simular' ou 'gerar-pdf'";
    assert_eq!(executar(&args(&[])).unwrap_err(), esperado);
    assert_eq!(executar(&args(&["abrir"])).unwrap_err(), esperado);
    assert_eq!(executar(&args(&["arquivo.pdf"])).unwrap_err(), esperado);
    assert_eq!(
        executar(&args(&["abrir", "arquivo.pdf"])).unwrap_err(),
        esperado
    );
}

#[test]
fn ajuda_aparece_como_primeiro_argumento_e_depois_do_subcomando() {
    for ajuda in ["--help", "-h"] {
        let geral = executar(&args(&[ajuda])).unwrap();
        assert!(geral.contains("plots simular"));
        assert!(geral.contains("plots gerar-pdf"));
        assert!(geral.contains("Nenhum comando envia arquivos à impressora"));

        let simular = executar(&args(&["simular", ajuda])).unwrap();
        assert!(simular.contains("plots simular"));
        assert!(!simular.contains("plots gerar-pdf"));

        let gerar = executar(&args(&["gerar-pdf", ajuda])).unwrap();
        assert!(gerar.contains("plots gerar-pdf"));
        assert!(!gerar.contains("plots simular"));
    }
}

#[test]
fn ajuda_pela_linha_de_comando_sai_com_codigo_zero() {
    let saida = executar_binario(&["simular", "--help"]);
    assert_eq!(saida.status.code(), Some(0));
    assert!(String::from_utf8(saida.stdout)
        .unwrap()
        .contains("plots simular"));
    assert!(saida.stderr.is_empty());

    let saida = executar_binario(&["gerar-pdf", "-h"]);
    assert_eq!(saida.status.code(), Some(0));
    assert!(String::from_utf8(saida.stdout)
        .unwrap()
        .contains("plots gerar-pdf"));
}

#[test]
fn erro_mostra_somente_o_uso_do_comando_pedido() {
    let simular = executar_binario(&[
        "simular",
        "--paginas",
        "x",
        "--lote",
        "2",
        "--impressora",
        "gaveta",
    ]);
    assert_eq!(simular.status.code(), Some(2));
    let erro = String::from_utf8(simular.stderr).unwrap();
    assert!(erro.contains("plots simular"));
    assert!(!erro.contains("plots gerar-pdf"));

    let gerar = executar_binario(&["gerar-pdf", "arquivo.pdf"]);
    assert_eq!(gerar.status.code(), Some(2));
    let erro = String::from_utf8(gerar.stderr).unwrap();
    assert!(erro.contains("plots gerar-pdf"));
    assert!(!erro.contains("plots simular"));

    let geral = executar_binario(&["abrir"]);
    assert_eq!(geral.status.code(), Some(2));
    let erro = String::from_utf8(geral.stderr).unwrap();
    assert!(erro.contains("plots simular"));
    assert!(erro.contains("plots gerar-pdf"));
}

#[test]
fn gerar_pdf_rejeita_opcao_repetida_sem_criar_arquivos() {
    let pasta = Pasta::nova("repetida");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_de_paginas(&[(595, 842), (595, 842)])).unwrap();
    let mut argumentos = argumentos_gerar(&entrada, pasta.caminho(), "10");
    argumentos.extend(["--lote".into(), "5".into()]);
    assert!(executar(&argumentos).unwrap_err().contains("repetida"));
    assert!(!pasta.caminho().join("lote-0001-passada-1.pdf").exists());
}

#[test]
fn gerar_pdf_recusa_pdf_criptografado_sem_criar_arquivos() {
    let pasta = Pasta::nova("criptografado");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_criptografado()).unwrap();
    let erro = executar(&argumentos_gerar(&entrada, pasta.caminho(), "10")).unwrap_err();
    assert!(erro.contains("criptografado"));
    assert_eq!(std::fs::read_dir(pasta.caminho()).unwrap().count(), 1);
}

#[test]
fn executar_com_io_aceita_entrada_vazia_em_ajuda() {
    let mut saida = Vec::new();
    let mut entrada = std::io::empty();
    executar_com_io(&args(&["--help"]), &mut entrada, &mut saida).unwrap();
    assert!(String::from_utf8(saida)
        .unwrap()
        .contains("plots gerar-pdf"));
}
