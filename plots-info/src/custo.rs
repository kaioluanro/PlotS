//! Consumo de tinta, custo e situação do cartucho.
//!
//! A conta parte da cobertura média de cada canal: o volume gasto é proporcional à cobertura, à
//! área da página em relação a uma folha A4 e à fração do reservatório que uma página cheia usaria
//! no rendimento declarado, ajustada pela qualidade.

use plots_core::{DimensoesPagina, LIMITE_COPIAS};
use thiserror::Error;

use crate::cobertura::Cobertura;
use crate::configuracao::Qualidade;
use crate::perfil::{ErroPerfil, PerfilDeTinta, CANAIS};

/// Área de uma folha A4 em milímetros quadrados, usada como referência de proporção.
pub const AREA_A4_MM2: f64 = 62370.0;

/// Consumo de tinta em mililitros, por canal.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Consumo {
    pub c: f64,
    pub m: f64,
    pub y: f64,
    pub k: f64,
}

impl Consumo {
    pub fn new(c: f64, m: f64, y: f64, k: f64) -> Self {
        Self { c, m, y, k }
    }

    /// Soma dos quatro canais, em mililitros.
    pub fn total(self) -> f64 {
        self.c + self.m + self.y + self.k
    }

    /// Os quatro canais na ordem C, M, Y, K.
    pub fn valores(self) -> [f64; CANAIS] {
        [self.c, self.m, self.y, self.k]
    }
}

/// Custo em dinheiro, por canal, na moeda do perfil.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Custo {
    pub c: f64,
    pub m: f64,
    pub y: f64,
    pub k: f64,
}

impl Custo {
    pub fn new(c: f64, m: f64, y: f64, k: f64) -> Self {
        Self { c, m, y, k }
    }

    /// Soma dos quatro canais, na moeda do perfil.
    pub fn total(self) -> f64 {
        self.c + self.m + self.y + self.k
    }

    /// Os quatro canais na ordem C, M, Y, K.
    pub fn valores(self) -> [f64; CANAIS] {
        [self.c, self.m, self.y, self.k]
    }
}

/// Tinta e custo de uma página, de uma seleção ou do documento.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Estimativa {
    pub tinta: Consumo,
    pub custo: Custo,
}

/// Situação do reservatório depois de imprimir a seleção.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EstadoDoCartucho {
    /// Quanto do reservatório foi usado, em percentual; pode passar de 100.
    pub percentual_consumido: f64,
    /// Cópias da mesma página até a primeira cor acabar; `u64::MAX` quando nada é consumido.
    pub copias_ate_esvaziar: u64,
    /// Canal que acaba primeiro.
    pub canal_que_acaba_primeiro: usize,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ErroCusto {
    #[error(transparent)]
    Perfil(#[from] ErroPerfil),
    #[error("as dimensões da página precisam ser positivas e finitas")]
    DimensaoInvalida,
    #[error("a cobertura {canal} precisa estar entre 0 e 100")]
    CoberturaInvalida { canal: usize },
    #[error("o consumo {canal} precisa ser finito e não negativo")]
    ConsumoInvalido { canal: usize },
    #[error("o número de cópias deve estar entre 1 e {LIMITE_COPIAS}")]
    CopiasInvalidas { copias: usize },
    #[error("o resultado do cálculo não é um número finito")]
    ResultadoInvalido,
}

/// Tinta gasta por uma página, em mililitros por canal.
///
/// A cobertura entra em pontos percentuais: 5% de cobertura em A4 gasta `V / P` no canal, que é a
/// fração do reservatório correspondente a uma página no rendimento declarado.
pub fn ml_da_pagina(
    cobertura: Cobertura,
    dimensoes: DimensoesPagina,
    perfil: &PerfilDeTinta,
    qualidade: Qualidade,
) -> Result<Consumo, ErroCusto> {
    perfil.validar()?;
    let largura = dimensoes.largura_mm;
    let altura = dimensoes.altura_mm;
    if !largura.is_finite() || !altura.is_finite() || largura <= 0.0 || altura <= 0.0 {
        return Err(ErroCusto::DimensaoInvalida);
    }
    let area = largura * altura;
    if !area.is_finite() || area <= 0.0 {
        return Err(ErroCusto::DimensaoInvalida);
    }
    let proporcao = area / AREA_A4_MM2;
    let fator = perfil.fatores_qualidade[qualidade.indice()];
    let coberturas = cobertura.valores();
    let mut ml = [0.0f64; CANAIS];
    for canal in 0..CANAIS {
        let valor = coberturas[canal];
        if !valor.is_finite() || !(0.0..=100.0).contains(&valor) {
            return Err(ErroCusto::CoberturaInvalida { canal });
        }
        let rendimento = f64::from(perfil.rendimentos_paginas[canal]);
        let fracao = perfil.volumes_ml[canal] / (5.0 * rendimento);
        ml[canal] = valor * proporcao * fracao * fator;
        if !ml[canal].is_finite() || ml[canal] < 0.0 {
            return Err(ErroCusto::ResultadoInvalido);
        }
    }
    Ok(Consumo::new(ml[0], ml[1], ml[2], ml[3]))
}

/// Custo, na moeda do perfil, do consumo informado.
pub fn custo_do_consumo(consumo: Consumo, perfil: &PerfilDeTinta) -> Result<Custo, ErroCusto> {
    perfil.validar()?;
    let ml = consumo.valores();
    let mut custo = [0.0f64; CANAIS];
    for canal in 0..CANAIS {
        if !ml[canal].is_finite() || ml[canal] < 0.0 {
            return Err(ErroCusto::ConsumoInvalido { canal });
        }
        custo[canal] = ml[canal] * perfil.precos_ml[canal];
        if !custo[canal].is_finite() || custo[canal] < 0.0 {
            return Err(ErroCusto::ResultadoInvalido);
        }
    }
    Ok(Custo::new(custo[0], custo[1], custo[2], custo[3]))
}

/// Tinta e custo de uma página.
pub fn estimativa_da_pagina(
    cobertura: Cobertura,
    dimensoes: DimensoesPagina,
    perfil: &PerfilDeTinta,
    qualidade: Qualidade,
) -> Result<Estimativa, ErroCusto> {
    let tinta = ml_da_pagina(cobertura, dimensoes, perfil, qualidade)?;
    let custo = custo_do_consumo(tinta, perfil)?;
    Ok(Estimativa { tinta, custo })
}

/// Soma das páginas escolhidas, repetida pelo número de cópias.
///
/// A lista vazia vale zero: uma seleção sem páginas não gasta tinta.
pub fn estimativa_da_selecao(
    paginas: &[Consumo],
    copias: usize,
    perfil: &PerfilDeTinta,
) -> Result<Estimativa, ErroCusto> {
    perfil.validar()?;
    if !(1..=LIMITE_COPIAS).contains(&copias) {
        return Err(ErroCusto::CopiasInvalidas { copias });
    }
    let mut tinta = [0.0f64; CANAIS];
    for pagina in paginas {
        let canais = pagina.valores();
        for canal in 0..CANAIS {
            if !canais[canal].is_finite() || canais[canal] < 0.0 {
                return Err(ErroCusto::ConsumoInvalido { canal });
            }
            tinta[canal] += canais[canal];
            if !tinta[canal].is_finite() {
                return Err(ErroCusto::ResultadoInvalido);
            }
        }
    }
    let copias = copias as f64;
    for valor in &mut tinta {
        *valor *= copias;
        if !valor.is_finite() {
            return Err(ErroCusto::ResultadoInvalido);
        }
    }
    let tinta = Consumo::new(tinta[0], tinta[1], tinta[2], tinta[3]);
    let custo = custo_do_consumo(tinta, perfil)?;
    Ok(Estimativa { tinta, custo })
}

/// Soma de todas as páginas do documento, em uma cópia.
pub fn estimativa_do_documento(
    paginas: &[Consumo],
    perfil: &PerfilDeTinta,
) -> Result<Estimativa, ErroCusto> {
    estimativa_da_selecao(paginas, 1, perfil)
}

/// Quanto a seleção consumiu do reservatório e quantas cópias ainda cabem.
///
/// `consumo` é a tinta da seleção inteira, já multiplicada pelas cópias. O percentual é a maior
/// fração entre os canais. A cor que acaba primeiro limita as cópias: quando um canal não gasta
/// nada por cópia, ele não limita; quando nenhum gasta, o resultado é `u64::MAX`.
pub fn cartucho(
    consumo: Consumo,
    copias: usize,
    perfil: &PerfilDeTinta,
) -> Result<EstadoDoCartucho, ErroCusto> {
    perfil.validar()?;
    if !(1..=LIMITE_COPIAS).contains(&copias) {
        return Err(ErroCusto::CopiasInvalidas { copias });
    }
    let canais = consumo.valores();
    let mut percentual = 0.0f64;
    let mut canal_mais_gasto = 0;
    let mut ate_esvaziar = u64::MAX;
    for canal in 0..CANAIS {
        if !canais[canal].is_finite() || canais[canal] < 0.0 {
            return Err(ErroCusto::ConsumoInvalido { canal });
        }
        let fracao = canais[canal] / perfil.volumes_ml[canal];
        if !fracao.is_finite() {
            return Err(ErroCusto::ResultadoInvalido);
        }
        if fracao * 100.0 > percentual {
            percentual = fracao * 100.0;
            canal_mais_gasto = canal;
        }
        let por_copia = canais[canal] / copias as f64;
        ate_esvaziar = ate_esvaziar.min(copias_por_canal(perfil.volumes_ml[canal], por_copia));
    }
    Ok(EstadoDoCartucho {
        percentual_consumido: percentual,
        copias_ate_esvaziar: ate_esvaziar,
        canal_que_acaba_primeiro: canal_mais_gasto,
    })
}

fn copias_por_canal(volume: f64, por_copia: f64) -> u64 {
    if por_copia <= 0.0 {
        return u64::MAX;
    }
    // O épsilon mantém uma conta exata, como 4500 cópias, longe do 4499 que a representação
    // binária poderia produzir ao arredondar para baixo.
    let bruto = (volume / por_copia + 1e-9).floor();
    if !bruto.is_finite() || bruto >= u64::MAX as f64 {
        u64::MAX
    } else {
        bruto as u64
    }
}

/// Página de maior custo pela soma dos quatro canais.
///
/// Devolve a posição na lista e o custo total. Valores que não sejam números finitos são
/// ignorados; a primeira página vence em caso de empate.
pub fn pagina_mais_cara(paginas: &[Custo]) -> Option<(usize, f64)> {
    let mut escolhida: Option<(usize, f64)> = None;
    for (indice, pagina) in paginas.iter().enumerate() {
        let total = pagina.total();
        if !total.is_finite() || total < 0.0 {
            continue;
        }
        if escolhida.is_none_or(|(_, maior)| total > maior) {
            escolhida = Some((indice, total));
        }
    }
    escolhida
}
