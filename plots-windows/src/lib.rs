//! Leitura das filas de impressão do Windows e do estado de cada uma.
//!
//! Este crate só lê. Nenhuma página é enviada a fila alguma, física ou virtual,
//! e nenhuma fila é alterada.
//!
//! O estado reportado vem do driver de cada fila: bits iguais podem ter sentidos
//! diferentes em drivers diferentes, por isso os bits sem tradução são preservados.

mod estado;
mod texto;

#[cfg(windows)]
mod sistema;

pub use estado::{Condicao, Estado};
pub use texto::texto_utf16_le;

/// Limite do buffer pedido ao spooler (8 MiB). Acima disso a listagem é recusada.
pub const LIMITE_BUFFER_BYTES: u32 = 8 * 1024 * 1024;

/// Uma fila do sistema, como o spooler a descreve.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fila {
    /// Nome exato da fila, sem alteração: é a chave dos perfis salvos.
    pub nome: String,
    /// Nome do driver de impressão.
    pub driver: String,
    /// Nome da porta; vazio quando o sistema não informa.
    pub porta: String,
    /// `true` quando é a impressora padrão do usuário.
    pub padrao: bool,
    /// Estado lido do spooler.
    pub estado: Estado,
}

/// Falhas ao listar as filas. Nenhuma delas indica que não há filas.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErroFilas {
    /// A listagem só existe no Windows.
    NaoSuportado,
    /// O sistema pediu um buffer maior que `LIMITE_BUFFER_BYTES`.
    Grande { bytes: u32 },
    /// O sistema recusou a chamada; o código é o HRESULT.
    Sistema(u32),
    /// A resposta do sistema não tem a forma esperada.
    Resposta(&'static str),
}

impl std::fmt::Display for ErroFilas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NaoSuportado => {
                write!(f, "a listagem de filas só está disponível no Windows")
            }
            Self::Grande { bytes } => write!(
                f,
                "a lista de filas pede {bytes} bytes, acima do limite de {LIMITE_BUFFER_BYTES}"
            ),
            Self::Sistema(codigo) => write!(
                f,
                "o Windows recusou a listagem de filas (código {codigo:#010X})"
            ),
            Self::Resposta(detalhe) => {
                write!(f, "resposta inesperada do spooler: {detalhe}")
            }
        }
    }
}

impl std::error::Error for ErroFilas {}

/// Lista as filas locais e conectadas. Lista vazia não é erro.
#[cfg(windows)]
pub fn listar_filas() -> Result<Vec<Fila>, ErroFilas> {
    sistema::listar()
}

/// Fora do Windows não há spooler a consultar.
#[cfg(not(windows))]
pub fn listar_filas() -> Result<Vec<Fila>, ErroFilas> {
    Err(ErroFilas::NaoSuportado)
}

/// Compara o nome da fila com o da impressora padrão, exatamente. "Sem padrão" não casa com nada.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn e_a_impressora_padrao(padrao: Option<&str>, nome: &str) -> bool {
    padrao.is_some_and(|padrao| padrao == nome)
}

#[cfg(test)]
mod testes_de_padrao {
    use super::*;

    #[test]
    fn nome_igual_ao_padrao_casa() {
        assert!(e_a_impressora_padrao(
            Some("Impressora Lançamento — Escritório"),
            "Impressora Lançamento — Escritório"
        ));
    }

    #[test]
    fn nome_com_diferenca_de_espaco_ou_caixa_nao_casa() {
        assert!(!e_a_impressora_padrao(Some("Fila 1"), "Fila 1 "));
        assert!(!e_a_impressora_padrao(Some("fila 1"), "Fila 1"));
        assert!(!e_a_impressora_padrao(Some("Fila 10"), "Fila 1"));
    }

    #[test]
    fn sem_padrao_nao_casa_com_nenhuma_fila() {
        assert!(!e_a_impressora_padrao(None, "Qualquer"));
        assert!(!e_a_impressora_padrao(None, ""));
    }
}

#[cfg(all(test, windows))]
mod testes_do_sistema {
    use super::*;

    /// Só exige que a chamada real não falhe; não depende de haver impressora instalada.
    #[test]
    fn enumerar_filas_do_sistema_nao_falha() {
        assert!(listar_filas().is_ok());
    }
}

#[cfg(all(test, not(windows)))]
mod testes_sem_windows {
    use super::*;

    #[test]
    fn fora_do_windows_a_listagem_recusa_com_mensagem_clara() {
        let erro = listar_filas().unwrap_err();
        assert_eq!(erro, ErroFilas::NaoSuportado);
        assert!(erro.to_string().contains("Windows"));
    }
}

#[cfg(test)]
mod testes_de_erro {
    use super::*;

    #[test]
    fn mensagens_de_erro_sao_em_portugues_e_mostram_o_codigo() {
        let sistema = ErroFilas::Sistema(0x8007_007A).to_string();
        assert!(sistema.contains("0x8007007A"));
        assert!(sistema.contains("recusou"));
        let grande = ErroFilas::Grande { bytes: 9_000_000 }.to_string();
        assert!(grande.contains("9000000"));
        assert!(grande.contains("limite"));
        assert!(ErroFilas::Resposta("x").to_string().contains("resposta"));
    }
}
