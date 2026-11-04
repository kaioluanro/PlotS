//! Os PDFs recebidos de uma fila. Só mexe nos arquivos que a recepção grava, pelo formato do nome.

use std::fs;
use std::io;
use std::path::Path;

/// Maior número de arquivos olhados de uma vez. A pasta pode ter sido mexida por fora.
const LIMITE_ARQUIVOS: usize = 1_000_000;

fn e_do_historico(nome: &str) -> bool {
    nome.strip_suffix(".pdf")
        .is_some_and(|base| !base.is_empty() && base.bytes().all(|b| b.is_ascii_digit()))
}

/// Quantos PDFs recebidos e quantos bytes. Sem pasta, não há nada.
pub fn contar(pasta: &Path) -> io::Result<(u64, u64)> {
    let mut arquivos = 0u64;
    let mut bytes = 0u64;
    percorrer(pasta, |_, tamanho| {
        arquivos = arquivos.saturating_add(1);
        bytes = bytes.saturating_add(tamanho);
        Ok(())
    })?;
    Ok((arquivos, bytes))
}

/// Apaga os PDFs recebidos e devolve quantos. A pasta fica, porque a recepção segue gravando nela.
pub fn limpar(pasta: &Path) -> io::Result<u64> {
    let mut apagados = 0u64;
    percorrer(pasta, |caminho, _| {
        fs::remove_file(caminho)?;
        apagados = apagados.saturating_add(1);
        Ok(())
    })?;
    Ok(apagados)
}

fn percorrer(pasta: &Path, mut acao: impl FnMut(&Path, u64) -> io::Result<()>) -> io::Result<()> {
    let leitura = match fs::read_dir(pasta) {
        Ok(leitura) => leitura,
        Err(erro) if erro.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(erro) => return Err(erro),
    };
    for entrada in leitura.take(LIMITE_ARQUIVOS) {
        let entrada = entrada?;
        let Some(nome) = entrada.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !e_do_historico(&nome) {
            continue;
        }
        // Atalho ou pasta com nome de PDF não é algo que a recepção gravou.
        let dados = fs::symlink_metadata(entrada.path())?;
        if dados.is_file() {
            acao(&entrada.path(), dados.len())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static CONTADOR: AtomicU32 = AtomicU32::new(0);

    struct Pasta(PathBuf);

    impl Pasta {
        fn nova() -> Self {
            let numero = CONTADOR.fetch_add(1, Ordering::Relaxed);
            let caminho = std::env::temp_dir()
                .join(format!("plotsim-historico-{}-{numero}", std::process::id()));
            fs::create_dir_all(&caminho).unwrap();
            Self(caminho)
        }
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn limpar_apaga_so_o_que_a_recepcao_grava() {
        let pasta = Pasta::nova();
        fs::write(pasta.0.join("00000001.pdf"), b"abc").unwrap();
        fs::write(pasta.0.join("00000002.pdf"), b"de").unwrap();
        fs::write(pasta.0.join("meu.pdf"), b"fica").unwrap();
        fs::write(pasta.0.join("00000003.txt"), b"fica").unwrap();
        fs::create_dir(pasta.0.join("00000004.pdf")).unwrap();
        assert_eq!(contar(&pasta.0).unwrap(), (2, 5));
        assert_eq!(limpar(&pasta.0).unwrap(), 2);
        assert_eq!(contar(&pasta.0).unwrap(), (0, 0));
        assert!(pasta.0.join("meu.pdf").exists());
        assert!(pasta.0.join("00000003.txt").exists());
        assert!(pasta.0.join("00000004.pdf").is_dir());
    }

    #[test]
    fn pasta_que_nao_existe_nao_e_erro() {
        let pasta = Pasta::nova();
        let ausente = pasta.0.join("nao-existe");
        assert_eq!(contar(&ausente).unwrap(), (0, 0));
        assert_eq!(limpar(&ausente).unwrap(), 0);
    }

    #[test]
    fn nome_do_historico_e_so_numero_com_a_extensao() {
        assert!(e_do_historico("00000001.pdf"));
        assert!(!e_do_historico(".pdf"));
        assert!(!e_do_historico("1a.pdf"));
        assert!(!e_do_historico("1.PDF"));
        assert!(!e_do_historico("1.pdf.tmp"));
    }
}
