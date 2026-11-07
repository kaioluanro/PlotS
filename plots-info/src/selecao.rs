//! Interpretação do campo de páginas do painel.
//!
//! O texto é digitado por quem usa o programa, então tudo aqui é tratado como hostil: o tamanho
//! é limitado antes de qualquer leitura, uma faixa só é expandida depois de caber no documento e
//! a quantidade de páginas distintas tem teto.

use std::collections::BTreeSet;

use plots_core::LIMITE_PAGINAS;
use thiserror::Error;

/// Maior texto aceito no campo de páginas.
pub const LIMITE_TEXTO_BYTES: usize = 4096;

/// Maior quantidade de páginas distintas em uma seleção.
pub const LIMITE_NUMEROS: usize = 10_000;

/// Páginas escolhidas no painel, em ordem crescente e sem repetição.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelecaoDePaginas {
    paginas: Vec<usize>,
    total: usize,
    documento_inteiro: bool,
}

impl SelecaoDePaginas {
    /// Números escolhidos, em ordem crescente.
    pub fn numeros(&self) -> &[usize] {
        &self.paginas
    }

    /// Quantas páginas foram escolhidas.
    pub fn quantidade(&self) -> usize {
        self.paginas.len()
    }

    /// Total de páginas do documento em que a seleção foi interpretada.
    pub fn total(&self) -> usize {
        self.total
    }

    /// `true` quando o campo vazio escolheu o documento inteiro.
    pub fn documento_inteiro(&self) -> bool {
        self.documento_inteiro
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErroSelecao {
    #[error("o texto da seleção tem {bytes} bytes e passa do limite de {LIMITE_TEXTO_BYTES}")]
    TextoGrande { bytes: usize },
    #[error("há um item vazio na seleção")]
    ItemVazio,
    #[error("o item {item:?} não é um número nem uma faixa de páginas")]
    ItemNaoNumerico { item: String },
    #[error("a página {pagina} não existe em um documento de {total} páginas")]
    ForaDoDocumento { pagina: usize, total: usize },
    #[error("a faixa {inicio}-{fim} está invertida")]
    FaixaInvertida { inicio: usize, fim: usize },
    #[error("a faixa {inicio}-{fim} não cabe em um documento de {total} páginas")]
    FaixaForaDoDocumento {
        inicio: usize,
        fim: usize,
        total: usize,
    },
    #[error("a seleção passa de {LIMITE_NUMEROS} páginas distintas")]
    NumerosDemais,
    #[error("o documento tem {total} páginas e passa do limite de {limite}")]
    DocumentoGrande { total: usize, limite: usize },
    #[error("não foi possível reservar memória para a seleção")]
    MemoriaInsuficiente,
}

/// Interpreta o texto do campo contra o total de páginas do documento.
///
/// Texto vazio ou só com espaços vale o documento inteiro. Vírgulas separam itens e `a-b` é uma
/// faixa inclusiva. Os espaços em volta de cada item e de cada extremidade da faixa são ignorados.
pub fn selecao_de_texto(texto: &str, total: usize) -> Result<SelecaoDePaginas, ErroSelecao> {
    if texto.len() > LIMITE_TEXTO_BYTES {
        return Err(ErroSelecao::TextoGrande { bytes: texto.len() });
    }
    if total > LIMITE_PAGINAS {
        return Err(ErroSelecao::DocumentoGrande {
            total,
            limite: LIMITE_PAGINAS,
        });
    }
    let texto = texto.trim();
    if texto.is_empty() {
        return documento_inteiro(total);
    }

    let mut escolhidas = BTreeSet::new();
    for item in texto.split(',') {
        let item = item.trim();
        if item.is_empty() {
            return Err(ErroSelecao::ItemVazio);
        }
        match item.split_once('-') {
            Some((inicio, fim)) => {
                if fim.contains('-') {
                    return Err(item_nao_numerico(item));
                }
                let inicio = numero(inicio.trim()).ok_or_else(|| item_nao_numerico(item))?;
                let fim = numero(fim.trim()).ok_or_else(|| item_nao_numerico(item))?;
                inserir_faixa(&mut escolhidas, inicio, fim, total)?;
            }
            None => {
                let pagina = numero(item).ok_or_else(|| item_nao_numerico(item))?;
                inserir_pagina(&mut escolhidas, pagina, total)?;
            }
        }
    }

    let mut paginas = Vec::new();
    paginas
        .try_reserve(escolhidas.len())
        .map_err(|_| ErroSelecao::MemoriaInsuficiente)?;
    paginas.extend(escolhidas);
    Ok(SelecaoDePaginas {
        paginas,
        total,
        documento_inteiro: false,
    })
}

fn documento_inteiro(total: usize) -> Result<SelecaoDePaginas, ErroSelecao> {
    let mut paginas = Vec::new();
    paginas
        .try_reserve(total)
        .map_err(|_| ErroSelecao::MemoriaInsuficiente)?;
    paginas.extend(1..=total);
    Ok(SelecaoDePaginas {
        paginas,
        total,
        documento_inteiro: true,
    })
}

fn numero(texto: &str) -> Option<usize> {
    texto.parse::<usize>().ok()
}

fn item_nao_numerico(item: &str) -> ErroSelecao {
    ErroSelecao::ItemNaoNumerico {
        item: item.to_owned(),
    }
}

fn inserir_pagina(
    escolhidas: &mut BTreeSet<usize>,
    pagina: usize,
    total: usize,
) -> Result<(), ErroSelecao> {
    if pagina < 1 || pagina > total {
        return Err(ErroSelecao::ForaDoDocumento { pagina, total });
    }
    escolhidas.insert(pagina);
    if escolhidas.len() > LIMITE_NUMEROS {
        return Err(ErroSelecao::NumerosDemais);
    }
    Ok(())
}

fn inserir_faixa(
    escolhidas: &mut BTreeSet<usize>,
    inicio: usize,
    fim: usize,
    total: usize,
) -> Result<(), ErroSelecao> {
    if inicio > fim {
        return Err(ErroSelecao::FaixaInvertida { inicio, fim });
    }
    if inicio < 1 || fim > total {
        return Err(ErroSelecao::FaixaForaDoDocumento { inicio, fim, total });
    }
    for pagina in inicio..=fim {
        escolhidas.insert(pagina);
        if escolhidas.len() > LIMITE_NUMEROS {
            return Err(ErroSelecao::NumerosDemais);
        }
    }
    Ok(())
}
