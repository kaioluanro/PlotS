//! O que só existe no Windows, em um lugar só: fora dele, tudo responde que não há suporte.

use plotsim_core::leitor::{ErroLeitura, PaginaMarcada};
use plotsim_core::spooler::Spooler;
use plotsim_core::SistemaDeFilas;
use std::path::Path;

#[cfg(windows)]
pub fn sistema_de_filas() -> impl SistemaDeFilas {
    plotsim_core::powershell::PowerShell::do_sistema()
}

#[cfg(windows)]
pub fn spooler() -> impl Spooler + Send + 'static {
    plotsim_core::spooler::SpoolerDoWindows
}

/// A marca de cada página do arquivo, lida pelo núcleo, que usa a mesma biblioteca de PDF da janela.
#[cfg(windows)]
pub fn ler_marcas(arquivo: &Path) -> Result<Vec<PaginaMarcada>, ErroLeitura> {
    plotsim_core::leitor::ler_marcas(arquivo)
}

#[cfg(not(windows))]
mod sem_suporte {
    use plotsim_core::spooler::{ErroSpooler, Spooler, TrabalhoNaFila};
    use plotsim_core::{ErroSistema, FilaDoSistema, SistemaDeFilas};

    pub struct SemSistema;

    impl SistemaDeFilas for SemSistema {
        fn filas(&self) -> Result<Vec<FilaDoSistema>, ErroSistema> {
            Err(ErroSistema::NaoSuportado)
        }
        fn porta_existe(&self, _: &str) -> Result<bool, ErroSistema> {
            Err(ErroSistema::NaoSuportado)
        }
        fn criar_porta(&self, _: &str) -> Result<(), ErroSistema> {
            Err(ErroSistema::NaoSuportado)
        }
        fn criar_fila(&self, _: &str, _: &str, _: &str) -> Result<(), ErroSistema> {
            Err(ErroSistema::NaoSuportado)
        }
        fn remover_fila(&self, _: &str) -> Result<(), ErroSistema> {
            Err(ErroSistema::NaoSuportado)
        }
        fn remover_porta(&self, _: &str) -> Result<(), ErroSistema> {
            Err(ErroSistema::NaoSuportado)
        }
    }

    impl Spooler for SemSistema {
        fn trabalhos(&self, _: &str) -> Result<Vec<TrabalhoNaFila>, ErroSpooler> {
            Err(ErroSpooler::NaoSuportado)
        }
    }
}

#[cfg(not(windows))]
pub fn sistema_de_filas() -> impl SistemaDeFilas {
    sem_suporte::SemSistema
}

#[cfg(not(windows))]
pub fn spooler() -> impl Spooler + Send + 'static {
    sem_suporte::SemSistema
}

#[cfg(not(windows))]
pub fn ler_marcas(_: &Path) -> Result<Vec<PaginaMarcada>, ErroLeitura> {
    Err(ErroLeitura::Biblioteca("só existe no Windows".to_string()))
}
