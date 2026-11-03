//! O que já foi impresso, guardado na pasta do usuário em `historico.toml`.
//!
//! Um arquivo próprio, separado dos perfis: nunca atrapalha a impressão nem é lido por outra coisa.
//! Guarda o nome do arquivo impresso, nunca a pasta dele. Tudo o que vem do arquivo é entrada hostil.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::armazenamento::gravar_atomico;

pub const NOME_HISTORICO: &str = "historico.toml";
/// Os registros mais antigos saem depois deste número.
pub const LIMITE_REGISTROS: usize = 500;
/// Cabe o teto de registros com nomes longos; um arquivo maior não é lido.
pub const LIMITE_ARQUIVO_HISTORICO: u64 = 16 * 1024 * 1024;
/// Abaixo disso, a medida diria mais do que sabe, e a velocidade não aparece.
pub const FOLHAS_MINIMAS_PARA_VELOCIDADE: u64 = 10;
/// A partir desta razão entre a mais rápida e a mais lenta, a confirmação avisa.
pub const FATOR_DE_AVISO: f64 = 1.25;

const VERSAO_HISTORICO: u32 = 1;
const NOME_PASTA: &str = "plots";
const MAXIMO_DO_NOME: usize = 255;
const MAXIMO_DE_IMPRESSORAS: usize = 16;
const MS_POR_MINUTO: f64 = 60_000.0;
// Trabalhos distintos podem terminar ao mesmo tempo; leitura e gravação formam uma operação.
static GRAVACAO_HISTORICO: Mutex<()> = Mutex::new(());

#[derive(Debug, Error)]
pub enum ErroHistorico {
    #[error("a pasta do usuário não está disponível")]
    PastaIndisponivel,
    #[error("o histórico não pôde ser lido")]
    Leitura(#[source] io::Error),
    #[error("o histórico é grande demais para ser lido")]
    Grande,
    /// O arquivo fica como está: nada é gravado por cima dele.
    #[error("o histórico está danificado")]
    Corrompido,
    #[error("o histórico não pôde ser gravado")]
    Gravacao(#[source] io::Error),
    #[error("o registro não pôde ser escrito")]
    Serializacao,
}

/// Como um envio, ou a parte dele em uma impressora, terminou.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultadoDoEnvio {
    Concluido,
    Cancelado,
    ComErro,
    /// Parou de ser acompanhado, como ao fechar a janela, antes de todos os lotes saírem.
    Incompleto,
}

/// O que uma fila trabalhou: só do envio de cada passada até ela confirmar. A espera da pessoa e os
/// avisos ficam de fora.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TempoDaImpressora {
    pub impressora: String,
    pub milissegundos: u64,
    pub folhas: u64,
    pub resultado: ResultadoDoEnvio,
    /// Saiu em duas faces, sem gesto. Ausente em registros antigos, de quando só havia o manual.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub automatico: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistroDeEnvio {
    /// Só o nome, sem a pasta.
    pub arquivo: String,
    /// Segundos desde 1970, em UTC.
    pub data: u64,
    pub impressoras: Vec<String>,
    /// Primeira e última página do trecho; sem trecho, foi o documento inteiro.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intervalo: Option<(u64, u64)>,
    pub paginas: u64,
    pub lotes: u64,
    pub resultado: ResultadoDoEnvio,
    pub lotes_faltando: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tempos: Vec<TempoDaImpressora>,
}

impl RegistroDeEnvio {
    /// Os limites valem ao gravar também: nenhum envio faz o arquivo crescer sem controle.
    fn limitado(mut self) -> Self {
        let nome = self.arquivo.rsplit(['/', '\\']).next().unwrap_or("");
        self.arquivo = limpar_texto(nome, MAXIMO_DO_NOME);
        self.impressoras.truncate(MAXIMO_DE_IMPRESSORAS);
        for impressora in &mut self.impressoras {
            *impressora = limpar_texto(impressora, MAXIMO_DO_NOME);
        }
        self.tempos.truncate(MAXIMO_DE_IMPRESSORAS);
        for tempo in &mut self.tempos {
            tempo.impressora = limpar_texto(&tempo.impressora, MAXIMO_DO_NOME);
            tempo.milissegundos = cabe_no_arquivo(tempo.milissegundos);
            tempo.folhas = cabe_no_arquivo(tempo.folhas);
        }
        self.data = cabe_no_arquivo(self.data);
        self.paginas = cabe_no_arquivo(self.paginas);
        self.lotes = cabe_no_arquivo(self.lotes);
        self.lotes_faltando = cabe_no_arquivo(self.lotes_faltando);
        self.intervalo = self
            .intervalo
            .map(|(inicio, fim)| (cabe_no_arquivo(inicio), cabe_no_arquivo(fim)));
        self
    }
}

/// O formato só guarda inteiros com sinal de 64 bits.
fn cabe_no_arquivo(valor: u64) -> u64 {
    valor.min(i64::MAX.unsigned_abs())
}

/// Caractere que muda o que se lê sem aparecer: direção do texto, largura zero e afins.
fn eh_invisivel(letra: char) -> bool {
    matches!(
        letra,
        '\u{00AD}'
            | '\u{061C}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{E0000}'..='\u{E007F}'
    )
}

/// Texto pronto para uma linha: sem controle nem invisível, e de tamanho limitado.
pub fn limpar_texto(texto: &str, maximo: usize) -> String {
    texto
        .chars()
        .filter(|letra| !letra.is_control() && !eh_invisivel(*letra))
        .take(maximo)
        .collect()
}

#[derive(Serialize, Deserialize)]
struct ArquivoHistorico {
    versao: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    registro: Vec<toml::Value>,
}

/// O conteúdo do arquivo. Um registro que não se lê fica de fora da lista, mas volta igual ao gravar.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Historico {
    valores: Vec<toml::Value>,
}

impl Historico {
    /// Os registros que se leem, na ordem do arquivo.
    pub fn registros(&self) -> Vec<RegistroDeEnvio> {
        self.valores
            .iter()
            .filter_map(|valor| valor.clone().try_into::<RegistroDeEnvio>().ok())
            .map(RegistroDeEnvio::limitado)
            .collect()
    }

    /// Quantos registros do arquivo não puderam ser lidos.
    pub fn ignorados(&self) -> usize {
        self.valores
            .iter()
            .filter(|valor| (*valor).clone().try_into::<RegistroDeEnvio>().is_err())
            .count()
    }

    pub fn total(&self) -> usize {
        self.valores.len()
    }
}

/// Caminho de `historico.toml`. Só calcula: não cria pasta nem arquivo.
pub fn caminho_padrao_do_historico() -> Result<PathBuf, ErroHistorico> {
    let pastas = directories::BaseDirs::new().ok_or(ErroHistorico::PastaIndisponivel)?;
    Ok(pastas.config_dir().join(NOME_PASTA).join(NOME_HISTORICO))
}

/// Lê o histórico. Arquivo ausente é histórico vazio; nada aqui muda o arquivo.
pub fn carregar_historico(caminho: &Path) -> Result<Historico, ErroHistorico> {
    let arquivo = match File::open(caminho) {
        Ok(arquivo) => arquivo,
        Err(erro) if erro.kind() == io::ErrorKind::NotFound => return Ok(Historico::default()),
        Err(erro) => return Err(ErroHistorico::Leitura(erro)),
    };
    let mut bytes = Vec::new();
    arquivo
        .take(LIMITE_ARQUIVO_HISTORICO.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(ErroHistorico::Leitura)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > LIMITE_ARQUIVO_HISTORICO {
        return Err(ErroHistorico::Grande);
    }
    let texto = std::str::from_utf8(&bytes).map_err(|_| ErroHistorico::Corrompido)?;
    let lido: ArquivoHistorico = toml::from_str(texto).map_err(|_| ErroHistorico::Corrompido)?;
    if lido.versao != VERSAO_HISTORICO {
        return Err(ErroHistorico::Corrompido);
    }
    Ok(Historico {
        valores: lido.registro,
    })
}

/// Soma um registro. Um arquivo que não abriu não é substituído: o registro novo se perde, e o que
/// já estava lá continua inteiro. A gravação é atômica, e os mais antigos saem depois do teto.
pub fn acrescentar_ao_historico(
    caminho: &Path,
    registro: RegistroDeEnvio,
) -> Result<(), ErroHistorico> {
    let _guarda = GRAVACAO_HISTORICO
        .lock()
        .unwrap_or_else(|erro| erro.into_inner());
    let mut valores = carregar_historico(caminho)?.valores;
    let novo =
        toml::Value::try_from(registro.limitado()).map_err(|_| ErroHistorico::Serializacao)?;
    valores.push(novo);
    let excesso = valores.len().saturating_sub(LIMITE_REGISTROS);
    valores.drain(..excesso);
    let texto = toml::to_string(&ArquivoHistorico {
        versao: VERSAO_HISTORICO,
        registro: valores,
    })
    .map_err(|_| ErroHistorico::Serializacao)?;
    if let Some(pasta) = caminho
        .parent()
        .filter(|pasta| !pasta.as_os_str().is_empty())
    {
        fs::create_dir_all(pasta).map_err(ErroHistorico::Gravacao)?;
    }
    gravar_atomico(caminho, texto.as_bytes()).map_err(ErroHistorico::Gravacao)
}

/// Só por pedido do usuário. Já não existir conta como apagado. Um caminho com outro nome só é
/// apagado se o conteúdo for mesmo um histórico. O que uma gravação interrompida deixou na pasta
/// traz a mesma lista, e sai junto.
pub fn apagar_historico(caminho: &Path) -> Result<(), ErroHistorico> {
    let _guarda = GRAVACAO_HISTORICO
        .lock()
        .unwrap_or_else(|erro| erro.into_inner());
    if caminho
        .file_name()
        .is_none_or(|nome| nome != NOME_HISTORICO)
    {
        carregar_historico(caminho)?;
    }
    remover_se_existir(caminho)?;
    apagar_temporarios(caminho)
}

fn remover_se_existir(caminho: &Path) -> Result<(), ErroHistorico> {
    match fs::remove_file(caminho) {
        Ok(()) => Ok(()),
        Err(erro) if erro.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(erro) => Err(ErroHistorico::Gravacao(erro)),
    }
}

/// Só os temporários deste arquivo: `.nome.qualquer-coisa.tmp`, na mesma pasta.
fn apagar_temporarios(caminho: &Path) -> Result<(), ErroHistorico> {
    let Some(nome) = caminho.file_name() else {
        return Ok(());
    };
    let inicio = format!(".{}.", nome.to_string_lossy());
    let pasta = caminho
        .parent()
        .filter(|pasta| !pasta.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let itens = match fs::read_dir(pasta) {
        Ok(itens) => itens,
        Err(erro) if erro.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(erro) => return Err(ErroHistorico::Gravacao(erro)),
    };
    for item in itens {
        let item = item.map_err(ErroHistorico::Gravacao)?;
        let nome_do_item = item.file_name();
        let nome_do_item = nome_do_item.to_string_lossy();
        let e_temporario = nome_do_item.starts_with(&inicio) && nome_do_item.ends_with(".tmp");
        if e_temporario && item.file_type().is_ok_and(|tipo| tipo.is_file()) {
            remover_se_existir(&item.path())?;
        }
    }
    Ok(())
}

/// Folhas por minuto desta impressora nesta rota, só com trabalhos que terminaram e pelo menos o
/// mínimo de folhas medidas. Sem isso, não há número: nada aqui estima.
pub fn velocidade_da_impressora(
    registros: &[RegistroDeEnvio],
    impressora: &str,
    automatico: bool,
) -> Option<f64> {
    let (milissegundos, folhas) = registros
        .iter()
        .flat_map(|registro| &registro.tempos)
        .filter(|tempo| {
            tempo.impressora == impressora
                && tempo.automatico == automatico
                && tempo.resultado == ResultadoDoEnvio::Concluido
                && tempo.milissegundos > 0
                && tempo.folhas > 0
        })
        .fold((0u64, 0u64), |(ms, folhas), tempo| {
            (
                ms.saturating_add(tempo.milissegundos),
                folhas.saturating_add(tempo.folhas),
            )
        });
    if folhas < FOLHAS_MINIMAS_PARA_VELOCIDADE || milissegundos == 0 {
        return None;
    }
    let velocidade = folhas as f64 / (milissegundos as f64 / MS_POR_MINUTO);
    (velocidade.is_finite() && velocidade > 0.0).then_some(velocidade)
}

/// A mais rápida, a mais lenta e quantas vezes uma é a outra.
#[derive(Clone, Debug, PartialEq)]
pub struct ComparacaoDeVelocidade {
    pub rapida: String,
    pub lenta: String,
    pub razao: f64,
}

/// Compara as impressoras marcadas. Precisa de duas ou mais, com medida de todas, e de uma diferença
/// de pelo menos 25%; faltando qualquer coisa, não há o que dizer.
pub fn comparar_velocidades(medidas: &[(String, Option<f64>)]) -> Option<ComparacaoDeVelocidade> {
    if medidas.len() < 2 {
        return None;
    }
    let mut rapida: Option<(&str, f64)> = None;
    let mut lenta: Option<(&str, f64)> = None;
    for (nome, medida) in medidas {
        let valor = (*medida).filter(|valor| valor.is_finite() && *valor > 0.0)?;
        if rapida.is_none_or(|(_, atual)| valor > atual) {
            rapida = Some((nome, valor));
        }
        if lenta.is_none_or(|(_, atual)| valor < atual) {
            lenta = Some((nome, valor));
        }
    }
    let ((rapida, maior), (lenta, menor)) = (rapida?, lenta?);
    let razao = maior / menor;
    (razao.is_finite() && razao >= FATOR_DE_AVISO).then(|| ComparacaoDeVelocidade {
        rapida: rapida.to_owned(),
        lenta: lenta.to_owned(),
        razao,
    })
}
