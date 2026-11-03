//! Ajuste do driver de cada fila, guardado como texto no arquivo de perfis.
//!
//! Fica fora da entrada do perfil: escolher outro modelo ou calibrar de novo não muda o que a
//! pessoa ajustou no driver, e uma fila com ajuste continua sem perfil até ganhar um. Este crate não
//! interpreta o conteúdo: quem o usa confere a estrutura antes de entregar ao driver.

use std::collections::BTreeMap;

use crate::perfis::Perfis;

/// Maior ajuste aceito por fila (32 KiB). O arquivo inteiro continua limitado a 1 MiB.
pub const LIMITE_AJUSTE_BYTES: usize = 32 * 1024;

/// Por que um ajuste não entrou no conjunto. O que já estava guardado continua igual.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AjusteRecusado {
    Vazio,
    Grande,
}

impl Perfis {
    /// Ajuste salvo para a fila com exatamente este nome.
    pub fn ajuste_do_driver(&self, fila: &str) -> Option<&[u8]> {
        self.ajustes_driver.get(fila).map(Vec::as_slice)
    }

    /// Guarda o ajuste da fila, no lugar do anterior.
    pub fn definir_ajuste_do_driver(
        &mut self,
        fila: &str,
        ajuste: Vec<u8>,
    ) -> Result<(), AjusteRecusado> {
        if ajuste.is_empty() {
            return Err(AjusteRecusado::Vazio);
        }
        if ajuste.len() > LIMITE_AJUSTE_BYTES {
            return Err(AjusteRecusado::Grande);
        }
        self.ajustes_driver.insert(fila.to_owned(), ajuste);
        self.ajustes_ignorados.retain(|nome| nome != fila);
        Ok(())
    }

    /// Volta a fila ao padrão dela; devolve o ajuste que estava guardado.
    pub fn remover_ajuste_do_driver(&mut self, fila: &str) -> Option<Vec<u8>> {
        self.ajustes_ignorados.retain(|nome| nome != fila);
        self.ajustes_driver.remove(fila)
    }

    /// Filas com ajuste salvo, em ordem alfabética.
    pub fn filas_com_ajuste(&self) -> impl Iterator<Item = &str> {
        self.ajustes_driver.keys().map(String::as_str)
    }

    /// Filas cujo ajuste estava corrompido no arquivo lido, em ordem alfabética.
    pub fn ajustes_ignorados(&self) -> &[String] {
        &self.ajustes_ignorados
    }
}

/// Lê do arquivo o que dá para usar. Um valor ruim não recusa o arquivo: a fila fica sem ajuste e
/// entra na lista de ignorados.
pub(crate) fn ler_do_arquivo(perfis: &mut Perfis, valores: BTreeMap<String, toml::Value>) {
    for (fila, valor) in valores {
        let ajuste = valor
            .as_str()
            .and_then(decodificar_base64)
            .filter(|bytes| bytes.len() <= LIMITE_AJUSTE_BYTES);
        match ajuste {
            Some(bytes) => {
                perfis.ajustes_driver.insert(fila, bytes);
            }
            None => perfis.ajustes_ignorados.push(fila),
        }
    }
}

/// Valores que vão para o arquivo.
pub(crate) fn para_o_arquivo(perfis: &Perfis) -> BTreeMap<String, toml::Value> {
    perfis
        .ajustes_driver
        .iter()
        .map(|(fila, bytes)| (fila.clone(), toml::Value::String(codificar_base64(bytes))))
        .collect()
}

const ALFABETO: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const COMPLEMENTO: char = '=';

fn sinal(indice: u32) -> char {
    ALFABETO
        .get((indice & 0x3F) as usize)
        .map_or(COMPLEMENTO, |letra| char::from(*letra))
}

pub(crate) fn codificar_base64(bytes: &[u8]) -> String {
    let mut texto = String::with_capacity(bytes.len().div_ceil(3).saturating_mul(4));
    for trio in bytes.chunks(3) {
        let [a, b, c] = [0, 1, 2].map(|i| u32::from(trio.get(i).copied().unwrap_or(0)));
        let bloco = (a << 16) | (b << 8) | c;
        texto.push(sinal(bloco >> 18));
        texto.push(sinal(bloco >> 12));
        texto.push(if trio.len() > 1 {
            sinal(bloco >> 6)
        } else {
            COMPLEMENTO
        });
        texto.push(if trio.len() > 2 {
            sinal(bloco)
        } else {
            COMPLEMENTO
        });
    }
    texto
}

fn valor_do_sinal(letra: u8) -> Option<u32> {
    ALFABETO
        .iter()
        .position(|candidata| *candidata == letra)
        .and_then(|posicao| u32::try_from(posicao).ok())
}

/// Só aceita a forma que `codificar_base64` escreve. O tamanho é conferido antes de reservar
/// memória, e qualquer outra coisa devolve `None`.
pub(crate) fn decodificar_base64(texto: &str) -> Option<Vec<u8>> {
    let sinais = texto.as_bytes();
    let teto = LIMITE_AJUSTE_BYTES.div_ceil(3).saturating_mul(4);
    if sinais.is_empty() || sinais.len() % 4 != 0 || sinais.len() > teto {
        return None;
    }
    let mut bytes = Vec::with_capacity(sinais.len() / 4 * 3);
    let ultimo = sinais.len() / 4 - 1;
    for (indice, quarteto) in sinais.chunks_exact(4).enumerate() {
        let [a, b, c, d] = <[u8; 4]>::try_from(quarteto).ok()?;
        let complemento = u8::try_from(COMPLEMENTO).ok()?;
        let (a, b) = (valor_do_sinal(a)?, valor_do_sinal(b)?);
        match (c == complemento, d == complemento) {
            (false, false) => {
                let (c, d) = (valor_do_sinal(c)?, valor_do_sinal(d)?);
                let bloco = (a << 18) | (b << 12) | (c << 6) | d;
                bytes.extend_from_slice(&[(bloco >> 16) as u8, (bloco >> 8) as u8, bloco as u8]);
            }
            (false, true) if indice == ultimo => {
                let c = valor_do_sinal(c)?;
                if c & 0b11 != 0 {
                    return None;
                }
                let bloco = (a << 18) | (b << 12) | (c << 6);
                bytes.extend_from_slice(&[(bloco >> 16) as u8, (bloco >> 8) as u8]);
            }
            (true, true) if indice == ultimo => {
                if b & 0b1111 != 0 {
                    return None;
                }
                bytes.push((((a << 18) | (b << 12)) >> 16) as u8);
            }
            _ => return None,
        }
    }
    Some(bytes)
}
