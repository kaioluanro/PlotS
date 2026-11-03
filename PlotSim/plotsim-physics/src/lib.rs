//! Modelo físico do frente e verso: folhas, pilhas, gesto e conferência do maço final.
//!
//! Este crate não conhece o programa que ele confere. A rotação de cada face sai da pose da folha
//! no espaço; não há tabela de resultados por modo.

mod conferencia;
mod geometria;
mod impressora;

pub use conferencia::{
    conferir, folhas_do_documento, ler_maco, FolhaLida, Idioma, Leitura, MacoLido, Motivo,
};
pub use geometria::{Giro, Rotacao, Vetor};
pub use impressora::{
    BordaQueEntra, Entrada, Envio, ErroGesto, Face, FaceNaEntrada, Folha, Gesto, Impressao,
    Impressora, Lado, ModoImpressora, Operacao, Pilha, ResumoDaPassada, TopoDaImagem, Uniao,
    LIMITE_FOLHAS,
};
