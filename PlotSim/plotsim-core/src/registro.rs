//! Arquivo com as filas virtuais conhecidas e o modo físico de cada uma. O arquivo pode ter sido
//! editado à mão: tudo que vem dele é conferido, e um arquivo ruim nunca é sobrescrito.

use crate::modo::ModoFisico;
use crate::nome::{ErroNome, NomeVirtual};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const VERSAO: u32 = 1;
/// Maior arquivo aceito, em bytes.
const LIMITE_ARQUIVO: u64 = 256 * 1024;
/// Quantas filas virtuais cabem no registro.
pub const LIMITE_FILAS: usize = 64;

/// Falhas ao ler ou gravar o registro.
#[derive(Debug)]
pub enum ErroRegistro {
    Leitura(PathBuf, io::Error),
    Gravacao(PathBuf, io::Error),
    Grande(PathBuf),
    Formato(PathBuf, String),
    Versao(u32),
    Duplicada(String),
    Cheio,
}

impl fmt::Display for ErroRegistro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Leitura(caminho, erro) => {
                write!(f, "não deu para ler {}: {erro}", caminho.display())
            }
            Self::Gravacao(caminho, erro) => {
                write!(f, "não deu para gravar {}: {erro}", caminho.display())
            }
            Self::Grande(caminho) => {
                write!(f, "{} passa de {LIMITE_ARQUIVO} bytes", caminho.display())
            }
            Self::Formato(caminho, detalhe) => {
                write!(f, "{} está inválido: {detalhe}", caminho.display())
            }
            Self::Versao(versao) => write!(f, "registro em versão desconhecida: {versao}"),
            Self::Duplicada(nome) => write!(f, "o registro repete a fila {nome}"),
            Self::Cheio => write!(f, "o registro já tem {LIMITE_FILAS} filas"),
        }
    }
}

impl std::error::Error for ErroRegistro {}

#[derive(Deserialize, Serialize)]
struct EmDisco {
    versao: u32,
    #[serde(default, rename = "fila")]
    filas: Vec<EntradaEmDisco>,
}

#[derive(Deserialize, Serialize)]
struct EntradaEmDisco {
    nome: String,
    modo: ModoFisico,
}

/// Uma fila conhecida.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entrada {
    pub nome: NomeVirtual,
    pub modo: ModoFisico,
}

/// Filas conhecidas, na ordem em que foram criadas.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Registro {
    entradas: Vec<Entrada>,
}

impl Registro {
    /// Lê o arquivo. Sem arquivo, o registro está vazio.
    pub fn carregar(caminho: &Path) -> Result<Self, ErroRegistro> {
        let texto = match ler_com_limite(caminho) {
            Ok(texto) => texto,
            Err(erro) if erro.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(erro) if erro.kind() == ErrorKind::FileTooLarge => {
                return Err(ErroRegistro::Grande(caminho.to_path_buf()))
            }
            Err(erro) => return Err(ErroRegistro::Leitura(caminho.to_path_buf(), erro)),
        };
        let formato = |detalhe: String| ErroRegistro::Formato(caminho.to_path_buf(), detalhe);
        let disco: EmDisco = toml::from_str(&texto).map_err(|erro| formato(erro.to_string()))?;
        if disco.versao != VERSAO {
            return Err(ErroRegistro::Versao(disco.versao));
        }
        if disco.filas.len() > LIMITE_FILAS {
            return Err(ErroRegistro::Cheio);
        }
        let mut registro = Self::default();
        for entrada in disco.filas {
            let nome = NomeVirtual::novo(&entrada.nome)
                .map_err(|erro: ErroNome| formato(erro.to_string()))?;
            if registro.obter(&nome).is_some() {
                return Err(ErroRegistro::Duplicada(entrada.nome));
            }
            registro.entradas.push(Entrada {
                nome,
                modo: entrada.modo,
            });
        }
        Ok(registro)
    }

    /// Grava de forma atômica: o arquivo anterior fica inteiro se algo falhar.
    pub fn salvar(&self, caminho: &Path) -> Result<(), ErroRegistro> {
        let disco = EmDisco {
            versao: VERSAO,
            filas: self
                .entradas
                .iter()
                .map(|entrada| EntradaEmDisco {
                    nome: entrada.nome.curto().to_string(),
                    modo: entrada.modo,
                })
                .collect(),
        };
        let texto = toml::to_string(&disco)
            .map_err(|erro| ErroRegistro::Formato(caminho.to_path_buf(), erro.to_string()))?;
        gravar_atomico(caminho, texto.as_bytes())
            .map_err(|erro| ErroRegistro::Gravacao(caminho.to_path_buf(), erro))
    }

    pub fn entradas(&self) -> &[Entrada] {
        &self.entradas
    }

    pub fn obter(&self, nome: &NomeVirtual) -> Option<&Entrada> {
        self.entradas
            .iter()
            .find(|entrada| entrada.nome.mesma_fila(nome))
    }

    /// Cabe mais esta fila? A dela mesma sempre cabe.
    pub fn cabe(&self, nome: &NomeVirtual) -> bool {
        self.obter(nome).is_some() || self.entradas.len() < LIMITE_FILAS
    }

    /// Acrescenta a fila ou troca o modo da que já existe.
    pub fn definir(&mut self, nome: NomeVirtual, modo: ModoFisico) -> Result<(), ErroRegistro> {
        if !self.cabe(&nome) {
            return Err(ErroRegistro::Cheio);
        }
        match self
            .entradas
            .iter_mut()
            .find(|entrada| entrada.nome.mesma_fila(&nome))
        {
            Some(entrada) => entrada.modo = modo,
            None => self.entradas.push(Entrada { nome, modo }),
        }
        Ok(())
    }

    pub fn retirar(&mut self, nome: &NomeVirtual) {
        self.entradas
            .retain(|entrada| !entrada.nome.mesma_fila(nome));
    }
}

fn ler_com_limite(caminho: &Path) -> io::Result<String> {
    let arquivo = File::open(caminho)?;
    let mut bytes = Vec::new();
    arquivo
        .take(LIMITE_ARQUIVO.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).map_or(true, |lidos| lidos > LIMITE_ARQUIVO) {
        return Err(io::Error::from(ErrorKind::FileTooLarge));
    }
    String::from_utf8(bytes).map_err(|erro| io::Error::new(ErrorKind::InvalidData, erro))
}

static CONTADOR: AtomicU64 = AtomicU64::new(0);

/// Temporário na mesma pasta do destino, com nome que nenhum outro processo usa.
pub(crate) fn caminho_temporario(destino: &Path) -> io::Result<PathBuf> {
    let nome = destino.file_name().ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            "o caminho não termina em nome de arquivo",
        )
    })?;
    let mut temporario = nome.to_os_string();
    temporario.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        CONTADOR.fetch_add(1, Ordering::Relaxed)
    ));
    Ok(destino.with_file_name(temporario))
}

/// Temporário na mesma pasta, gravação até o disco e troca. Em falha, só o temporário é apagado.
pub(crate) fn gravar_atomico(destino: &Path, conteudo: &[u8]) -> io::Result<()> {
    if let Some(pasta) = destino.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(pasta)?;
    }
    let temporario = caminho_temporario(destino)?;
    let mut arquivo = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporario)?;
    let resultado = arquivo
        .write_all(conteudo)
        .and_then(|()| arquivo.sync_all())
        .and_then(|()| {
            drop(arquivo);
            fs::rename(&temporario, destino)
        });
    if resultado.is_err() {
        let _ = fs::remove_file(&temporario);
    }
    resultado
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::modo::{FaceDeSaida, Saida};
    use crate::testes_comuns::PastaTemporaria;

    const TRASEIRA: ModoFisico = ModoFisico {
        saida: Saida::Traseira,
        face: FaceDeSaida::ParaCima,
    };
    const GAVETA: ModoFisico = ModoFisico {
        saida: Saida::Gaveta,
        face: FaceDeSaida::ParaBaixo,
    };

    fn nome(texto: &str) -> NomeVirtual {
        NomeVirtual::novo(texto).unwrap()
    }

    #[test]
    fn sem_arquivo_o_registro_esta_vazio() {
        let pasta = PastaTemporaria::nova();
        let registro = Registro::carregar(&pasta.caminho("filas.toml")).unwrap();
        assert!(registro.entradas().is_empty());
    }

    #[test]
    fn modo_volta_igual_depois_de_gravar_e_ler() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.caminho("sub/filas.toml");
        let mut registro = Registro::default();
        registro.definir(nome("Jato"), TRASEIRA).unwrap();
        registro.definir(nome("Laser Ação"), GAVETA).unwrap();
        registro.salvar(&caminho).unwrap();
        assert_eq!(Registro::carregar(&caminho).unwrap(), registro);
        assert_eq!(fs::read_dir(caminho.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn definir_de_novo_troca_o_modo_sem_duplicar_mesmo_com_outra_caixa() {
        let mut registro = Registro::default();
        registro.definir(nome("Jato"), TRASEIRA).unwrap();
        registro.definir(nome("JATO"), GAVETA).unwrap();
        assert_eq!(registro.entradas().len(), 1);
        assert_eq!(registro.obter(&nome("jato")).unwrap().modo, GAVETA);
    }

    #[test]
    fn arquivo_ruim_vira_erro_e_fica_como_estava() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.caminho("filas.toml");
        for ruim in [
            "isto nao e toml [",
            "versao = 99",
            "versao = 1\n[[fila]]\nnome = \"a;b\"\nmodo = { saida = \"traseira\", face = \"para_cima\" }\n",
            "versao = 1\n[[fila]]\nnome = \"a\"\nmodo = { saida = \"voando\", face = \"para_cima\" }\n",
            "versao = 1\n[[fila]]\nnome = \"a\"\nmodo = { saida = \"traseira\", face = \"para_cima\" }\n[[fila]]\nnome = \"A\"\nmodo = { saida = \"traseira\", face = \"para_cima\" }\n",
        ] {
            fs::write(&caminho, ruim).unwrap();
            assert!(Registro::carregar(&caminho).is_err(), "{ruim} passou");
            assert_eq!(fs::read_to_string(&caminho).unwrap(), ruim);
        }
    }

    #[test]
    fn arquivo_grande_demais_e_recusado() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.caminho("filas.toml");
        let tamanho = usize::try_from(LIMITE_ARQUIVO).unwrap() + 1;
        fs::write(&caminho, vec![b'#'; tamanho]).unwrap();
        assert!(matches!(
            Registro::carregar(&caminho),
            Err(ErroRegistro::Grande(_))
        ));
    }

    #[test]
    fn registro_tem_limite_de_filas() {
        let mut registro = Registro::default();
        for numero in 0..LIMITE_FILAS {
            registro
                .definir(nome(&format!("f{numero}")), TRASEIRA)
                .unwrap();
        }
        assert!(matches!(
            registro.definir(nome("mais uma"), TRASEIRA),
            Err(ErroRegistro::Cheio)
        ));
        registro.definir(nome("f0"), GAVETA).unwrap();
    }

    #[test]
    fn falha_na_gravacao_nao_estraga_o_anterior_nem_deixa_temporario() {
        let pasta = PastaTemporaria::nova();
        let destino = pasta.caminho("alvo");
        fs::create_dir(&destino).unwrap();
        // Um destino que é pasta faz a troca falhar depois da escrita do temporário.
        assert!(gravar_atomico(&destino, b"novo").is_err());
        assert_eq!(fs::read_dir(pasta.raiz()).unwrap().count(), 1);
    }
}
