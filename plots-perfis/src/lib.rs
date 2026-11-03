//! Perfis de impressão salvos por fila, guardados em `perfis.toml`.
//!
//! Este crate é o único que lê e grava o arquivo. O núcleo de planejamento
//! (`plots-core`) continua sem acesso a disco.

mod armazenamento;
mod erro;
mod formato;
mod grupos;
mod perfis;

#[cfg(test)]
mod testes;
#[cfg(test)]
mod testes_grupos;

pub use armazenamento::{caminho_padrao, carregar, salvar, LIMITE_ARQUIVO_BYTES, NOME_ARQUIVO};
pub use erro::ErroPerfis;
pub use formato::VERSAO_ATUAL;
pub use grupos::{ErroGrupo, Grupo, LIMITE_GRUPOS, LIMITE_IMPRESSORAS_GRUPO, LIMITE_NOME_GRUPO};
pub use perfis::{EntradaPerfil, Perfis};
