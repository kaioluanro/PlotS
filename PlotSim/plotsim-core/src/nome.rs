//! Nome de uma fila virtual. O nome vem de fora e acaba em comandos do sistema e em caminhos,
//! então só passa o que está na lista de caracteres aceitos.

use std::fmt;

/// Toda fila do simulador começa com este texto. É o que separa as filas dele das outras.
pub const PREFIXO_FILA: &str = "PlotSim — ";
/// Maior nome aceito, em caracteres, sem contar o prefixo.
pub const LIMITE_NOME: usize = 64;
/// Maior trecho legível do nome dentro do nome da pasta.
const LIMITE_TRECHO_DA_PASTA: usize = 32;

/// Por que um nome foi recusado.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErroNome {
    Vazio,
    Longo,
    /// Primeiro caractere fora da lista aceita.
    Caractere(char),
    /// Espaço ou ponto nas pontas: o Windows os corta em alguns caminhos e o nome mudaria.
    Pontas,
    /// A fila do sistema não começa com o prefixo do simulador.
    SemPrefixo,
}

impl fmt::Display for ErroNome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vazio => f.write_str("o nome da impressora virtual está vazio"),
            Self::Longo => write!(f, "o nome passa de {LIMITE_NOME} caracteres"),
            Self::Caractere(c) => write!(f, "o nome tem um caractere não aceito: {c:?}"),
            Self::Pontas => f.write_str("o nome não pode começar nem terminar com espaço ou ponto"),
            Self::SemPrefixo => write!(f, "a fila não começa com \"{PREFIXO_FILA}\""),
        }
    }
}

impl std::error::Error for ErroNome {}

/// Nome validado, sem o prefixo.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NomeVirtual {
    curto: String,
}

fn caractere_aceito(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')')
}

impl NomeVirtual {
    pub fn novo(curto: &str) -> Result<Self, ErroNome> {
        if curto.is_empty() {
            return Err(ErroNome::Vazio);
        }
        if curto.chars().count() > LIMITE_NOME {
            return Err(ErroNome::Longo);
        }
        if let Some(c) = curto.chars().find(|c| !caractere_aceito(*c)) {
            return Err(ErroNome::Caractere(c));
        }
        let ponta = |c: Option<char>| matches!(c, Some(' ' | '.'));
        if ponta(curto.chars().next()) || ponta(curto.chars().last()) {
            return Err(ErroNome::Pontas);
        }
        Ok(Self {
            curto: curto.to_string(),
        })
    }

    /// Nome de uma fila do sistema, com o prefixo.
    pub fn da_fila(fila: &str) -> Result<Self, ErroNome> {
        Self::novo(
            fila.strip_prefix(PREFIXO_FILA)
                .ok_or(ErroNome::SemPrefixo)?,
        )
    }

    pub fn curto(&self) -> &str {
        &self.curto
    }

    /// Nome completo da fila no Windows.
    pub fn fila(&self) -> String {
        format!("{PREFIXO_FILA}{}", self.curto)
    }

    /// O Windows não diferencia maiúsculas em nomes de fila: `Jato` e `JATO` são a mesma.
    pub fn mesma_fila(&self, outro: &Self) -> bool {
        self.curto.to_lowercase() == outro.curto.to_lowercase()
    }

    /// Nome da pasta da fila: trecho legível e o resumo do nome inteiro. O resumo evita colisão
    /// entre nomes que só diferem em acentos e impede nomes reservados, como `CON`.
    pub fn pasta(&self) -> String {
        let minusculo = self.curto.to_lowercase();
        let trecho: String = minusculo
            .chars()
            .take(LIMITE_TRECHO_DA_PASTA)
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        format!("{trecho}-{:08x}", resumo(minusculo.as_bytes()))
    }
}

/// FNV-1a de 32 bits: estável entre versões, ao contrário do `Hash` da biblioteca padrão.
fn resumo(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5u32, |acumulado, byte| {
        (acumulado ^ u32::from(*byte)).wrapping_mul(0x0100_0193)
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn nome_comum_vira_fila_com_prefixo() {
        let nome = NomeVirtual::novo("Jato (virtual)").unwrap();
        assert_eq!(nome.fila(), "PlotSim — Jato (virtual)");
        assert_eq!(NomeVirtual::da_fila(&nome.fila()).unwrap(), nome);
    }

    #[test]
    fn nomes_hostis_sao_recusados() {
        for hostil in [
            "x\"; Remove-Printer *; \"",
            "$(Remove-Printer *)",
            "a`b",
            "a'b",
            "..\\..\\Windows",
            "a/b",
            "a,LocalOnly",
            "\\\\servidor\\fila",
            "linha\nnova",
            "nulo\0fim",
            "*",
            "a?",
            "[a]",
            "a;b",
            "a|b",
            "a&b",
            "%TEMP%",
            "a\u{202e}b",
        ] {
            assert!(
                matches!(NomeVirtual::novo(hostil), Err(ErroNome::Caractere(_))),
                "{hostil:?} passou"
            );
        }
    }

    #[test]
    fn limites_do_nome() {
        assert_eq!(NomeVirtual::novo(""), Err(ErroNome::Vazio));
        assert_eq!(NomeVirtual::novo(" a"), Err(ErroNome::Pontas));
        assert_eq!(NomeVirtual::novo("a."), Err(ErroNome::Pontas));
        assert!(NomeVirtual::novo(&"é".repeat(LIMITE_NOME)).is_ok());
        assert_eq!(
            NomeVirtual::novo(&"é".repeat(LIMITE_NOME + 1)),
            Err(ErroNome::Longo)
        );
    }

    #[test]
    fn fila_alheia_nao_vira_nome() {
        assert_eq!(
            NomeVirtual::da_fila("Microsoft Print to PDF"),
            Err(ErroNome::SemPrefixo)
        );
        assert_eq!(
            NomeVirtual::da_fila("PlotSim - Jato"),
            Err(ErroNome::SemPrefixo)
        );
    }

    #[test]
    fn pasta_e_estavel_sem_colisao_e_sem_nome_reservado() {
        let pasta = |nome: &str| NomeVirtual::novo(nome).unwrap().pasta();
        assert_eq!(pasta("Jato"), pasta("JATO"));
        assert_ne!(pasta("Ação"), pasta("Acao"));
        assert_eq!(pasta("CON"), format!("con-{:08x}", resumo(b"con")));
        assert!(pasta(&"ç-".repeat(32))
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-'));
        assert_eq!(resumo(b"a"), 0xe40c_292c);
    }

    #[test]
    fn caixa_nao_separa_filas() {
        let a = NomeVirtual::novo("Ação").unwrap();
        let b = NomeVirtual::novo("AÇÃO").unwrap();
        assert!(a.mesma_fila(&b));
    }
}
