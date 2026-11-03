use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use plots_core::{BordaEntrada, GestoRecolocar, ModeloImpressora, PerfilImpressora, ViradaMaco};

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

fn perfil_com(passadas: (bool, bool, bool), gesto: (ViradaMaco, BordaEntrada)) -> PerfilImpressora {
    PerfilImpressora {
        passada1_inversa: passadas.0,
        passada2_inversa: passadas.1,
        verso_girar_180: passadas.2,
        gesto_recolocar: GestoRecolocar {
            virada: gesto.0,
            borda_primeiro: gesto.1,
        },
    }
}

fn entrada_completa() -> EntradaPerfil {
    EntradaPerfil {
        apelido: Some("Jato — escritório".to_owned()),
        perfil: perfil_com(
            (true, false, true),
            (ViradaMaco::NaoVirar, BordaEntrada::Longa),
        ),
        gesto_texto: Some("Vire o maço e encoste a borda curta primeiro".to_owned()),
        modelo: None,
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
    for mascara in 0u8..48 {
        let passadas = (mascara & 1 != 0, mascara & 2 != 0, mascara & 4 != 0);
        let borda = if mascara & 8 != 0 {
            BordaEntrada::Longa
        } else {
            BordaEntrada::Curta
        };
        let virada = match mascara / 16 {
            0 => ViradaMaco::NaoVirar,
            1 => ViradaMaco::PelaLateral,
            _ => ViradaMaco::PelaBordaDeCima,
        };
        let entrada = EntradaPerfil {
            apelido: None,
            perfil: perfil_com(passadas, (virada, borda)),
            gesto_texto: None,
            modelo: None,
        };
        let fila = format!("fila-{mascara}");
        perfis.inserir(&fila, entrada.clone());
        esperados.push((fila, entrada));
    }
    salvar(&pasta.arquivo(), &perfis).expect("gravação");
    let lidos = carregar(&pasta.arquivo()).expect("leitura");
    assert_eq!(lidos.len(), 48);
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

const SEM_CAMPOS_NOVOS: &str = "versao = 1\n\n[filas.\"x\"]\npassada1_inversa = true\npassada2_inversa = false\nverso_girar_180 = false\nvirar_maco = true\nborda_primeiro = \"curta\"\n";

/// Grava o texto no arquivo de teste e devolve o que a leitura responde.
fn ler_texto(pasta: &Pasta, texto: &str) -> Result<Perfis, ErroPerfis> {
    fs::write(pasta.arquivo(), texto).expect("escrita do arquivo de teste");
    carregar(&pasta.arquivo())
}

/// Entrada de arquivo com o modelo e os valores dele, para variar um campo de cada vez.
fn texto_do_jato() -> String {
    "versao = 1\n\n[filas.\"x\"]\npassada1_inversa = true\npassada2_inversa = true\nverso_girar_180 = true\nvirar_maco = true\nborda_primeiro = \"curta\"\nborda_virada = \"cima\"\nmodelo = \"jato\"\n".to_owned()
}

#[test]
fn arquivo_sem_modelo_nem_borda_da_virada_continua_abrindo() {
    let pasta = Pasta::nova("antigo");
    let lidos = ler_texto(&pasta, SEM_CAMPOS_NOVOS).expect("leitura");
    let entrada = lidos.obter("x").expect("perfil da fila");
    assert_eq!(entrada.modelo, None);
    assert_eq!(
        entrada.perfil.gesto_recolocar.virada,
        ViradaMaco::PelaLateral
    );
    assert!(entrada.perfil.passada1_inversa);
}

#[test]
fn arquivo_antigo_regravado_nao_ganha_campos_nem_muda_de_versao() {
    let pasta = Pasta::nova("antigo-regravado");
    let lidos = ler_texto(&pasta, SEM_CAMPOS_NOVOS).expect("leitura");
    salvar(&pasta.arquivo(), &lidos).expect("gravação");
    let texto = fs::read_to_string(pasta.arquivo()).expect("leitura do texto");
    assert!(texto.starts_with("versao = 1"), "{texto}");
    assert!(!texto.contains("modelo"), "{texto}");
    assert!(!texto.contains("borda_virada"), "{texto}");
}

#[test]
fn modelo_e_virada_vao_e_voltam_nos_dois_modelos() {
    for (modelo, codigo) in [
        (ModeloImpressora::JatoDeTinta, "jato"),
        (ModeloImpressora::Laser, "laser"),
    ] {
        let pasta = Pasta::nova("modelo");
        let mut perfis = Perfis::new();
        perfis.definir_modelo("Fila", modelo);
        salvar(&pasta.arquivo(), &perfis).expect("gravação");
        let texto = fs::read_to_string(pasta.arquivo()).expect("leitura do texto");
        assert!(texto.contains(&format!("modelo = \"{codigo}\"")), "{texto}");
        assert_eq!(
            texto.contains("borda_virada = \"cima\""),
            modelo == ModeloImpressora::JatoDeTinta,
            "{texto}"
        );
        let lidos = carregar(&pasta.arquivo()).expect("leitura");
        assert_eq!(lidos, perfis);
        let entrada = lidos.obter("Fila").expect("perfil da fila");
        assert_eq!(entrada.modelo, Some(modelo));
        assert_eq!(entrada.perfil, modelo.perfil());
    }
}

#[test]
fn definir_modelo_mantem_o_apelido_e_tira_o_texto_do_gesto() {
    let mut perfis = Perfis::new();
    perfis.inserir("Fila", entrada_completa());
    perfis.definir_modelo("Fila", ModeloImpressora::Laser);
    let entrada = perfis.obter("Fila").expect("perfil da fila");
    assert_eq!(entrada.apelido, entrada_completa().apelido);
    assert_eq!(entrada.gesto_texto, None);
    assert_eq!(entrada.perfil, ModeloImpressora::Laser.perfil());
    assert_eq!(perfis.len(), 1);
}

/// O arquivo com a fila `x` variada e, ao lado dela, uma fila `y` boa que não pode ser afetada.
fn com_fila_boa_ao_lado(texto_de_x: &str) -> String {
    let fila_y = texto_do_jato().replace("[filas.\"x\"]", "[filas.\"y\"]");
    let fila_y = fila_y.trim_start_matches("versao = 1\n");
    format!("{texto_de_x}{fila_y}")
}

/// A fila `x` abriu sem modelo, com os valores do arquivo, e a fila `y` continua como estava.
fn confere_x_personalizada(lidos: &Perfis, perfil_de_x: PerfilImpressora) {
    let x = lidos.obter("x").expect("perfil da fila x");
    assert_eq!(x.modelo, None);
    assert_eq!(x.perfil, perfil_de_x);
    let y = lidos.obter("y").expect("perfil da fila y");
    assert_eq!(y.modelo, Some(ModeloImpressora::JatoDeTinta));
    assert_eq!(y.perfil, ModeloImpressora::JatoDeTinta.perfil());
}

#[test]
fn modelo_desconhecido_vira_perfil_personalizado_sem_alterar_o_arquivo() {
    let pasta = Pasta::nova("modelo-desconhecido");
    let casos = [
        texto_do_jato().replace("\"jato\"", "\"matricial\""),
        texto_do_jato().replace("\"jato\"", "\"Jato\""),
        texto_do_jato().replace("modelo = \"jato\"", "modelo = 1"),
    ];
    for texto_de_x in casos {
        let texto = com_fila_boa_ao_lado(&texto_de_x);
        let lidos = ler_texto(&pasta, &texto).expect(&texto);
        confere_x_personalizada(&lidos, ModeloImpressora::JatoDeTinta.perfil());
        assert_eq!(fs::read_to_string(pasta.arquivo()).unwrap(), texto);
    }
}

#[test]
fn modelo_com_valores_de_outro_vira_perfil_personalizado_com_os_valores_gravados() {
    let pasta = Pasta::nova("modelo-contraditorio");
    let jato = ModeloImpressora::JatoDeTinta.perfil();
    let casos = [
        (
            texto_do_jato().replace("modelo = \"jato\"", "modelo = \"laser\""),
            jato,
        ),
        (
            texto_do_jato().replace("verso_girar_180 = true", "verso_girar_180 = false"),
            PerfilImpressora {
                verso_girar_180: false,
                ..jato
            },
        ),
        (
            texto_do_jato().replace("borda_virada = \"cima\"\n", ""),
            PerfilImpressora {
                gesto_recolocar: GestoRecolocar {
                    virada: ViradaMaco::PelaLateral,
                    ..jato.gesto_recolocar
                },
                ..jato
            },
        ),
        (
            texto_do_jato().replace("borda_primeiro = \"curta\"", "borda_primeiro = \"longa\""),
            PerfilImpressora {
                gesto_recolocar: GestoRecolocar {
                    borda_primeiro: BordaEntrada::Longa,
                    ..jato.gesto_recolocar
                },
                ..jato
            },
        ),
    ];
    for (texto_de_x, esperado) in casos {
        let texto = com_fila_boa_ao_lado(&texto_de_x);
        let lidos = ler_texto(&pasta, &texto).expect(&texto);
        confere_x_personalizada(&lidos, esperado);
    }
}

#[test]
fn borda_da_virada_em_perfil_que_nao_vira_e_tratada_como_nao_virar() {
    let pasta = Pasta::nova("sem-virar-com-borda");
    let texto = SEM_CAMPOS_NOVOS.replace(
        "virar_maco = true\n",
        "virar_maco = false\nborda_virada = \"cima\"\n",
    );
    let lidos = ler_texto(&pasta, &texto).expect(&texto);
    let entrada = lidos.obter("x").expect("perfil da fila");
    assert_eq!(entrada.perfil.gesto_recolocar.virada, ViradaMaco::NaoVirar);
    assert_eq!(entrada.modelo, None);
    salvar(&pasta.arquivo(), &lidos).expect("gravação");
    let regravado = fs::read_to_string(pasta.arquivo()).expect("leitura do texto");
    assert!(!regravado.contains("borda_virada"), "{regravado}");
}

/// Entrada com os valores e o modelo do laser, que não vira o maço.
fn texto_do_laser() -> String {
    let laser = ModeloImpressora::Laser.perfil();
    format!(
        "versao = 1\n\n[filas.\"x\"]\npassada1_inversa = {}\npassada2_inversa = {}\nverso_girar_180 = {}\nvirar_maco = false\nborda_primeiro = \"curta\"\nmodelo = \"laser\"\n",
        laser.passada1_inversa, laser.passada2_inversa, laser.verso_girar_180
    )
}

#[test]
fn borda_da_virada_invalida_abre_sem_modelo_com_as_outras_filas_intactas() {
    let pasta = Pasta::nova("borda-da-virada-invalida");
    let jato = ModeloImpressora::JatoDeTinta.perfil();
    let pela_lateral = PerfilImpressora {
        gesto_recolocar: GestoRecolocar {
            virada: ViradaMaco::PelaLateral,
            ..jato.gesto_recolocar
        },
        ..jato
    };
    let sem_borda = texto_do_laser().replace(
        "virar_maco = false\n",
        "virar_maco = false\nborda_virada = \"baixo\"\n",
    );
    let casos = [
        (
            texto_do_jato().replace("\"cima\"", "\"baixo\""),
            pela_lateral,
        ),
        (
            texto_do_jato().replace("borda_virada = \"cima\"", "borda_virada = 1"),
            pela_lateral,
        ),
        (sem_borda, ModeloImpressora::Laser.perfil()),
    ];
    for (texto_de_x, esperado) in casos {
        let texto = com_fila_boa_ao_lado(&texto_de_x);
        let lidos = ler_texto(&pasta, &texto).expect(&texto);
        confere_x_personalizada(&lidos, esperado);
        assert_eq!(fs::read_to_string(pasta.arquivo()).unwrap(), texto);
    }
}

#[test]
fn borda_de_entrada_invalida_abre_sem_modelo_com_as_outras_filas_intactas() {
    let pasta = Pasta::nova("borda-de-entrada-invalida");
    let casos = [
        texto_do_jato().replace(
            "borda_primeiro = \"curta\"",
            "borda_primeiro = \"diagonal\"",
        ),
        texto_do_jato().replace("borda_primeiro = \"curta\"", "borda_primeiro = 1"),
    ];
    for texto_de_x in casos {
        let texto = com_fila_boa_ao_lado(&texto_de_x);
        let lidos = ler_texto(&pasta, &texto).expect(&texto);
        confere_x_personalizada(&lidos, ModeloImpressora::JatoDeTinta.perfil());
        assert_eq!(fs::read_to_string(pasta.arquivo()).unwrap(), texto);
    }
}

#[test]
fn entrada_com_borda_invalida_regravada_fica_com_valores_validos() {
    let pasta = Pasta::nova("borda-invalida-regravada");
    let texto = texto_do_jato().replace("\"cima\"", "\"baixo\"").replace(
        "borda_primeiro = \"curta\"",
        "borda_primeiro = \"diagonal\"",
    );
    let lidos = ler_texto(&pasta, &texto).expect(&texto);
    salvar(&pasta.arquivo(), &lidos).expect("gravação");
    let regravado = fs::read_to_string(pasta.arquivo()).expect("leitura do texto");
    assert!(!regravado.contains("baixo"), "{regravado}");
    assert!(!regravado.contains("diagonal"), "{regravado}");
    assert_eq!(carregar(&pasta.arquivo()).expect("leitura"), lidos);
}

#[test]
fn borda_de_entrada_ausente_continua_recusando_o_arquivo() {
    let pasta = Pasta::nova("borda-de-entrada-ausente");
    let texto = texto_do_jato().replace("borda_primeiro = \"curta\"\n", "");
    let erro = ler_texto(&pasta, &texto).expect_err(&texto);
    assert!(matches!(erro, ErroPerfis::TomlInvalido { .. }), "{erro:?}");
}

#[test]
fn definir_modelo_conserta_uma_entrada_incoerente() {
    let pasta = Pasta::nova("conserta-incoerente");
    let mut perfis = Perfis::new();
    perfis.inserir(
        "Fila",
        EntradaPerfil {
            modelo: Some(ModeloImpressora::Laser),
            apelido: Some("Sala".to_owned()),
            ..EntradaPerfil::default()
        },
    );
    salvar(&pasta.arquivo(), &perfis).expect_err("gravação recusada antes do conserto");
    perfis.definir_modelo("Fila", ModeloImpressora::Laser);
    salvar(&pasta.arquivo(), &perfis).expect("gravação depois do conserto");
    let lidos = carregar(&pasta.arquivo()).expect("leitura");
    let entrada = lidos.obter("Fila").expect("perfil da fila");
    assert_eq!(entrada.modelo, Some(ModeloImpressora::Laser));
    assert_eq!(entrada.perfil, ModeloImpressora::Laser.perfil());
    assert_eq!(entrada.apelido.as_deref(), Some("Sala"));
}

#[test]
fn entrada_incoerente_lida_do_arquivo_pode_ser_consertada_e_regravada() {
    let pasta = Pasta::nova("conserta-do-arquivo");
    let texto =
        com_fila_boa_ao_lado(&texto_do_jato().replace("modelo = \"jato\"", "modelo = \"laser\""));
    let mut lidos = ler_texto(&pasta, &texto).expect(&texto);
    lidos.definir_modelo("x", ModeloImpressora::Laser);
    salvar(&pasta.arquivo(), &lidos).expect("gravação");
    let relidos = carregar(&pasta.arquivo()).expect("leitura");
    let x = relidos.obter("x").expect("perfil da fila x");
    assert_eq!(x.modelo, Some(ModeloImpressora::Laser));
    assert_eq!(x.perfil, ModeloImpressora::Laser.perfil());
    let y = relidos.obter("y").expect("perfil da fila y");
    assert_eq!(y.modelo, Some(ModeloImpressora::JatoDeTinta));
}

#[test]
fn nao_grava_modelo_com_valores_de_outro() {
    let pasta = Pasta::nova("gravar-contraditorio");
    let mut perfis = Perfis::new();
    perfis.inserir(
        "Fila",
        EntradaPerfil {
            modelo: Some(ModeloImpressora::Laser),
            ..EntradaPerfil::default()
        },
    );
    let erro = salvar(&pasta.arquivo(), &perfis).expect_err("gravação recusada");
    assert!(matches!(erro, ErroPerfis::Serializacao { .. }), "{erro:?}");
    assert!(!pasta.arquivo().exists());
}
