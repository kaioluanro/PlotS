use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use plots_core::distribuicao::ModoDistribuicao;
use plots_core::ModeloImpressora;

use crate::armazenamento::{carregar, salvar, FALHA_NA_GRAVACAO};
use crate::erro::ErroPerfis;
use crate::grupos::{ErroGrupo, LIMITE_GRUPOS, LIMITE_IMPRESSORAS_GRUPO, LIMITE_NOME_GRUPO};
use crate::Perfis;

static CONTADOR: AtomicU64 = AtomicU64::new(0);

struct Pasta(PathBuf);

impl Pasta {
    fn nova() -> Self {
        let caminho = std::env::temp_dir().join(format!(
            "plots-perfis-grupos-{}-{}",
            std::process::id(),
            CONTADOR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&caminho).unwrap();
        Self(caminho)
    }

    fn arquivo(&self) -> PathBuf {
        self.0.join("perfis.toml")
    }
}

impl Drop for Pasta {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn duas() -> Vec<String> {
    vec!["fila:Sala".to_owned(), "simulada:gaveta".to_owned()]
}

#[test]
fn grupo_salvo_volta_igual_e_os_perfis_continuam_la() {
    let pasta = Pasta::nova();
    let mut perfis = Perfis::new();
    perfis.definir_modelo("Sala", ModeloImpressora::Laser);
    perfis
        .salvar_grupo("Todas do escritório", &duas(), ModoDistribuicao::Divisao)
        .unwrap();
    perfis
        .salvar_grupo(
            "nome \"com\" = aspas [e] # sinais",
            &duas(),
            ModoDistribuicao::Espelho,
        )
        .unwrap();
    salvar(&pasta.arquivo(), &perfis).unwrap();
    let lidos = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(lidos, perfis);
    assert_eq!(
        lidos.grupo("Todas do escritório").unwrap().modo,
        ModoDistribuicao::Divisao
    );
    assert_eq!(
        lidos.obter("Sala").unwrap().modelo,
        Some(ModeloImpressora::Laser)
    );
}

#[test]
fn arquivo_com_grupos_e_aceito() {
    let pasta = Pasta::nova();
    fs::write(
        pasta.arquivo(),
        "versao = 1\n\n[grupos.Todas]\nimpressoras = [\"simulada:gaveta\"]\nmodo = \"divisao\"\n",
    )
    .unwrap();
    let lidos = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(
        lidos.grupo("Todas").unwrap().impressoras,
        ["simulada:gaveta"]
    );
}

#[test]
fn arquivo_antigo_sem_grupos_carrega_e_regrava_sem_mudar() {
    let pasta = Pasta::nova();
    let mut antigo = Perfis::new();
    antigo.definir_modelo("Sala", ModeloImpressora::JatoDeTinta);
    salvar(&pasta.arquivo(), &antigo).unwrap();
    let antes = fs::read_to_string(pasta.arquivo()).unwrap();
    assert!(
        !antes.contains("grupos"),
        "sem grupos, nada novo no arquivo"
    );
    let lidos = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(lidos.grupos().count(), 0);
    salvar(&pasta.arquivo(), &lidos).unwrap();
    assert_eq!(fs::read_to_string(pasta.arquivo()).unwrap(), antes);
}

#[test]
fn mudar_um_perfil_nao_perde_os_grupos() {
    let pasta = Pasta::nova();
    let mut perfis = Perfis::new();
    perfis
        .salvar_grupo("Sala", &duas(), ModoDistribuicao::Divisao)
        .unwrap();
    salvar(&pasta.arquivo(), &perfis).unwrap();
    let mut lidos = carregar(&pasta.arquivo()).unwrap();
    lidos.definir_modelo("Nova", ModeloImpressora::Laser);
    salvar(&pasta.arquivo(), &lidos).unwrap();
    let relidos = carregar(&pasta.arquivo()).unwrap();
    assert!(relidos.grupo("Sala").is_some());
    assert!(relidos.obter("Nova").is_some());
}

#[test]
fn nome_repetido_e_recusado_sem_substituir() {
    let mut perfis = Perfis::new();
    perfis
        .salvar_grupo("Escritório", &duas(), ModoDistribuicao::Divisao)
        .unwrap();
    for repetido in ["Escritório", " escritório ", "ESCRITÓRIO"] {
        assert_eq!(
            perfis.salvar_grupo(
                repetido,
                &["fila:Outra".to_owned()],
                ModoDistribuicao::Espelho
            ),
            Err(ErroGrupo::NomeRepetido)
        );
    }
    let grupo = perfis.grupo("Escritório").unwrap();
    assert_eq!(grupo.modo, ModoDistribuicao::Divisao);
    assert_eq!(grupo.impressoras, duas());
}

#[test]
fn nome_hostil_e_recusado() {
    let mut perfis = Perfis::new();
    let modo = ModoDistribuicao::Espelho;
    let grande = "x".repeat(LIMITE_NOME_GRUPO + 1);
    let casos = [
        ("", ErroGrupo::NomeVazio),
        (" \t ", ErroGrupo::NomeVazio),
        (grande.as_str(), ErroGrupo::NomeGrande),
        ("a\nb", ErroGrupo::NomeInvalido),
        ("a\u{0}b", ErroGrupo::NomeInvalido),
        ("a\u{202E}b", ErroGrupo::NomeInvalido),
        ("\u{200B}", ErroGrupo::NomeInvalido),
    ];
    for (nome, erro) in casos {
        assert_eq!(
            perfis.salvar_grupo(nome, &duas(), modo),
            Err(erro),
            "{nome:?}"
        );
    }
    assert_eq!(perfis.grupos().count(), 0);
    let no_limite = "é".repeat(LIMITE_NOME_GRUPO);
    assert_eq!(perfis.salvar_grupo(&no_limite, &duas(), modo), Ok(()));
}

#[test]
fn nome_guardado_fica_sem_espacos_nas_pontas() {
    let mut perfis = Perfis::new();
    perfis
        .salvar_grupo("  Sala  ", &duas(), ModoDistribuicao::Espelho)
        .unwrap();
    assert!(perfis.grupo("Sala").is_some());
}

#[test]
fn impressoras_vazias_repetidas_e_demais() {
    let mut perfis = Perfis::new();
    let modo = ModoDistribuicao::Espelho;
    assert_eq!(
        perfis.salvar_grupo("a", &[], modo),
        Err(ErroGrupo::SemImpressoras)
    );
    assert_eq!(
        perfis.salvar_grupo("a", &[" ".to_owned()], modo),
        Err(ErroGrupo::SemImpressoras)
    );
    let demais: Vec<String> = (0..=LIMITE_IMPRESSORAS_GRUPO)
        .map(|numero| format!("fila:{numero}"))
        .collect();
    assert_eq!(
        perfis.salvar_grupo("a", &demais, modo),
        Err(ErroGrupo::MuitasImpressoras)
    );
    let repetidas = ["fila:A".to_owned(), "fila:A".to_owned()];
    perfis.salvar_grupo("a", &repetidas, modo).unwrap();
    assert_eq!(perfis.grupo("a").unwrap().impressoras, ["fila:A"]);
}

#[test]
fn quantidade_de_grupos_tem_limite() {
    let mut perfis = Perfis::new();
    let modo = ModoDistribuicao::Espelho;
    for numero in 0..LIMITE_GRUPOS {
        perfis
            .salvar_grupo(&format!("g{numero}"), &duas(), modo)
            .unwrap();
    }
    assert_eq!(
        perfis.salvar_grupo("mais um", &duas(), modo),
        Err(ErroGrupo::MuitosGrupos)
    );
}

#[test]
fn apagar_tira_so_aquele_e_recusa_o_que_nao_existe() {
    let mut perfis = Perfis::new();
    let modo = ModoDistribuicao::Espelho;
    perfis.salvar_grupo("a", &duas(), modo).unwrap();
    perfis.salvar_grupo("b", &duas(), modo).unwrap();
    assert!(perfis.apagar_grupo("a").is_ok());
    assert_eq!(perfis.apagar_grupo("a"), Err(ErroGrupo::Inexistente));
    assert!(perfis.grupo("b").is_some());
}

#[test]
fn arquivo_que_nem_e_toml_nao_abre_e_fica_como_esta() {
    let pasta = Pasta::nova();
    let texto = "versao = 1\n[grupos.a\nimpressoras = 3\n";
    fs::write(pasta.arquivo(), texto).unwrap();
    let erro = carregar(&pasta.arquivo()).expect_err(texto);
    assert!(matches!(erro, ErroPerfis::TomlInvalido { .. }), "{erro:?}");
    assert_eq!(fs::read_to_string(pasta.arquivo()).unwrap(), texto);
}

#[test]
fn grupo_de_campos_errados_e_ignorado_um_a_um() {
    let pasta = Pasta::nova();
    for texto in [
        "versao = 1\n[grupos.a]\nimpressoras = 3\nmodo = \"espelho\"\n",
        "versao = 1\n[grupos.a]\nmodo = \"espelho\"\n",
        "versao = 1\ngrupos = 7\n",
        "versao = 1\n[grupos.a]\nimpressoras = [\"x\"]\nmodo = \"espelho\"\nresto = 1\n",
    ] {
        fs::write(pasta.arquivo(), texto).unwrap();
        let lidos = carregar(&pasta.arquivo()).expect(texto);
        assert_eq!(lidos.grupos().count(), 0, "{texto}");
        assert_eq!(lidos.grupos_ignorados(), 1, "{texto}");
    }
}

#[test]
fn modo_desconhecido_vale_como_espelho_e_nao_recusa_o_arquivo() {
    let pasta = Pasta::nova();
    for modo in ["\"qualquer\"", "7", "[1, 2]"] {
        let texto = format!("versao = 1\n[grupos.a]\nimpressoras = [\"x\"]\nmodo = {modo}\n");
        fs::write(pasta.arquivo(), texto).unwrap();
        let lidos = carregar(&pasta.arquivo()).unwrap();
        assert_eq!(lidos.grupo("a").unwrap().modo, ModoDistribuicao::Espelho);
    }
}

#[test]
fn falha_na_gravacao_mantem_grupos_e_perfis_anteriores() {
    let pasta = Pasta::nova();
    let mut perfis = Perfis::new();
    perfis.definir_modelo("Sala", ModeloImpressora::Laser);
    salvar(&pasta.arquivo(), &perfis).unwrap();
    let antes = fs::read(pasta.arquivo()).unwrap();
    perfis
        .salvar_grupo("a", &duas(), ModoDistribuicao::Espelho)
        .unwrap();
    FALHA_NA_GRAVACAO.with(|falha| falha.set(true));
    assert!(salvar(&pasta.arquivo(), &perfis).is_err());
    assert_eq!(fs::read(pasta.arquivo()).unwrap(), antes);
}

const FILA_VALIDA: &str = r#"versao = 1

[filas.Sala]
passada1_inversa = false
passada2_inversa = true
verso_girar_180 = false
virar_maco = true
borda_primeiro = "curta"
"#;

#[test]
fn grupo_de_formato_errado_nao_impede_o_uso_dos_perfis() {
    let pasta = Pasta::nova();
    let grupos = r#"
[grupos.Certo]
impressoras = ["fila:Sala"]
modo = "divisao"

[grupos.Errado]
impressoras = 3
modo = "divisao"
"#;
    fs::write(pasta.arquivo(), format!("{FILA_VALIDA}{grupos}")).unwrap();
    let lidos = carregar(&pasta.arquivo()).unwrap();
    assert!(lidos.obter("Sala").is_some());
    assert!(lidos.grupo("Certo").is_some());
    assert!(lidos.grupo("Errado").is_none());
    assert_eq!(lidos.grupos_ignorados(), 1);
}

#[test]
fn grupo_ignorado_continua_no_arquivo_depois_de_gravar() {
    let pasta = Pasta::nova();
    let grupos = r#"
[grupos.Errado]
impressoras = 3
modo = "divisao"

[grupos.Vazio]
impressoras = []
modo = "espelho"
"#;
    fs::write(pasta.arquivo(), format!("{FILA_VALIDA}{grupos}")).unwrap();
    let mut lidos = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(lidos.grupos_ignorados(), 2);
    lidos
        .salvar_grupo("Novo", &duas(), ModoDistribuicao::Espelho)
        .unwrap();
    assert_eq!(
        lidos.salvar_grupo(" errado ", &duas(), ModoDistribuicao::Espelho),
        Err(ErroGrupo::NomeRepetido)
    );
    salvar(&pasta.arquivo(), &lidos).unwrap();
    let texto = fs::read_to_string(pasta.arquivo()).unwrap();
    assert!(texto.contains("impressoras = 3"), "{texto}");
    let relido = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(relido.grupos_ignorados(), 2);
    assert!(relido.grupo("Novo").is_some());
    assert!(relido.obter("Sala").is_some());
}

#[test]
fn grupos_que_nem_sao_tabela_nao_impedem_os_perfis() {
    let pasta = Pasta::nova();
    fs::write(pasta.arquivo(), format!("grupos = 3\n{FILA_VALIDA}")).unwrap();
    let mut lidos = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(lidos.grupos_ignorados(), 1);
    assert!(lidos.obter("Sala").is_some());
    lidos
        .salvar_grupo("Novo", &duas(), ModoDistribuicao::Espelho)
        .unwrap();
    salvar(&pasta.arquivo(), &lidos).unwrap();
    let relido = carregar(&pasta.arquivo()).unwrap();
    assert!(relido.grupo("Novo").is_some());
    assert_eq!(relido.grupos_ignorados(), 1);
}

#[test]
fn nomes_iguais_na_tela_ficam_so_com_o_primeiro() {
    let pasta = Pasta::nova();
    let grupos = r#"
[grupos.Casa]
impressoras = ["fila:Sala"]
modo = "espelho"

[grupos.casa]
impressoras = ["fila:Sala"]
modo = "divisao"
"#;
    fs::write(pasta.arquivo(), format!("{FILA_VALIDA}{grupos}")).unwrap();
    let lidos = carregar(&pasta.arquivo()).unwrap();
    assert!(lidos.grupo("Casa").is_some());
    assert!(lidos.grupo("casa").is_none());
    assert_eq!(lidos.grupos_ignorados(), 1);
}

#[test]
fn nome_hostil_vindo_do_arquivo_nao_vira_grupo() {
    let pasta = Pasta::nova();
    let longo = "x".repeat(LIMITE_NOME_GRUPO + 1);
    let invisivel = r#"
[grupos."a\u202Eb"]
impressoras = ["fila:Sala"]
modo = "espelho"
"#;
    let grande = format!("\n[grupos.{longo}]\nimpressoras = [\"fila:Sala\"]\nmodo = \"espelho\"\n");
    fs::write(pasta.arquivo(), format!("{FILA_VALIDA}{invisivel}{grande}")).unwrap();
    let lidos = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(lidos.grupos().count(), 0);
    assert_eq!(lidos.grupos_ignorados(), 2);
    assert!(lidos.obter("Sala").is_some());
}
