//! Leitura do PDF de um trabalho, com a mesma barreira e o mesmo limite da abertura do documento.

use crate::documento::{abrir_para_leitura, CodigoErro, ErroDocumento, LIMITE_BYTES_PDF};
use plots_core::DimensoesPagina;
use plots_pdf::{ErroPdf, PdfOrigem};
use std::fs::File;
use std::io::{ErrorKind, Read};

/// PDF aberto para um trabalho.
pub struct PdfLido {
    #[cfg_attr(not(windows), allow(dead_code))]
    pub origem: PdfOrigem,
    pub dimensoes: Vec<DimensoesPagina>,
    pub assinado: bool,
    /// Impressão dos mesmos bytes que `origem` carrega: confere se o arquivo mudou.
    pub impressao: String,
}

/// Impressão de um conteúdo: FNV-1a de 64 bits sobre todos os bytes, com o tamanho. Qualquer troca
/// do arquivo muda o texto, inclusive a medida de uma página. Alimentada em blocos, dá o mesmo texto
/// que uma única chamada com o conteúdo inteiro.
#[derive(Clone, Copy, Debug)]
pub struct Impressao {
    hash: u64,
    total: u64,
}

impl Default for Impressao {
    fn default() -> Self {
        Self {
            hash: 0xcbf2_9ce4_8422_2325,
            total: 0,
        }
    }
}

impl Impressao {
    pub fn acrescentar(&mut self, bloco: &[u8]) {
        self.total = self.total.saturating_add(bloco.len() as u64);
        for byte in bloco {
            self.hash ^= u64::from(*byte);
            self.hash = self.hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn texto(&self) -> String {
        format!("{:016x}:{}", self.hash, self.total)
    }
}

/// Impressão de um conteúdo inteiro, pronta para comparar com a guardada.
pub fn impressao_dos_bytes(bytes: &[u8]) -> String {
    let mut impressao = Impressao::default();
    impressao.acrescentar(bytes);
    impressao.texto()
}

fn ler_bytes(caminho: &str, limite: u64) -> Result<Vec<u8>, CodigoErro> {
    let de_leitura = |falha: std::io::Error| match falha.kind() {
        ErrorKind::NotFound => CodigoErro::NaoEncontrado,
        _ => CodigoErro::Inacessivel,
    };
    let arquivo = File::open(caminho).map_err(de_leitura)?;
    let tamanho = arquivo.metadata().map_err(de_leitura)?.len();
    if tamanho > limite {
        return Err(CodigoErro::Grande);
    }
    let capacidade = usize::try_from(tamanho).map_err(|_| CodigoErro::Grande)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacidade)
        .map_err(|_| CodigoErro::Grande)?;
    arquivo
        .take(limite.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(de_leitura)?;
    if u64::try_from(bytes.len()).map_or(true, |lidos| lidos > limite) {
        return Err(CodigoErro::Grande);
    }
    Ok(bytes)
}

/// Le o conteudo do documento: o PDF original, ou o PDF derivado de uma imagem. O que o arquivo
/// nao deixar ler vira recusa.
pub fn ler_pdf(caminho: &str) -> Result<PdfLido, ErroDocumento> {
    let (documento, conteudo) = abrir_para_leitura(caminho)?;
    let erro = |codigo| ErroDocumento {
        codigo,
        nome: documento.nome.clone(),
    };
    let bytes = ler_bytes(&conteudo, LIMITE_BYTES_PDF).map_err(erro)?;
    let origem = PdfOrigem::carregar(&bytes).map_err(|falha| {
        erro(match falha {
            ErroPdf::Criptografado => CodigoErro::Criptografado,
            _ => CodigoErro::PdfInvalido,
        })
    })?;
    let impressao = impressao_dos_bytes(&bytes);
    let dimensoes = origem
        .dimensoes_paginas()
        .map_err(|_| erro(CodigoErro::PdfInvalido))?;
    let assinado = origem
        .tem_assinatura_aplicada()
        .map_err(|_| erro(CodigoErro::PdfInvalido))?;
    Ok(PdfLido {
        origem,
        dimensoes,
        assinado,
        impressao,
    })
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::path::PathBuf;

    fn caminho_temporario(nome: &str, conteudo: &[u8]) -> PathBuf {
        let caminho =
            std::env::temp_dir().join(format!("plots-app-leitura-{}-{nome}", std::process::id()));
        std::fs::write(&caminho, conteudo).unwrap();
        caminho
    }

    #[test]
    fn arquivo_acima_do_limite_e_recusado_sem_ler() {
        let caminho = caminho_temporario("grande.pdf", b"123456");
        let lido = ler_bytes(&caminho.to_string_lossy(), 5);
        assert_eq!(lido, Err(CodigoErro::Grande));
        std::fs::remove_file(caminho).unwrap();
    }

    #[test]
    fn caminho_inexistente_vira_recusa() {
        let erro = ler_pdf("c:/nao/existe/nada.pdf").err().unwrap();
        assert_eq!(erro.codigo, CodigoErro::NaoEncontrado);
    }

    #[test]
    fn conteudo_que_nao_e_pdf_vira_recusa() {
        let caminho = caminho_temporario("falso.pdf", b"nao sou pdf");
        let erro = ler_pdf(&caminho.to_string_lossy()).err().unwrap();
        assert_eq!(erro.codigo, CodigoErro::PdfInvalido);
        std::fs::remove_file(caminho).unwrap();
    }
}
