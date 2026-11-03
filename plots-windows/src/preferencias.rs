//! Ajuste do driver de cada fila, visto de fora deste crate: a resposta da janela do driver, as
//! falhas ao falar com ele e de onde o envio tira o ajuste salvo.
//!
//! O ajuste é um bloco opaco de bytes. Quem o guarda não precisa saber o que há nele, e nada aqui
//! altera o padrão da fila no Windows.

use crate::erro::ErroEnvio;
use std::sync::OnceLock;

/// Como a janela de preferências do driver terminou.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RespostaDaJanela {
    /// A pessoa aceitou: estes são os bytes a guardar para a fila.
    Aceita(Vec<u8>),
    /// A pessoa cancelou: nada muda.
    Cancelada,
}

/// Por que não deu para falar com o driver da fila.
#[derive(Clone, Debug, PartialEq)]
pub enum ErroPreferencias {
    /// A fila não existe mais no sistema.
    FilaInexistente,
    /// O driver ou o sistema recusou a chamada.
    Envio(ErroEnvio),
}

impl std::fmt::Display for ErroPreferencias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FilaInexistente => f.write_str("a fila não existe mais no Windows"),
            Self::Envio(erro) => write!(f, "{erro}"),
        }
    }
}

impl std::error::Error for ErroPreferencias {}

/// De onde o envio tira o ajuste salvo e a quem conta que o driver não o aceitou.
pub trait FonteDeAjustes: Send + Sync {
    /// Ajuste salvo para a fila com exatamente este nome.
    fn ajuste(&self, fila: &str) -> Option<Vec<u8>>;
    /// O driver não aceitou este ajuste, e o envio seguiu com o padrão da fila.
    fn recusado(&self, fila: &str, ajuste: &[u8]);
}

static FONTE: OnceLock<Box<dyn FonteDeAjustes>> = OnceLock::new();

/// Liga a fonte uma vez por processo. Sem fonte, o envio usa o padrão da fila.
pub fn ligar_fonte_de_ajustes(fonte: Box<dyn FonteDeAjustes>) {
    // Uma segunda chamada não troca a fonte: o ajuste de um trabalho não muda de origem.
    let _ = FONTE.set(fonte);
}

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn ajuste_salvo(fila: &str) -> Option<Vec<u8>> {
    FONTE.get().and_then(|fonte| fonte.ajuste(fila))
}

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn avisar_recusa(fila: &str, ajuste: &[u8]) {
    if let Some(fonte) = FONTE.get() {
        fonte.recusado(fila, ajuste);
    }
}

/// Fora do Windows não há driver a perguntar.
#[cfg(not(windows))]
pub fn abrir_janela_do_driver(
    _fila: &str,
    _dono: isize,
    _salvo: Option<&[u8]>,
) -> Result<RespostaDaJanela, ErroPreferencias> {
    Err(ErroPreferencias::Envio(ErroEnvio::NaoSuportado))
}

/// Fora do Windows não há janela aberta.
#[cfg(not(windows))]
pub fn ha_janela_do_driver_aberta() -> bool {
    false
}

#[cfg(not(windows))]
pub fn trazer_janela_do_driver_para_frente(_fila: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(not(windows))]
pub fn fechar_janelas_do_driver(_prazo: std::time::Duration) -> bool {
    true
}

#[cfg(not(windows))]
pub fn encerrar_janelas_do_driver(_prazo: std::time::Duration) -> bool {
    true
}

#[cfg(not(windows))]
pub fn ajuste_pede_duas_faces(_ajuste: &[u8]) -> bool {
    false
}
