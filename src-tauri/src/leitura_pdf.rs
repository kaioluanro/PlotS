//! Leitura do PDF de um trabalho, com a mesma barreira e o mesmo limite da abertura do documento.

use crate::documento::{abrir, CodigoErro, ErroDocumento, LIMITE_BYTES_PDF};
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

/// Le o PDF. O que o arquivo nao deixar ler vira recusa.
pub fn ler_pdf(caminho: &str) -> Result<PdfLido, ErroDocumento> {
    let documento = abrir(caminho)?;
    let erro = |codigo| ErroDocumento {
        codigo,
        nome: documento.nome.clone(),
    };
    let bytes = ler_bytes(caminho, LIMITE_BYTES_PDF).map_err(erro)?;
    let origem = PdfOrigem::carregar(&bytes).map_err(|falha| {
        erro(match falha {
            ErroPdf::Criptografado => CodigoErro::Criptografado,
            _ => CodigoErro::PdfInvalido,
        })
    })?;
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
