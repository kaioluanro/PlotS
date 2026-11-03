use plots_cli::{executar, executar_com_io};
use plots_core::{GestoRecolocar, ModeloImpressora, PerfilImpressora, ViradaMaco};
use plots_perfis::{carregar, salvar, EntradaPerfil, Perfis};
use std::io::Cursor;
use std::path::{Path, PathBuf};

const PERGUNTA_1: &str = "O verso da folha 1 é o 2? (s/n)";
const AVISO_ASSINATURA: &str = "Aviso: este PDF tem assinatura digital. As cópias geradas servem só para imprimir: mostram a aparência da assinatura, mas não a contêm. Para enviar ou guardar o documento, use o arquivo original.";

fn args(itens: &[&str]) -> Vec<String> {
    itens.iter().map(|s| (*s).to_string()).collect()
}

/// Pasta temporária dentro da pasta de build do teste, removida ao sair de escopo.
struct Pasta(PathBuf);

impl Pasta {
    fn nova(nome: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static CONTADOR: AtomicU64 = AtomicU64::new(0);
        let caminho = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "calibrar-imprimir-{nome}-{}-{}",
            std::process::id(),
            CONTADOR.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&caminho).unwrap();
        Self(caminho)
    }

    fn caminho(&self) -> &Path {
        &self.0
    }

    fn quantidade_de_entradas(&self) -> usize {
        std::fs::read_dir(&self.0).unwrap().count()
    }
}

impl Drop for Pasta {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Roda o comando com a entrada informada e devolve o resultado junto com tudo que foi escrito.
fn rodar(argumentos: &[String], entrada: &str) -> (Result<(), String>, String) {
    let mut saida = Vec::new();
    let mut leitor = Cursor::new(entrada.as_bytes().to_vec());
    let resultado = executar_com_io(argumentos, &mut leitor, &mut saida);
    (resultado, String::from_utf8(saida).unwrap())
}

fn calibrar_args(fila: &str, saida: &Path, perfis: &Path) -> Vec<String> {
    vec![
        "calibrar".into(),
        fila.into(),
        "--saida".into(),
        saida.to_string_lossy().into_owned(),
        "--perfis".into(),
        perfis.to_string_lossy().into_owned(),
    ]
}

fn imprimir_args(
    arquivo: &Path,
    impressora: &str,
    lote: &str,
    saida: &Path,
    perfis: &Path,
) -> Vec<String> {
    vec![
        "imprimir".into(),
        arquivo.to_string_lossy().into_owned(),
        "--impressora".into(),
        impressora.into(),
        "--lote".into(),
        lote.into(),
        "--saida".into(),
        saida.to_string_lossy().into_owned(),
        "--perfis".into(),
        perfis.to_string_lossy().into_owned(),
    ]
}

fn com_simular(mut argumentos: Vec<String>) -> Vec<String> {
    argumentos.push("--simular".into());
    argumentos
}

/// Documento com uma página por tamanho informado.
fn pdf_de_paginas(tamanhos: &[(i64, i64)]) -> Vec<u8> {
    use lopdf::{dictionary, Document, Object};
    let mut doc = Document::with_version("1.5");
    let raiz = doc.new_object_id();
    let mut filhos = Vec::new();
    for (largura, altura) in tamanhos {
        let pagina = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => raiz,
            "MediaBox" => vec![0.into(), 0.into(), (*largura).into(), (*altura).into()],
            "Resources" => dictionary! {}
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

fn pdf_a4(paginas: usize) -> Vec<u8> {
    pdf_de_paginas(&vec![(595, 842); paginas])
}

/// Duas páginas; a primeira tem um campo de assinatura já aplicado.
fn pdf_com_assinatura() -> Vec<u8> {
    use lopdf::{dictionary, Document, Object};
    let mut doc = Document::with_version("1.5");
    let raiz = doc.new_object_id();
    let aparencia = doc.add_object(lopdf::Stream::new(
        dictionary! {"Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 10.into(), 10.into()]},
        b"q Q".to_vec(),
    ));
    let valor = doc.add_object(dictionary! {"Type" => "Sig", "Filter" => "Adobe.PPKLite"});
    let mut campo = dictionary! {"Type" => "Annot", "Subtype" => "Widget", "FT" => "Sig", "T" => Object::string_literal("a"), "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()], "AP" => dictionary! {"N" => aparencia}};
    campo.set("V", valor);
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

fn perfil_com(p1: bool, p2: bool, giro: bool) -> PerfilImpressora {
    PerfilImpressora {
        passada1_inversa: p1,
        passada2_inversa: p2,
        verso_girar_180: giro,
        gesto_recolocar: GestoRecolocar::default(),
    }
}

// ---- calibrar ----

#[test]
fn calibrar_grava_o_perfil_conforme_as_respostas() {
    for (q1, q2) in [("s", "s"), ("s", "n"), ("n", "s"), ("n", "n")] {
        let pasta = Pasta::nova("respostas");
        let perfis = pasta.caminho().join("perfis.toml");
        let (resultado, saida) = rodar(
            &calibrar_args("Jato", pasta.caminho(), &perfis),
            &format!("{q1}\n{q2}\n"),
        );
        assert!(resultado.is_ok(), "{resultado:?}\n{saida}");
        let lidos = carregar(&perfis).unwrap();
        let entrada = lidos.obter("Jato").expect("perfil gravado");
        assert!(!entrada.perfil.passada1_inversa);
        assert_eq!(entrada.perfil.passada2_inversa, q1 == "n");
        assert_eq!(entrada.perfil.verso_girar_180, q2 == "s");
        assert!(saida.contains("só vale depois de conferido em um lote"));
        assert!(pasta.caminho().join("calibracao.pdf").is_file());
    }
}

#[test]
fn calibrar_ajusta_o_perfil_salvo_e_mantem_apelido_e_texto() {
    let pasta = Pasta::nova("salvo");
    let perfis = pasta.caminho().join("perfis.toml");
    let mut anteriores = Perfis::new();
    anteriores.inserir(
        "Jato",
        EntradaPerfil {
            apelido: Some("Jato — escritório".into()),
            perfil: perfil_com(true, true, false),
            gesto_texto: Some("virar o maço".into()),
            modelo: None,
        },
    );
    salvar(&perfis, &anteriores).unwrap();
    let (resultado, _) = rodar(&calibrar_args("Jato", pasta.caminho(), &perfis), "n\ns\n");
    assert!(resultado.is_ok(), "{resultado:?}");
    let lidos = carregar(&perfis).unwrap();
    let entrada = lidos.obter("Jato").unwrap();
    assert!(entrada.perfil.passada1_inversa);
    assert!(!entrada.perfil.passada2_inversa);
    assert!(entrada.perfil.verso_girar_180);
    assert_eq!(entrada.apelido.as_deref(), Some("Jato — escritório"));
    assert_eq!(entrada.gesto_texto.as_deref(), Some("virar o maço"));
}

#[test]
fn calibrar_repete_a_pergunta_quando_a_resposta_e_invalida() {
    let pasta = Pasta::nova("invalida");
    let perfis = pasta.caminho().join("perfis.toml");
    let (resultado, saida) = rodar(
        &calibrar_args("Jato", pasta.caminho(), &perfis),
        "talvez\ns\nn\n",
    );
    assert!(resultado.is_ok(), "{resultado:?}\n{saida}");
    assert_eq!(saida.matches(PERGUNTA_1).count(), 2, "{saida}");
    assert!(saida.contains("Resposta inválida"));
    let lidos = carregar(&perfis).unwrap();
    let entrada = lidos.obter("Jato").unwrap();
    assert!(!entrada.perfil.passada2_inversa);
    assert!(!entrada.perfil.verso_girar_180);
}

#[test]
fn calibrar_fim_da_entrada_cancela_sem_gravar_perfil() {
    for entrada in ["", "s\n"] {
        let pasta = Pasta::nova("cancela");
        let perfis = pasta.caminho().join("perfis.toml");
        let (resultado, _) = rodar(&calibrar_args("Jato", pasta.caminho(), &perfis), entrada);
        let erro = resultado.unwrap_err();
        assert!(erro.contains("cancelada"), "{erro}");
        assert!(!perfis.exists());
    }
}

#[test]
fn calibrar_recusa_arquivo_de_perfis_invalido_sem_alterar() {
    let pasta = Pasta::nova("invalido");
    let perfis = pasta.caminho().join("perfis.toml");
    let original = b"isto nao e toml = = =".to_vec();
    std::fs::write(&perfis, &original).unwrap();
    let (resultado, _) = rodar(&calibrar_args("Jato", pasta.caminho(), &perfis), "s\ns\n");
    assert!(resultado.is_err());
    assert_eq!(std::fs::read(&perfis).unwrap(), original);
}

#[test]
fn calibrar_nao_sobrescreve_calibracao_existente() {
    let pasta = Pasta::nova("existente");
    let perfis = pasta.caminho().join("perfis.toml");
    let calibracao = pasta.caminho().join("calibracao.pdf");
    std::fs::write(&calibracao, b"x").unwrap();
    let (resultado, _) = rodar(&calibrar_args("Jato", pasta.caminho(), &perfis), "s\ns\n");
    assert!(resultado.unwrap_err().contains("já existe"));
    assert_eq!(std::fs::read(&calibracao).unwrap(), b"x");
    assert!(!perfis.exists());
}

#[test]
fn calibrar_ajuda_mostra_o_uso_do_proprio_comando() {
    let ajuda = executar(&args(&["calibrar", "--help"])).unwrap();
    assert!(ajuda.contains("plots calibrar"));
    assert!(!ajuda.contains("plots gerar-pdf"));
}

// ---- imprimir ----

#[test]
fn impressora_simulada_sem_simular_recusa_e_nao_gera_arquivos() {
    let pasta = Pasta::nova("sem-simular");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_a4(8)).unwrap();
    let perfis = pasta.caminho().join("perfis.toml");
    let argumentos = imprimir_args(&entrada, "traseira", "2", pasta.caminho(), &perfis);
    let erro = executar(&argumentos).unwrap_err();
    assert_eq!(
        erro,
        "as impressoras simuladas só existem com --simular; para enviar de verdade, informe uma fila com perfil salvo"
    );
    assert_eq!(pasta.quantidade_de_entradas(), 1);
}

/// Argumentos do envio real: sem --simular e sem --saida.
fn envio_real_args(arquivo: &Path, fila: &str, perfis: &Path) -> Vec<String> {
    vec![
        "imprimir".into(),
        arquivo.to_string_lossy().into_owned(),
        "--impressora".into(),
        fila.into(),
        "--lote".into(),
        "2".into(),
        "--perfis".into(),
        perfis.to_string_lossy().into_owned(),
    ]
}

/// Nome que nenhuma fila do sistema usa: os testes nunca alcançam o spooler.
const FILA_INEXISTENTE: &str = "Fila de teste que não existe neste sistema";

fn perfil_salvo_para(fila: &str, perfis: &Path) {
    let mut conjunto = Perfis::default();
    conjunto.inserir(fila, EntradaPerfil::default());
    salvar(perfis, &conjunto).unwrap();
}

#[test]
fn envio_real_sem_perfil_salvo_pede_a_calibracao() {
    let pasta = Pasta::nova("real-sem-perfil");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_a4(4)).unwrap();
    let perfis = pasta.caminho().join("perfis.toml");
    let (resultado, saida) = rodar(
        &envio_real_args(&entrada, FILA_INEXISTENTE, &perfis),
        "s
",
    );
    let erro = resultado.unwrap_err();
    assert!(erro.contains("não tem perfil salvo"), "{erro}");
    assert!(erro.contains("plots calibrar"), "{erro}");
    assert!(!saida.contains("Enviar a frente"), "{saida}");
}

#[test]
fn envio_real_para_fila_que_nao_existe_recusa_sem_perguntar() {
    let pasta = Pasta::nova("real-sem-fila");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_a4(4)).unwrap();
    let perfis = pasta.caminho().join("perfis.toml");
    perfil_salvo_para(FILA_INEXISTENTE, &perfis);
    let (resultado, saida) = rodar(
        &envio_real_args(&entrada, FILA_INEXISTENTE, &perfis),
        "s
s
",
    );
    let erro = resultado.unwrap_err();
    assert!(erro.contains("plots filas"), "{erro}");
    assert!(!saida.contains("Enviar a frente"), "{saida}");
}

#[test]
fn envio_real_recusa_pdf_protegido_antes_de_olhar_a_fila() {
    let mut doc = lopdf::Document::load_mem(&pdf_a4(2)).unwrap();
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
    let pasta = Pasta::nova("real-protegido");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, bytes).unwrap();
    let perfis = pasta.caminho().join("perfis.toml");
    perfil_salvo_para(FILA_INEXISTENTE, &perfis);
    let erro = executar(&envio_real_args(&entrada, FILA_INEXISTENTE, &perfis)).unwrap_err();
    assert!(erro.contains("criptografado"), "{erro}");
}

#[test]
fn envio_real_recusa_pdf_que_so_tem_senha_de_dono() {
    let mut doc = lopdf::Document::load_mem(&pdf_a4(2)).unwrap();
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
        user_password: "",
        permissions: lopdf::Permissions::empty(),
    };
    let estado = lopdf::EncryptionState::try_from(versao).unwrap();
    doc.encrypt(&estado).unwrap();
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    let pasta = Pasta::nova("real-senha-de-dono");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, bytes).unwrap();
    let perfis = pasta.caminho().join("perfis.toml");
    perfil_salvo_para(FILA_INEXISTENTE, &perfis);
    let erro = executar(&envio_real_args(&entrada, FILA_INEXISTENTE, &perfis)).unwrap_err();
    assert!(erro.contains("criptografado"), "{erro}");
}

#[test]
fn opcoes_de_um_modo_sao_recusadas_no_outro() {
    let pasta = Pasta::nova("modos");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_a4(4)).unwrap();
    let perfis = pasta.caminho().join("perfis.toml");
    perfil_salvo_para(FILA_INEXISTENTE, &perfis);

    let mut simulado = com_simular(imprimir_args(
        &entrada,
        "traseira",
        "2",
        pasta.caminho(),
        &perfis,
    ));
    simulado.push("--arquivo-saida".into());
    simulado.push(pasta.caminho().to_string_lossy().into_owned());
    let erro = executar(&simulado).unwrap_err();
    assert!(erro.contains("--arquivo-saida"), "{erro}");

    let com_saida = imprimir_args(&entrada, FILA_INEXISTENTE, "2", pasta.caminho(), &perfis);
    let erro = executar(&com_saida).unwrap_err();
    assert!(erro.contains("--saida"), "{erro}");

    let mut com_folhas = envio_real_args(&entrada, FILA_INEXISTENTE, &perfis);
    com_folhas.push("--folhas-finais".into());
    let erro = executar(&com_folhas).unwrap_err();
    assert!(erro.contains("--folhas-finais"), "{erro}");
}

#[test]
fn ajuda_de_imprimir_descreve_o_envio_e_a_confirmacao() {
    let ajuda = executar(&args(&["imprimir", "--help"])).unwrap();
    assert!(ajuda.contains("--arquivo-saida PASTA"), "{ajuda}");
    assert!(ajuda.contains("confirmação"), "{ajuda}");
    assert!(
        ajuda.contains(
            "O verso só é oferecido depois de o trabalho da frente terminar ou sair da fila do spooler. Isso não garante que o papel já saiu: espere a última folha antes de recolocar o maço."
        ),
        "{ajuda}"
    );
    assert!(
        ajuda.contains(
            "--arquivo-saida PASTA grava cada passada em um arquivo novo dentro de PASTA, pela própria fila; foi conferido só na fila de PDF do Windows."
        ),
        "{ajuda}"
    );
    let geral = executar(&args(&["--help"])).unwrap();
    assert!(!geral.contains("Nenhum comando envia"), "{geral}");
}

#[test]
fn imprimir_fila_sem_perfil_manda_rodar_calibrar() {
    let pasta = Pasta::nova("sem-perfil");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_a4(8)).unwrap();
    let perfis = pasta.caminho().join("perfis.toml");
    let argumentos = com_simular(imprimir_args(
        &entrada,
        "Jato — X",
        "2",
        pasta.caminho(),
        &perfis,
    ));
    let erro = executar(&argumentos).unwrap_err();
    assert!(erro.contains("plots calibrar"), "{erro}");
    assert!(erro.contains("Jato — X"), "{erro}");
    assert_eq!(pasta.quantidade_de_entradas(), 1);
}

#[test]
fn imprimir_dois_lotes_completos_nas_duas_impressoras_simuladas() {
    for impressora in [
        "traseira",
        "gaveta",
        "Simulada — Traseira",
        "Simulada — Gaveta",
    ] {
        let pasta = Pasta::nova("dois-lotes");
        let entrada = pasta.caminho().join("entrada.pdf");
        std::fs::write(&entrada, pdf_a4(8)).unwrap();
        let perfis = pasta.caminho().join("perfis.toml");
        let argumentos = com_simular(imprimir_args(
            &entrada,
            impressora,
            "2",
            pasta.caminho(),
            &perfis,
        ));
        let (resultado, saida) = rodar(&argumentos, "\n\n");
        assert!(resultado.is_ok(), "{impressora}: {resultado:?}\n{saida}");
        assert!(saida.contains("Lote 1 de 2"), "{saida}");
        assert!(saida.contains("Lote 2 de 2"), "{saida}");
        assert_eq!(
            saida
                .matches("Resultado do lote: 2 de 2 folhas corretas")
                .count(),
            2,
            "{impressora}\n{saida}"
        );
        assert!(pasta.caminho().join("lote-0002-passada-2.pdf").is_file());
    }
}

#[test]
fn imprimir_com_perfil_salvo_confere_a_sequencia_do_perfil() {
    for (p1, p2, giro) in [
        (false, false, false),
        (false, false, true),
        (false, true, false),
        (false, true, true),
        (true, false, false),
        (true, false, true),
        (true, true, false),
        (true, true, true),
    ] {
        let pasta = Pasta::nova("perfil-salvo");
        let entrada = pasta.caminho().join("entrada.pdf");
        std::fs::write(&entrada, pdf_a4(8)).unwrap();
        let perfis = pasta.caminho().join("perfis.toml");
        let mut salvos = Perfis::new();
        salvos.inserir(
            "Jato — X",
            EntradaPerfil {
                perfil: perfil_com(p1, p2, giro),
                ..Default::default()
            },
        );
        salvar(&perfis, &salvos).unwrap();
        let argumentos = com_simular(imprimir_args(
            &entrada,
            "Jato — X",
            "2",
            pasta.caminho(),
            &perfis,
        ));
        let (resultado, saida) = rodar(&argumentos, "\n\n");
        let contexto = format!("perfil ({p1}, {p2}, {giro})\n{saida}");
        assert!(resultado.is_ok(), "{contexto}");
        assert_eq!(
            saida
                .matches("Resultado do lote: 2 de 2 folhas corretas")
                .count(),
            2,
            "{contexto}"
        );
        assert!(
            saida.contains("confere a sequência, não a máquina"),
            "{contexto}"
        );
    }
}

#[test]
fn imprimir_cancela_no_meio_e_mantem_os_pdfs_gerados() {
    for (entrada_usuario, lote_parado) in [("\n", "lote 2 de 2"), ("", "lote 1 de 2")] {
        let pasta = Pasta::nova("cancela-meio");
        let entrada = pasta.caminho().join("entrada.pdf");
        std::fs::write(&entrada, pdf_a4(8)).unwrap();
        let perfis = pasta.caminho().join("perfis.toml");
        let argumentos = com_simular(imprimir_args(
            &entrada,
            "gaveta",
            "2",
            pasta.caminho(),
            &perfis,
        ));
        let (resultado, saida) = rodar(&argumentos, entrada_usuario);
        let erro = resultado.unwrap_err();
        assert!(erro.contains(lote_parado), "{erro}\n{saida}");
        assert!(erro.contains("cancelada"), "{erro}");
        assert!(pasta.caminho().join("lote-0001-passada-1.pdf").is_file());
        assert!(pasta.caminho().join("lote-0002-passada-1.pdf").is_file());
    }
}

#[test]
fn imprimir_ordem_impares_avisa_a_ordem_das_passadas() {
    let pasta = Pasta::nova("impares");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_a4(4)).unwrap();
    let perfis = pasta.caminho().join("perfis.toml");
    let mut argumentos = com_simular(imprimir_args(
        &entrada,
        "traseira",
        "2",
        pasta.caminho(),
        &perfis,
    ));
    argumentos.extend(["--ordem".into(), "impares".into()]);
    let (resultado, saida) = rodar(&argumentos, "\n");
    assert!(resultado.is_ok(), "{resultado:?}\n{saida}");
    assert!(
        saida.contains("Ordem das passadas: ímpares primeiro."),
        "{saida}"
    );
}

#[test]
fn imprimir_mostra_o_aviso_de_assinatura() {
    let pasta = Pasta::nova("assinado");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_com_assinatura()).unwrap();
    let perfis = pasta.caminho().join("perfis.toml");
    let argumentos = com_simular(imprimir_args(
        &entrada,
        "traseira",
        "1",
        pasta.caminho(),
        &perfis,
    ));
    let (resultado, saida) = rodar(&argumentos, "\n\n");
    assert!(resultado.is_ok(), "{resultado:?}\n{saida}");
    assert!(saida.contains(AVISO_ASSINATURA), "{saida}");
}

#[test]
fn imprimir_mostra_o_aviso_de_orientacao_mista() {
    let pasta = Pasta::nova("mista");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_de_paginas(&[(595, 842), (842, 595)])).unwrap();
    let perfis = pasta.caminho().join("perfis.toml");
    let argumentos = com_simular(imprimir_args(
        &entrada,
        "traseira",
        "1",
        pasta.caminho(),
        &perfis,
    ));
    let (resultado, saida) = rodar(&argumentos, "\n");
    assert!(resultado.is_ok(), "{resultado:?}\n{saida}");
    assert!(saida.contains("mistura retrato e paisagem"), "{saida}");
}

#[test]
fn imprimir_ajuda_mostra_o_uso_do_proprio_comando() {
    let ajuda = executar(&args(&["imprimir", "--help"])).unwrap();
    assert!(ajuda.contains("plots imprimir"));
    assert!(!ajuda.contains("plots gerar-pdf"));
    assert!(ajuda.contains("--simular"));
}

#[test]
fn imprimir_com_folhas_finais_grava_o_arquivo_de_folhas_do_lote() {
    let pasta = Pasta::nova("folhas-finais");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_a4(4)).unwrap();
    let perfis = pasta.caminho().join("perfis.toml");
    let mut argumentos = com_simular(imprimir_args(
        &entrada,
        "gaveta",
        "2",
        pasta.caminho(),
        &perfis,
    ));
    argumentos.push("--folhas-finais".into());
    let (resultado, saida) = rodar(&argumentos, "\n");
    assert!(resultado.is_ok(), "{resultado:?}\n{saida}");
    assert!(pasta
        .caminho()
        .join("lote-0001-folhas-finais.pdf")
        .is_file());
    assert!(!pasta.caminho().join("lote-0002-folhas-finais.pdf").exists());
}

// ---- fluxo completo: gesto, linhas da simulação e calibração de simuladas ----

/// Roda `imprimir --simular` de uma pasta própria, com dois lotes de uma folha.
fn imprimir_simulado(
    pasta: &Pasta,
    impressora: &str,
    perfis: &Path,
) -> (Result<(), String>, String) {
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_a4(4)).unwrap();
    let saida = pasta.caminho().join("saida");
    std::fs::create_dir_all(&saida).unwrap();
    let argumentos = com_simular(imprimir_args(&entrada, impressora, "1", &saida, perfis));
    rodar(&argumentos, "\n\n")
}

fn perfil_salvo_em(pasta: &Pasta, fila: &str, entrada: EntradaPerfil) -> PathBuf {
    let perfis = pasta.caminho().join("perfis.toml");
    let mut salvos = Perfis::new();
    salvos.inserir(fila, entrada);
    salvar(&perfis, &salvos).unwrap();
    perfis
}

#[test]
fn gesto_das_simuladas_indica_o_lado_que_o_simulador_confere() {
    for (impressora, lado) in [("traseira", "baixo"), ("gaveta", "baixo")] {
        let pasta = Pasta::nova("gesto-lado");
        let perfis = pasta.caminho().join("perfis.toml");
        let (resultado, saida) = imprimir_simulado(&pasta, impressora, &perfis);
        assert!(resultado.is_ok(), "{saida}");
        let esperado = format!(
            "Pegue o maço como saiu, não embaralhe, e coloque na entrada com a face impressa para {lado}."
        );
        assert!(saida.contains(&esperado), "{impressora}\n{saida}");
    }
}

#[test]
fn saida_da_simulacao_nao_afirma_envio_real_nem_confirmacao_do_spooler() {
    let pasta = Pasta::nova("sem-afirmar");
    let perfis = pasta.caminho().join("perfis.toml");
    let (resultado, saida) = imprimir_simulado(&pasta, "traseira", &perfis);
    assert!(resultado.is_ok(), "{saida}");
    assert!(!saida.contains("Frente enviada"), "{saida}");
    assert!(!saida.contains("o spooler confirmou"), "{saida}");
    assert!(saida.contains(
        "  Frente simulada: envio e confirmação do spooler foram só simulados; nada foi à impressora."
    ));
    assert_eq!(
        saida.matches("folhas corretas (na simulação)").count(),
        2,
        "{saida}"
    );
    assert!(saida.contains(
        "Todos os lotes foram conferidos na simulação (2 no total). Isso não confere nenhuma impressora real."
    ));
}

#[test]
fn jato_mostra_a_virada_pela_borda_de_cima_na_segunda_linha() {
    let pasta = Pasta::nova("gesto-jato");
    let perfis = pasta.caminho().join("perfis.toml");
    let (resultado, saida) = imprimir_simulado(&pasta, "traseira", &perfis);
    assert!(resultado.is_ok(), "{saida}");
    assert!(
        saida.contains(
            "  Pegue o maço como saiu, não embaralhe, e coloque na entrada com a face impressa para baixo.\n  Vire o maço pela borda de cima, como um bloco de notas.\n"
        ),
        "{saida}"
    );
}

#[test]
fn laser_mostra_que_o_maco_nao_e_virado() {
    let pasta = Pasta::nova("gesto-laser");
    let perfis = pasta.caminho().join("perfis.toml");
    let (resultado, saida) = imprimir_simulado(&pasta, "gaveta", &perfis);
    assert!(resultado.is_ok(), "{saida}");
    assert!(
        saida.contains(
            "  Pegue o maço como saiu, não embaralhe, e coloque na entrada com a face impressa para baixo.\n  Não vire o maço.\n"
        ),
        "{saida}"
    );
}

#[test]
fn linha_na_gaveta_so_aparece_na_gaveta_simulada() {
    let pasta = Pasta::nova("linha-gaveta");
    let perfis = pasta.caminho().join("perfis.toml");
    let (_, traseira) = imprimir_simulado(&pasta, "traseira", &perfis);
    assert!(!traseira.contains("Na gaveta,"), "{traseira}");

    let pasta = Pasta::nova("linha-gaveta-2");
    let perfis = pasta.caminho().join("perfis.toml");
    let (resultado, gaveta) = imprimir_simulado(&pasta, "gaveta", &perfis);
    assert!(resultado.is_ok(), "{gaveta}");
    assert!(
        gaveta.contains("Na gaveta, vire o maço antes de ler as folhas."),
        "{gaveta}"
    );

    let pasta = Pasta::nova("linha-gaveta-perfil");
    let perfis = perfil_salvo_em(
        &pasta,
        "Jato — X",
        EntradaPerfil {
            perfil: perfil_com(false, false, false),
            ..Default::default()
        },
    );
    let entrada_pdf = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada_pdf, pdf_a4(4)).unwrap();
    std::fs::create_dir_all(pasta.caminho().join("saida")).unwrap();
    let (resultado, salvo) = rodar(
        &com_simular(imprimir_args(
            &entrada_pdf,
            "Jato — X",
            "1",
            &pasta.caminho().join("saida"),
            &perfis,
        )),
        "\n\n",
    );
    assert!(resultado.is_ok(), "{salvo}");
    assert!(!salvo.contains("Na gaveta,"), "{salvo}");
}

#[test]
fn perfil_salvo_usa_frase_sem_lado_e_mostra_o_texto_do_gesto() {
    for virada in [ViradaMaco::PelaLateral, ViradaMaco::NaoVirar] {
        let pasta = Pasta::nova("perfil-gesto");
        let perfil = PerfilImpressora {
            gesto_recolocar: GestoRecolocar {
                virada,
                borda_primeiro: plots_core::BordaEntrada::Curta,
            },
            ..Default::default()
        };
        let perfis = perfil_salvo_em(
            &pasta,
            "Jato — X",
            EntradaPerfil {
                perfil,
                gesto_texto: Some("Texto do gesto salvo.".into()),
                ..Default::default()
            },
        );
        let entrada = pasta.caminho().join("entrada.pdf");
        std::fs::write(&entrada, pdf_a4(4)).unwrap();
        let saida_dir = pasta.caminho().join("saida");
        std::fs::create_dir_all(&saida_dir).unwrap();
        let argumentos = com_simular(imprimir_args(
            &entrada,
            "Jato — X",
            "1",
            &saida_dir,
            &perfis,
        ));
        let (resultado, saida) = rodar(&argumentos, "\n\n");
        let contexto = format!("{virada:?}\n{saida}");
        assert!(resultado.is_ok(), "{contexto}");
        assert!(
            saida.contains(
                "  Pegue o maço como saiu, não embaralhe, e recoloque do mesmo jeito que na calibração.\n  Texto do gesto salvo.\n"
            ),
            "{contexto}"
        );
        assert!(!saida.contains("para cima."), "{contexto}");
        assert!(!saida.contains("para baixo."), "{contexto}");
        assert!(!saida.contains("Na gaveta,"), "{contexto}");
    }
}

#[test]
fn calibrar_recusa_nomes_de_impressora_simulada_sem_gerar_nada() {
    for nome in [
        "traseira",
        "gaveta",
        "Simulada — Traseira",
        "Simulada — Gaveta",
    ] {
        let pasta = Pasta::nova("recusa-simulada");
        let perfis = pasta.caminho().join("perfis.toml");
        let (resultado, _) = rodar(&calibrar_args(nome, pasta.caminho(), &perfis), "s\ns\n");
        assert_eq!(
            resultado.unwrap_err(),
            format!("'{nome}' é uma impressora simulada e não precisa de calibração")
        );
        assert!(!pasta.caminho().join("calibracao.pdf").exists());
        assert!(!perfis.exists());
    }
}

#[test]
fn calibrar_cancelamento_cita_o_pdf_e_diz_que_nada_foi_gravado() {
    let pasta = Pasta::nova("cancela-pdf");
    let perfis = pasta.caminho().join("perfis.toml");
    let (resultado, _) = rodar(&calibrar_args("Jato", pasta.caminho(), &perfis), "s\n");
    let erro = resultado.unwrap_err();
    assert!(
        erro.contains(&format!(
            "calibracao.pdf ficou em {}",
            pasta.caminho().display()
        )),
        "{erro}"
    );
    assert!(erro.contains("nenhum perfil foi gravado"), "{erro}");
    assert!(!perfis.exists());
}

// ---- ajuda geral ----

#[test]
fn ajuda_geral_lista_todos_os_comandos_e_quem_nao_envia() {
    let geral = executar(&args(&["--help"])).unwrap();
    assert!(geral.contains("Só 'imprimir' sem --simular envia à impressora"));
    let mut comandos = Vec::new();
    for linha in geral.lines() {
        if let Some(resto) = linha.trim_start().strip_prefix("plots ") {
            let comando = resto.split_whitespace().next().unwrap_or("").to_string();
            comandos.push((comando, linha.to_string()));
        }
    }
    let esperados = [
        "simular",
        "gerar-pdf",
        "gerar-calibracao",
        "imprimir",
        "calibrar",
        "perfil",
        "filas",
    ];
    for esperado in esperados {
        assert!(
            comandos.iter().any(|(c, _)| c == esperado),
            "falta {esperado} na ajuda geral"
        );
    }
    for (comando, linha) in &comandos {
        if comando == "imprimir" {
            assert!(linha.contains("--simular"), "{linha}");
        } else {
            assert!(esperados.contains(&comando.as_str()), "{comando}");
        }
    }
}

// ---- perfil pronto de um modelo ----

fn perfil_args(fila: &str, modelo: &str, perfis: &Path) -> Vec<String> {
    vec![
        "perfil".into(),
        fila.into(),
        "--modelo".into(),
        modelo.into(),
        "--perfis".into(),
        perfis.to_string_lossy().into_owned(),
    ]
}

/// Grava o perfil do modelo e simula a impressão de 6 páginas em um lote, dando Enter no gesto.
fn imprimir_seis_paginas_com_modelo(codigo: &str) -> String {
    imprimir_seis_paginas_com_modelo_e_opcoes(codigo, &[])
}

fn imprimir_seis_paginas_com_modelo_e_opcoes(codigo: &str, opcoes: &[&str]) -> String {
    let pasta = Pasta::nova("modelo-imprimir");
    let perfis = pasta.caminho().join("perfis.toml");
    let (resultado, saida) = rodar(&perfil_args("Fila X", codigo, &perfis), "");
    assert!(resultado.is_ok(), "{resultado:?}\n{saida}");
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_a4(6)).unwrap();
    let saida_dir = pasta.caminho().join("saida");
    std::fs::create_dir_all(&saida_dir).unwrap();
    let mut argumentos = com_simular(imprimir_args(&entrada, "Fila X", "3", &saida_dir, &perfis));
    argumentos.extend(opcoes.iter().map(|opcao| (*opcao).to_owned()));
    let (resultado, saida) = rodar(&argumentos, "\n");
    assert!(resultado.is_ok(), "{resultado:?}\n{saida}");
    saida
}

#[test]
fn imprimir_impares_primeiro_com_cada_modelo_confere_as_tres_folhas() {
    for codigo in ["jato", "laser"] {
        let saida = imprimir_seis_paginas_com_modelo_e_opcoes(codigo, &["--ordem", "impares"]);
        assert!(
            saida.contains("Ordem das passadas: ímpares primeiro."),
            "{saida}"
        );
        assert!(
            saida.contains("3 de 3 folhas corretas"),
            "{codigo}\n{saida}"
        );
    }
}

#[test]
fn perfil_grava_o_modelo_de_uma_fila_que_nao_existe() {
    for (codigo, modelo) in [
        ("jato", ModeloImpressora::JatoDeTinta),
        ("laser", ModeloImpressora::Laser),
    ] {
        let pasta = Pasta::nova("perfil-modelo");
        let perfis = pasta.caminho().join("pasta nova").join("perfis.toml");
        let (resultado, saida) = rodar(&perfil_args(FILA_INEXISTENTE, codigo, &perfis), "");
        assert!(resultado.is_ok(), "{resultado:?}");
        let lidos = carregar(&perfis).unwrap();
        let entrada = lidos.obter(FILA_INEXISTENTE).expect("perfil gravado");
        assert_eq!(entrada.modelo, Some(modelo));
        assert_eq!(entrada.perfil, modelo.perfil());
        assert!(saida.contains("não foi medido nesta máquina"), "{saida}");
        assert!(saida.contains("confira com 2 ou 3 folhas"), "{saida}");
    }
}

#[test]
fn perfil_preserva_o_apelido_e_as_outras_filas() {
    let pasta = Pasta::nova("perfil-apelido");
    let perfis = perfil_salvo_em(
        &pasta,
        "Fila X",
        EntradaPerfil {
            apelido: Some("Sala".into()),
            perfil: perfil_com(false, true, true),
            gesto_texto: Some("Texto antigo.".into()),
            ..Default::default()
        },
    );
    let mut salvos = carregar(&perfis).unwrap();
    salvos.inserir("Outra", EntradaPerfil::default());
    salvar(&perfis, &salvos).unwrap();
    let (resultado, _) = rodar(&perfil_args("Fila X", "laser", &perfis), "");
    assert!(resultado.is_ok(), "{resultado:?}");
    let lidos = carregar(&perfis).unwrap();
    let entrada = lidos.obter("Fila X").unwrap();
    assert_eq!(entrada.apelido.as_deref(), Some("Sala"));
    assert_eq!(entrada.gesto_texto, None);
    assert_eq!(entrada.perfil, ModeloImpressora::Laser.perfil());
    assert_eq!(lidos.obter("Outra"), Some(&EntradaPerfil::default()));
}

#[test]
fn perfil_recusa_simulada_com_a_mesma_mensagem_de_calibrar() {
    for nome in [
        "traseira",
        "gaveta",
        "Simulada — Traseira",
        "Simulada — Gaveta",
    ] {
        let pasta = Pasta::nova("perfil-simulada");
        let perfis = pasta.caminho().join("perfis.toml");
        let (de_perfil, _) = rodar(&perfil_args(nome, "jato", &perfis), "");
        let (de_calibrar, _) = rodar(&calibrar_args(nome, pasta.caminho(), &perfis), "");
        assert_eq!(de_perfil.unwrap_err(), de_calibrar.unwrap_err());
        assert!(!perfis.exists());
    }
}

#[test]
fn perfil_recusa_argumentos_ruins_sem_gravar() {
    let pasta = Pasta::nova("perfil-ruim");
    let perfis = pasta.caminho().join("perfis.toml");
    let caminho = perfis.to_string_lossy().into_owned();
    let casos: [(&[&str], &str); 7] = [
        (&["perfil"], "informe o nome da impressora"),
        (
            &["perfil", "--modelo", "jato"],
            "informe o nome da impressora",
        ),
        (
            &["perfil", "  ", "--modelo", "jato"],
            "informe o nome da impressora",
        ),
        (&["perfil", "Fila"], "informe --modelo jato|laser"),
        (
            &["perfil", "Fila", "--modelo"],
            "faltou valor para --modelo",
        ),
        (
            &["perfil", "Fila", "--modelo", "matricial"],
            "--modelo deve ser jato ou laser",
        ),
        (
            &["perfil", "Fila", "--modelo", "jato", "--modelo", "laser"],
            "opção desconhecida ou repetida: --modelo",
        ),
    ];
    for (argumentos, esperado) in casos {
        let mut linha: Vec<String> = argumentos.iter().map(|&a| a.to_owned()).collect();
        if !argumentos.contains(&"--modelo") || argumentos.len() > 3 {
            linha.extend(["--perfis".to_owned(), caminho.clone()]);
        }
        let (resultado, _) = rodar(&linha, "");
        assert_eq!(resultado.unwrap_err(), esperado, "{linha:?}");
        assert!(!perfis.exists(), "{linha:?}");
    }
}

#[test]
fn perfil_nao_toca_em_arquivo_de_perfis_danificado() {
    let pasta = Pasta::nova("perfil-danificado");
    let perfis = pasta.caminho().join("perfis.toml");
    std::fs::write(&perfis, "isto não é toml = = =").unwrap();
    let (resultado, _) = rodar(&perfil_args("Fila", "jato", &perfis), "");
    assert!(resultado.unwrap_err().contains("não é um TOML válido"));
    assert_eq!(
        std::fs::read_to_string(&perfis).unwrap(),
        "isto não é toml = = ="
    );
}

#[test]
fn imprimir_com_jato_vira_pela_borda_de_cima_e_confere_as_seis_paginas() {
    let saida = imprimir_seis_paginas_com_modelo("jato");
    assert!(
        saida.contains("Modelo Jato de tinta: não foi medido nesta máquina"),
        "{saida}"
    );
    assert!(saida.contains("  Frente: 6, 4, 2\n"), "{saida}");
    assert!(saida.contains("  Verso: 5, 3, 1\n"), "{saida}");
    assert!(
        saida.contains(
            "coloque na entrada com a face impressa para baixo.\n  Vire o maço pela borda de cima, como um bloco de notas.\n"
        ),
        "{saida}"
    );
    assert!(saida.contains("3 de 3 folhas corretas"), "{saida}");
    assert!(!saida.contains("coerente com o perfil salvo"), "{saida}");
    assert!(!saida.contains("Na gaveta,"), "{saida}");
}

#[test]
fn imprimir_com_laser_nao_vira_e_confere_as_seis_paginas() {
    let saida = imprimir_seis_paginas_com_modelo("laser");
    assert!(
        saida.contains("Modelo Laser: não foi medido nesta máquina"),
        "{saida}"
    );
    assert!(saida.contains("  Frente: 6, 4, 2\n"), "{saida}");
    assert!(saida.contains("  Verso: 1, 3, 5\n"), "{saida}");
    assert!(
        saida.contains("coloque na entrada com a face impressa para baixo.\n  Não vire o maço.\n"),
        "{saida}"
    );
    assert!(saida.contains("3 de 3 folhas corretas"), "{saida}");
    assert!(!saida.contains("coerente com o perfil salvo"), "{saida}");
    assert!(
        saida.contains("Na gaveta, vire o maço antes de ler as folhas."),
        "{saida}"
    );
}

#[test]
fn calibrar_tira_o_modelo_e_mantem_o_gesto_do_modelo() {
    for respostas in ["s\nn\n", "n\ns\n"] {
        let pasta = Pasta::nova("calibrar-limpa-modelo");
        let perfis = pasta.caminho().join("perfis.toml");
        let (resultado, _) = rodar(&perfil_args("Fila X", "jato", &perfis), "");
        assert!(resultado.is_ok(), "{resultado:?}");
        let (resultado, _) = rodar(
            &calibrar_args("Fila X", pasta.caminho(), &perfis),
            respostas,
        );
        assert!(resultado.is_ok(), "{resultado:?}");
        let lidos = carregar(&perfis).unwrap();
        let entrada = lidos.obter("Fila X").unwrap();
        assert_eq!(entrada.modelo, None);
        assert_eq!(
            entrada.perfil.gesto_recolocar.virada,
            ViradaMaco::PelaBordaDeCima
        );
    }
}

#[test]
fn perfil_sem_modelo_com_virada_pela_borda_de_cima_passa_pelo_simulador_coerente() {
    let pasta = Pasta::nova("sem-modelo-cima");
    let perfis = perfil_salvo_em(
        &pasta,
        "Fila X",
        EntradaPerfil {
            perfil: ModeloImpressora::JatoDeTinta.perfil(),
            ..Default::default()
        },
    );
    let entrada = pasta.caminho().join("entrada.pdf");
    std::fs::write(&entrada, pdf_a4(6)).unwrap();
    let saida_dir = pasta.caminho().join("saida");
    std::fs::create_dir_all(&saida_dir).unwrap();
    let argumentos = com_simular(imprimir_args(&entrada, "Fila X", "3", &saida_dir, &perfis));
    let (resultado, saida) = rodar(&argumentos, "\n");
    assert!(resultado.is_ok(), "{resultado:?}\n{saida}");
    assert!(saida.contains("coerente com o perfil salvo"), "{saida}");
    assert!(
        saida.contains("do mesmo jeito que na calibração"),
        "{saida}"
    );
    assert!(!saida.contains("não foi medido"), "{saida}");
    assert!(saida.contains("3 de 3 folhas corretas"), "{saida}");
}

#[test]
fn ajuda_do_perfil_explica_o_modelo() {
    for argumentos in [vec!["perfil", "--help"], vec!["perfil", "-h"]] {
        let linha: Vec<String> = argumentos.iter().map(|&a| a.to_owned()).collect();
        let (resultado, saida) = rodar(&linha, "");
        assert!(resultado.is_ok());
        assert!(
            saida.contains("plots perfil NOME --modelo jato|laser"),
            "{saida}"
        );
        assert!(saida.contains("confira com 2 ou 3 folhas"), "{saida}");
    }
    let (_, geral) = rodar(&["--help".to_owned()], "");
    assert!(
        geral.contains("plots perfil NOME --modelo jato|laser"),
        "{geral}"
    );
}
