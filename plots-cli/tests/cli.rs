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
fn comando_ausente_ou_desconhecido_lista_os_comandos_validos() {
    let esperado =
        "comando ausente ou desconhecido; use 'simular', 'gerar-pdf', 'gerar-calibracao', 'imprimir', 'calibrar' ou 'filas'";
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
fn filas_mostra_ajuda_propria_sem_listar_as_filas() {
    for ajuda in ["--help", "-h"] {
        let geral = executar(&args(&["filas", ajuda])).unwrap();
        assert!(geral.contains("plots filas"));
        assert!(!geral.contains("plots simular"));
        assert!(geral.contains("nada é enviado"));
    }
}

#[test]
fn filas_recusa_opcao_desconhecida_e_valor_faltando() {
    assert_eq!(
        executar(&args(&["filas", "--perfis"])).unwrap_err(),
        "faltou valor para --perfis"
    );
    assert!(executar(&args(&["filas", "--enviar", "x"]))
        .unwrap_err()
        .starts_with("opção desconhecida"));
}

#[cfg(windows)]
#[test]
fn filas_lista_sem_gravar_perfis_e_diz_que_nada_foi_enviado() {
    let pasta = Pasta::nova("filas");
    let perfis = pasta.caminho().join("perfis.toml");
    let saida = executar(&args(&["filas", "--perfis", &perfis.to_string_lossy()])).unwrap();
    assert!(saida.contains("nada foi enviado a nenhuma impressora"));
    assert!(saida.contains("Impressoras simuladas"));
    assert!(!perfis.exists());
}

#[cfg(not(windows))]
#[test]
fn filas_fora_do_windows_recusa_com_mensagem_clara() {
    let pasta = Pasta::nova("filas-sem-windows");
    let perfis = pasta.caminho().join("perfis.toml");
    let erro = executar(&args(&["filas", "--perfis", &perfis.to_string_lossy()])).unwrap_err();
    assert!(erro.contains("só está disponível no Windows"));
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

fn caminho_texto(caminho: &Path) -> String {
    caminho.to_string_lossy().into_owned()
}

fn caixa_de_pagina(doc: &lopdf::Document, pagina: lopdf::ObjectId) -> Vec<f32> {
    doc.get_object(pagina)
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"MediaBox")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|valor| valor.as_float().unwrap())
        .collect()
}

#[test]
fn gerar_calibracao_grava_quatro_paginas_no_papel_padrao() {
    let pasta = Pasta::nova("calibracao-padrao");
    let saida = executar(&args(&[
        "gerar-calibracao",
        "--saida",
        &caminho_texto(pasta.caminho()),
    ]))
    .unwrap();
    assert!(saida.contains("Nenhum arquivo foi enviado à impressora"));
    let doc = lopdf::Document::load(pasta.caminho().join("calibracao.pdf")).unwrap();
    let paginas: Vec<_> = doc.get_pages().into_values().collect();
    assert_eq!(paginas.len(), 4);
    for pagina in &paginas {
        assert_eq!(caixa_de_pagina(&doc, *pagina), vec![0.0, 0.0, 595.0, 842.0]);
    }
    let conteudos: Vec<Vec<u8>> = paginas
        .iter()
        .map(|pagina| doc.get_page_content(*pagina))
        .collect();
    for i in 0..conteudos.len() {
        for j in (i + 1)..conteudos.len() {
            assert_ne!(conteudos[i], conteudos[j]);
        }
    }
}

#[test]
fn gerar_calibracao_aceita_cada_papel_com_sua_caixa() {
    for (papel, largura, altura) in [
        ("a4", 595.0, 842.0),
        ("a5", 420.0, 595.0),
        ("carta", 612.0, 792.0),
        ("oficio", 612.0, 936.0),
        ("legal", 612.0, 1008.0),
    ] {
        let pasta = Pasta::nova(papel);
        executar(&args(&[
            "gerar-calibracao",
            "--saida",
            &caminho_texto(pasta.caminho()),
            "--papel",
            papel,
        ]))
        .unwrap();
        let doc = lopdf::Document::load(pasta.caminho().join("calibracao.pdf")).unwrap();
        let paginas: Vec<_> = doc.get_pages().into_values().collect();
        assert_eq!(paginas.len(), 4, "papel {papel}");
        for pagina in paginas {
            assert_eq!(
                caixa_de_pagina(&doc, pagina),
                vec![0.0, 0.0, largura, altura],
                "papel {papel}"
            );
        }
    }
}

#[test]
fn gerar_calibracao_nao_sobrescreve_arquivo_existente() {
    let pasta = Pasta::nova("calibracao-existente");
    let saida = caminho_texto(pasta.caminho());
    executar(&args(&["gerar-calibracao", "--saida", &saida])).unwrap();
    let arquivo = pasta.caminho().join("calibracao.pdf");
    let conteudo = std::fs::read(&arquivo).unwrap();
    let erro = executar(&args(&["gerar-calibracao", "--saida", &saida])).unwrap_err();
    assert!(erro.contains("já existe"));
    assert_eq!(std::fs::read(&arquivo).unwrap(), conteudo);
}

#[test]
fn gerar_calibracao_rejeita_entradas_invalidas_sem_criar_arquivo() {
    let pasta = Pasta::nova("calibracao-invalida");
    let saida = caminho_texto(pasta.caminho());
    let erro = executar(&args(&[
        "gerar-calibracao",
        "--saida",
        &saida,
        "--papel",
        "tabloide",
    ]))
    .unwrap_err();
    assert!(erro.contains("papel desconhecido"));
    let erro = executar(&args(&[
        "gerar-calibracao",
        "--saida",
        &saida,
        "--saida",
        &saida,
    ]))
    .unwrap_err();
    assert!(erro.contains("repetida"));
    let erro = executar(&args(&["gerar-calibracao"])).unwrap_err();
    assert!(erro.contains("--saida"));
    let inexistente = pasta.caminho().join("nao-existe");
    let erro = executar(&args(&[
        "gerar-calibracao",
        "--saida",
        &caminho_texto(&inexistente),
    ]))
    .unwrap_err();
    assert!(erro.contains("diretório"));
    assert_eq!(std::fs::read_dir(pasta.caminho()).unwrap().count(), 0);
}

#[test]
fn ordem_impares_inverte_as_passadas_e_mantem_folhas_certas() {
    for (impressora, frente) in [("traseira", "3, 1"), ("gaveta", "1, 3")] {
        let saida = executar(&args(&[
            "simular",
            "--paginas",
            "4",
            "--lote",
            "2",
            "--impressora",
            impressora,
            "--ordem",
            "impares",
        ]))
        .unwrap();
        assert!(saida.contains("Ordem das passadas: ímpares primeiro"));
        assert!(saida.contains(&format!("Passada 1: {frente}")));
        assert!(saida.contains("1|2: certa"));
        assert!(saida.contains("3|4: certa"));
        assert!(saida.contains("2 de 2 folhas corretas"));
    }
}

#[test]
fn ordem_padrao_nao_mostra_aviso_de_ordem() {
    let saida = executar(&args(&[
        "simular",
        "--paginas",
        "4",
        "--lote",
        "2",
        "--impressora",
        "gaveta",
        "--ordem",
        "pares",
    ]))
    .unwrap();
    assert!(!saida.contains("Ordem das passadas"));
    assert!(saida.contains("Passada 1: 2, 4"));
}

#[test]
fn ordem_invalida_ou_repetida_da_erro_claro_em_simular_e_gerar_pdf() {
    let erro = executar(&args(&[
        "simular",
        "--paginas",
        "2",
        "--lote",
        "1",
        "--impressora",
        "gaveta",
        "--ordem",
        "meio",
    ]))
    .unwrap_err();
    assert!(erro.contains("--ordem deve ser pares ou impares"));
    let erro = executar(&args(&[
        "simular",
        "--paginas",
        "2",
        "--lote",
        "1",
        "--impressora",
        "gaveta",
        "--ordem",
        "pares",
        "--ordem",
        "impares",
    ]))
    .unwrap_err();
    assert!(erro.contains("--ordem foi informado mais de uma vez"));

    let pasta = Pasta::nova("ordem-invalida");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_de_paginas(&[(595, 842), (595, 842)])).unwrap();
    let mut invalida = argumentos_gerar(&entrada, pasta.caminho(), "10");
    invalida.extend(["--ordem".into(), "meio".into()]);
    assert!(executar(&invalida)
        .unwrap_err()
        .contains("--ordem deve ser pares ou impares"));
    let mut repetida = argumentos_gerar(&entrada, pasta.caminho(), "10");
    repetida.extend([
        "--ordem".into(),
        "pares".into(),
        "--ordem".into(),
        "pares".into(),
    ]);
    assert!(executar(&repetida)
        .unwrap_err()
        .contains("--ordem foi informado mais de uma vez"));
    assert!(!pasta.caminho().join("lote-0001-passada-1.pdf").exists());
}

#[test]
fn gerar_pdf_com_ordem_impares_grava_passadas_trocadas() {
    let pasta = Pasta::nova("gerar-impares");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(
        &entrada,
        pdf_de_paginas(&[(595, 842), (595, 842), (595, 842), (595, 842)]),
    )
    .unwrap();
    let mut argumentos = argumentos_gerar(&entrada, pasta.caminho(), "2");
    argumentos.extend(["--ordem".into(), "impares".into()]);
    let saida = executar(&argumentos).unwrap();
    assert!(saida.contains("Ordem das passadas: ímpares primeiro"));
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
        1
    );
    let segunda = lopdf::Document::load(pasta.caminho().join("lote-0001-passada-2.pdf")).unwrap();
    let pagina = *segunda.get_pages().values().next().unwrap();
    assert_eq!(
        segunda
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
}

#[test]
fn gesto_sem_virar_marca_folhas_erradas_e_sai_com_codigo_zero() {
    let saida = executar_binario(&[
        "simular",
        "--paginas",
        "4",
        "--lote",
        "2",
        "--impressora",
        "traseira",
        "--simular-erro",
        "sem-virar",
    ]);
    assert_eq!(saida.status.code(), Some(0));
    let texto = String::from_utf8(saida.stdout).unwrap();
    assert!(texto.contains("Gesto errado simulado: sem-virar"));
    assert!(texto.contains("0 de 2 folhas corretas"));
    assert_eq!(texto.matches(": errada").count(), 2);
    assert_eq!(texto.matches(": certa").count(), 0);
}

#[test]
fn gesto_girado_marca_verso_de_cabeca_para_baixo_nas_duas_impressoras() {
    for impressora in ["traseira", "gaveta"] {
        let saida = executar_binario(&[
            "simular",
            "--paginas",
            "4",
            "--lote",
            "2",
            "--impressora",
            impressora,
            "--simular-erro",
            "girado",
        ]);
        assert_eq!(saida.status.code(), Some(0));
        let texto = String::from_utf8(saida.stdout).unwrap();
        assert!(texto.contains("Gesto errado simulado: girado"));
        assert!(texto.contains("0 de 2 folhas corretas"));
        assert_eq!(
            texto.matches("o verso está de cabeça para baixo").count(),
            2
        );
    }
}

#[test]
fn gesto_com_folha_a_menos_mostra_erro_e_reimprime_o_lote() {
    let saida = executar_binario(&[
        "simular",
        "--paginas",
        "4",
        "--lote",
        "2",
        "--impressora",
        "gaveta",
        "--simular-erro",
        "folha-a-menos",
    ]);
    assert_eq!(saida.status.code(), Some(0));
    let texto = String::from_utf8(saida.stdout).unwrap();
    assert!(texto.contains("Gesto errado simulado: folha-a-menos"));
    assert!(texto.contains("Erro no lote: quantidade de folhas: necessárias 2, disponíveis 1"));
    assert!(texto.contains("Reimprimindo o lote desde a frente"));
    assert_eq!(texto.matches("Passada 1:").count(), 2);
    assert!(texto.contains("2 de 2 folhas corretas"));
    assert!(texto.contains("1|2: certa"));
    assert!(texto.contains("3|4: certa"));
}

#[test]
fn simular_correto_diz_certa_por_folha_e_resultado_do_lote() {
    let saida = executar_binario(&[
        "simular",
        "--paginas",
        "5",
        "--lote",
        "2",
        "--impressora",
        "traseira",
    ]);
    assert_eq!(saida.status.code(), Some(0));
    let texto = String::from_utf8(saida.stdout).unwrap();
    assert!(texto.contains("1|2: certa"));
    assert!(texto.contains("3|4: certa"));
    assert!(texto.contains("5|branca: certa"));
    assert!(texto.contains("Resultado do lote: 2 de 2 folhas corretas"));
    assert!(texto.contains("Resultado do lote: 1 de 1 folhas corretas"));
    assert!(!texto.contains("errada"));
}

#[test]
fn simular_erro_rejeita_valor_invalido_ou_repetido() {
    let erro = executar(&args(&[
        "simular",
        "--paginas",
        "2",
        "--lote",
        "1",
        "--impressora",
        "gaveta",
        "--simular-erro",
        "torto",
    ]))
    .unwrap_err();
    assert!(erro.contains("--simular-erro deve ser sem-virar, girado ou folha-a-menos"));
    let erro = executar(&args(&[
        "simular",
        "--paginas",
        "2",
        "--lote",
        "1",
        "--impressora",
        "gaveta",
        "--simular-erro",
        "girado",
        "--simular-erro",
        "girado",
    ]))
    .unwrap_err();
    assert!(erro.contains("--simular-erro foi informado mais de uma vez"));
}

#[test]
fn ajuda_mostra_opcoes_novas_e_o_comando_de_calibracao() {
    let geral = executar(&args(&["--help"])).unwrap();
    assert!(geral.contains("gerar-calibracao"));
    assert!(geral.contains("--ordem pares|impares"));
    assert!(geral.contains("--simular-erro sem-virar|girado|folha-a-menos"));
    assert!(geral.contains("--papel a4|a5|carta|oficio|legal"));

    let simular = executar(&args(&["simular", "--help"])).unwrap();
    assert!(simular.contains("--ordem pares|impares"));
    assert!(simular.contains("folha-a-menos"));

    let gerar = executar(&args(&["gerar-pdf", "--help"])).unwrap();
    assert!(gerar.contains("--ordem pares|impares"));

    let calibracao = executar(&args(&["gerar-calibracao", "--help"])).unwrap();
    assert!(calibracao.contains("plots gerar-calibracao"));
    assert!(calibracao.contains("--papel a4|a5|carta|oficio|legal"));
    assert!(!calibracao.contains("plots simular"));
    assert!(!calibracao.contains("plots gerar-pdf"));
}

#[test]
fn erro_de_gerar_calibracao_mostra_somente_o_proprio_uso() {
    let saida = executar_binario(&["gerar-calibracao", "--papel", "tabloide"]);
    assert_eq!(saida.status.code(), Some(2));
    let erro = String::from_utf8(saida.stderr).unwrap();
    assert!(erro.contains("plots gerar-calibracao"));
    assert!(!erro.contains("plots simular"));
    assert!(!erro.contains("plots gerar-pdf"));
}

/// PDF com uma página por tamanho dado; cada conteúdo traz "% pagina N" com o número da página.
fn pdf_com_conteudo(tamanhos: &[(i64, i64)]) -> Vec<u8> {
    use lopdf::{dictionary, Document, Object, Stream};
    let mut doc = Document::with_version("1.5");
    let raiz = doc.new_object_id();
    let mut filhos = Vec::new();
    for (i, (largura, altura)) in tamanhos.iter().enumerate() {
        let conteudo = doc.add_object(Stream::new(
            dictionary! {},
            format!("% pagina {}\n", i + 1).into_bytes(),
        ));
        let pagina = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => raiz,
            "MediaBox" => vec![0.into(), 0.into(), (*largura).into(), (*altura).into()],
            "Resources" => dictionary! {}, "Contents" => conteudo,
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

fn argumentos_folhas(
    entrada: &Path,
    saida: &Path,
    impressora: &str,
    ordem: Option<&str>,
) -> Vec<String> {
    let mut argumentos = vec![
        "gerar-pdf".to_string(),
        caminho_texto(entrada),
        "--lote".into(),
        "10".into(),
        "--impressora".into(),
        impressora.into(),
        "--saida".into(),
        caminho_texto(saida),
        "--folhas-finais".into(),
    ];
    if let Some(ordem) = ordem {
        argumentos.extend(["--ordem".to_string(), ordem.to_string()]);
    }
    argumentos
}

/// Face de uma folha final: número da página de origem e matriz de posicionamento.
#[derive(Debug)]
struct Face {
    pagina: usize,
    matriz: Vec<f32>,
}

/// Folha final lida do arquivo; uma face ausente é uma face em branco.
#[derive(Debug)]
struct FolhaLida {
    caixa: Vec<f32>,
    frente: Option<Face>,
    verso: Option<Face>,
}

fn ler_folhas(caminho: &Path) -> Vec<FolhaLida> {
    let doc = lopdf::Document::load(caminho).unwrap();
    doc.get_pages()
        .values()
        .map(|&id| {
            let pagina = doc.get_object(id).unwrap().as_dict().unwrap();
            let caixa = pagina
                .get(b"MediaBox")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_float().unwrap())
                .collect();
            let recursos = doc
                .dereference(pagina.get(b"Resources").unwrap())
                .unwrap()
                .1
                .as_dict()
                .unwrap();
            let xobjetos = match recursos.get(b"XObject") {
                Ok(valor) => doc.dereference(valor).unwrap().1.as_dict().unwrap().clone(),
                Err(_) => lopdf::Dictionary::new(),
            };
            let ler = |nome: &[u8]| -> Option<Face> {
                let referencia = xobjetos.get(nome).ok()?.as_reference().ok()?;
                let forma = doc.get_object(referencia).ok()?.as_stream().ok()?;
                let texto = String::from_utf8(forma.decompressed_content().ok()?).ok()?;
                let pagina = texto.trim().strip_prefix("% pagina ")?.parse().ok()?;
                let matriz = forma
                    .dict
                    .get(b"Matrix")
                    .ok()?
                    .as_array()
                    .ok()?
                    .iter()
                    .map(|v| v.as_float().unwrap())
                    .collect();
                Some(Face { pagina, matriz })
            };
            FolhaLida {
                caixa,
                frente: ler(b"Frente"),
                verso: ler(b"Verso"),
            }
        })
        .collect()
}

/// Rótulo "esquerda|direita" da folha como está no arquivo; a face em branco sai como "branca".
fn rotulo_lado_a_lado(folha: &FolhaLida) -> String {
    [&folha.frente, &folha.verso]
        .iter()
        .map(|face| {
            face.as_ref()
                .map_or("branca".to_string(), |f| f.pagina.to_string())
        })
        .collect::<Vec<_>>()
        .join("|")
}

#[test]
fn folhas_finais_seguem_a_ordem_do_simulador_nas_duas_impressoras() {
    for total in [4usize, 5] {
        for impressora in ["traseira", "gaveta"] {
            for ordem in [None, Some("impares")] {
                let pasta = Pasta::nova("folhas-ordem");
                let entrada = pasta.caminho().join("entrada.pdf");
                std::fs::write(&entrada, pdf_com_conteudo(&vec![(595, 842); total])).unwrap();
                executar(&argumentos_folhas(
                    &entrada,
                    pasta.caminho(),
                    impressora,
                    ordem,
                ))
                .unwrap();
                let mut simular = vec![
                    "simular".to_string(),
                    "--paginas".into(),
                    total.to_string(),
                    "--lote".into(),
                    "10".into(),
                    "--impressora".into(),
                    impressora.into(),
                ];
                if let Some(ordem) = ordem {
                    simular.extend(["--ordem".to_string(), ordem.to_string()]);
                }
                let relatorio = executar(&simular).unwrap();
                let esperados: Vec<String> = relatorio
                    .lines()
                    .filter_map(|l| l.strip_prefix("    "))
                    .filter_map(|l| l.strip_suffix(": certa"))
                    .map(String::from)
                    .collect();
                let folhas = ler_folhas(&pasta.caminho().join("lote-0001-folhas-finais.pdf"));
                let obtidos: Vec<String> = folhas.iter().map(rotulo_lado_a_lado).collect();
                assert_eq!(obtidos, esperados, "{impressora} {ordem:?} total {total}");
                assert_eq!(folhas.len(), total.div_ceil(2));
                for folha in &folhas {
                    assert_eq!(folha.caixa, vec![0.0, 0.0, 1190.0, 842.0]);
                    for face in [&folha.frente, &folha.verso].into_iter().flatten() {
                        assert_eq!(face.matriz[..4], [1.0, 0.0, 0.0, 1.0]);
                    }
                }
            }
        }
    }
}

#[test]
fn folhas_finais_com_faces_retrato_e_paisagem_ficam_na_ordem_do_documento() {
    for impressora in ["traseira", "gaveta"] {
        let pasta = Pasta::nova("folhas-faces-diferentes");
        let entrada = pasta.caminho().join("entrada.pdf");
        std::fs::write(&entrada, pdf_com_conteudo(&[(595, 842), (842, 595)])).unwrap();
        executar(&argumentos_folhas(
            &entrada,
            pasta.caminho(),
            impressora,
            None,
        ))
        .unwrap();
        let folhas = ler_folhas(&pasta.caminho().join("lote-0001-folhas-finais.pdf"));
        assert_eq!(folhas.len(), 1, "{impressora}");
        assert_eq!(rotulo_lado_a_lado(&folhas[0]), "1|2", "{impressora}");
        assert_eq!(
            folhas[0].caixa,
            vec![0.0, 0.0, 1437.0, 842.0],
            "{impressora}"
        );
    }
}

#[test]
fn folhas_finais_somam_as_faces_e_seguem_o_papel_de_cada_lote() {
    let pasta = Pasta::nova("folhas-papeis");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(
        &entrada,
        pdf_com_conteudo(&[(595, 842), (595, 842), (420, 595), (420, 595)]),
    )
    .unwrap();
    executar(&argumentos_folhas(
        &entrada,
        pasta.caminho(),
        "gaveta",
        None,
    ))
    .unwrap();
    let a4 = ler_folhas(&pasta.caminho().join("lote-0001-folhas-finais.pdf"));
    assert_eq!(a4.len(), 1);
    assert_eq!(a4[0].caixa, vec![0.0, 0.0, 1190.0, 842.0]);
    let a5 = ler_folhas(&pasta.caminho().join("lote-0002-folhas-finais.pdf"));
    assert_eq!(a5.len(), 1);
    assert_eq!(a5[0].caixa, vec![0.0, 0.0, 840.0, 595.0]);
}

#[test]
fn sem_folhas_finais_a_pasta_recebe_so_as_passadas() {
    let pasta = Pasta::nova("sem-folhas");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_com_conteudo(&[(595, 842), (595, 842)])).unwrap();
    executar(&argumentos_gerar(&entrada, pasta.caminho(), "10")).unwrap();
    let mut nomes: Vec<String> = std::fs::read_dir(pasta.caminho())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|nome| nome != "entrada.pdf")
        .collect();
    nomes.sort();
    assert_eq!(
        nomes,
        vec!["lote-0001-passada-1.pdf", "lote-0001-passada-2.pdf"]
    );
}

#[test]
fn folhas_finais_nao_sobrescrevem_e_removem_as_passadas_criadas() {
    let pasta = Pasta::nova("folhas-existente");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_com_conteudo(&[(595, 842), (595, 842)])).unwrap();
    let existente = pasta.caminho().join("lote-0001-folhas-finais.pdf");
    std::fs::write(&existente, b"existente").unwrap();
    let erro = executar(&argumentos_folhas(
        &entrada,
        pasta.caminho(),
        "gaveta",
        None,
    ))
    .unwrap_err();
    assert!(erro.contains("lote-0001-folhas-finais.pdf"), "{erro}");
    assert_eq!(std::fs::read(&existente).unwrap(), b"existente");
    assert!(!pasta.caminho().join("lote-0001-passada-1.pdf").exists());
    assert!(!pasta.caminho().join("lote-0001-passada-2.pdf").exists());
}

#[test]
fn ajuda_de_gerar_pdf_explica_folhas_finais() {
    let geral = executar(&args(&["--help"])).unwrap();
    assert!(geral.contains("--folhas-finais"));
    let gerar = executar(&args(&["gerar-pdf", "--help"])).unwrap();
    assert!(gerar.contains("--folhas-finais"));
    assert!(gerar.contains("anotações"));
}

#[test]
fn folhas_finais_repetida_e_recusada_sem_criar_arquivos() {
    let pasta = Pasta::nova("folhas-repetida");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_com_conteudo(&[(595, 842), (595, 842)])).unwrap();
    let mut argumentos = argumentos_folhas(&entrada, pasta.caminho(), "gaveta", None);
    argumentos.push("--folhas-finais".into());
    let erro = executar(&argumentos).unwrap_err();
    assert!(erro.contains("mais de uma vez"), "{erro}");
    assert_eq!(std::fs::read_dir(pasta.caminho()).unwrap().count(), 1);
}

/// Duas páginas; a primeira tem um campo de assinatura já aplicado, se pedido.
fn pdf_com_assinatura(assinado: bool) -> Vec<u8> {
    use lopdf::{dictionary, Document, Object};
    let mut doc = Document::with_version("1.5");
    let raiz = doc.new_object_id();
    let aparencia = doc.add_object(lopdf::Stream::new(
        dictionary! {"Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 10.into(), 10.into()]},
        b"q Q".to_vec(),
    ));
    let valor = doc.add_object(dictionary! {"Type" => "Sig", "Filter" => "Adobe.PPKLite"});
    let mut campo = dictionary! {"Type" => "Annot", "Subtype" => "Widget", "FT" => "Sig", "T" => Object::string_literal("a"), "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()], "AP" => dictionary! {"N" => aparencia}};
    if assinado {
        campo.set("V", valor);
    }
    let widget = doc.add_object(campo);
    let mut filhos = Vec::new();
    for i in 0..2 {
        let mut pagina = dictionary! {"Type" => "Page", "Parent" => raiz, "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()], "Resources" => dictionary! {}};
        if i == 0 {
            pagina.set("Annots", vec![Object::Reference(widget)]);
        }
        filhos.push(Object::Reference(doc.add_object(pagina)));
    }
    doc.objects.insert(
        raiz,
        Object::Dictionary(dictionary! {"Type" => "Pages", "Count" => 2, "Kids" => filhos}),
    );
    let catalogo = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => raiz});
    doc.trailer.set("Root", catalogo);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

const AVISO_ASSINATURA: &str = "Aviso: este PDF tem assinatura digital. As cópias geradas servem só para imprimir: mostram a aparência da assinatura, mas não a contêm. Para enviar ou guardar o documento, use o arquivo original.";

#[test]
fn gerar_pdf_avisa_quando_o_pdf_tem_assinatura_e_so_entao() {
    let pasta = Pasta::nova("com-marca");
    let entrada = pasta.caminho().join("assinado.pdf");
    std::fs::write(&entrada, pdf_com_assinatura(true)).unwrap();
    let saida = executar(&argumentos_gerar(&entrada, pasta.caminho(), "10")).unwrap();
    let aviso = saida.find(AVISO_ASSINATURA).expect(&saida);
    assert!(aviso < saida.find("Gerados").unwrap());

    let outra = Pasta::nova("sem-marca");
    let campo_vazio = outra.caminho().join("campo-vazio.pdf");
    std::fs::write(&campo_vazio, pdf_com_assinatura(false)).unwrap();
    let saida = executar(&argumentos_gerar(&campo_vazio, outra.caminho(), "10")).unwrap();
    assert!(!saida.contains("assinatura"), "{saida}");
    let terceira = Pasta::nova("sem-marca-2");
    let comum = terceira.caminho().join("comum.pdf");
    std::fs::write(&comum, pdf_de_paginas(&[(595, 842), (595, 842)])).unwrap();
    let saida = executar(&argumentos_gerar(&comum, terceira.caminho(), "10")).unwrap();
    assert!(!saida.contains("assinatura"), "{saida}");
}

#[test]
fn ajuda_do_gerar_pdf_explica_que_a_copia_nao_contem_a_assinatura() {
    let ajuda = executar(&["gerar-pdf".to_string(), "--help".to_string()]).unwrap();
    assert!(ajuda.contains("Se o PDF tiver assinatura digital, as passadas mostram a aparência dela, mas não a contêm: servem só para imprimir e não comprovam a assinatura. Para enviar ou guardar o documento, use o arquivo original."));
}
