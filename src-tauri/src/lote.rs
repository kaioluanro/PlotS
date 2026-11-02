//! Limite de caminhos abertos de uma vez, igual para a linha de comando e para o arrasto.

use serde::Serialize;

pub const MAX_CAMINHOS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoteCaminhos {
    pub caminhos: Vec<String>,
    /// Havia mais caminhos do que o limite, e o restante foi descartado.
    pub cortado: bool,
}

/// Fica com os primeiros caminhos, sem percorrer mais do que o necessario para saber se sobrou.
pub fn limitar(caminhos: impl IntoIterator<Item = String>) -> LoteCaminhos {
    let mut iterador = caminhos.into_iter();
    let primeiros = iterador.by_ref().take(MAX_CAMINHOS).collect();
    LoteCaminhos {
        caminhos: primeiros,
        cortado: iterador.next().is_some(),
    }
}

/// Argumentos da linha de comando sem as opcoes: o que comeca com `--` nunca e um caminho.
pub fn dos_argumentos(argumentos: impl IntoIterator<Item = String>) -> LoteCaminhos {
    limitar(
        argumentos
            .into_iter()
            .filter(|argumento| !argumento.starts_with("--")),
    )
}

#[cfg(test)]
mod testes {
    use super::*;

    fn numerados(quantos: usize) -> Vec<String> {
        (0..quantos).map(|n| format!("{n}.pdf")).collect()
    }

    #[test]
    fn dentro_do_limite_nao_corta() {
        let lote = limitar(numerados(MAX_CAMINHOS));
        assert_eq!(lote.caminhos.len(), MAX_CAMINHOS);
        assert!(!lote.cortado);
    }

    #[test]
    fn acima_do_limite_fica_com_os_primeiros_e_avisa() {
        assert_eq!(MAX_CAMINHOS, 32);
        let lote = limitar(numerados(MAX_CAMINHOS + 1));
        assert_eq!(lote.caminhos, numerados(MAX_CAMINHOS));
        assert!(lote.cortado);
    }

    #[test]
    fn lista_vazia_nao_corta() {
        assert_eq!(
            limitar(Vec::new()),
            LoteCaminhos {
                caminhos: Vec::new(),
                cortado: false,
            }
        );
    }

    #[test]
    fn argumentos_com_dois_tracos_sao_ignorados() {
        let lote = dos_argumentos(
            ["--opcao", "a.pdf", "--outra=1", "-b.pdf", "c d.pdf"].map(String::from),
        );
        assert_eq!(lote.caminhos, ["a.pdf", "-b.pdf", "c d.pdf"]);
        assert!(!lote.cortado);
    }

    #[test]
    fn opcoes_nao_contam_para_o_limite() {
        let mut argumentos = vec!["--opcao".to_owned(); 40];
        argumentos.extend(numerados(MAX_CAMINHOS));
        let lote = dos_argumentos(argumentos);
        assert_eq!(lote.caminhos.len(), MAX_CAMINHOS);
        assert!(!lote.cortado);
    }
}
