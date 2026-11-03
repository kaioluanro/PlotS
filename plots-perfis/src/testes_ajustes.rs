//! Ajustes do driver no arquivo de perfis: formato, teto e convivência com o resto.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use plots_core::ModeloImpressora;

use crate::ajustes::{decodificar_base64, LIMITE_AJUSTE_BYTES};
use crate::{carregar, salvar, AjusteRecusado, EntradaPerfil, Perfis};

static CONTADOR: AtomicU64 = AtomicU64::new(0);

struct Pasta(PathBuf);

impl Pasta {
    fn nova() -> Self {
        let sequencia = CONTADOR.fetch_add(1, Ordering::Relaxed);
        let caminho = std::env::temp_dir().join(format!(
            "plots-perfis-ajustes-{}-{sequencia}",
            std::process::id()
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

fn todos_os_bytes() -> Vec<u8> {
    (0..=255u8).chain((0..=255u8).rev()).collect()
}

#[test]
fn ajuste_vai_ao_arquivo_e_volta_igual_em_qualquer_tamanho() {
    let pasta = Pasta::nova();
    let mut perfis = Perfis::new();
    for tamanho in [1usize, 2, 3, 4, 511, 512] {
        let bytes: Vec<u8> = todos_os_bytes().into_iter().take(tamanho).collect();
        let fila = format!("Fila {tamanho} — \"sala\" \\ ç");
        perfis.definir_ajuste_do_driver(&fila, bytes).unwrap();
    }
    salvar(&pasta.arquivo(), &perfis).unwrap();
    let lidos = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(lidos, perfis);
    assert!(lidos.ajustes_ignorados().is_empty());
}

#[test]
fn ajuste_salvo_nao_faz_a_fila_parecer_ter_perfil() {
    let mut perfis = Perfis::new();
    perfis
        .definir_ajuste_do_driver("Sala", vec![1, 2, 3])
        .unwrap();
    assert!(perfis.obter("Sala").is_none());
    assert!(perfis.is_empty());
    assert_eq!(perfis.ajuste_do_driver("Sala"), Some(&[1u8, 2, 3][..]));
}

#[test]
fn escolher_modelo_ou_remover_o_perfil_nao_apaga_o_ajuste_da_fila() {
    let mut perfis = Perfis::new();
    perfis.definir_ajuste_do_driver("Sala", vec![9]).unwrap();
    perfis.definir_modelo("Sala", ModeloImpressora::Laser);
    assert_eq!(perfis.ajuste_do_driver("Sala"), Some(&[9u8][..]));
    perfis.remover("Sala");
    assert_eq!(perfis.ajuste_do_driver("Sala"), Some(&[9u8][..]));
}

#[test]
fn remover_o_ajuste_nao_mexe_no_perfil_da_mesma_fila() {
    let pasta = Pasta::nova();
    let mut perfis = Perfis::new();
    perfis.definir_modelo("Sala", ModeloImpressora::Laser);
    perfis.definir_ajuste_do_driver("Sala", vec![9]).unwrap();
    assert_eq!(perfis.remover_ajuste_do_driver("Sala"), Some(vec![9]));
    assert_eq!(perfis.remover_ajuste_do_driver("Sala"), None);
    salvar(&pasta.arquivo(), &perfis).unwrap();
    let texto = fs::read_to_string(pasta.arquivo()).unwrap();
    assert!(!texto.contains("ajustes_driver"), "{texto}");
    let lidos = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(
        lidos.obter("Sala").and_then(|entrada| entrada.modelo),
        Some(ModeloImpressora::Laser)
    );
}

#[test]
fn arquivo_antigo_sem_ajustes_continua_carregando() {
    let pasta = Pasta::nova();
    fs::write(
        pasta.arquivo(),
        "versao = 1\n\n[filas.Sala]\npassada1_inversa = true\npassada2_inversa = false\n\
         verso_girar_180 = false\nvirar_maco = false\nborda_primeiro = \"curta\"\n",
    )
    .unwrap();
    let lidos = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(lidos.len(), 1);
    assert_eq!(lidos.ajuste_do_driver("Sala"), None);
    assert_eq!(lidos.filas_com_ajuste().count(), 0);
}

#[test]
fn ajuste_corrompido_e_ignorado_com_aviso_sem_perder_o_resto() {
    let pasta = Pasta::nova();
    fs::write(
        pasta.arquivo(),
        "versao = 1\n\n[filas.Sala]\npassada1_inversa = true\npassada2_inversa = false\n\
         verso_girar_180 = false\nvirar_maco = false\nborda_primeiro = \"curta\"\n\n\
         [ajustes_driver]\nBoa = \"AQID\"\nTexto = \"nao e base64!\"\nNumero = 12\nVazia = \"\"\n",
    )
    .unwrap();
    let lidos = carregar(&pasta.arquivo()).unwrap();
    assert!(lidos.obter("Sala").is_some());
    assert_eq!(lidos.ajuste_do_driver("Boa"), Some(&[1u8, 2, 3][..]));
    assert_eq!(lidos.ajuste_do_driver("Texto"), None);
    assert_eq!(lidos.ajuste_do_driver("Numero"), None);
    assert_eq!(lidos.ajuste_do_driver("Vazia"), None);
    assert_eq!(lidos.ajustes_ignorados(), ["Numero", "Texto", "Vazia"]);
}

#[test]
fn gravar_de_novo_mantem_as_outras_filas_quando_um_ajuste_estava_corrompido() {
    let pasta = Pasta::nova();
    fs::write(
        pasta.arquivo(),
        "versao = 1\n\n[ajustes_driver]\nBoa = \"AQID\"\nRuim = \"###\"\n",
    )
    .unwrap();
    let lidos = carregar(&pasta.arquivo()).unwrap();
    salvar(&pasta.arquivo(), &lidos).unwrap();
    let relidos = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(relidos.ajuste_do_driver("Boa"), Some(&[1u8, 2, 3][..]));
    assert!(relidos.ajustes_ignorados().is_empty());
}

#[test]
fn ajuste_no_teto_passa_e_acima_dele_e_recusado_sem_mudar_o_anterior() {
    let mut perfis = Perfis::new();
    let no_teto = vec![7u8; LIMITE_AJUSTE_BYTES];
    perfis
        .definir_ajuste_do_driver("Sala", no_teto.clone())
        .unwrap();
    let acima = vec![7u8; LIMITE_AJUSTE_BYTES + 1];
    assert_eq!(
        perfis.definir_ajuste_do_driver("Sala", acima),
        Err(AjusteRecusado::Grande)
    );
    assert_eq!(perfis.ajuste_do_driver("Sala"), Some(no_teto.as_slice()));
    assert_eq!(
        perfis.definir_ajuste_do_driver("Sala", Vec::new()),
        Err(AjusteRecusado::Vazio)
    );
}

#[test]
fn texto_acima_do_teto_nao_e_decodificado_no_arquivo() {
    let pasta = Pasta::nova();
    let grande = "A".repeat((LIMITE_AJUSTE_BYTES / 3 + 1) * 4);
    fs::write(
        pasta.arquivo(),
        format!("versao = 1\n\n[ajustes_driver]\nSala = \"{grande}\"\n"),
    )
    .unwrap();
    let lidos = carregar(&pasta.arquivo()).unwrap();
    assert_eq!(lidos.ajuste_do_driver("Sala"), None);
    assert_eq!(lidos.ajustes_ignorados(), ["Sala"]);
}

#[test]
fn muitos_ajustes_acima_do_limite_do_arquivo_nao_apagam_o_que_ja_estava_gravado() {
    let pasta = Pasta::nova();
    let mut perfis = Perfis::new();
    perfis.inserir("Sala", EntradaPerfil::default());
    salvar(&pasta.arquivo(), &perfis).unwrap();
    let antes = fs::read(pasta.arquivo()).unwrap();
    for numero in 0..30 {
        perfis
            .definir_ajuste_do_driver(&format!("Fila {numero}"), vec![1u8; LIMITE_AJUSTE_BYTES])
            .unwrap();
    }
    assert!(salvar(&pasta.arquivo(), &perfis).is_err());
    assert_eq!(fs::read(pasta.arquivo()).unwrap(), antes);
}

#[test]
fn base64_recusa_o_que_nao_e_canonico() {
    assert_eq!(decodificar_base64("AQID"), Some(vec![1, 2, 3]));
    assert_eq!(decodificar_base64("AQI="), Some(vec![1, 2]));
    assert_eq!(decodificar_base64("AQ=="), Some(vec![1]));
    for ruim in [
        "", "A", "AQ", "AQI", "AQ=", "A===", "====", "=AQI", "A=QI", "AQ=I", "AQID ", " AQID",
        "AQ\nID", "AQ==AQID", "AR==", "AQJ=", "çQID", "AQ-_",
    ] {
        assert_eq!(decodificar_base64(ruim), None, "{ruim:?}");
    }
}

#[test]
fn ajuste_acima_de_32_kib_e_recusado() {
    let mut perfis = Perfis::new();
    assert_eq!(LIMITE_AJUSTE_BYTES, 32 * 1024);
    assert_eq!(
        perfis.definir_ajuste_do_driver("Sala", vec![1u8; 32 * 1024 + 1]),
        Err(AjusteRecusado::Grande)
    );
    assert_eq!(
        perfis.definir_ajuste_do_driver("Sala", vec![1u8; 32 * 1024]),
        Ok(())
    );
}
