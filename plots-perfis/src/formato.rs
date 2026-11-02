use std::collections::BTreeMap;
use std::path::Path;

use plots_core::{BordaEntrada, GestoRecolocar, PerfilImpressora};
use serde::{Deserialize, Serialize};

use crate::erro::ErroPerfis;
use crate::perfis::{EntradaPerfil, Perfis};

/// Versão do formato gravado em `perfis.toml`.
pub const VERSAO_ATUAL: u32 = 1;

/// Lê só a versão, ignorando o resto, para recusar formatos futuros com mensagem clara.
#[derive(Deserialize)]
struct Cabecalho {
    versao: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArquivoPerfis {
    versao: u32,
    #[serde(default)]
    filas: BTreeMap<String, EntradaArquivo>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntradaArquivo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    apelido: Option<String>,
    passada1_inversa: bool,
    passada2_inversa: bool,
    verso_girar_180: bool,
    virar_maco: bool,
    borda_primeiro: BordaArquivo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gesto_texto: Option<String>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum BordaArquivo {
    Curta,
    Longa,
}

impl From<BordaEntrada> for BordaArquivo {
    fn from(borda: BordaEntrada) -> Self {
        match borda {
            BordaEntrada::Curta => Self::Curta,
            BordaEntrada::Longa => Self::Longa,
        }
    }
}

impl From<BordaArquivo> for BordaEntrada {
    fn from(borda: BordaArquivo) -> Self {
        match borda {
            BordaArquivo::Curta => Self::Curta,
            BordaArquivo::Longa => Self::Longa,
        }
    }
}

impl From<EntradaArquivo> for EntradaPerfil {
    fn from(entrada: EntradaArquivo) -> Self {
        EntradaPerfil {
            apelido: entrada.apelido,
            perfil: PerfilImpressora {
                passada1_inversa: entrada.passada1_inversa,
                passada2_inversa: entrada.passada2_inversa,
                verso_girar_180: entrada.verso_girar_180,
                gesto_recolocar: GestoRecolocar {
                    virar_maco: entrada.virar_maco,
                    borda_primeiro: entrada.borda_primeiro.into(),
                },
            },
            gesto_texto: entrada.gesto_texto,
        }
    }
}

impl From<&EntradaPerfil> for EntradaArquivo {
    fn from(entrada: &EntradaPerfil) -> Self {
        let perfil = entrada.perfil;
        EntradaArquivo {
            apelido: entrada.apelido.clone(),
            passada1_inversa: perfil.passada1_inversa,
            passada2_inversa: perfil.passada2_inversa,
            verso_girar_180: perfil.verso_girar_180,
            virar_maco: perfil.gesto_recolocar.virar_maco,
            borda_primeiro: perfil.gesto_recolocar.borda_primeiro.into(),
            gesto_texto: entrada.gesto_texto.clone(),
        }
    }
}

/// Interpreta o texto de `perfis.toml`. Não altera nada em disco.
pub(crate) fn decodificar(texto: &str, caminho: &Path) -> Result<Perfis, ErroPerfis> {
    let cabecalho: Cabecalho = toml::from_str(texto).map_err(|erro| ErroPerfis::TomlInvalido {
        caminho: caminho.to_path_buf(),
        detalhe: erro.to_string(),
    })?;
    if cabecalho.versao != VERSAO_ATUAL {
        return Err(ErroPerfis::VersaoDesconhecida {
            caminho: caminho.to_path_buf(),
            versao: cabecalho.versao,
        });
    }
    let arquivo: ArquivoPerfis =
        toml::from_str(texto).map_err(|erro| ErroPerfis::TomlInvalido {
            caminho: caminho.to_path_buf(),
            detalhe: erro.to_string(),
        })?;
    let mut perfis = Perfis::new();
    for (fila, entrada) in arquivo.filas {
        perfis.inserir(&fila, entrada.into());
    }
    Ok(perfis)
}

/// Gera o texto de `perfis.toml` para o conjunto de perfis.
pub(crate) fn codificar(perfis: &Perfis) -> Result<String, ErroPerfis> {
    let arquivo = ArquivoPerfis {
        versao: VERSAO_ATUAL,
        filas: perfis
            .filas()
            .map(|(fila, entrada)| (fila.to_owned(), EntradaArquivo::from(entrada)))
            .collect(),
    };
    toml::to_string(&arquivo).map_err(|erro| ErroPerfis::Serializacao {
        detalhe: erro.to_string(),
    })
}
