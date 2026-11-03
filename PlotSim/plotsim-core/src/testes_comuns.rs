//! Apoio dos testes de unidade.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static CONTADOR: AtomicU64 = AtomicU64::new(0);

/// Pasta só deste teste, apagada ao terminar.
pub(crate) struct PastaTemporaria {
    raiz: PathBuf,
}

impl PastaTemporaria {
    pub(crate) fn nova() -> Self {
        let raiz = std::env::temp_dir().join(format!(
            "plotsim-{}-{}",
            std::process::id(),
            CONTADOR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&raiz).unwrap();
        Self { raiz }
    }

    pub(crate) fn raiz(&self) -> &Path {
        &self.raiz
    }

    pub(crate) fn caminho(&self, relativo: &str) -> PathBuf {
        relativo
            .split('/')
            .fold(self.raiz.clone(), |caminho, parte| caminho.join(parte))
    }
}

impl Drop for PastaTemporaria {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.raiz);
    }
}
