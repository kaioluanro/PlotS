use plots_core::distribuicao::ModoDistribuicao;
use thiserror::Error;

use crate::perfis::Perfis;

/// Maior nome aceito, em caracteres.
pub const LIMITE_NOME_GRUPO: usize = 60;
/// Quantos grupos o arquivo guarda.
pub const LIMITE_GRUPOS: usize = 100;
/// Quantas impressoras um grupo guarda.
pub const LIMITE_IMPRESSORAS_GRUPO: usize = 64;

/// Uma seleção salva: as impressoras, pelo identificador, e o modo de distribuição.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Grupo {
    pub impressoras: Vec<String>,
    pub modo: ModoDistribuicao,
}

/// Um grupo do arquivo que não passou na conferência, como estava escrito.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GrupoIgnorado(pub(crate) toml::Value);

// Só compara o que veio do mesmo arquivo; um número sem valor definido apenas torna dois diferentes.
impl Eq for GrupoIgnorado {}

/// Por que um grupo não foi salvo nem apagado. Nada muda quando há recusa.
#[derive(Debug, Eq, PartialEq, Error)]
pub enum ErroGrupo {
    #[error("o nome do grupo está vazio")]
    NomeVazio,
    #[error("o nome do grupo passa de {LIMITE_NOME_GRUPO} caracteres")]
    NomeGrande,
    #[error("o nome do grupo tem caracteres que não podem ser mostrados")]
    NomeInvalido,
    #[error("já existe um grupo com este nome")]
    NomeRepetido,
    #[error("o grupo precisa de ao menos uma impressora")]
    SemImpressoras,
    #[error("o grupo passa de {LIMITE_IMPRESSORAS_GRUPO} impressoras")]
    MuitasImpressoras,
    #[error("já existem {LIMITE_GRUPOS} grupos")]
    MuitosGrupos,
    #[error("este grupo não existe mais")]
    Inexistente,
}

/// Controles e marcas invisíveis fariam dois nomes iguais na tela valerem como diferentes.
fn caractere_invisivel(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{00AD}'
                | '\u{061C}'
                | '\u{180E}'
                | '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2060}'..='\u{206F}'
                | '\u{FEFF}'
        )
}

fn nome_conferido(nome: &str) -> Result<&str, ErroGrupo> {
    let nome = nome.trim();
    if nome.is_empty() {
        return Err(ErroGrupo::NomeVazio);
    }
    if nome.chars().nth(LIMITE_NOME_GRUPO).is_some() {
        return Err(ErroGrupo::NomeGrande);
    }
    if nome.chars().any(caractere_invisivel) {
        return Err(ErroGrupo::NomeInvalido);
    }
    Ok(nome)
}

fn impressoras_conferidas(impressoras: &[String]) -> Result<Vec<String>, ErroGrupo> {
    let mut unicas: Vec<String> = Vec::new();
    for impressora in impressoras {
        if unicas.len() >= LIMITE_IMPRESSORAS_GRUPO {
            return Err(ErroGrupo::MuitasImpressoras);
        }
        if !impressora.trim().is_empty() && !unicas.contains(impressora) {
            unicas.push(impressora.clone());
        }
    }
    if unicas.is_empty() {
        return Err(ErroGrupo::SemImpressoras);
    }
    Ok(unicas)
}

/// Dois nomes que só mudam em maiúsculas ou espaços nas pontas são o mesmo para quem escolhe.
fn mesmo_nome(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

impl Perfis {
    /// Grupos em ordem alfabética do nome.
    pub fn grupos(&self) -> impl Iterator<Item = (&str, &Grupo)> {
        self.grupos
            .iter()
            .map(|(nome, grupo)| (nome.as_str(), grupo))
    }

    /// Grupo com este nome, escrito como foi salvo.
    pub fn grupo(&self, nome: &str) -> Option<&Grupo> {
        self.grupos.get(nome)
    }

    /// Quantos grupos do arquivo não puderam ser usados. Continuam no arquivo.
    pub fn grupos_ignorados(&self) -> usize {
        self.grupos_ignorados.len()
    }

    /// Guarda um grupo lido do arquivo, com as mesmas regras do grupo salvo pela interface. Nome com
    /// espaços nas pontas é recusado em vez de corrigido: a gravação escreveria outro nome.
    pub(crate) fn aceitar_grupo_lido(&mut self, nome: &str, grupo: Grupo) -> Result<(), ErroGrupo> {
        if nome_conferido(nome)? != nome {
            return Err(ErroGrupo::NomeInvalido);
        }
        if self.grupos.keys().any(|outro| mesmo_nome(outro, nome)) {
            return Err(ErroGrupo::NomeRepetido);
        }
        if self.grupos.len() >= LIMITE_GRUPOS {
            return Err(ErroGrupo::MuitosGrupos);
        }
        let impressoras = impressoras_conferidas(&grupo.impressoras)?;
        self.grupos.insert(
            nome.to_owned(),
            Grupo {
                impressoras,
                modo: grupo.modo,
            },
        );
        Ok(())
    }

    /// Guarda um grupo novo. Um nome já usado é recusado, sem substituir o que existe.
    pub fn salvar_grupo(
        &mut self,
        nome: &str,
        impressoras: &[String],
        modo: ModoDistribuicao,
    ) -> Result<(), ErroGrupo> {
        let nome = nome_conferido(nome)?;
        let repetido = |outro: &String| mesmo_nome(outro, nome);
        if self.grupos.keys().any(repetido) || self.grupos_ignorados.keys().any(repetido) {
            return Err(ErroGrupo::NomeRepetido);
        }
        if self.grupos.len() >= LIMITE_GRUPOS {
            return Err(ErroGrupo::MuitosGrupos);
        }
        let impressoras = impressoras_conferidas(impressoras)?;
        self.grupos
            .insert(nome.to_owned(), Grupo { impressoras, modo });
        Ok(())
    }

    /// Apaga o grupo com exatamente este nome.
    pub fn apagar_grupo(&mut self, nome: &str) -> Result<Grupo, ErroGrupo> {
        self.grupos.remove(nome).ok_or(ErroGrupo::Inexistente)
    }
}
