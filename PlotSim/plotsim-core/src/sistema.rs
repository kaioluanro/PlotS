//! O que o gerenciador de filas pede ao sistema. Os testes usam uma implementação em memória.

use std::fmt;

/// Uma fila como o sistema a descreve.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilaDoSistema {
    pub nome: String,
    pub driver: String,
    pub porta: String,
}

/// Falhas do sistema ao mexer em filas e portas.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErroSistema {
    /// O Windows negou por falta de permissão. Não há outro caminho dentro do programa.
    PrecisaAdministrador,
    /// O Windows recusou por outro motivo. `mensagem` é o texto dele, cortado.
    Recusado {
        codigo: String,
        mensagem: String,
    },
    TempoEsgotado,
    /// O comando nem chegou a rodar, ou devolveu mais do que o limite.
    Processo(String),
    /// A leitura das filas falhou.
    Leitura(String),
    NaoSuportado,
}

impl fmt::Display for ErroSistema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrecisaAdministrador => f.write_str(
                "o Windows pediu permissão de administrador para mexer em filas e portas",
            ),
            Self::Recusado { codigo, mensagem } => {
                write!(f, "o Windows recusou o pedido ({codigo}): {mensagem}")
            }
            Self::TempoEsgotado => f.write_str("o comando do sistema não terminou a tempo"),
            Self::Processo(texto) => write!(f, "não deu para rodar o comando: {texto}"),
            Self::Leitura(texto) => write!(f, "não deu para ler as filas: {texto}"),
            Self::NaoSuportado => f.write_str("filas virtuais só existem no Windows"),
        }
    }
}

impl std::error::Error for ErroSistema {}

/// Operações de filas e portas. Nenhuma recebe nome sem validar: quem chama é o gerenciador.
pub trait SistemaDeFilas {
    fn filas(&self) -> Result<Vec<FilaDoSistema>, ErroSistema>;
    fn porta_existe(&self, porta: &str) -> Result<bool, ErroSistema>;
    fn criar_porta(&self, porta: &str) -> Result<(), ErroSistema>;
    fn criar_fila(&self, fila: &str, driver: &str, porta: &str) -> Result<(), ErroSistema>;
    fn remover_fila(&self, fila: &str) -> Result<(), ErroSistema>;
    fn remover_porta(&self, porta: &str) -> Result<(), ErroSistema>;
}
