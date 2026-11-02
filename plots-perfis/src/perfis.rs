use std::collections::BTreeMap;

use plots_core::PerfilImpressora;

/// Perfil salvo para uma fila de impressão.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EntradaPerfil {
    /// Nome amigável opcional, exibido no lugar do nome técnico da fila.
    pub apelido: Option<String>,
    /// Parâmetros medidos pela calibração.
    pub perfil: PerfilImpressora,
    /// Texto livre que descreve o gesto de recolocação do maço.
    pub gesto_texto: Option<String>,
}

/// Conjunto de perfis, indexado pelo nome exato da fila.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Perfis {
    filas: BTreeMap<String, EntradaPerfil>,
}

impl Perfis {
    /// Conjunto vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Perfil da fila com exatamente este nome.
    pub fn obter(&self, fila: &str) -> Option<&EntradaPerfil> {
        self.filas.get(fila)
    }

    /// Insere ou substitui o perfil da fila; devolve o anterior, se houver.
    pub fn inserir(&mut self, fila: &str, entrada: EntradaPerfil) -> Option<EntradaPerfil> {
        self.filas.insert(fila.to_owned(), entrada)
    }

    /// Remove o perfil da fila e o devolve, se existir.
    pub fn remover(&mut self, fila: &str) -> Option<EntradaPerfil> {
        self.filas.remove(fila)
    }

    /// Quantidade de filas com perfil.
    pub fn len(&self) -> usize {
        self.filas.len()
    }

    /// `true` quando nenhuma fila tem perfil.
    pub fn is_empty(&self) -> bool {
        self.filas.is_empty()
    }

    /// Filas e perfis em ordem alfabética do nome da fila.
    pub fn filas(&self) -> impl Iterator<Item = (&str, &EntradaPerfil)> {
        self.filas
            .iter()
            .map(|(nome, entrada)| (nome.as_str(), entrada))
    }
}
