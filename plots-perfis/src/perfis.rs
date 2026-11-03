use std::collections::BTreeMap;

use plots_core::{ModeloImpressora, PerfilImpressora};

use crate::grupos::{Grupo, GrupoIgnorado};

/// Perfil salvo para uma fila de impressão.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EntradaPerfil {
    /// Nome amigável opcional, exibido no lugar do nome técnico da fila.
    pub apelido: Option<String>,
    /// Parâmetros do perfil: os do modelo, ou os ajustados pela calibração.
    pub perfil: PerfilImpressora,
    /// Texto livre que descreve o gesto de recolocação do maço.
    pub gesto_texto: Option<String>,
    /// Modelo de que os parâmetros vieram; `None` quando foram medidos ou escritos à mão.
    pub modelo: Option<ModeloImpressora>,
    /// A impressora imprime as duas faces sozinha nos papéis aceitos. Nos outros, vale o gesto de
    /// `modelo`, que nesse caso sempre existe.
    pub automatico: bool,
}

/// Conjunto de perfis, indexado pelo nome exato da fila.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Perfis {
    filas: BTreeMap<String, EntradaPerfil>,
    /// Seleções salvas, pelo nome: quem lê e grava os perfis leva os grupos junto.
    pub(crate) grupos: BTreeMap<String, Grupo>,
    /// O que estava no arquivo e não vale como grupo: não aparece, e volta igual na próxima gravação.
    pub(crate) grupos_ignorados: BTreeMap<String, GrupoIgnorado>,
    pub(crate) ajustes_driver: BTreeMap<String, Vec<u8>>,
    pub(crate) ajustes_ignorados: Vec<String>,
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

    /// Dá à fila o perfil pronto do modelo, mesmo que ela não tenha perfil ainda.
    ///
    /// O apelido fica. O texto do gesto sai, porque descrevia o perfil anterior.
    pub fn definir_modelo(&mut self, fila: &str, modelo: ModeloImpressora) {
        let apelido = self.filas.remove(fila).and_then(|entrada| entrada.apelido);
        self.inserir(
            fila,
            EntradaPerfil {
                apelido,
                perfil: modelo.perfil(),
                gesto_texto: None,
                modelo: Some(modelo),
                automatico: false,
            },
        );
    }

    /// Marca a fila como frente e verso automático. Sem `gesto`, os outros papéis seguem o gesto do
    /// modelo que ela já tinha, ou o do jato de tinta.
    pub fn definir_automatico(&mut self, fila: &str, gesto: Option<ModeloImpressora>) {
        let gesto = gesto
            .or_else(|| self.obter(fila).and_then(|entrada| entrada.modelo))
            .unwrap_or(ModeloImpressora::JatoDeTinta);
        self.definir_modelo(fila, gesto);
        if let Some(entrada) = self.filas.get_mut(fila) {
            entrada.automatico = true;
        }
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
