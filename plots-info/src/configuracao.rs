//! Configuração da estimativa: perfil ativo, qualidade, resolução, modo de cor e tamanho da
//! imagem.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::perfil::LIMITE_ID_BYTES;

/// Identificador do perfil entregue como padrão.
pub const PERFIL_PADRAO: &str = "tanque-generico";

/// Resoluções de análise aceitas, em dpi.
pub const DPIS_SUPORTADOS: [u32; 2] = [150, 300];

/// Qualidade escolhida, que multiplica o consumo pelos fatores do perfil.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Qualidade {
    Rascunho,
    Normal,
    Alta,
}

impl Qualidade {
    /// Posição do fator correspondente em `PerfilDeTinta::fatores_qualidade`.
    pub const fn indice(self) -> usize {
        match self {
            Self::Rascunho => 0,
            Self::Normal => 1,
            Self::Alta => 2,
        }
    }
}

/// Como a página é analisada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModoDeCor {
    /// Quatro canais, vindos do conversor ou da fórmula simples.
    Cmyk,
    /// Só a fórmula simples, sem o conversor externo.
    Simples,
}

/// Como o tamanho físico de uma imagem é decidido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TamanhoDaImagem {
    /// Página A4 com a imagem ajustada e o branco em volta.
    A4,
    /// Tamanho físico calculado pelo dpi declarado na imagem.
    Dpi,
}

/// Configuração guardada ao lado dos perfis de tinta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfiguracaoDeTinta {
    pub perfil: String,
    pub qualidade: Qualidade,
    pub dpi: u32,
    pub modo: ModoDeCor,
    pub cinzas_so_preto: bool,
    pub tamanho_da_imagem: TamanhoDaImagem,
}

impl Default for ConfiguracaoDeTinta {
    fn default() -> Self {
        Self {
            perfil: PERFIL_PADRAO.to_owned(),
            qualidade: Qualidade::Normal,
            dpi: 150,
            modo: ModoDeCor::Cmyk,
            cinzas_so_preto: true,
            tamanho_da_imagem: TamanhoDaImagem::A4,
        }
    }
}

impl ConfiguracaoDeTinta {
    /// Confere os campos que têm mais de um valor aceito.
    pub fn validar(&self) -> Result<(), ErroConfiguracao> {
        if self.perfil.trim().is_empty() {
            return Err(ErroConfiguracao::PerfilVazio);
        }
        if self.perfil.len() > LIMITE_ID_BYTES {
            return Err(ErroConfiguracao::PerfilGrande {
                bytes: self.perfil.len(),
            });
        }
        if !DPIS_SUPORTADOS.contains(&self.dpi) {
            return Err(ErroConfiguracao::DpiNaoSuportado { dpi: self.dpi });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErroConfiguracao {
    #[error("o perfil de tinta não foi indicado")]
    PerfilVazio,
    #[error("o identificador do perfil tem {bytes} bytes e passa do limite de {LIMITE_ID_BYTES}")]
    PerfilGrande { bytes: usize },
    #[error("a resolução {dpi} dpi não é suportada; use 150 ou 300")]
    DpiNaoSuportado { dpi: u32 },
}
