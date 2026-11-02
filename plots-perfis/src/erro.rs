use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Falhas ao ler, interpretar ou gravar o arquivo de perfis.
#[derive(Debug, Error)]
pub enum ErroPerfis {
    #[error("não foi possível localizar a pasta de configurações do usuário")]
    PastaUsuarioIndisponivel,
    #[error("não foi possível ler o arquivo de perfis {caminho}: {origem}")]
    Leitura {
        caminho: PathBuf,
        #[source]
        origem: io::Error,
    },
    #[error("não foi possível gravar o arquivo de perfis {caminho}: {origem}")]
    Gravacao {
        caminho: PathBuf,
        #[source]
        origem: io::Error,
    },
    #[error("o arquivo de perfis {caminho} tem mais de 1 MB")]
    ArquivoGrande { caminho: PathBuf },
    #[error("o arquivo de perfis {caminho} não é um TOML válido: {detalhe}")]
    TomlInvalido { caminho: PathBuf, detalhe: String },
    #[error("o arquivo de perfis {caminho} usa a versão {versao}, que este programa não conhece")]
    VersaoDesconhecida { caminho: PathBuf, versao: u32 },
    #[error("os perfis não puderam ser convertidos para gravação: {detalhe}")]
    Serializacao { detalhe: String },
    #[error("os perfis ocupam mais de 1 MB e não foram gravados")]
    ConteudoGrande,
}
