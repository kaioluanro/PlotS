use plots_cli::executar;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).into()).collect()
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
    let error = executar(&args(&[
        "simular",
        "arquivo.pdf",
        "--impressora",
        "traseira",
    ]))
    .unwrap_err();
    assert!(error.contains("PDF não é aceito"));
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

#[test]
fn gera_passadas_com_mudanca_de_papel_e_nao_sobrescreve() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CONTADOR: AtomicU64 = AtomicU64::new(0);
    let pasta = std::env::temp_dir().join(format!(
        "plots-cli-{}-{}",
        std::process::id(),
        CONTADOR.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&pasta).unwrap();
    let entrada = pasta.join("entrada.pdf");
    std::fs::write(
        &entrada,
        pdf_de_paginas(&[(595, 842), (595, 842), (420, 595), (420, 595), (420, 595)]),
    )
    .unwrap();
    let argumentos = vec![
        "gerar-pdf".into(),
        entrada.to_string_lossy().into_owned(),
        "--lote".into(),
        "10".into(),
        "--impressora".into(),
        "gaveta".into(),
        "--saida".into(),
        pasta.to_string_lossy().into_owned(),
    ];
    let resultado = executar(&argumentos).unwrap();
    assert!(resultado.contains("4 PDFs"));
    let primeira = lopdf::Document::load(pasta.join("lote-0001-passada-1.pdf")).unwrap();
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
    let ultima = lopdf::Document::load(pasta.join("lote-0002-passada-1.pdf")).unwrap();
    assert_eq!(ultima.get_pages().len(), 2);
    let conteudo_original = std::fs::read(pasta.join("lote-0001-passada-1.pdf")).unwrap();
    assert!(executar(&argumentos).is_err());
    assert_eq!(
        std::fs::read(pasta.join("lote-0001-passada-1.pdf")).unwrap(),
        conteudo_original
    );
    std::fs::remove_dir_all(pasta).unwrap();
}

#[test]
fn rejeita_paginas_de_tamanhos_incompativeis_na_mesma_folha() {
    let pasta = std::env::temp_dir().join(format!("plots-incompativel-{}", std::process::id()));
    std::fs::create_dir_all(&pasta).unwrap();
    let entrada = pasta.join("entrada.pdf");
    std::fs::write(&entrada, pdf_de_paginas(&[(595, 842), (420, 595)])).unwrap();
    let argumentos = vec![
        "gerar-pdf".into(),
        entrada.to_string_lossy().into_owned(),
        "--lote".into(),
        "10".into(),
        "--impressora".into(),
        "gaveta".into(),
        "--saida".into(),
        pasta.to_string_lossy().into_owned(),
    ];
    assert!(executar(&argumentos)
        .unwrap_err()
        .contains("tamanhos incompatíveis"));
    assert!(!pasta.join("lote-0001-passada-1.pdf").exists());
    std::fs::remove_dir_all(pasta).unwrap();
}
