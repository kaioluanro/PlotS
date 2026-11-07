//! Perfil de tinta: volumes, rendimentos, preços por mililitro e fatores de qualidade.
//!
//! O perfil é um dado de entrada, por isso todo valor numérico é validado antes de qualquer
//! cálculo: nada de zero no divisor, nada de negativo e nada de infinito.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Quantidade de canais de tinta.
pub const CANAIS: usize = 4;

/// Nomes dos canais na ordem em que aparecem nos vetores do perfil.
pub const NOMES_DOS_CANAIS: [&str; CANAIS] = ["ciano", "magenta", "amarelo", "preto"];

/// Maior identificador aceito, em bytes.
pub const LIMITE_ID_BYTES: usize = 64;

/// Maior nome de perfil aceito, em bytes.
pub const LIMITE_NOME_BYTES: usize = 200;

/// Maior volume de tinta aceito por canal, em mililitros.
pub const LIMITE_VOLUME_ML: f64 = 10_000.0;

/// Maior rendimento aceito por canal, em páginas.
pub const LIMITE_RENDIMENTO_PAGINAS: u32 = 10_000_000;

/// Maior preço aceito por mililitro, na moeda do perfil.
pub const LIMITE_PRECO_ML: f64 = 10_000.0;

/// Maior fator de qualidade aceito.
pub const LIMITE_FATOR_QUALIDADE: f64 = 10.0;

/// Perfil de tinta de uma impressora.
///
/// Os quatro canais seguem a ordem C, M, Y e K. Os fatores de qualidade seguem a ordem rascunho,
/// normal e alta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerfilDeTinta {
    pub id: String,
    pub nome: String,
    pub moeda: String,
    pub volumes_ml: [f64; CANAIS],
    pub rendimentos_paginas: [u32; CANAIS],
    pub precos_ml: [f64; CANAIS],
    pub fatores_qualidade: [f64; 3],
}

impl PerfilDeTinta {
    /// Confere os campos antes de o perfil ser usado em um cálculo ou gravado.
    pub fn validar(&self) -> Result<(), ErroPerfil> {
        if self.id.trim().is_empty() {
            return Err(ErroPerfil::IdVazio);
        }
        if self.id.len() > LIMITE_ID_BYTES {
            return Err(ErroPerfil::IdGrande {
                bytes: self.id.len(),
            });
        }
        if self.nome.trim().is_empty() {
            return Err(ErroPerfil::NomeVazio);
        }
        if self.nome.len() > LIMITE_NOME_BYTES {
            return Err(ErroPerfil::NomeGrande {
                bytes: self.nome.len(),
            });
        }
        let moeda = self.moeda.as_bytes();
        if moeda.len() != 3 || !moeda.iter().all(u8::is_ascii_uppercase) {
            return Err(ErroPerfil::MoedaInvalida {
                moeda: self.moeda.clone(),
            });
        }
        for canal in 0..CANAIS {
            let volume = self.volumes_ml[canal];
            if !volume.is_finite() || volume <= 0.0 || volume > LIMITE_VOLUME_ML {
                return Err(ErroPerfil::VolumeInvalido { canal });
            }
            let rendimento = self.rendimentos_paginas[canal];
            if rendimento == 0 || rendimento > LIMITE_RENDIMENTO_PAGINAS {
                return Err(ErroPerfil::RendimentoInvalido { canal });
            }
            let preco = self.precos_ml[canal];
            if !preco.is_finite() || preco < 0.0 || preco > LIMITE_PRECO_ML {
                return Err(ErroPerfil::PrecoInvalido { canal });
            }
        }
        for indice in 0..3 {
            let fator = self.fatores_qualidade[indice];
            if !fator.is_finite() || fator <= 0.0 || fator > LIMITE_FATOR_QUALIDADE {
                return Err(ErroPerfil::FatorInvalido { indice });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErroPerfil {
    #[error("o identificador do perfil não pode ficar vazio")]
    IdVazio,
    #[error("o identificador do perfil tem {bytes} bytes e passa do limite de {LIMITE_ID_BYTES}")]
    IdGrande { bytes: usize },
    #[error("o nome do perfil não pode ficar vazio")]
    NomeVazio,
    #[error("o nome do perfil tem {bytes} bytes e passa do limite de {LIMITE_NOME_BYTES}")]
    NomeGrande { bytes: usize },
    #[error("a moeda {moeda:?} não é um código de três letras maiúsculas")]
    MoedaInvalida { moeda: String },
    #[error("o volume do canal {canal} precisa ser positivo e finito")]
    VolumeInvalido { canal: usize },
    #[error(
        "o rendimento do canal {canal} precisa estar entre 1 e {LIMITE_RENDIMENTO_PAGINAS} páginas"
    )]
    RendimentoInvalido { canal: usize },
    #[error("o preço do canal {canal} precisa ser finito e não negativo")]
    PrecoInvalido { canal: usize },
    #[error("o fator de qualidade {indice} precisa ser positivo e finito")]
    FatorInvalido { indice: usize },
}

/// Conjunto de perfis, consultado pelo identificador.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PerfisDeTinta {
    perfis: Vec<PerfilDeTinta>,
}

impl PerfisDeTinta {
    /// Conjunto vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Os perfis inclusos, que o programa entrega na primeira abertura.
    pub fn padrao() -> Self {
        Self {
            perfis: vec![
                perfil_de_tanque(),
                perfil_de_cartucho(),
                perfil_hp_smart_tank_110(),
                perfil_epson_l3150(),
                perfil_epson_l6270(),
            ],
        }
    }

    /// Perfis na ordem guardada.
    pub fn perfis(&self) -> &[PerfilDeTinta] {
        &self.perfis
    }

    /// Perfil com este identificador exato.
    pub fn buscar(&self, id: &str) -> Option<&PerfilDeTinta> {
        self.perfis.iter().find(|perfil| perfil.id == id)
    }

    /// `true` quando não há nenhum perfil.
    pub fn vazio(&self) -> bool {
        self.perfis.is_empty()
    }

    /// Valida e guarda o perfil, trocando o de mesmo identificador no lugar dele.
    ///
    /// Devolve `true` quando havia um perfil com o mesmo identificador.
    pub fn substituir(&mut self, perfil: PerfilDeTinta) -> Result<bool, ErroPerfil> {
        perfil.validar()?;
        match self
            .perfis
            .iter_mut()
            .find(|guardado| guardado.id == perfil.id)
        {
            Some(guardado) => {
                *guardado = perfil;
                Ok(true)
            }
            None => {
                self.perfis.push(perfil);
                Ok(false)
            }
        }
    }

    /// Remove o perfil com este identificador; `false` quando ele não existia.
    pub fn remover(&mut self, id: &str) -> bool {
        let antes = self.perfis.len();
        self.perfis.retain(|perfil| perfil.id != id);
        self.perfis.len() != antes
    }
}

fn perfil_de_tanque() -> PerfilDeTinta {
    PerfilDeTinta {
        id: "tanque-generico".to_owned(),
        nome: "Tanque de tinta genérico".to_owned(),
        moeda: "BRL".to_owned(),
        volumes_ml: [65.0, 65.0, 65.0, 65.0],
        rendimentos_paginas: [6500, 6500, 6500, 4500],
        precos_ml: [0.77, 0.77, 0.77, 1.11],
        fatores_qualidade: [0.6, 1.0, 1.3],
    }
}

fn perfil_de_cartucho() -> PerfilDeTinta {
    PerfilDeTinta {
        id: "cartucho-generico".to_owned(),
        nome: "Cartucho de jato de tinta genérico".to_owned(),
        moeda: "BRL".to_owned(),
        volumes_ml: [8.0, 8.0, 8.0, 8.0],
        rendimentos_paginas: [400, 400, 400, 300],
        precos_ml: [1.2, 1.2, 1.2, 1.8],
        fatores_qualidade: [0.6, 1.0, 1.3],
    }
}

// Os perfis de tanque abaixo repetem o volume do frasco principal (preto) nas quatro cores como
// ponto de partida; os frascos coloridos do GT51 (GT52) e do 504 são menores, então o custo por
// mL das cores é aproximado. O rendimento é um ponto de partida editável na engrenagem do painel.

fn perfil_hp_smart_tank_110() -> PerfilDeTinta {
    PerfilDeTinta {
        id: "hp-smart-tank-110-gt51".to_owned(),
        nome: "HP Smart Tank 110 (GT51)".to_owned(),
        moeda: "BRL".to_owned(),
        volumes_ml: [90.0, 90.0, 90.0, 90.0],
        rendimentos_paginas: [6500, 6500, 6500, 4500],
        precos_ml: [30.0 / 90.0, 30.0 / 90.0, 30.0 / 90.0, 30.0 / 90.0],
        fatores_qualidade: [0.6, 1.0, 1.3],
    }
}

fn perfil_epson_l3150() -> PerfilDeTinta {
    PerfilDeTinta {
        id: "epson-l3150-544".to_owned(),
        nome: "Epson EcoTank L3150 (544)".to_owned(),
        moeda: "BRL".to_owned(),
        volumes_ml: [65.0, 65.0, 65.0, 65.0],
        rendimentos_paginas: [6500, 6500, 6500, 4500],
        precos_ml: [33.0 / 65.0, 33.0 / 65.0, 33.0 / 65.0, 33.0 / 65.0],
        fatores_qualidade: [0.6, 1.0, 1.3],
    }
}

fn perfil_epson_l6270() -> PerfilDeTinta {
    PerfilDeTinta {
        id: "epson-l6270-504".to_owned(),
        nome: "Epson EcoTank L6270 (504)".to_owned(),
        moeda: "BRL".to_owned(),
        volumes_ml: [127.0, 127.0, 127.0, 127.0],
        rendimentos_paginas: [6500, 6500, 6500, 4500],
        precos_ml: [55.0 / 127.0, 55.0 / 127.0, 55.0 / 127.0, 55.0 / 127.0],
        fatores_qualidade: [0.6, 1.0, 1.3],
    }
}
