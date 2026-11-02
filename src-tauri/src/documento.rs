//! Validacao do caminho de um documento antes de ele virar uma aba.

use serde::Serialize;
use std::io::ErrorKind;
use std::path::Path;

/// Documento aceito para abrir em uma aba.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Documento {
    pub caminho: String,
    pub nome: String,
}

/// Motivo da recusa. A interface traduz o codigo; o nome identifica o arquivo na mensagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodigoErro {
    NaoEncontrado,
    NaoEArquivo,
    NaoEPdf,
    Inacessivel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErroDocumento {
    pub codigo: CodigoErro,
    pub nome: String,
}

/// Aceita o caminho quando ele existe, e um arquivo e tem extensao `.pdf`, em qualquer caixa.
pub fn abrir(caminho: &str) -> Result<Documento, ErroDocumento> {
    let alvo = Path::new(caminho);
    let nome = nome_de_exibicao(caminho);
    let erro = |codigo| ErroDocumento {
        codigo,
        nome: nome.clone(),
    };

    let metadados = std::fs::metadata(alvo).map_err(|falha| {
        erro(match falha.kind() {
            ErrorKind::NotFound => CodigoErro::NaoEncontrado,
            _ => CodigoErro::Inacessivel,
        })
    })?;
    if !metadados.is_file() {
        return Err(erro(CodigoErro::NaoEArquivo));
    }
    let eh_pdf = alvo
        .extension()
        .is_some_and(|extensao| extensao.eq_ignore_ascii_case("pdf"));
    if !eh_pdf {
        return Err(erro(CodigoErro::NaoEPdf));
    }

    Ok(Documento {
        caminho: caminho.to_owned(),
        nome: nome.clone(),
    })
}

/// Ultimo componente do caminho; sem ele (raiz de disco, texto vazio) devolve o caminho inteiro.
fn nome_de_exibicao(caminho: &str) -> String {
    Path::new(caminho).file_name().map_or_else(
        || caminho.to_owned(),
        |nome| nome.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static CONTADOR: AtomicU32 = AtomicU32::new(0);

    /// Pasta temporaria exclusiva do teste, removida ao sair de escopo.
    struct PastaTemporaria(PathBuf);

    impl PastaTemporaria {
        fn nova() -> Self {
            let numero = CONTADOR.fetch_add(1, Ordering::Relaxed);
            let pasta = std::env::temp_dir().join(format!(
                "plots-app-documento-{}-{numero}",
                std::process::id()
            ));
            fs::create_dir_all(&pasta).expect("criar pasta temporaria");
            Self(pasta)
        }

        fn arquivo(&self, nome: &str) -> String {
            let caminho = self.0.join(nome);
            fs::write(&caminho, b"%PDF-1.7\n").expect("criar arquivo");
            caminho.to_string_lossy().into_owned()
        }
    }

    impl Drop for PastaTemporaria {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn aceita_arquivo_pdf_e_devolve_o_nome_de_exibicao() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.arquivo("relatorio.pdf");
        assert_eq!(
            abrir(&caminho),
            Ok(Documento {
                caminho: caminho.clone(),
                nome: "relatorio.pdf".to_owned(),
            })
        );
    }

    #[test]
    fn aceita_extensao_em_maiusculas() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.arquivo("SCAN.PDF");
        assert_eq!(
            abrir(&caminho).map(|doc| doc.nome),
            Ok("SCAN.PDF".to_owned())
        );
    }

    #[test]
    fn aceita_caminho_com_acento_e_espaco() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.arquivo("Relatório de produção – março.pdf");
        assert_eq!(
            abrir(&caminho).map(|doc| doc.nome),
            Ok("Relatório de produção – março.pdf".to_owned())
        );
    }

    #[test]
    fn recusa_arquivo_inexistente() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.0.join("sumiu.pdf");
        assert_eq!(
            abrir(&caminho.to_string_lossy()),
            Err(ErroDocumento {
                codigo: CodigoErro::NaoEncontrado,
                nome: "sumiu.pdf".to_owned(),
            })
        );
    }

    #[test]
    fn recusa_pasta_mesmo_com_nome_terminado_em_pdf() {
        let pasta = PastaTemporaria::nova();
        let subpasta = pasta.0.join("capitulos.pdf");
        fs::create_dir_all(&subpasta).expect("criar subpasta");
        assert_eq!(
            abrir(&subpasta.to_string_lossy()),
            Err(ErroDocumento {
                codigo: CodigoErro::NaoEArquivo,
                nome: "capitulos.pdf".to_owned(),
            })
        );
    }

    #[test]
    fn recusa_outra_extensao_e_arquivo_sem_extensao() {
        let pasta = PastaTemporaria::nova();
        for nome in ["notas.txt", "pdf", "falso.pdf.exe"] {
            let caminho = pasta.arquivo(nome);
            assert_eq!(
                abrir(&caminho),
                Err(ErroDocumento {
                    codigo: CodigoErro::NaoEPdf,
                    nome: nome.to_owned(),
                })
            );
        }
    }

    #[test]
    fn caminho_vazio_ou_invalido_vira_erro_sem_derrubar_o_app() {
        assert_eq!(
            abrir("").map_err(|erro| erro.codigo),
            Err(CodigoErro::NaoEncontrado)
        );
        assert_eq!(
            abrir("a\0b.pdf").map_err(|erro| erro.codigo),
            Err(CodigoErro::Inacessivel)
        );
    }
}
