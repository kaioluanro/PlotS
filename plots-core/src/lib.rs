//! Planejamento puro de passadas para impressão manual frente e verso.

use std::collections::TryReserveError;

pub mod distribuicao;

/// Maior quantidade de páginas que o planejador aceita em uma chamada.
pub const LIMITE_PAGINAS: usize = 100_000;

/// Uma página original, ou a folha em branco acrescentada ao fim de um documento ímpar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pagina {
    /// Número de página de origem, começando em 1. `None` representa uma página em branco.
    pub numero: Option<usize>,
    /// Indica que o conteúdo deve ser girado em 180 graus ao ser enviado.
    pub girar_180: bool,
}

impl Pagina {
    fn original(numero: usize) -> Self {
        Self {
            numero: Some(numero),
            girar_180: false,
        }
    }

    fn em_branco() -> Self {
        Self {
            numero: None,
            girar_180: false,
        }
    }
}

/// Borda da folha que entra primeiro na impressora ao recolocar o maço.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BordaEntrada {
    Curta,
    Longa,
}

/// Instrução física registrada no perfil calibrado.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GestoRecolocar {
    /// `true` quando é preciso virar o maço para expor os versos.
    pub virar_maco: bool,
    pub borda_primeiro: BordaEntrada,
}

impl Default for GestoRecolocar {
    fn default() -> Self {
        Self {
            virar_maco: true,
            borda_primeiro: BordaEntrada::Curta,
        }
    }
}

/// Perfil medido para uma fila de impressão.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PerfilImpressora {
    pub passada1_inversa: bool,
    pub passada2_inversa: bool,
    pub verso_girar_180: bool,
    pub gesto_recolocar: GestoRecolocar,
}

/// Ajusta o perfil usado para imprimir um teste de calibração de duas folhas.
///
/// O teste deve conter apenas as páginas 1 a 4, sem perda de folhas nem
/// sobreimpressão, com setas de orientação comparáveis nas duas faces. As
/// respostas descrevem o resultado obtido com `perfil` e o gesto registrado
/// nele. A resposta de orientação compara as setas das páginas 1 e 2, mesmo
/// quando elas saíram em folhas diferentes. Se o par estiver incorreto, a
/// ordem da segunda passada é invertida em relação à usada no teste; se as
/// setas divergirem, a rotação da segunda passada é alternada. Esta função
/// não verifica se a impressora ou o gesto foram identificados corretamente.
pub fn ajustar_perfil_por_calibracao(
    mut perfil: PerfilImpressora,
    pagina_2_no_verso_da_1: bool,
    pagina_2_invertida: bool,
) -> PerfilImpressora {
    perfil.passada2_inversa ^= !pagina_2_no_verso_da_1;
    perfil.verso_girar_180 ^= pagina_2_invertida;
    perfil
}

/// Qual lado do conjunto de páginas é enviado primeiro.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OrdemPassadas {
    #[default]
    ParesPrimeiro,
    ImparesPrimeiro,
}

/// Um lote com suas duas sequências de envio.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lote {
    /// Páginas pertencentes ao lote na ordem original, incluindo eventual branco final.
    pub paginas: Vec<Pagina>,
    /// Primeira sequência a enviar, conforme `ordem_passadas`.
    pub passada1: Vec<Pagina>,
    /// Segunda sequência a enviar, conforme `ordem_passadas`.
    pub passada2: Vec<Pagina>,
}

#[derive(Debug)]
pub enum ErroPlano {
    TamanhoLoteZero,
    LimitePaginasExcedido,
    TamanhoExcessivo,
    ReservaFalhou(TryReserveError),
}

impl std::fmt::Display for ErroPlano {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TamanhoLoteZero => f.write_str("o tamanho do lote deve ser positivo"),
            Self::LimitePaginasExcedido => {
                write!(f, "o documento excede o limite de {LIMITE_PAGINAS} páginas")
            }
            Self::TamanhoExcessivo => f.write_str("o documento é grande demais para planejar"),
            Self::ReservaFalhou(_) => f.write_str("não foi possível reservar memória para o plano"),
        }
    }
}

impl std::error::Error for ErroPlano {}

/// Divide um documento em lotes consecutivos de folhas e calcula as duas passadas.
///
/// Cada lote contém no máximo `folhas_por_lote` folhas (duas páginas por folha).
/// Se o documento tiver quantidade ímpar de páginas, uma página em branco é anexada
/// ao fim antes da divisão, portanto ela pertence à passada das páginas pares.
pub fn plano_documento(
    total_paginas: usize,
    folhas_por_lote: usize,
    ordem_passadas: OrdemPassadas,
    perfil: PerfilImpressora,
) -> Result<Vec<Lote>, ErroPlano> {
    if folhas_por_lote == 0 {
        return Err(ErroPlano::TamanhoLoteZero);
    }
    if total_paginas > LIMITE_PAGINAS {
        return Err(ErroPlano::LimitePaginasExcedido);
    }
    let total_com_branco = total_paginas
        .checked_add(total_paginas % 2)
        .ok_or(ErroPlano::TamanhoExcessivo)?;
    let paginas_por_lote = folhas_por_lote
        .checked_mul(2)
        .ok_or(ErroPlano::TamanhoExcessivo)?;
    let quantidade_lotes =
        total_com_branco / paginas_por_lote + usize::from(total_com_branco % paginas_por_lote != 0);
    let mut lotes = Vec::new();
    lotes
        .try_reserve(quantidade_lotes)
        .map_err(ErroPlano::ReservaFalhou)?;

    let mut inicio = 0usize;
    while inicio < total_com_branco {
        let fim = inicio
            .saturating_add(paginas_por_lote)
            .min(total_com_branco);
        let mut paginas = Vec::new();
        paginas
            .try_reserve(fim - inicio)
            .map_err(ErroPlano::ReservaFalhou)?;
        for indice in inicio..fim {
            let pagina = if indice < total_paginas {
                Pagina::original(indice + 1)
            } else {
                Pagina::em_branco()
            };
            paginas.push(pagina);
        }

        let (mut pares, mut impares) = (Vec::new(), Vec::new());
        pares
            .try_reserve((paginas.len() + 1) / 2)
            .map_err(ErroPlano::ReservaFalhou)?;
        impares
            .try_reserve(paginas.len() / 2)
            .map_err(ErroPlano::ReservaFalhou)?;
        for (indice, pagina) in paginas.iter().copied().enumerate() {
            // O índice global ímpar corresponde aos rótulos pares (2, 4, 6…).
            if (inicio + indice) % 2 == 1 {
                pares.push(pagina);
            } else {
                impares.push(pagina);
            }
        }
        let (mut passada1, mut passada2) = match ordem_passadas {
            OrdemPassadas::ParesPrimeiro => (pares, impares),
            OrdemPassadas::ImparesPrimeiro => (impares, pares),
        };
        if perfil.passada1_inversa {
            passada1.reverse();
        }
        if perfil.passada2_inversa {
            passada2.reverse();
        }
        if perfil.verso_girar_180 {
            for pagina in &mut passada2 {
                pagina.girar_180 = true;
            }
        }
        lotes.push(Lote {
            paginas,
            passada1,
            passada2,
        });
        inicio = fim;
    }
    Ok(lotes)
}

/// Tolerância usada para reconhecer formatos nominais e comparar as duas faces
/// de uma folha, em milímetros.
pub const TOLERANCIA_PAPEL_MM: f64 = 2.0;

/// Dimensões de uma página PDF expressas em milímetros.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DimensoesPagina {
    pub largura_mm: f64,
    pub altura_mm: f64,
}

impl DimensoesPagina {
    pub const fn new(largura_mm: f64, altura_mm: f64) -> Self {
        Self {
            largura_mm,
            altura_mm,
        }
    }
}

/// Formato nominal reconhecido. As dimensões do formato seguem retrato;
/// `Paisagem` registra a orientação observada no documento.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormatoPapel {
    A4,
    A5,
    Carta,
    Oficio,
    Legal,
    Personalizado,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrientacaoPapel {
    Retrato,
    Paisagem,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Papel {
    pub formato: FormatoPapel,
    pub orientacao: OrientacaoPapel,
    /// Medidas normalizadas para retrato (largura menor primeiro).
    pub largura_mm: f64,
    pub altura_mm: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ErroPapel {
    DimensaoInvalida { pagina: usize },
    ParIncompativel { frente: usize, verso: usize },
    LoteZero,
    LimitePaginasExcedido,
    ReservaFalhou,
}

impl std::fmt::Display for ErroPapel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DimensaoInvalida { pagina } => {
                write!(f, "dimensões inválidas na página {pagina}")
            }
            Self::ParIncompativel { frente, verso } => write!(
                f,
                "as páginas {frente} e {verso} têm tamanhos incompatíveis para a mesma folha"
            ),
            Self::LoteZero => f.write_str("o tamanho do lote deve ser positivo"),
            Self::LimitePaginasExcedido => {
                write!(f, "o documento excede o limite de {LIMITE_PAGINAS} páginas")
            }
            Self::ReservaFalhou => f.write_str("não foi possível reservar memória para os lotes"),
        }
    }
}

impl std::error::Error for ErroPapel {}

/// Classifica dimensões positivas e finitas, aceitando formatos nominais com
/// tolerância de 2 mm e preservando a orientação retrato/paisagem.
pub fn reconhecer_papel(dimensoes: DimensoesPagina) -> Result<Papel, ErroPapel> {
    let w = dimensoes.largura_mm;
    let h = dimensoes.altura_mm;
    if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
        return Err(ErroPapel::DimensaoInvalida { pagina: 0 });
    }
    let (menor, maior, orientacao) = if w <= h {
        (w, h, OrientacaoPapel::Retrato)
    } else {
        (h, w, OrientacaoPapel::Paisagem)
    };
    let formatos = [
        (FormatoPapel::A4, 210.0, 297.0),
        (FormatoPapel::A5, 148.0, 210.0),
        (FormatoPapel::Carta, 216.0, 279.0),
        (FormatoPapel::Oficio, 216.0, 330.0),
        (FormatoPapel::Legal, 216.0, 356.0),
    ];
    let formato = formatos
        .iter()
        .find(|(_, fw, fh)| {
            (menor - fw).abs() <= TOLERANCIA_PAPEL_MM && (maior - fh).abs() <= TOLERANCIA_PAPEL_MM
        })
        .map(|(f, _, _)| *f)
        .unwrap_or(FormatoPapel::Personalizado);
    Ok(Papel {
        formato,
        orientacao,
        largura_mm: menor,
        altura_mm: maior,
    })
}

/// Um conjunto de folhas consecutivas do mesmo papel, limitado ao tamanho
/// solicitado. `paginas` contém índices de página originais (base 1); `None`
/// representa a face em branco final de um documento ímpar.
#[derive(Clone, Debug, PartialEq)]
pub struct LotePapel {
    pub papel: Papel,
    pub paginas: Vec<Option<usize>>,
    pub passada1: Vec<Pagina>,
    pub passada2: Vec<Pagina>,
}

/// Valida os pares frente/verso e divide as folhas em lotes contíguos por papel.
/// Uma última página sem verso é pareada com uma face em branco.
pub fn lotes_por_papel(
    paginas: &[DimensoesPagina],
    folhas_por_lote: usize,
) -> Result<Vec<LotePapel>, ErroPapel> {
    plano_lotes_por_papel(
        paginas,
        folhas_por_lote,
        OrdemPassadas::default(),
        PerfilImpressora::default(),
    )
}

/// Planeja lotes por formato e calcula as passadas mantendo os números originais
/// do documento, mesmo quando um lote começa em uma página de índice não zero.
pub fn plano_lotes_por_papel(
    paginas: &[DimensoesPagina],
    folhas_por_lote: usize,
    ordem_passadas: OrdemPassadas,
    perfil: PerfilImpressora,
) -> Result<Vec<LotePapel>, ErroPapel> {
    if folhas_por_lote == 0 {
        return Err(ErroPapel::LoteZero);
    }
    if paginas.len() > LIMITE_PAGINAS {
        return Err(ErroPapel::LimitePaginasExcedido);
    }
    let reserva =
        |vec: &mut Vec<Papel>, n| vec.try_reserve(n).map_err(|_| ErroPapel::ReservaFalhou);
    let mut papeis = Vec::new();
    reserva(&mut papeis, paginas.len())?;
    for (i, dims) in paginas.iter().copied().enumerate() {
        reconhecer_papel(dims)
            .map_err(|_| ErroPapel::DimensaoInvalida { pagina: i + 1 })
            .map(|p| papeis.push(p))?;
    }
    let mut folhas: Vec<(Papel, Option<usize>, Option<usize>)> = Vec::new();
    folhas
        .try_reserve(paginas.len().saturating_add(1) / 2)
        .map_err(|_| ErroPapel::ReservaFalhou)?;
    for i in (0..paginas.len()).step_by(2) {
        let frente = papeis[i];
        let verso = papeis.get(i + 1).copied();
        if let Some(v) = verso {
            let mesmo_formato_nominal =
                frente.formato != FormatoPapel::Personalizado && frente.formato == v.formato;
            if !mesmo_formato_nominal
                && ((frente.largura_mm - v.largura_mm).abs() > TOLERANCIA_PAPEL_MM
                    || (frente.altura_mm - v.altura_mm).abs() > TOLERANCIA_PAPEL_MM)
            {
                return Err(ErroPapel::ParIncompativel {
                    frente: i + 1,
                    verso: i + 2,
                });
            }
        }
        folhas.push((frente, Some(i + 1), verso.map(|_| i + 2)));
    }
    let mut lotes: Vec<LotePapel> = Vec::new();
    for (papel, frente, verso) in folhas {
        let mesmo_papel = lotes.last().is_some_and(|l| {
            let mesmo_formato = l.papel.formato == papel.formato
                && l.papel.orientacao == papel.orientacao
                && if papel.formato == FormatoPapel::Personalizado {
                    (l.papel.largura_mm - papel.largura_mm).abs() <= TOLERANCIA_PAPEL_MM
                        && (l.papel.altura_mm - papel.altura_mm).abs() <= TOLERANCIA_PAPEL_MM
                } else {
                    true
                };
            mesmo_formato && l.paginas.len() / 2 < folhas_por_lote
        });
        if !mesmo_papel {
            lotes.try_reserve(1).map_err(|_| ErroPapel::ReservaFalhou)?;
            lotes.push(LotePapel {
                papel,
                paginas: Vec::new(),
                passada1: Vec::new(),
                passada2: Vec::new(),
            });
        }
        if let Some(lote) = lotes.last_mut() {
            lote.paginas
                .try_reserve(2)
                .map_err(|_| ErroPapel::ReservaFalhou)?;
            lote.passada1
                .try_reserve(1)
                .map_err(|_| ErroPapel::ReservaFalhou)?;
            lote.passada2
                .try_reserve(1)
                .map_err(|_| ErroPapel::ReservaFalhou)?;
            lote.paginas.push(frente);
            lote.paginas.push(verso);
            let pagina_frente = frente
                .map(Pagina::original)
                .unwrap_or_else(Pagina::em_branco);
            let pagina_verso = verso
                .map(Pagina::original)
                .unwrap_or_else(Pagina::em_branco);
            let (primeira, segunda) = match ordem_passadas {
                OrdemPassadas::ParesPrimeiro => (pagina_verso, pagina_frente),
                OrdemPassadas::ImparesPrimeiro => (pagina_frente, pagina_verso),
            };
            lote.passada1.push(primeira);
            lote.passada2.push(segunda);
        }
    }
    for lote in &mut lotes {
        if perfil.passada1_inversa {
            lote.passada1.reverse();
        }
        if perfil.passada2_inversa {
            lote.passada2.reverse();
        }
        if perfil.verso_girar_180 {
            for pagina in &mut lote.passada2 {
                pagina.girar_180 = true;
            }
        }
    }
    Ok(lotes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibracao_ajusta_as_respostas_relativas_ao_perfil_testado() {
        for ordem_inicial in [false, true] {
            for giro_inicial in [false, true] {
                for par_correto in [false, true] {
                    for invertida in [false, true] {
                        let perfil = PerfilImpressora {
                            passada1_inversa: true,
                            passada2_inversa: ordem_inicial,
                            verso_girar_180: giro_inicial,
                            gesto_recolocar: GestoRecolocar {
                                virar_maco: false,
                                borda_primeiro: BordaEntrada::Longa,
                            },
                        };
                        let ajustado =
                            ajustar_perfil_por_calibracao(perfil, par_correto, invertida);
                        assert_eq!(ajustado.passada2_inversa, ordem_inicial ^ !par_correto);
                        assert_eq!(ajustado.verso_girar_180, giro_inicial ^ invertida);
                        assert_eq!(ajustado.passada1_inversa, perfil.passada1_inversa);
                        assert_eq!(ajustado.gesto_recolocar, perfil.gesto_recolocar);
                    }
                }
            }
        }
    }

    fn nums(paginas: &[Pagina]) -> Vec<Option<usize>> {
        paginas.iter().map(|p| p.numero).collect()
    }

    #[test]
    fn rejeita_lote_zero_e_aceita_documento_vazio() {
        assert!(matches!(
            plano_documento(1, 0, OrdemPassadas::default(), PerfilImpressora::default()),
            Err(ErroPlano::TamanhoLoteZero)
        ));
        assert!(
            plano_documento(0, 1, OrdemPassadas::default(), PerfilImpressora::default())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn uma_folha_tem_frente_par_vazia_e_verso_impar() {
        let lotes =
            plano_documento(1, 1, OrdemPassadas::default(), PerfilImpressora::default()).unwrap();
        assert_eq!(lotes.len(), 1);
        assert_eq!(nums(&lotes[0].passada1), vec![None]);
        assert_eq!(nums(&lotes[0].passada2), vec![Some(1)]);
    }

    #[test]
    fn divide_em_lotes_consecutivos_e_completa_branco_no_final() {
        let lotes =
            plano_documento(7, 2, OrdemPassadas::default(), PerfilImpressora::default()).unwrap();
        assert_eq!(lotes.len(), 2);
        assert_eq!(nums(&lotes[0].passada1), vec![Some(2), Some(4)]);
        assert_eq!(nums(&lotes[0].passada2), vec![Some(1), Some(3)]);
        assert_eq!(nums(&lotes[1].passada1), vec![Some(6), None]);
        assert_eq!(nums(&lotes[1].passada2), vec![Some(5), Some(7)]);
    }

    #[test]
    fn ordem_de_passadas_e_configuravel() {
        let lotes = plano_documento(
            4,
            2,
            OrdemPassadas::ImparesPrimeiro,
            PerfilImpressora::default(),
        )
        .unwrap();
        assert_eq!(nums(&lotes[0].passada1), vec![Some(1), Some(3)]);
        assert_eq!(nums(&lotes[0].passada2), vec![Some(2), Some(4)]);
    }

    #[test]
    fn inversao_e_rotacao_seguem_o_numero_da_passada_configurada() {
        let perfil = PerfilImpressora {
            passada1_inversa: true,
            passada2_inversa: false,
            verso_girar_180: true,
            ..PerfilImpressora::default()
        };
        let lotes = plano_documento(4, 2, OrdemPassadas::ImparesPrimeiro, perfil).unwrap();
        assert_eq!(nums(&lotes[0].passada1), vec![Some(3), Some(1)]);
        assert_eq!(nums(&lotes[0].passada2), vec![Some(2), Some(4)]);
        assert!(lotes[0].passada1.iter().all(|p| !p.girar_180));
        assert!(lotes[0].passada2.iter().all(|p| p.girar_180));
    }

    #[test]
    fn perfil_inverte_passadas_independentemente_e_gira_segunda() {
        for mascara in 0u8..8 {
            let perfil = PerfilImpressora {
                passada1_inversa: mascara & 1 != 0,
                passada2_inversa: mascara & 2 != 0,
                verso_girar_180: mascara & 4 != 0,
                ..PerfilImpressora::default()
            };
            let lotes = plano_documento(6, 3, OrdemPassadas::default(), perfil).unwrap();
            let mut pares = vec![Some(2), Some(4), Some(6)];
            let mut impares = vec![Some(1), Some(3), Some(5)];
            if perfil.passada1_inversa {
                pares.reverse();
            }
            if perfil.passada2_inversa {
                impares.reverse();
            }
            assert_eq!(nums(&lotes[0].passada1), pares);
            assert_eq!(nums(&lotes[0].passada2), impares);
            assert!(lotes[0].passada1.iter().all(|p| !p.girar_180));
            assert!(lotes[0]
                .passada2
                .iter()
                .all(|p| p.girar_180 == perfil.verso_girar_180));
        }
    }

    #[test]
    fn extremos_de_tamanho_sao_rejeitados_sem_estouro() {
        assert!(matches!(
            plano_documento(
                usize::MAX,
                1,
                OrdemPassadas::default(),
                PerfilImpressora::default()
            ),
            Err(ErroPlano::LimitePaginasExcedido)
        ));
        assert!(matches!(
            plano_documento(
                1,
                usize::MAX,
                OrdemPassadas::default(),
                PerfilImpressora::default()
            ),
            Err(ErroPlano::TamanhoExcessivo)
        ));
    }

    #[test]
    fn rejeita_documento_acima_do_limite_antes_de_planejar() {
        assert!(matches!(
            plano_documento(
                LIMITE_PAGINAS + 1,
                1,
                OrdemPassadas::default(),
                PerfilImpressora::default()
            ),
            Err(ErroPlano::LimitePaginasExcedido)
        ));
    }

    #[test]
    fn reconhece_formatos_orientacao_e_personalizado_com_tolerancia() {
        let a4 = reconhecer_papel(DimensoesPagina::new(211.5, 297.0)).unwrap();
        assert_eq!(a4.formato, FormatoPapel::A4);
        assert_eq!(a4.orientacao, OrientacaoPapel::Retrato);
        let carta = reconhecer_papel(DimensoesPagina::new(279.0, 216.0)).unwrap();
        assert_eq!(carta.formato, FormatoPapel::Carta);
        assert_eq!(carta.orientacao, OrientacaoPapel::Paisagem);
        assert_eq!(
            reconhecer_papel(DimensoesPagina::new(100.0, 200.0))
                .unwrap()
                .formato,
            FormatoPapel::Personalizado
        );
    }

    #[test]
    fn rejeita_dimensoes_nao_finitas_ou_nao_positivas() {
        for dims in [
            DimensoesPagina::new(0.0, 297.0),
            DimensoesPagina::new(-1.0, 10.0),
            DimensoesPagina::new(f64::NAN, 10.0),
            DimensoesPagina::new(10.0, f64::INFINITY),
        ] {
            assert!(matches!(
                reconhecer_papel(dims),
                Err(ErroPapel::DimensaoInvalida { .. })
            ));
        }
    }

    #[test]
    fn valida_par_frente_verso_e_divide_quando_o_papel_muda() {
        let paginas = [
            DimensoesPagina::new(210.0, 297.0),
            DimensoesPagina::new(211.0, 296.0),
            DimensoesPagina::new(210.0, 297.0),
            DimensoesPagina::new(210.0, 297.0),
            DimensoesPagina::new(148.0, 210.0),
            DimensoesPagina::new(148.0, 210.0),
        ];
        let lotes = lotes_por_papel(&paginas, 1).unwrap();
        assert_eq!(lotes.len(), 3);
        assert_eq!(lotes[0].papel.formato, FormatoPapel::A4);
        assert_eq!(lotes[0].paginas, vec![Some(1), Some(2)]);
        assert_eq!(lotes[1].paginas, vec![Some(3), Some(4)]);
        assert_eq!(lotes[2].papel.formato, FormatoPapel::A5);
    }

    #[test]
    fn detecta_folhas_com_faces_de_tamanhos_incompativeis_e_completa_impar() {
        let incompatible = [
            DimensoesPagina::new(210.0, 297.0),
            DimensoesPagina::new(148.0, 210.0),
        ];
        assert!(matches!(
            lotes_por_papel(&incompatible, 10),
            Err(ErroPapel::ParIncompativel {
                frente: 1,
                verso: 2
            })
        ));
        let odd = [DimensoesPagina::new(210.0, 297.0)];
        assert_eq!(
            lotes_por_papel(&odd, 10).unwrap()[0].paginas,
            vec![Some(1), None]
        );
        assert!(matches!(lotes_por_papel(&odd, 0), Err(ErroPapel::LoteZero)));
    }

    #[test]
    fn faces_no_mesmo_formato_nominal_sao_compativeis_e_passadas_preservam_indices() {
        let paginas = [
            DimensoesPagina::new(208.0, 295.0),
            DimensoesPagina::new(212.0, 299.0),
            DimensoesPagina::new(148.0, 210.0),
            DimensoesPagina::new(150.0, 212.0),
            DimensoesPagina::new(210.0, 297.0),
            DimensoesPagina::new(210.0, 297.0),
        ];
        let perfil = PerfilImpressora {
            passada1_inversa: true,
            passada2_inversa: true,
            verso_girar_180: true,
            ..PerfilImpressora::default()
        };
        let lotes = plano_lotes_por_papel(&paginas, 5, OrdemPassadas::default(), perfil).unwrap();
        assert_eq!(lotes.len(), 3);
        assert_eq!(nums(&lotes[0].passada1), vec![Some(2)]);
        assert_eq!(nums(&lotes[0].passada2), vec![Some(1)]);
        assert_eq!(nums(&lotes[1].passada1), vec![Some(4)]);
        assert_eq!(nums(&lotes[1].passada2), vec![Some(3)]);
        assert_eq!(nums(&lotes[2].passada1), vec![Some(6)]);
        assert!(lotes.iter().flat_map(|l| &l.passada2).all(|p| p.girar_180));
    }

    #[test]
    fn agrupa_duas_folhas_do_mesmo_formato_nominal_com_dimensoes_diferentes() {
        let paginas = [
            DimensoesPagina::new(208.0, 297.0),
            DimensoesPagina::new(208.0, 297.0),
            DimensoesPagina::new(212.0, 297.0),
            DimensoesPagina::new(212.0, 297.0),
        ];
        let lotes = lotes_por_papel(&paginas, 10).unwrap();
        assert_eq!(lotes.len(), 1);
        assert_eq!(lotes[0].paginas, vec![Some(1), Some(2), Some(3), Some(4)]);
    }

    #[test]
    fn limita_paginas_antes_de_planejar_e_aceita_limite_exato() {
        let acima = vec![DimensoesPagina::new(210.0, 297.0); LIMITE_PAGINAS + 1];
        assert!(matches!(
            lotes_por_papel(&acima, 1),
            Err(ErroPapel::LimitePaginasExcedido)
        ));
        let no_limite = vec![DimensoesPagina::new(210.0, 297.0); LIMITE_PAGINAS];
        let lotes = lotes_por_papel(&no_limite, LIMITE_PAGINAS / 2).unwrap();
        assert_eq!(lotes.len(), 1);
        assert_eq!(lotes[0].paginas.len(), LIMITE_PAGINAS);
    }
}
