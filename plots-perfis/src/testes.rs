use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use plots_core::{BordaEntrada, GestoRecolocar, PerfilImpressora};

use crate::armazenamento::{
    caminho_padrao, carregar, salvar, FALHA_NA_GRAVACAO, LIMITE_ARQUIVO_BYTES,
};
use crate::erro::ErroPerfis;
use crate::{EntradaPerfil, Perfis};

static CONTADOR_PASTAS: AtomicU64 = AtomicU64::new(0);

/// Pasta temporária exclusiva do teste, removida ao fim.
struct Pasta(PathBuf);

impl Pasta {
    fn nova(nome: &str) -> Self {
        let sequencia = CONTADOR_PASTAS.fetch_add(1, Ordering::Relaxed);
        let caminho = std::env::temp_dir().join(format!(
            "plots-perfis-teste-{nome}-{}-{sequencia}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&caminho);
        fs::create_dir_all(&caminho).expect("pasta temporária de teste");
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

fn perfil_com(passadas: (bool, bool, bool), gesto: (bool, BordaEntrada)) -> PerfilImpressora {
    PerfilImpressora {
        passada1_inversa: passadas.0,
        passada2_inversa: passadas.1,
        verso_girar_180: passadas.2,
        gesto_recolocar: GestoRecolocar {
            virar_maco: gesto.0,
            borda_primeiro: gesto.1,
        },
    }
}

fn entrada_completa() -> EntradaPerfil {
    EntradaPerfil {
        apelido: Some("Jato — escritório".to_owned()),
        perfil: perfil_com((true, false, true), (false, BordaEntrada::Longa)),
        gesto_texto: Some("Vire o maço e encoste a borda curta primeiro".to_owned()),
    }
}

fn bytes_do_arquivo(caminho: &PathBuf) -> Vec<u8> {
    fs::read(caminho).expect("leitura do arquivo de teste")
}

fn nomes_na_pasta(pasta: &Pasta) -> Vec<String> {
    let mut nomes: Vec<String> = fs::read_dir(&pasta.0)
        .expect("listagem da pasta de teste")
        .filter_map(|entrada| entrada.ok())
        .map(|entrada| entrada.file_name().to_string_lossy().into_owned())
        .collect();
    nomes.sort();
    nomes
}

#[test]
fn ida_e_volta_preserva_todos_os_campos() {
    let pasta = Pasta::nova("campos");
    let mut perfis = Perfis::new();
    perfis.inserir("Laser do escritório 1020", entrada_completa());
    salvar(&pasta.arquivo(), &perfis).expect("gravação");
    let lidos = carregar(&pasta.arquivo()).expect("leitura");
    assert_eq!(lidos, perfis);
    assert_eq!(
        lidos.obter("Laser do escritório 1020"),
        Some(&entrada_completa())
    );
}

#[test]
fn todas_as_combinacoes_de_perfil_voltam_iguais() {
    let pasta = Pasta::nova("combinacoes");
    let mut perfis = Perfis::new();
    let mut esperados = Vec::new();
    for mascara in 0u8..32 {
        let passadas = (mascara & 1 != 0, mascara & 2 != 0, mascara & 4 != 0);
        let borda = if mascara & 16 != 0 {
            BordaEntrada::Longa
        } else {
            BordaEntrada::Curta
        };
        let gesto = (mascara & 8 != 0, borda);
        let entrada = EntradaPerfil {
            apelido: None,
            perfil: perfil_com(passadas, gesto),
            gesto_texto: None,
        };
        let fila = format!("fila-{mascara}");
        perfis.inserir(&fila, entrada.clone());
        esperados.push((fila, entrada));
    }
    salvar(&pasta.arquivo(), &perfis).expect("gravação");
    let lidos = carregar(&pasta.arquivo()).expect("leitura");
    assert_eq!(lidos.len(), 32);
    for (fila, entrada) in &esperados {
        assert_eq!(lidos.obter(fila), Some(entrada), "fila {fila}");
    }
}

#[test]
fn nomes_de_fila_hostis_sao_preservados() {
    let nomes = [
        "Jato — escritório",
        "C:\\spool\\fila",
        "\\\\servidor\\Fila $1",
        "fila \"aspas\" 'simples'",
        "[tabela]",
        "chave = valor",
        "linha\nnova",
        "tab\tsim",
        "#comentario",
        "",
        "é ü ç",
        "impressora 🖨 azul",
    ];
    let pasta = Pasta::nova("hostis");
    let mut perfis = Perfis::new();
    for (indice, nome) in nomes.iter().enumerate() {
        let entrada = EntradaPerfil {
            apelido: Some(format!("apelido {indice}")),
            ..EntradaPerfil::default()
        };
        perfis.inserir(nome, entrada);
    }
    salvar(&pasta.arquivo(), &perfis).expect("gravação");
    let lidos = carregar(&pasta.arquivo()).expect("leitura");
    assert_eq!(lidos, perfis);
    for (indice, nome) in nomes.iter().enumerate() {
        let entrada = lidos.obter(nome).expect("fila presente após ida e volta");
        assert_eq!(entrada.apelido, Some(format!("apelido {indice}")));
    }
}

#[test]
fn arquivo_ausente_resulta_em_lista_vazia_sem_criar_nada() {
    let pasta = Pasta::nova("ausente");
    let lidos = carregar(&pasta.arquivo()).expect("arquivo ausente não é erro");
    assert!(lidos.is_empty());
    assert!(nomes_na_pasta(&pasta).is_empty());
}

#[test]
fn toml_invalido_gera_erro_com_caminho_e_nao_altera_o_arquivo() {
    let pasta = Pasta::nova("invalido");
    let caminho = pasta.arquivo();
    let original = b"versao = 1\n[filas\nisto nao fecha".to_vec();
    fs::write(&caminho, &original).expect("escrita de teste");
    let erro = carregar(&caminho).expect_err("TOML inválido deve falhar");
    assert!(matches!(erro, ErroPerfis::TomlInvalido { .. }));
    assert!(erro.to_string().contains(&caminho.display().to_string()));
    assert_eq!(bytes_do_arquivo(&caminho), original);
}

#[test]
fn versao_futura_gera_erro_e_nao_altera_o_arquivo() {
    let pasta = Pasta::nova("futura");
    let caminho = pasta.arquivo();
    let original = b"versao = 2\n[filas.\"x\"]\ncampo_novo = true\n".to_vec();
    fs::write(&caminho, &original).expect("escrita de teste");
    let erro = carregar(&caminho).expect_err("versão futura deve falhar");
    assert!(matches!(
        erro,
        ErroPerfis::VersaoDesconhecida { versao: 2, .. }
    ));
    assert_eq!(bytes_do_arquivo(&caminho), original);
}

#[test]
fn versao_nao_numerica_ou_ausente_gera_erro_de_formato() {
    let pasta = Pasta::nova("versao-ruim");
    let caminho = pasta.arquivo();
    for texto in ["versao = \"1\"\n", "filas = {}\n", ""] {
        fs::write(&caminho, texto).expect("escrita de teste");
        let erro = carregar(&caminho).expect_err("versão inválida deve falhar");
        assert!(
            matches!(erro, ErroPerfis::TomlInvalido { .. }),
            "texto {texto:?}"
        );
        assert_eq!(bytes_do_arquivo(&caminho), texto.as_bytes());
    }
}

#[test]
fn campo_desconhecido_em_entrada_e_rejeitado_sem_alterar_o_arquivo() {
    let pasta = Pasta::nova("campo-extra");
    let caminho = pasta.arquivo();
    let original = b"versao = 1\n\n[filas.\"x\"]\npassada1_inversa = true\npassada2_inversa = false\nverso_girar_180 = false\nvirar_maco = true\nborda_primeiro = \"curta\"\nchave_extra = 1\n".to_vec();
    fs::write(&caminho, &original).expect("escrita de teste");
    let erro = carregar(&caminho).expect_err("campo desconhecido deve falhar");
    assert!(matches!(erro, ErroPerfis::TomlInvalido { .. }));
    assert_eq!(bytes_do_arquivo(&caminho), original);
}

#[test]
fn arquivo_com_bytes_nao_utf8_gera_erro_sem_alterar_o_arquivo() {
    let pasta = Pasta::nova("utf8");
    let caminho = pasta.arquivo();
    let original = vec![b'v', 0xFF, 0xFE, b'\n'];
    fs::write(&caminho, &original).expect("escrita de teste");
    let erro = carregar(&caminho).expect_err("texto fora de UTF-8 deve falhar");
    assert!(matches!(erro, ErroPerfis::TomlInvalido { .. }));
    assert_eq!(bytes_do_arquivo(&caminho), original);
}

#[test]
fn arquivo_maior_que_o_limite_gera_erro_sem_alterar_o_arquivo() {
    let pasta = Pasta::nova("grande");
    let caminho = pasta.arquivo();
    let limite = usize::try_from(LIMITE_ARQUIVO_BYTES).expect("limite cabe em usize");
    let original = vec![b'#'; limite + 1];
    fs::write(&caminho, &original).expect("escrita de teste");
    let erro = carregar(&caminho).expect_err("arquivo grande deve falhar");
    assert!(matches!(erro, ErroPerfis::ArquivoGrande { .. }));
    assert_eq!(bytes_do_arquivo(&caminho), original);
}

#[test]
fn arquivo_exatamente_no_limite_e_aceito() {
    let pasta = Pasta::nova("limite");
    let caminho = pasta.arquivo();
    let limite = usize::try_from(LIMITE_ARQUIVO_BYTES).expect("limite cabe em usize");
    let mut texto = String::from("versao = 1\n#");
    let falta = limite - texto.len() - 1;
    texto.push_str(&"x".repeat(falta));
    texto.push('\n');
    assert_eq!(texto.len(), limite);
    fs::write(&caminho, texto.as_bytes()).expect("escrita de teste");
    let lidos = carregar(&caminho).expect("arquivo no limite deve ser aceito");
    assert!(lidos.is_empty());
}

#[test]
fn salvar_cria_a_pasta_inexistente() {
    let pasta = Pasta::nova("pasta-nova");
    let caminho = pasta.0.join("sub").join("pasta").join("perfis.toml");
    let mut perfis = Perfis::new();
    perfis.inserir("fila", entrada_completa());
    salvar(&caminho, &perfis).expect("gravação com pasta inexistente");
    assert_eq!(carregar(&caminho).expect("leitura"), perfis);
}

#[test]
fn salvar_deixa_so_o_arquivo_final_na_pasta() {
    let pasta = Pasta::nova("sem-temporario");
    let mut perfis = Perfis::new();
    perfis.inserir("fila", entrada_completa());
    salvar(&pasta.arquivo(), &perfis).expect("gravação");
    assert_eq!(nomes_na_pasta(&pasta), vec!["perfis.toml".to_owned()]);
}

#[test]
fn falha_de_escrita_preserva_o_arquivo_anterior() {
    let pasta = Pasta::nova("falha");
    let caminho = pasta.arquivo();
    let mut anteriores = Perfis::new();
    anteriores.inserir("antiga", entrada_completa());
    salvar(&caminho, &anteriores).expect("gravação inicial");
    let antes = bytes_do_arquivo(&caminho);

    let mut novos = Perfis::new();
    novos.inserir("nova", EntradaPerfil::default());
    FALHA_NA_GRAVACAO.with(|falha| falha.set(true));
    let erro = salvar(&caminho, &novos).expect_err("falha simulada deve propagar");
    assert!(matches!(erro, ErroPerfis::Gravacao { .. }));

    assert_eq!(bytes_do_arquivo(&caminho), antes);
    assert_eq!(nomes_na_pasta(&pasta), vec!["perfis.toml".to_owned()]);
}

#[test]
fn destino_que_e_pasta_falha_sem_deixar_temporario() {
    let pasta = Pasta::nova("destino-pasta");
    let caminho = pasta.0.join("perfis.toml");
    fs::create_dir_all(&caminho).expect("pasta no lugar do arquivo");
    let erro = salvar(&caminho, &Perfis::new()).expect_err("destino que é pasta deve falhar");
    assert!(matches!(erro, ErroPerfis::Gravacao { .. }));
    assert_eq!(nomes_na_pasta(&pasta), vec!["perfis.toml".to_owned()]);
}

#[test]
fn conteudo_acima_do_limite_nao_e_gravado() {
    let pasta = Pasta::nova("conteudo-grande");
    let caminho = pasta.arquivo();
    let mut perfis = Perfis::new();
    let apelido = "a".repeat(usize::try_from(LIMITE_ARQUIVO_BYTES).expect("limite cabe em usize"));
    perfis.inserir(
        "fila",
        EntradaPerfil {
            apelido: Some(apelido),
            ..EntradaPerfil::default()
        },
    );
    let erro = salvar(&caminho, &perfis).expect_err("conteúdo grande deve falhar");
    assert!(matches!(erro, ErroPerfis::ConteudoGrande));
    assert!(nomes_na_pasta(&pasta).is_empty());
}

#[test]
fn obter_inserir_e_remover_trabalham_por_nome_exato() {
    let mut perfis = Perfis::new();
    assert!(perfis.obter("fila").is_none());
    assert!(perfis.inserir("fila", entrada_completa()).is_none());
    assert_eq!(perfis.len(), 1);
    assert!(perfis.obter("Fila").is_none());

    let substituida = EntradaPerfil::default();
    assert_eq!(
        perfis.inserir("fila", substituida.clone()),
        Some(entrada_completa())
    );
    assert_eq!(perfis.obter("fila"), Some(&substituida));

    assert_eq!(perfis.remover("fila"), Some(substituida));
    assert!(perfis.remover("fila").is_none());
    assert!(perfis.is_empty());
}

#[test]
fn caminho_padrao_termina_em_plots_perfis_toml() {
    let caminho = caminho_padrao().expect("pasta do usuário disponível");
    assert_eq!(
        caminho.file_name().and_then(|nome| nome.to_str()),
        Some("perfis.toml")
    );
    assert_eq!(
        caminho
            .parent()
            .and_then(|pasta| pasta.file_name())
            .and_then(|nome| nome.to_str()),
        Some("plots")
    );
}
