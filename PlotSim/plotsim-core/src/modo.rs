//! Modo físico de uma fila virtual, só como dado guardado. Quem interpreta é o modelo físico.

use serde::{Deserialize, Serialize};

/// Por onde o maço sai da impressora simulada.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Saida {
    Traseira,
    Gaveta,
}

/// Como a face impressa fica na bandeja de saída.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FaceDeSaida {
    ParaCima,
    ParaBaixo,
}

/// As duas escolhas que definem o comportamento físico de uma fila.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModoFisico {
    pub saida: Saida,
    pub face: FaceDeSaida,
}
