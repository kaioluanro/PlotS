//! A opção "o driver inverte a ordem". Na fila real, fica no perfil dela. Na simulada, só existe no
//! modo de teste: ela passa a imitar um driver que inverte, e a opção vive só enquanto o programa
//! estiver aberto, sem tocar no arquivo de perfis.

use crate::impressoras::{self, GRAVACAO_DE_PERFIS};
use crate::trabalhos::DriverDaSimulada;
use plots_perfis::ErroPerfis;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

/// Com esta variável preenchida, as simuladas imitam um driver que inverte a ordem.
pub const VARIAVEL_TESTE: &str = "PLOTS_TESTE_DRIVER_INVERTE";

static SIMULADAS_COM_A_OPCAO: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

pub fn modo_de_teste() -> bool {
    std::env::var_os(VARIAVEL_TESTE).is_some_and(|valor| !valor.is_empty())
}

pub fn simulada_compensa(id: &str) -> bool {
    SIMULADAS_COM_A_OPCAO
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains(id)
}

pub fn driver_da_simulada(id: &str) -> DriverDaSimulada {
    let inverte = modo_de_teste();
    DriverDaSimulada {
        inverte,
        compensa: inverte && simulada_compensa(id),
    }
}

/// Por que a opção não foi gravada; a interface escolhe a frase. Nada muda no arquivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FalhaDaInversao {
    SemPerfil,
    Gravacao,
}

/// Grava a opção no perfil desta fila. Sem perfil, ou com o arquivo ilegível, nada é escrito.
pub(crate) fn gravar_inversao(
    caminho: Result<PathBuf, ErroPerfis>,
    fila: &str,
    inverte: bool,
) -> Result<(), FalhaDaInversao> {
    let _vez = GRAVACAO_DE_PERFIS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let caminho = caminho.map_err(|_| FalhaDaInversao::Gravacao)?;
    let mut perfis = plots_perfis::carregar(&caminho).map_err(|_| FalhaDaInversao::Gravacao)?;
    if !perfis.definir_driver_inverte(fila, inverte) {
        return Err(FalhaDaInversao::SemPerfil);
    }
    plots_perfis::salvar(&caminho, &perfis).map_err(|_| FalhaDaInversao::Gravacao)
}

/// A interface manda o identificador da linha. O disco pode demorar: a thread é própria.
#[tauri::command(async)]
pub fn definir_inversao(impressora: String, inverte: bool) -> Result<(), FalhaDaInversao> {
    if impressora.starts_with("simulada:") {
        if !modo_de_teste() {
            return Err(FalhaDaInversao::SemPerfil);
        }
        let mut simuladas = SIMULADAS_COM_A_OPCAO
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if inverte {
            simuladas.insert(impressora);
        } else {
            simuladas.remove(&impressora);
        }
        return Ok(());
    }
    let fila = impressora.strip_prefix("fila:").unwrap_or(&impressora);
    gravar_inversao(impressoras::caminho_dos_perfis_do_sistema(), fila, inverte)
}

#[cfg(test)]
mod testes {
    use super::*;
    use plots_core::ModeloImpressora;
    use plots_perfis::Perfis;
    use std::sync::atomic::{AtomicU64, Ordering};

    static PROXIMO: AtomicU64 = AtomicU64::new(0);

    struct Pasta(PathBuf);

    impl Pasta {
        fn nova() -> Self {
            let numero = PROXIMO.fetch_add(1, Ordering::Relaxed);
            let pasta = std::env::temp_dir()
                .join(format!("plots-inversao-{}-{numero}", std::process::id()));
            std::fs::create_dir_all(&pasta).unwrap();
            Self(pasta)
        }
        fn perfis(&self) -> PathBuf {
            self.0.join("perfis.toml")
        }
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_opcao_vai_para_o_perfil_da_fila_e_volta_ao_ler() {
        let pasta = Pasta::nova();
        let mut perfis = Perfis::new();
        perfis.definir_modelo("Sala", ModeloImpressora::Laser);
        plots_perfis::salvar(&pasta.perfis(), &perfis).unwrap();

        gravar_inversao(Ok(pasta.perfis()), "Sala", true).unwrap();
        let lido = plots_perfis::carregar(&pasta.perfis()).unwrap();
        assert!(lido.obter("Sala").unwrap().driver_inverte);

        gravar_inversao(Ok(pasta.perfis()), "Sala", false).unwrap();
        let lido = plots_perfis::carregar(&pasta.perfis()).unwrap();
        assert!(!lido.obter("Sala").unwrap().driver_inverte);
    }

    #[test]
    fn sem_perfil_nada_e_gravado() {
        let pasta = Pasta::nova();
        let mut perfis = Perfis::new();
        perfis.definir_modelo("Sala", ModeloImpressora::Laser);
        plots_perfis::salvar(&pasta.perfis(), &perfis).unwrap();
        let antes = std::fs::read(pasta.perfis()).unwrap();
        assert_eq!(
            gravar_inversao(Ok(pasta.perfis()), "Outra", true),
            Err(FalhaDaInversao::SemPerfil)
        );
        assert_eq!(std::fs::read(pasta.perfis()).unwrap(), antes);
    }

    #[test]
    fn arquivo_danificado_nao_e_sobrescrito() {
        let pasta = Pasta::nova();
        std::fs::write(pasta.perfis(), "isto nao e toml [").unwrap();
        assert_eq!(
            gravar_inversao(Ok(pasta.perfis()), "Sala", true),
            Err(FalhaDaInversao::Gravacao)
        );
        assert_eq!(
            std::fs::read_to_string(pasta.perfis()).unwrap(),
            "isto nao e toml ["
        );
    }

    #[test]
    fn fora_do_modo_de_teste_a_simulada_nao_inverte_nem_compensa() {
        // A variável nunca é preenchida nos testes: ligá-la aqui atingiria as outras threads.
        assert!(!modo_de_teste());
        assert_eq!(
            driver_da_simulada("simulada:gaveta"),
            DriverDaSimulada::default()
        );
    }
}
