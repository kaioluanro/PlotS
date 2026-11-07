//! Perfis de tinta e configuração da estimativa, guardados em `plotinfo.toml` ao lado do
//! `perfis.toml`.
//!
//! A leitura é tolerante: um arquivo ausente entrega os perfis padrão; um arquivo ilegível ou
//! grande demais também entrega os padrões, com um aviso. A gravação é atômica e nunca deixa um
//! temporário para trás.

use plots_info::{ConfiguracaoDeTinta, PerfilDeTinta, PerfisDeTinta};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

/// Nome do arquivo guardado ao lado de `perfis.toml`.
pub const NOME_ARQUIVO: &str = "plotinfo.toml";

/// Versão do esquema gravado.
pub const VERSAO_ARQUIVO: u32 = 1;

/// Tamanho máximo aceito para ler ou gravar o arquivo (1 MiB).
pub const LIMITE_ARQUIVO_BYTES: u64 = 1024 * 1024;

static GRAVACAO: Mutex<()> = Mutex::new(());
static CONTADOR_TEMPORARIOS: AtomicU64 = AtomicU64::new(0);

/// Por que uma gravação ou uma leitura obrigatória não foi concluída.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodigoFalhaPlotInfo {
    PastaIndisponivel,
    NaoLeu,
    NaoGravou,
    ArquivoGrande,
    ConteudoGrande,
    VersaoDesconhecida,
    PerfilInvalido,
    ConfiguracaoInvalida,
    PerfilNaoEncontrado,
    Interrompido,
}

/// O que a leitura tolerante encontrou de diferente do esperado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AvisoPlotInfo {
    PastaIndisponivel,
    ArquivoIlegivel,
    ArquivoGrande,
    ArquivoCorrompido,
    VersaoDesconhecida,
    PerfilIgnorado,
    ConfiguracaoIgnorada,
}

/// Falha de um comando, em forma de código; a interface traduz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FalhaPlotInfo {
    pub codigo: CodigoFalhaPlotInfo,
    pub detalhe: Option<String>,
}

impl FalhaPlotInfo {
    pub fn novo(codigo: CodigoFalhaPlotInfo) -> Self {
        Self {
            codigo,
            detalhe: None,
        }
    }

    pub fn com_detalhe(codigo: CodigoFalhaPlotInfo, detalhe: impl ToString) -> Self {
        Self {
            codigo,
            detalhe: Some(detalhe.to_string()),
        }
    }
}

impl std::fmt::Display for FalhaPlotInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.codigo)?;
        if let Some(detalhe) = &self.detalhe {
            write!(f, ": {detalhe}")?;
        }
        Ok(())
    }
}

impl std::error::Error for FalhaPlotInfo {}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EstadoDosPerfisDeTinta {
    pub perfis: Vec<PerfilDeTinta>,
    pub aviso: Option<AvisoPlotInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EstadoDaConfiguracaoDeTinta {
    pub configuracao: ConfiguracaoDeTinta,
    pub aviso: Option<AvisoPlotInfo>,
}

#[derive(Debug, Clone, PartialEq)]
struct Lido {
    perfis: PerfisDeTinta,
    configuracao: ConfiguracaoDeTinta,
    aviso: Option<AvisoPlotInfo>,
}

impl Lido {
    fn padrao(aviso: Option<AvisoPlotInfo>) -> Self {
        Self {
            perfis: PerfisDeTinta::padrao(),
            configuracao: ConfiguracaoDeTinta::default(),
            aviso,
        }
    }
}

#[derive(Deserialize)]
struct Cabecalho {
    versao: u32,
}

#[derive(Deserialize)]
struct ArquivoLido {
    #[serde(default)]
    configuracao: Option<toml::Value>,
    #[serde(default)]
    perfis: Vec<toml::Value>,
}

#[derive(Serialize)]
struct ArquivoParaGravar<'a> {
    versao: u32,
    configuracao: &'a ConfiguracaoDeTinta,
    perfis: &'a [PerfilDeTinta],
}

enum LeituraDoArquivo {
    Ausente,
    Bytes(Vec<u8>),
    Grande,
    Falhou,
}

/// Caminho de `plotinfo.toml`, na mesma pasta de `perfis.toml`.
pub fn caminho_padrao() -> Result<PathBuf, FalhaPlotInfo> {
    plots_perfis::caminho_padrao()
        .map(|caminho| caminho.with_file_name(NOME_ARQUIVO))
        .map_err(|erro| FalhaPlotInfo::com_detalhe(CodigoFalhaPlotInfo::PastaIndisponivel, erro))
}

fn ler_do_padrao() -> Lido {
    match caminho_padrao() {
        Ok(caminho) => ler_do_caminho(&caminho),
        Err(_) => Lido::padrao(Some(AvisoPlotInfo::PastaIndisponivel)),
    }
}

fn ler_bytes(caminho: &Path) -> LeituraDoArquivo {
    let arquivo = match File::open(caminho) {
        Ok(arquivo) => arquivo,
        Err(erro) if erro.kind() == io::ErrorKind::NotFound => return LeituraDoArquivo::Ausente,
        Err(_) => return LeituraDoArquivo::Falhou,
    };
    let mut bytes = Vec::new();
    if arquivo
        .take(LIMITE_ARQUIVO_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .is_err()
    {
        return LeituraDoArquivo::Falhou;
    }
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > LIMITE_ARQUIVO_BYTES {
        LeituraDoArquivo::Grande
    } else {
        LeituraDoArquivo::Bytes(bytes)
    }
}

fn ler_do_caminho(caminho: &Path) -> Lido {
    match ler_bytes(caminho) {
        LeituraDoArquivo::Ausente => Lido::padrao(None),
        LeituraDoArquivo::Grande => Lido::padrao(Some(AvisoPlotInfo::ArquivoGrande)),
        LeituraDoArquivo::Falhou => Lido::padrao(Some(AvisoPlotInfo::ArquivoIlegivel)),
        LeituraDoArquivo::Bytes(bytes) => interpretar(&bytes),
    }
}

fn interpretar(bytes: &[u8]) -> Lido {
    let Ok(texto) = std::str::from_utf8(bytes) else {
        return Lido::padrao(Some(AvisoPlotInfo::ArquivoCorrompido));
    };
    let Ok(cabecalho) = toml::from_str::<Cabecalho>(texto) else {
        return Lido::padrao(Some(AvisoPlotInfo::ArquivoCorrompido));
    };
    if cabecalho.versao != VERSAO_ARQUIVO {
        return Lido::padrao(Some(AvisoPlotInfo::VersaoDesconhecida));
    }
    let Ok(arquivo) = toml::from_str::<ArquivoLido>(texto) else {
        return Lido::padrao(Some(AvisoPlotInfo::ArquivoCorrompido));
    };

    let mut aviso = None;
    let mut perfis = PerfisDeTinta::new();
    let mut identificadores = BTreeSet::new();
    for valor in arquivo.perfis {
        let tentativa: Result<PerfilDeTinta, _> = valor.try_into();
        let Ok(perfil) = tentativa else {
            marcar(&mut aviso, AvisoPlotInfo::PerfilIgnorado);
            continue;
        };
        if perfil.validar().is_err() {
            marcar(&mut aviso, AvisoPlotInfo::PerfilIgnorado);
            continue;
        }
        if !identificadores.insert(perfil.id.clone()) {
            marcar(&mut aviso, AvisoPlotInfo::PerfilIgnorado);
            continue;
        }
        if perfis.substituir(perfil).is_err() {
            marcar(&mut aviso, AvisoPlotInfo::PerfilIgnorado);
        }
    }
    if perfis.vazio() {
        perfis = PerfisDeTinta::padrao();
        marcar(&mut aviso, AvisoPlotInfo::PerfilIgnorado);
    }

    let mut configuracao = match arquivo.configuracao {
        Some(valor) => {
            let tentativa: Result<ConfiguracaoDeTinta, _> = valor.try_into();
            match tentativa {
                Ok(configuracao) if configuracao.validar().is_ok() => configuracao,
                _ => {
                    marcar(&mut aviso, AvisoPlotInfo::ConfiguracaoIgnorada);
                    ConfiguracaoDeTinta::default()
                }
            }
        }
        None => ConfiguracaoDeTinta::default(),
    };
    if perfis.buscar(&configuracao.perfil).is_none() {
        if let Some(primeiro) = perfis.perfis().first() {
            configuracao.perfil = primeiro.id.clone();
            marcar(&mut aviso, AvisoPlotInfo::ConfiguracaoIgnorada);
        }
    }

    Lido {
        perfis,
        configuracao,
        aviso,
    }
}

fn marcar(aviso: &mut Option<AvisoPlotInfo>, novo: AvisoPlotInfo) {
    if aviso.is_none() {
        *aviso = Some(novo);
    }
}

/// Lê para uma gravação. Um arquivo que existe mas não pôde ser interpretado não é sobrescrito:
/// a gravação recusa, para não perder o que estava lá.
fn ler_para_gravar(caminho: &Path) -> Result<Lido, FalhaPlotInfo> {
    let lido = ler_do_caminho(caminho);
    match lido.aviso {
        Some(AvisoPlotInfo::ArquivoGrande) => {
            Err(FalhaPlotInfo::novo(CodigoFalhaPlotInfo::ArquivoGrande))
        }
        Some(AvisoPlotInfo::ArquivoCorrompido) | Some(AvisoPlotInfo::ArquivoIlegivel) => {
            Err(FalhaPlotInfo::novo(CodigoFalhaPlotInfo::NaoLeu))
        }
        Some(AvisoPlotInfo::VersaoDesconhecida) => {
            Err(FalhaPlotInfo::novo(CodigoFalhaPlotInfo::VersaoDesconhecida))
        }
        _ => Ok(lido),
    }
}

fn gravar_em(caminho: &Path, lido: &Lido) -> Result<(), FalhaPlotInfo> {
    lido.configuracao.validar().map_err(|erro| {
        FalhaPlotInfo::com_detalhe(CodigoFalhaPlotInfo::ConfiguracaoInvalida, erro)
    })?;
    for perfil in lido.perfis.perfis() {
        perfil.validar().map_err(|erro| {
            FalhaPlotInfo::com_detalhe(CodigoFalhaPlotInfo::PerfilInvalido, erro)
        })?;
    }
    if lido.perfis.buscar(&lido.configuracao.perfil).is_none() {
        return Err(FalhaPlotInfo::novo(
            CodigoFalhaPlotInfo::PerfilNaoEncontrado,
        ));
    }
    let texto = toml::to_string(&ArquivoParaGravar {
        versao: VERSAO_ARQUIVO,
        configuracao: &lido.configuracao,
        perfis: lido.perfis.perfis(),
    })
    .map_err(|erro| FalhaPlotInfo::com_detalhe(CodigoFalhaPlotInfo::NaoGravou, erro))?;
    if u64::try_from(texto.len()).unwrap_or(u64::MAX) > LIMITE_ARQUIVO_BYTES {
        return Err(FalhaPlotInfo::novo(CodigoFalhaPlotInfo::ConteudoGrande));
    }
    if let Some(pasta) = caminho
        .parent()
        .filter(|pasta| !pasta.as_os_str().is_empty())
    {
        fs::create_dir_all(pasta)
            .map_err(|erro| FalhaPlotInfo::com_detalhe(CodigoFalhaPlotInfo::NaoGravou, erro))?;
    }
    gravar_atomico(caminho, texto.as_bytes())
        .map_err(|erro| FalhaPlotInfo::com_detalhe(CodigoFalhaPlotInfo::NaoGravou, erro))
}

fn gravar_atomico(destino: &Path, conteudo: &[u8]) -> io::Result<()> {
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

fn salvar_perfil_em(caminho: &Path, perfil: PerfilDeTinta) -> Result<Lido, FalhaPlotInfo> {
    perfil
        .validar()
        .map_err(|erro| FalhaPlotInfo::com_detalhe(CodigoFalhaPlotInfo::PerfilInvalido, erro))?;
    let _vez = GRAVACAO.lock().unwrap_or_else(PoisonError::into_inner);
    let mut lido = ler_para_gravar(caminho)?;
    lido.perfis
        .substituir(perfil)
        .map_err(|erro| FalhaPlotInfo::com_detalhe(CodigoFalhaPlotInfo::PerfilInvalido, erro))?;
    lido.aviso = None;
    gravar_em(caminho, &lido)?;
    Ok(lido)
}

fn apagar_perfil_em(caminho: &Path, id: &str) -> Result<Lido, FalhaPlotInfo> {
    let _vez = GRAVACAO.lock().unwrap_or_else(PoisonError::into_inner);
    let mut lido = ler_para_gravar(caminho)?;
    if !lido.perfis.remover(id) {
        return Err(FalhaPlotInfo::novo(
            CodigoFalhaPlotInfo::PerfilNaoEncontrado,
        ));
    }
    // O programa sempre precisa de um perfil para calcular: ao apagar o último, os padrão voltam.
    if lido.perfis.vazio() {
        lido.perfis = PerfisDeTinta::padrao();
    }
    if lido.perfis.buscar(&lido.configuracao.perfil).is_none() {
        lido.configuracao.perfil = lido
            .perfis
            .perfis()
            .first()
            .map(|perfil| perfil.id.clone())
            .unwrap_or_default();
    }
    lido.aviso = None;
    gravar_em(caminho, &lido)?;
    Ok(lido)
}

fn definir_configuracao_em(
    caminho: &Path,
    configuracao: ConfiguracaoDeTinta,
) -> Result<Lido, FalhaPlotInfo> {
    configuracao.validar().map_err(|erro| {
        FalhaPlotInfo::com_detalhe(CodigoFalhaPlotInfo::ConfiguracaoInvalida, erro)
    })?;
    let _vez = GRAVACAO.lock().unwrap_or_else(PoisonError::into_inner);
    let mut lido = ler_para_gravar(caminho)?;
    if lido.perfis.buscar(&configuracao.perfil).is_none() {
        return Err(FalhaPlotInfo::novo(
            CodigoFalhaPlotInfo::PerfilNaoEncontrado,
        ));
    }
    lido.configuracao = configuracao;
    lido.aviso = None;
    gravar_em(caminho, &lido)?;
    Ok(lido)
}

/// Perfil de tinta guardado com este identificador exato. A leitura é a mesma dos comandos: sem
/// arquivo, valem os perfis padrão.
pub(crate) fn perfil_guardado(id: &str) -> Option<PerfilDeTinta> {
    ler_do_padrao().perfis.buscar(id).cloned()
}

/// Perfis de tinta guardados, ou os padrão quando não há arquivo.
#[tauri::command]
pub async fn perfis_de_tinta() -> EstadoDosPerfisDeTinta {
    tauri::async_runtime::spawn_blocking(|| {
        let lido = ler_do_padrao();
        EstadoDosPerfisDeTinta {
            perfis: lido.perfis.perfis().to_vec(),
            aviso: lido.aviso,
        }
    })
    .await
    .unwrap_or_else(|_| EstadoDosPerfisDeTinta {
        perfis: PerfisDeTinta::padrao().perfis().to_vec(),
        aviso: Some(AvisoPlotInfo::ArquivoIlegivel),
    })
}

/// Valida e guarda um perfil, trocando o de mesmo identificador.
#[tauri::command]
pub async fn salvar_perfil_de_tinta(
    perfil: PerfilDeTinta,
) -> Result<EstadoDosPerfisDeTinta, FalhaPlotInfo> {
    tauri::async_runtime::spawn_blocking(move || {
        let caminho = caminho_padrao()?;
        let lido = salvar_perfil_em(&caminho, perfil)?;
        Ok(EstadoDosPerfisDeTinta {
            perfis: lido.perfis.perfis().to_vec(),
            aviso: lido.aviso,
        })
    })
    .await
    .unwrap_or_else(|_| Err(FalhaPlotInfo::novo(CodigoFalhaPlotInfo::Interrompido)))
}

/// Remove um perfil. O ativo passa para o primeiro que restar; sem nenhum, os padrão voltam.
#[tauri::command]
pub async fn apagar_perfil_de_tinta(id: String) -> Result<EstadoDosPerfisDeTinta, FalhaPlotInfo> {
    tauri::async_runtime::spawn_blocking(move || {
        let caminho = caminho_padrao()?;
        let lido = apagar_perfil_em(&caminho, &id)?;
        Ok(EstadoDosPerfisDeTinta {
            perfis: lido.perfis.perfis().to_vec(),
            aviso: lido.aviso,
        })
    })
    .await
    .unwrap_or_else(|_| Err(FalhaPlotInfo::novo(CodigoFalhaPlotInfo::Interrompido)))
}

/// Configuração guardada, ou a padrão quando não há arquivo.
#[tauri::command]
pub async fn configuracao_de_tinta() -> EstadoDaConfiguracaoDeTinta {
    tauri::async_runtime::spawn_blocking(|| {
        let lido = ler_do_padrao();
        EstadoDaConfiguracaoDeTinta {
            configuracao: lido.configuracao,
            aviso: lido.aviso,
        }
    })
    .await
    .unwrap_or_else(|_| EstadoDaConfiguracaoDeTinta {
        configuracao: ConfiguracaoDeTinta::default(),
        aviso: Some(AvisoPlotInfo::ArquivoIlegivel),
    })
}

/// Valida e guarda a configuração; o perfil indicado precisa existir.
#[tauri::command]
pub async fn definir_configuracao_de_tinta(
    configuracao: ConfiguracaoDeTinta,
) -> Result<EstadoDaConfiguracaoDeTinta, FalhaPlotInfo> {
    tauri::async_runtime::spawn_blocking(move || {
        let caminho = caminho_padrao()?;
        let lido = definir_configuracao_em(&caminho, configuracao)?;
        Ok(EstadoDaConfiguracaoDeTinta {
            configuracao: lido.configuracao,
            aviso: lido.aviso,
        })
    })
    .await
    .unwrap_or_else(|_| Err(FalhaPlotInfo::novo(CodigoFalhaPlotInfo::Interrompido)))
}

#[cfg(test)]
mod testes {
    use super::*;
    use plots_info::{Qualidade, TamanhoDaImagem};

    static CONTADOR: AtomicU64 = AtomicU64::new(0);

    struct Pasta(PathBuf);

    impl Pasta {
        fn nova() -> Self {
            let sequencia = CONTADOR.fetch_add(1, Ordering::Relaxed);
            let caminho = std::env::temp_dir()
                .join(format!("plots-plotinfo-{}-{sequencia}", std::process::id()));
            fs::create_dir_all(&caminho).unwrap();
            Self(caminho)
        }

        fn arquivo(&self) -> PathBuf {
            self.0.join(NOME_ARQUIVO)
        }
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn perfil_novo(id: &str, nome: &str) -> PerfilDeTinta {
        PerfilDeTinta {
            id: id.to_owned(),
            nome: nome.to_owned(),
            moeda: "BRL".to_owned(),
            volumes_ml: [50.0, 50.0, 50.0, 50.0],
            rendimentos_paginas: [5000, 5000, 5000, 5000],
            precos_ml: [1.0, 2.0, 3.0, 4.0],
            fatores_qualidade: [0.6, 1.0, 1.3],
        }
    }

    fn perfil_toml(id: &str, nome: &str) -> String {
        format!(
            "[[perfis]]\nid = {id:?}\nnome = {nome:?}\nmoeda = \"BRL\"\n\
             volumes_ml = [50.0, 50.0, 50.0, 50.0]\n\
             rendimentos_paginas = [5000, 5000, 5000, 5000]\n\
             precos_ml = [1.0, 2.0, 3.0, 4.0]\n\
             fatores_qualidade = [0.6, 1.0, 1.3]\n"
        )
    }

    fn cabecalho(perfil: &str) -> String {
        format!(
            "versao = 1\n\n[configuracao]\nperfil = {perfil:?}\nqualidade = \"normal\"\n\
             dpi = 150\nmodo = \"cmyk\"\ncinzas_so_preto = true\ntamanho_da_imagem = \"a4\"\n\n"
        )
    }

    fn nomes_de_temporarios(pasta: &Path) -> Vec<String> {
        fs::read_dir(pasta)
            .unwrap()
            .filter_map(|entrada| entrada.ok())
            .map(|entrada| entrada.file_name().to_string_lossy().into_owned())
            .filter(|nome| nome.ends_with(".tmp"))
            .collect()
    }

    #[test]
    fn o_arquivo_fica_ao_lado_do_arquivo_de_perfis() {
        let caminho = caminho_padrao().unwrap();
        assert_eq!(
            caminho.file_name().and_then(|nome| nome.to_str()),
            Some(NOME_ARQUIVO)
        );
        let perfis = plots_perfis::caminho_padrao().unwrap();
        assert_eq!(caminho.parent(), perfis.parent());
    }

    #[test]
    fn arquivo_ausente_entrega_os_padroes_sem_criar_arquivo() {
        let pasta = Pasta::nova();
        let lido = ler_do_caminho(&pasta.arquivo());
        assert_eq!(lido.perfis, PerfisDeTinta::padrao());
        assert_eq!(lido.configuracao, ConfiguracaoDeTinta::default());
        assert_eq!(lido.aviso, None);
        assert!(!pasta.arquivo().exists());
    }

    #[test]
    fn round_trip_preserva_perfis_e_configuracao() {
        let pasta = Pasta::nova();
        let mut lido = Lido::padrao(None);
        lido.perfis
            .substituir(perfil_novo("meu", "Meu perfil"))
            .unwrap();
        lido.configuracao.perfil = "meu".to_owned();
        lido.configuracao.qualidade = Qualidade::Alta;
        lido.configuracao.dpi = 300;
        lido.configuracao.cinzas_so_preto = false;
        lido.configuracao.tamanho_da_imagem = TamanhoDaImagem::Dpi;
        gravar_em(&pasta.arquivo(), &lido).unwrap();

        let relido = ler_do_caminho(&pasta.arquivo());
        assert_eq!(relido.perfis, lido.perfis);
        assert_eq!(relido.configuracao, lido.configuracao);
        assert_eq!(relido.aviso, None);

        let texto = fs::read_to_string(pasta.arquivo()).unwrap();
        assert!(texto.contains("versao = 1"));
        assert!(texto.contains("[configuracao]"));
        assert!(texto.contains("[[perfis]]"));
        assert!(texto.contains("cinzas_so_preto = false"));
        assert!(texto.contains("tamanho_da_imagem = \"dpi\""));
    }

    #[test]
    fn arquivo_corrompido_cai_nos_padroes_e_avisa_sem_apagar() {
        let pasta = Pasta::nova();
        fs::write(pasta.arquivo(), "isto nao e toml = = =").unwrap();
        let lido = ler_do_caminho(&pasta.arquivo());
        assert_eq!(lido.perfis, PerfisDeTinta::padrao());
        assert_eq!(lido.configuracao, ConfiguracaoDeTinta::default());
        assert_eq!(lido.aviso, Some(AvisoPlotInfo::ArquivoCorrompido));
        assert_eq!(
            fs::read_to_string(pasta.arquivo()).unwrap(),
            "isto nao e toml = = ="
        );
    }

    #[test]
    fn arquivo_grande_demais_e_recusado_na_leitura_e_na_gravacao() {
        let pasta = Pasta::nova();
        fs::write(
            pasta.arquivo(),
            vec![b'x'; LIMITE_ARQUIVO_BYTES as usize + 1],
        )
        .unwrap();
        let lido = ler_do_caminho(&pasta.arquivo());
        assert_eq!(lido.perfis, PerfisDeTinta::padrao());
        assert_eq!(lido.aviso, Some(AvisoPlotInfo::ArquivoGrande));

        let erro = salvar_perfil_em(&pasta.arquivo(), perfil_novo("novo", "Novo")).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaPlotInfo::ArquivoGrande);
        assert_eq!(
            fs::metadata(pasta.arquivo()).unwrap().len(),
            LIMITE_ARQUIVO_BYTES + 1
        );
    }

    #[test]
    fn versao_desconhecida_cai_nos_padroes_e_nao_e_sobrescrita() {
        let pasta = Pasta::nova();
        fs::write(pasta.arquivo(), "versao = 2\n").unwrap();
        let lido = ler_do_caminho(&pasta.arquivo());
        assert_eq!(lido.aviso, Some(AvisoPlotInfo::VersaoDesconhecida));
        assert_eq!(lido.perfis, PerfisDeTinta::padrao());

        let erro = salvar_perfil_em(&pasta.arquivo(), perfil_novo("novo", "Novo")).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaPlotInfo::VersaoDesconhecida);
        assert_eq!(fs::read_to_string(pasta.arquivo()).unwrap(), "versao = 2\n");
    }

    #[test]
    fn arquivo_corrompido_nao_e_sobrescrito_por_uma_gravacao() {
        let pasta = Pasta::nova();
        fs::write(pasta.arquivo(), "= =").unwrap();
        let erro = salvar_perfil_em(&pasta.arquivo(), perfil_novo("novo", "Novo")).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaPlotInfo::NaoLeu);
        assert_eq!(fs::read_to_string(pasta.arquivo()).unwrap(), "= =");
    }

    #[test]
    fn perfil_invalido_no_arquivo_e_ignorado_sem_recusar_os_outros() {
        let pasta = Pasta::nova();
        let texto = format!(
            "{}{}{}",
            cabecalho("bom"),
            perfil_toml("bom", "Bom"),
            perfil_toml("", "Sem identificador")
        );
        fs::write(pasta.arquivo(), texto).unwrap();
        let lido = ler_do_caminho(&pasta.arquivo());
        assert_eq!(lido.perfis.perfis().len(), 1);
        assert!(lido.perfis.buscar("bom").is_some());
        assert_eq!(lido.aviso, Some(AvisoPlotInfo::PerfilIgnorado));
    }

    #[test]
    fn identificador_repetido_no_arquivo_mantem_o_primeiro() {
        let pasta = Pasta::nova();
        let texto = format!(
            "{}{}{}",
            cabecalho("repetido"),
            perfil_toml("repetido", "Primeiro"),
            perfil_toml("repetido", "Segundo")
        );
        fs::write(pasta.arquivo(), texto).unwrap();
        let lido = ler_do_caminho(&pasta.arquivo());
        assert_eq!(lido.perfis.perfis().len(), 1);
        assert_eq!(
            lido.perfis.buscar("repetido").map(|perfil| &perfil.nome),
            Some(&"Primeiro".to_owned())
        );
        assert_eq!(lido.aviso, Some(AvisoPlotInfo::PerfilIgnorado));
    }

    #[test]
    fn configuracao_invalida_no_arquivo_cai_na_padrao_e_avisa() {
        let pasta = Pasta::nova();
        let texto = format!(
            "versao = 1\n\n[configuracao]\nperfil = \"bom\"\nqualidade = \"normal\"\n\
             dpi = 200\nmodo = \"cmyk\"\ncinzas_so_preto = true\ntamanho_da_imagem = \"a4\"\n\n{}",
            perfil_toml("bom", "Bom")
        );
        fs::write(pasta.arquivo(), texto).unwrap();
        let lido = ler_do_caminho(&pasta.arquivo());
        assert_eq!(lido.configuracao.dpi, 150);
        assert_eq!(lido.configuracao.qualidade, Qualidade::Normal);
        assert_eq!(lido.configuracao.perfil, "bom");
        assert_eq!(lido.aviso, Some(AvisoPlotInfo::ConfiguracaoIgnorada));
    }

    #[test]
    fn configuracao_apontando_para_perfil_ausente_usa_o_primeiro() {
        let pasta = Pasta::nova();
        let texto = format!("{}{}", cabecalho("sumiu"), perfil_toml("bom", "Bom"));
        fs::write(pasta.arquivo(), texto).unwrap();
        let lido = ler_do_caminho(&pasta.arquivo());
        assert_eq!(lido.configuracao.perfil, "bom");
        assert_eq!(lido.aviso, Some(AvisoPlotInfo::ConfiguracaoIgnorada));
    }

    #[test]
    fn salvar_perfil_com_o_mesmo_identificador_substitui_nao_duplica() {
        let pasta = Pasta::nova();
        gravar_em(&pasta.arquivo(), &Lido::padrao(None)).unwrap();
        let mut perfil = PerfisDeTinta::padrao()
            .buscar("tanque-generico")
            .unwrap()
            .clone();
        perfil.nome = "Tanque novo".to_owned();
        let lido = salvar_perfil_em(&pasta.arquivo(), perfil).unwrap();
        assert_eq!(lido.perfis.perfis().len(), 5);
        assert_eq!(
            lido.perfis
                .buscar("tanque-generico")
                .map(|perfil| &perfil.nome),
            Some(&"Tanque novo".to_owned())
        );
        let relido = ler_do_caminho(&pasta.arquivo());
        assert_eq!(relido.perfis, lido.perfis);
    }

    #[test]
    fn perfil_invalido_nao_altera_o_arquivo() {
        let pasta = Pasta::nova();
        gravar_em(&pasta.arquivo(), &Lido::padrao(None)).unwrap();
        let antes = fs::read(pasta.arquivo()).unwrap();
        let mut perfil = perfil_novo("novo", "Novo");
        perfil.id.clear();
        let erro = salvar_perfil_em(&pasta.arquivo(), perfil).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaPlotInfo::PerfilInvalido);
        assert_eq!(fs::read(pasta.arquivo()).unwrap(), antes);
    }

    #[test]
    fn apagar_o_perfil_ativo_troca_a_configuracao_para_outro() {
        let pasta = Pasta::nova();
        let mut lido = Lido::padrao(None);
        lido.perfis
            .substituir(perfil_novo("segundo", "Segundo"))
            .unwrap();
        lido.configuracao.perfil = "segundo".to_owned();
        gravar_em(&pasta.arquivo(), &lido).unwrap();

        let lido = apagar_perfil_em(&pasta.arquivo(), "segundo").unwrap();
        assert!(lido.perfis.buscar("segundo").is_none());
        assert_eq!(lido.configuracao.perfil, "tanque-generico");
        assert_eq!(
            ler_do_caminho(&pasta.arquivo()).configuracao.perfil,
            "tanque-generico"
        );
    }

    #[test]
    fn apagar_o_ultimo_perfil_traz_os_padroes_de_volta() {
        let pasta = Pasta::nova();
        let mut lido = Lido {
            perfis: PerfisDeTinta::new(),
            configuracao: ConfiguracaoDeTinta::default(),
            aviso: None,
        };
        lido.perfis
            .substituir(perfil_novo("unico", "Único"))
            .unwrap();
        lido.configuracao.perfil = "unico".to_owned();
        gravar_em(&pasta.arquivo(), &lido).unwrap();

        let lido = apagar_perfil_em(&pasta.arquivo(), "unico").unwrap();
        assert_eq!(lido.perfis, PerfisDeTinta::padrao());
        assert_eq!(lido.configuracao.perfil, "tanque-generico");
    }

    #[test]
    fn apagar_perfil_inexistente_vira_erro_sem_gravar() {
        let pasta = Pasta::nova();
        gravar_em(&pasta.arquivo(), &Lido::padrao(None)).unwrap();
        let antes = fs::read(pasta.arquivo()).unwrap();
        let erro = apagar_perfil_em(&pasta.arquivo(), "nao-existe").unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaPlotInfo::PerfilNaoEncontrado);
        assert_eq!(fs::read(pasta.arquivo()).unwrap(), antes);
    }

    #[test]
    fn configuracao_invalida_ou_com_perfil_ausente_nao_e_gravada() {
        let pasta = Pasta::nova();
        gravar_em(&pasta.arquivo(), &Lido::padrao(None)).unwrap();
        let antes = fs::read(pasta.arquivo()).unwrap();

        let mut configuracao = ConfiguracaoDeTinta::default();
        configuracao.dpi = 200;
        let erro = definir_configuracao_em(&pasta.arquivo(), configuracao).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaPlotInfo::ConfiguracaoInvalida);

        let mut configuracao = ConfiguracaoDeTinta::default();
        configuracao.perfil = "nao-existe".to_owned();
        let erro = definir_configuracao_em(&pasta.arquivo(), configuracao).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaPlotInfo::PerfilNaoEncontrado);
        assert_eq!(fs::read(pasta.arquivo()).unwrap(), antes);
    }

    #[test]
    fn configuracao_valida_e_gravada_e_relida() {
        let pasta = Pasta::nova();
        gravar_em(&pasta.arquivo(), &Lido::padrao(None)).unwrap();
        let mut configuracao = ConfiguracaoDeTinta::default();
        configuracao.qualidade = Qualidade::Rascunho;
        configuracao.modo = plots_info::ModoDeCor::Simples;
        let lido = definir_configuracao_em(&pasta.arquivo(), configuracao.clone()).unwrap();
        assert_eq!(lido.configuracao, configuracao);
        assert_eq!(ler_do_caminho(&pasta.arquivo()).configuracao, configuracao);
    }

    #[test]
    fn gravacao_atomica_nao_deixa_temporario_para_tras() {
        let pasta = Pasta::nova();
        gravar_em(&pasta.arquivo(), &Lido::padrao(None)).unwrap();
        gravar_em(&pasta.arquivo(), &Lido::padrao(None)).unwrap();
        assert!(nomes_de_temporarios(&pasta.0).is_empty());
        assert!(pasta.arquivo().exists());
    }

    #[test]
    fn pasta_de_destino_que_nao_pode_ser_criada_vira_falha_de_gravacao() {
        let pasta = Pasta::nova();
        let bloqueio = pasta.0.join("bloqueio");
        fs::write(&bloqueio, b"x").unwrap();
        let caminho = bloqueio.join(NOME_ARQUIVO);
        let erro = gravar_em(&caminho, &Lido::padrao(None)).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaPlotInfo::NaoGravou);
    }

    #[test]
    fn estados_dos_comandos_mostram_perfis_e_aviso() {
        let perfis = EstadoDosPerfisDeTinta {
            perfis: PerfisDeTinta::padrao().perfis().to_vec(),
            aviso: Some(AvisoPlotInfo::ArquivoGrande),
        };
        assert_eq!(perfis.perfis.len(), 5);
        assert_eq!(perfis.aviso, Some(AvisoPlotInfo::ArquivoGrande));
        let configuracao = EstadoDaConfiguracaoDeTinta {
            configuracao: ConfiguracaoDeTinta::default(),
            aviso: None,
        };
        assert_eq!(configuracao.configuracao.perfil, "tanque-generico");
    }
}
