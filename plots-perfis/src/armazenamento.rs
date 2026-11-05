#[cfg(test)]
use std::cell::Cell;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::erro::ErroPerfis;
use crate::formato;
use crate::perfis::Perfis;

/// Nome do arquivo de perfis dentro da pasta de configurações do PlotS.
pub const NOME_ARQUIVO: &str = "perfis.toml";

/// Arquivo que marca a versão portátil: ao lado do executável, guarda o perfil na mesma pasta.
pub const MARCADOR_PORTATIL: &str = "PLOTS-PORTATIL.txt";

/// Tamanho máximo aceito para ler ou gravar o arquivo de perfis (1 MiB).
pub const LIMITE_ARQUIVO_BYTES: u64 = 1024 * 1024;

const NOME_PASTA: &str = "plots";

static CONTADOR_TEMPORARIOS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
thread_local! {
    pub(crate) static FALHA_NA_GRAVACAO: Cell<bool> = const { Cell::new(false) };
}

/// Caminho de `perfis.toml`: ao lado do executável quando o marcador da versão portátil está
/// ali, ou na pasta de configurações do usuário nos outros casos.
///
/// Apenas calcula o caminho: não cria pasta nem arquivo.
pub fn caminho_padrao() -> Result<PathBuf, ErroPerfis> {
    let pasta_exe = pasta_do_executavel();
    if let Some(pasta) = pasta_exe.as_deref().filter(|pasta| tem_marcador(pasta)) {
        return Ok(pasta.join(NOME_ARQUIVO));
    }
    let pasta_de_configuracao = pasta_de_configuracao()?;
    Ok(caminho_para(None, &pasta_de_configuracao))
}

/// Caminho do arquivo de perfis para uma pasta de executável e uma de configuração.
///
/// A pasta do executável só vence quando o marcador da versão portátil é um arquivo ali.
/// Apenas calcula o caminho: não cria pasta nem arquivo.
pub fn caminho_para(pasta_do_exe: Option<&Path>, pasta_de_configuracao: &Path) -> PathBuf {
    match pasta_do_exe.filter(|pasta| tem_marcador(pasta)) {
        Some(pasta) => pasta.join(NOME_ARQUIVO),
        None => pasta_de_configuracao.join(NOME_PASTA).join(NOME_ARQUIVO),
    }
}

/// Traz o perfil da pasta de configuração para o lado do executável na primeira abertura
/// portátil. Nunca sobrescreve o arquivo local e, sem o marcador, não faz nada.
pub fn migrar_para_portatil() -> Result<(), ErroPerfis> {
    let Some(pasta) = pasta_do_executavel().filter(|pasta| tem_marcador(pasta)) else {
        return Ok(());
    };
    let origem = pasta_de_configuracao()?.join(NOME_PASTA).join(NOME_ARQUIVO);
    migrar(&origem, &pasta.join(NOME_ARQUIVO))
}

/// Copia os perfis de `origem` para `destino` quando o destino ainda não existe.
///
/// A origem é validada antes de qualquer gravação e continua intacta.
pub(crate) fn migrar(origem: &Path, destino: &Path) -> Result<(), ErroPerfis> {
    match destino.try_exists() {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(erro) => return Err(leitura(destino, erro)),
    }
    match origem.try_exists() {
        Ok(true) => {}
        Ok(false) => return Ok(()),
        Err(erro) => return Err(leitura(origem, erro)),
    }
    let perfis = carregar(origem)?;
    salvar(destino, &perfis)
}

fn pasta_do_executavel() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
}

fn pasta_de_configuracao() -> Result<PathBuf, ErroPerfis> {
    directories::BaseDirs::new()
        .map(|pastas| pastas.config_dir().to_path_buf())
        .ok_or(ErroPerfis::PastaUsuarioIndisponivel)
}

fn tem_marcador(pasta: &Path) -> bool {
    pasta.join(MARCADOR_PORTATIL).is_file()
}

/// Lê o arquivo de perfis. Arquivo ausente resulta em conjunto vazio.
///
/// Erros de leitura, de formato ou de tamanho não alteram o arquivo.
pub fn carregar(caminho: &Path) -> Result<Perfis, ErroPerfis> {
    let arquivo = match File::open(caminho) {
        Ok(arquivo) => arquivo,
        Err(erro) if erro.kind() == io::ErrorKind::NotFound => return Ok(Perfis::new()),
        Err(erro) => return Err(leitura(caminho, erro)),
    };
    let mut bytes = Vec::new();
    arquivo
        .take(LIMITE_ARQUIVO_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|erro| leitura(caminho, erro))?;
    if tamanho_em_u64(bytes.len()) > LIMITE_ARQUIVO_BYTES {
        return Err(ErroPerfis::ArquivoGrande {
            caminho: caminho.to_path_buf(),
        });
    }
    let texto = std::str::from_utf8(&bytes).map_err(|_| ErroPerfis::TomlInvalido {
        caminho: caminho.to_path_buf(),
        detalhe: "o conteúdo não está em UTF-8".to_owned(),
    })?;
    formato::decodificar(texto, caminho)
}

/// Grava os perfis de forma atômica: arquivo temporário na mesma pasta, flush e troca.
///
/// Se a gravação falhar, o arquivo anterior permanece igual e o temporário é removido.
/// Cria a pasta do destino quando ela não existe.
pub fn salvar(caminho: &Path, perfis: &Perfis) -> Result<(), ErroPerfis> {
    let texto = formato::codificar(perfis)?;
    if tamanho_em_u64(texto.len()) > LIMITE_ARQUIVO_BYTES {
        return Err(ErroPerfis::ConteudoGrande);
    }
    if let Some(pasta) = caminho
        .parent()
        .filter(|pasta| !pasta.as_os_str().is_empty())
    {
        fs::create_dir_all(pasta).map_err(|erro| gravacao(caminho, erro))?;
    }
    gravar_atomico(caminho, texto.as_bytes()).map_err(|erro| gravacao(caminho, erro))
}

pub(crate) fn gravar_atomico(destino: &Path, conteudo: &[u8]) -> io::Result<()> {
    let temporario = caminho_temporario(destino)?;
    // Só remove o temporário depois de criá-lo aqui; um nome já existente nunca é apagado.
    let arquivo = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporario)?;
    let resultado = escrever_e_trocar(arquivo, &temporario, destino, conteudo);
    if resultado.is_err() {
        let _ = fs::remove_file(&temporario);
    }
    resultado
}

fn escrever_e_trocar(
    mut arquivo: File,
    temporario: &Path,
    destino: &Path,
    conteudo: &[u8],
) -> io::Result<()> {
    arquivo.write_all(conteudo)?;
    verificar_falha_simulada()?;
    arquivo.sync_all()?;
    drop(arquivo);
    fs::rename(temporario, destino)
}

fn caminho_temporario(destino: &Path) -> io::Result<PathBuf> {
    let nome = destino.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "o caminho não termina em nome de arquivo",
        )
    })?;
    let sequencia = CONTADOR_TEMPORARIOS.fetch_add(1, Ordering::Relaxed);
    let nome_temporario = format!(
        ".{}.{}-{}.tmp",
        nome.to_string_lossy(),
        std::process::id(),
        sequencia
    );
    Ok(destino.with_file_name(nome_temporario))
}

#[cfg(test)]
fn verificar_falha_simulada() -> io::Result<()> {
    if FALHA_NA_GRAVACAO.with(|falha| falha.replace(false)) {
        return Err(io::Error::other("falha simulada na gravação"));
    }
    Ok(())
}

#[cfg(not(test))]
fn verificar_falha_simulada() -> io::Result<()> {
    Ok(())
}

fn tamanho_em_u64(tamanho: usize) -> u64 {
    u64::try_from(tamanho).unwrap_or(u64::MAX)
}

fn leitura(caminho: &Path, origem: io::Error) -> ErroPerfis {
    ErroPerfis::Leitura {
        caminho: caminho.to_path_buf(),
        origem,
    }
}

fn gravacao(caminho: &Path, origem: io::Error) -> ErroPerfis {
    ErroPerfis::Gravacao {
        caminho: caminho.to_path_buf(),
        origem,
    }
}
