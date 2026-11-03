//! Núcleo do simulador de impressoras: filas virtuais do Windows e recepção dos trabalhos.
//!
//! Criar ou remover uma fila muda o Windows do usuário, então só acontece a pedido, só em filas
//! com o prefixo do simulador e com a porta dentro da pasta dele.

pub mod filas;
pub mod leitor;
pub mod modo;
pub mod nome;
pub mod pastas;
pub mod powershell;
pub mod recepcao;
pub mod registro;
pub mod sistema;
pub mod spooler;

#[cfg(test)]
mod testes_comuns;

pub use filas::{ErroFilas, FilaVirtual, Filas, Remocao, Situacao, DRIVER_DAS_FILAS};
pub use modo::{FaceDeSaida, ModoFisico, Saida};
pub use nome::{ErroNome, NomeVirtual, LIMITE_NOME, PREFIXO_FILA};
pub use pastas::{ErroPasta, Pastas, ARQUIVO_DA_PORTA, VARIAVEL_CONFIGURACAO};
pub use registro::{ErroRegistro, Registro, LIMITE_FILAS};
pub use sistema::{ErroSistema, FilaDoSistema, SistemaDeFilas};
