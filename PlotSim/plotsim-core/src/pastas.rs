//! Onde o simulador guarda as coisas: configuração e histórico de um lado, portas do outro.

use crate::nome::NomeVirtual;
use std::env;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

/// Troca a pasta de configuração. É como os testes e a suíte ficam longe da pasta do usuário.
pub const VARIAVEL_CONFIGURACAO: &str = "PLOTSIM_CONFIG";
/// Nome da pasta do simulador em `%APPDATA%`.
const PASTA_CONFIGURACAO: &str = "plotsim";
/// Nome da pasta do simulador em `%ProgramData%`.
const PASTA_PORTAS: &str = "PlotSim";
/// Arquivo que o Windows grava a cada trabalho recebido.
pub const ARQUIVO_DA_PORTA: &str = "saida.pdf";
const LIMITE_CAMINHO: usize = 200;

/// Por que um caminho foi recusado. O nome da porta é passado a um comando do sistema, então só
/// entra caminho absoluto, local e com caracteres simples.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErroPasta {
    SemVariavel(&'static str),
    Relativo,
    Longo,
    Caractere(char),
    Subida,
}

impl fmt::Display for ErroPasta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemVariavel(nome) => write!(f, "a variável de ambiente {nome} não está definida"),
            Self::Relativo => f.write_str("o caminho precisa ser absoluto e local"),
            Self::Longo => write!(f, "o caminho passa de {LIMITE_CAMINHO} caracteres"),
            Self::Caractere(c) => write!(f, "o caminho tem um caractere não aceito: {c:?}"),
            Self::Subida => f.write_str("o caminho não pode ter \"..\""),
        }
    }
}

impl std::error::Error for ErroPasta {}

/// Confere o texto de um caminho que vai para um comando do sistema.
pub(crate) fn validar_caminho(caminho: &Path) -> Result<String, ErroPasta> {
    let texto = caminho.to_str().ok_or(ErroPasta::Relativo)?;
    if !caminho.is_absolute() || texto.starts_with(r"\\") {
        return Err(ErroPasta::Relativo);
    }
    if texto.chars().count() > LIMITE_CAMINHO {
        return Err(ErroPasta::Longo);
    }
    let dois_pontos = if cfg!(windows) { Some(1) } else { None };
    for (posicao, c) in texto.char_indices() {
        let aceito = c.is_alphanumeric()
            || c == MAIN_SEPARATOR
            || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')' | '~')
            || (c == ':' && Some(posicao) == dois_pontos);
        if !aceito {
            return Err(ErroPasta::Caractere(c));
        }
    }
    if caminho
        .components()
        .any(|parte| matches!(parte, std::path::Component::ParentDir))
    {
        return Err(ErroPasta::Subida);
    }
    Ok(texto.to_string())
}

/// As duas raízes do simulador.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pastas {
    configuracao: PathBuf,
    portas: PathBuf,
}

impl Pastas {
    /// Raízes escolhidas pelo chamador.
    pub fn novas(configuracao: PathBuf, portas: PathBuf) -> Result<Self, ErroPasta> {
        validar_caminho(&portas)?;
        Ok(Self {
            configuracao,
            portas,
        })
    }

    /// Raízes do sistema: `%APPDATA%/plotsim`, ou a pasta de `PLOTSIM_CONFIG`, e
    /// `%ProgramData%/PlotSim`.
    pub fn do_sistema() -> Result<Self, ErroPasta> {
        Self::das_variaveis(
            env::var_os(VARIAVEL_CONFIGURACAO),
            env::var_os("APPDATA"),
            env::var_os("ProgramData"),
        )
    }

    fn das_variaveis(
        sobreposta: Option<OsString>,
        dados_do_usuario: Option<OsString>,
        dados_do_sistema: Option<OsString>,
    ) -> Result<Self, ErroPasta> {
        let configuracao = match sobreposta.filter(|pasta| !pasta.is_empty()) {
            Some(pasta) => PathBuf::from(pasta),
            None => PathBuf::from(dados_do_usuario.ok_or(ErroPasta::SemVariavel("APPDATA"))?)
                .join(PASTA_CONFIGURACAO),
        };
        let portas = PathBuf::from(dados_do_sistema.ok_or(ErroPasta::SemVariavel("ProgramData"))?)
            .join(PASTA_PORTAS);
        Self::novas(configuracao, portas)
    }

    pub fn configuracao(&self) -> &Path {
        &self.configuracao
    }

    /// Arquivo com as filas conhecidas e o modo físico de cada uma.
    pub fn registro(&self) -> PathBuf {
        self.configuracao.join("filas.toml")
    }

    pub fn pasta_da_porta(&self, nome: &NomeVirtual) -> PathBuf {
        self.portas.join(nome.pasta())
    }

    /// Arquivo da porta. Uma pasta por fila, para que duas filas nunca escrevam no mesmo arquivo.
    pub fn porta(&self, nome: &NomeVirtual) -> Result<String, ErroPasta> {
        validar_caminho(&self.pasta_da_porta(nome).join(ARQUIVO_DA_PORTA))
    }

    pub fn historico(&self, nome: &NomeVirtual) -> PathBuf {
        self.configuracao.join("historico").join(nome.pasta())
    }
}

#[cfg(all(test, windows))]
mod testes {
    use super::*;

    fn portas(texto: &str) -> Result<Pastas, ErroPasta> {
        Pastas::novas(PathBuf::from(r"C:\config"), PathBuf::from(texto))
    }

    #[test]
    fn porta_fica_dentro_da_raiz_em_pasta_propria() {
        let pastas = portas(r"C:\ProgramData\PlotSim").unwrap();
        let nome = NomeVirtual::novo("Jato").unwrap();
        let porta = pastas.porta(&nome).unwrap();
        assert!(porta.starts_with(r"C:\ProgramData\PlotSim\jato-"));
        assert!(porta.ends_with(r"\saida.pdf"));
    }

    #[test]
    fn pasta_sobreposta_vale_sobre_a_do_usuario_e_as_portas_ficam_na_do_sistema() {
        let variavel = |texto: &str| Some(OsString::from(texto));
        let padrao =
            Pastas::das_variaveis(None, variavel(r"C:\Dados"), variavel(r"C:\ProgramData"))
                .unwrap();
        assert_eq!(padrao.registro(), Path::new(r"C:\Dados\plotsim\filas.toml"));
        let nome = NomeVirtual::novo("Jato").unwrap();
        assert!(padrao
            .porta(&nome)
            .unwrap()
            .starts_with(r"C:\ProgramData\PlotSim\"));
        let sobreposta = Pastas::das_variaveis(
            variavel(r"D:\teste"),
            variavel(r"C:\Dados"),
            variavel(r"C:\ProgramData"),
        )
        .unwrap();
        assert_eq!(sobreposta.registro(), Path::new(r"D:\teste\filas.toml"));
        assert!(sobreposta
            .historico(&nome)
            .starts_with(Path::new(r"D:\teste\historico")));
        let vazia = Pastas::das_variaveis(
            variavel(""),
            variavel(r"C:\Dados"),
            variavel(r"C:\ProgramData"),
        )
        .unwrap();
        assert_eq!(vazia, padrao);
        assert_eq!(
            Pastas::das_variaveis(None, None, variavel(r"C:\ProgramData")),
            Err(ErroPasta::SemVariavel("APPDATA"))
        );
        assert_eq!(
            Pastas::das_variaveis(variavel(r"D:\teste"), None, None),
            Err(ErroPasta::SemVariavel("ProgramData"))
        );
    }

    #[test]
    fn raiz_hostil_e_recusada() {
        assert_eq!(portas(r"relativa\pasta"), Err(ErroPasta::Relativo));
        assert_eq!(portas(r"\\servidor\pasta"), Err(ErroPasta::Relativo));
        assert_eq!(portas(r"\\?\C:\pasta"), Err(ErroPasta::Relativo));
        assert_eq!(portas(r"C:\a\..\b"), Err(ErroPasta::Subida));
        for caminho in [
            r"C:\a,b", r"C:\a$b", r"C:\a`b", "C:\\a\"b", r"C:\a'b", r"C:\a;b", r"C:\a*b",
            r"C:\a:b", "C:\\a\nb", "C:\\a\0b",
        ] {
            assert!(
                matches!(portas(caminho), Err(ErroPasta::Caractere(_))),
                "{caminho:?} passou"
            );
        }
        assert_eq!(
            portas(&format!(r"C:\{}", "a".repeat(LIMITE_CAMINHO))),
            Err(ErroPasta::Longo)
        );
    }
}
