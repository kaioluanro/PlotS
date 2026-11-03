//! Planejamento puro de passadas para impressão manual frente e verso.

use std::collections::TryReserveError;

pub mod ajuste;
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

/// Como o maço é virado antes de voltar à entrada.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViradaMaco {
    /// O maço volta como saiu.
    NaoVirar,
    /// Vira pela lateral, como a página de um livro.
    PelaLateral,
    /// Vira pela borda de cima, como um bloco de notas.
    PelaBordaDeCima,
}

/// Instrução física registrada no perfil.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GestoRecolocar {
    pub virada: ViradaMaco,
    pub borda_primeiro: BordaEntrada,
}

impl GestoRecolocar {
    /// `true` quando é preciso virar o maço, por qualquer borda, para expor os versos.
    pub const fn vira_o_maco(self) -> bool {
        !matches!(self.virada, ViradaMaco::NaoVirar)
    }
}

impl Default for GestoRecolocar {
    fn default() -> Self {
        Self {
            virada: ViradaMaco::PelaLateral,
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

/// Configuração pronta para um tipo de máquina, usada sem medir a impressora.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModeloImpressora {
    /// Entrada traseira: sai com a face impressa para cima, e o maço vira pela borda de cima.
    JatoDeTinta,
    /// Gaveta: sai com a face impressa para baixo, e o maço volta sem ser virado.
    Laser,
}

impl ModeloImpressora {
    /// Código curto usado no arquivo de perfis, na linha de comando e na interface.
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::JatoDeTinta => "jato",
            Self::Laser => "laser",
        }
    }

    /// Modelo com este código exato.
    pub fn de_codigo(codigo: &str) -> Option<Self> {
        match codigo {
            "jato" => Some(Self::JatoDeTinta),
            "laser" => Some(Self::Laser),
            _ => None,
        }
    }

    /// Ordem das passadas e giro do verso. Nos dois modelos a frente sai da última página para
    /// a primeira; só o jato inverte o verso, e o imprime girado 180°.
    pub const fn perfil(self) -> PerfilImpressora {
        let jato = matches!(self, Self::JatoDeTinta);
        PerfilImpressora {
            passada1_inversa: true,
            passada2_inversa: jato,
            verso_girar_180: jato,
            gesto_recolocar: self.gesto(),
        }
    }

    /// Gesto de recolocação: o jato vira o maço pela borda de cima; a laser não vira.
    pub const fn gesto(self) -> GestoRecolocar {
        GestoRecolocar {
            virada: match self {
                Self::JatoDeTinta => ViradaMaco::PelaBordaDeCima,
                Self::Laser => ViradaMaco::NaoVirar,
            },
            borda_primeiro: BordaEntrada::Curta,
        }
    }
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
    /// A impressora imprime as duas faces sozinha: `passada1` é um trabalho só, na ordem do
    /// documento, e `passada2` fica vazia.
    pub duas_faces: bool,
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
            duas_faces: false,
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
    DimensaoInvalida {
        pagina: usize,
    },
    ParIncompativel {
        frente: usize,
        verso: usize,
    },
    LoteZero,
    LimitePaginasExcedido,
    ReservaFalhou,
    IntervaloInvalido {
        inicio: usize,
        fim: usize,
        total: usize,
    },
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
            Self::IntervaloInvalido { inicio, fim, total } => write!(
                f,
                "o trecho de {inicio} a {fim} não cabe em um documento de {total} páginas"
            ),
        }
    }
}

impl std::error::Error for ErroPapel {}

/// Um trecho contínuo do documento, com os números de página começando em 1 e o fim incluído.
/// Os números vêm de fora: só valem depois de `validar` contra o total de páginas.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntervaloPaginas {
    pub inicio: usize,
    pub fim: usize,
}

impl IntervaloPaginas {
    /// Aceita apenas `1 <= inicio <= fim <= total`.
    pub fn validar(self, total: usize) -> Result<Self, ErroPapel> {
        if self.inicio >= 1 && self.inicio <= self.fim && self.fim <= total {
            Ok(self)
        } else {
            Err(ErroPapel::IntervaloInvalido {
                inicio: self.inicio,
                fim: self.fim,
                total,
            })
        }
    }

    /// Quantas páginas o trecho tem; zero quando o fim vem antes do início.
    pub fn quantidade(self) -> usize {
        self.fim
            .saturating_sub(self.inicio)
            .saturating_add(usize::from(self.inicio <= self.fim))
    }
}

/// As páginas que entram no plano e quantas vieram antes delas no documento: somar essa conta ao
/// índice do recorte devolve o número original da página.
fn recortar_paginas(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
) -> Result<(&[DimensoesPagina], usize), ErroPapel> {
    let Some(intervalo) = intervalo else {
        return Ok((paginas, 0));
    };
    let valido = intervalo.validar(paginas.len())?;
    let invalido = || ErroPapel::IntervaloInvalido {
        inicio: intervalo.inicio,
        fim: intervalo.fim,
        total: paginas.len(),
    };
    let antes = valido.inicio.checked_sub(1).ok_or_else(invalido)?;
    let recorte = paginas.get(antes..valido.fim).ok_or_else(invalido)?;
    Ok((recorte, antes))
}

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

/// Orientação observada de uma página. Páginas quadradas, dentro da tolerância
/// do módulo, não têm orientação.
fn orientacao_observada(dimensoes: DimensoesPagina) -> Option<OrientacaoPapel> {
    if (dimensoes.largura_mm - dimensoes.altura_mm).abs() <= TOLERANCIA_PAPEL_MM {
        None
    } else if dimensoes.altura_mm > dimensoes.largura_mm {
        Some(OrientacaoPapel::Retrato)
    } else {
        Some(OrientacaoPapel::Paisagem)
    }
}

/// Devolve os pares (frente, verso), em números de página começando em 1, cujas
/// faces têm orientações diferentes, como retrato e paisagem. Páginas quadradas
/// e a página final sem verso não entram. Os tamanhos não são comparados aqui;
/// essa verificação continua em `plano_lotes_por_papel`.
pub fn pares_com_orientacao_mista(
    paginas: &[DimensoesPagina],
) -> Result<Vec<(usize, usize)>, ErroPapel> {
    pares_mistos_do_intervalo(paginas, None)
}

/// Como `pares_com_orientacao_mista`, olhando só as folhas do trecho, que começam na primeira
/// página dele. Os pares continuam com os números do documento.
pub fn pares_mistos_do_intervalo(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
) -> Result<Vec<(usize, usize)>, ErroPapel> {
    if paginas.len() > LIMITE_PAGINAS {
        return Err(ErroPapel::LimitePaginasExcedido);
    }
    let (paginas, antes) = recortar_paginas(paginas, intervalo)?;
    let mut orientacoes: Vec<Option<OrientacaoPapel>> = Vec::new();
    orientacoes
        .try_reserve(paginas.len())
        .map_err(|_| ErroPapel::ReservaFalhou)?;
    for (i, dims) in paginas.iter().copied().enumerate() {
        reconhecer_papel(dims).map_err(|_| ErroPapel::DimensaoInvalida {
            pagina: antes + i + 1,
        })?;
        orientacoes.push(orientacao_observada(dims));
    }
    let mut pares = Vec::new();
    pares
        .try_reserve(paginas.len() / 2)
        .map_err(|_| ErroPapel::ReservaFalhou)?;
    for i in (0..orientacoes.len()).step_by(2) {
        let frente = orientacoes.get(i).copied().flatten();
        let verso = orientacoes.get(i + 1).copied().flatten();
        if let (Some(f), Some(v)) = (frente, verso) {
            if f != v {
                pares.push((antes + i + 1, antes + i + 2));
            }
        }
    }
    Ok(pares)
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
    /// A impressora imprime as duas faces sozinha: `passada1` é um trabalho só, na ordem do
    /// documento, e `passada2` fica vazia.
    pub duas_faces: bool,
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
    plano_lotes_do_intervalo(paginas, None, folhas_por_lote, ordem_passadas, perfil)
}

/// Como `plano_lotes_por_papel`, olhando só o trecho pedido: as folhas começam na primeira página
/// dele, um trecho ímpar recebe o verso em branco e os números continuam sendo os do documento.
/// Sem trecho, vale o documento inteiro.
pub fn plano_lotes_do_intervalo(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
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
    let (paginas, antes) = recortar_paginas(paginas, intervalo)?;
    let reserva =
        |vec: &mut Vec<Papel>, n| vec.try_reserve(n).map_err(|_| ErroPapel::ReservaFalhou);
    let mut papeis = Vec::new();
    reserva(&mut papeis, paginas.len())?;
    for (i, dims) in paginas.iter().copied().enumerate() {
        reconhecer_papel(dims)
            .map_err(|_| ErroPapel::DimensaoInvalida {
                pagina: antes + i + 1,
            })
            .map(|p| papeis.push(p))?;
    }
    let mut folhas: Vec<(Papel, Option<usize>, Option<usize>)> = Vec::new();
    folhas
        .try_reserve(paginas.len().saturating_add(1) / 2)
        .map_err(|_| ErroPapel::ReservaFalhou)?;
    for i in (0..paginas.len()).step_by(2) {
        let Some(frente) = papeis.get(i).copied() else {
            continue;
        };
        let verso = papeis.get(i + 1).copied();
        if let Some(v) = verso {
            let mesmo_formato_nominal =
                frente.formato != FormatoPapel::Personalizado && frente.formato == v.formato;
            if !mesmo_formato_nominal
                && ((frente.largura_mm - v.largura_mm).abs() > TOLERANCIA_PAPEL_MM
                    || (frente.altura_mm - v.altura_mm).abs() > TOLERANCIA_PAPEL_MM)
            {
                return Err(ErroPapel::ParIncompativel {
                    frente: antes + i + 1,
                    verso: antes + i + 2,
                });
            }
        }
        folhas.push((frente, Some(antes + i + 1), verso.map(|_| antes + i + 2)));
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
                duas_faces: false,
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

/// Papéis que seguem pelo frente e verso automático. A regra é conservadora: outra máquina pode
/// aceitar mais, mas o fluxo manual funciona em todas.
pub const fn papel_aceita_duas_faces(papel: &Papel) -> bool {
    matches!(papel.formato, FormatoPapel::A4 | FormatoPapel::Carta)
}

/// Como repartir os lotes de uma impressora que imprime as duas faces sozinha.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepartoAutomatico {
    /// Sem gesto não há maço a limitar: um trabalho por trecho seguido do mesmo papel.
    PorTrechoDePapel,
    /// Na divisão, os lotes são os mesmos de todas as impressoras, e cada um vira um trabalho.
    PorLote,
}

/// Plano de uma impressora que imprime as duas faces sozinha. Os papéis aceitos saem em um trabalho
/// na ordem do documento, sem inverter, sem girar e sem face em branco. Os outros papéis seguem o
/// fluxo manual de `perfil`. As folhas e a ordem no documento são as mesmas do plano manual.
///
/// A página deitada segue o que o envio já faz: gira para caber na folha em pé. A virada é sempre
/// pela borda longa da folha, que nessa página é a borda de cima: ela se lê virando como um
/// calendário, e não como um livro.
pub fn plano_automatico_do_intervalo(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
    folhas_por_lote: usize,
    ordem_passadas: OrdemPassadas,
    perfil: PerfilImpressora,
    reparto: RepartoAutomatico,
) -> Result<Vec<LotePapel>, ErroPapel> {
    let plano =
        plano_lotes_do_intervalo(paginas, intervalo, folhas_por_lote, ordem_passadas, perfil)?;
    let mut lotes: Vec<LotePapel> = Vec::new();
    lotes
        .try_reserve(plano.len())
        .map_err(|_| ErroPapel::ReservaFalhou)?;
    for lote in plano {
        if !papel_aceita_duas_faces(&lote.papel) {
            lotes.push(lote);
            continue;
        }
        let mut documento = Vec::new();
        documento
            .try_reserve(lote.paginas.len())
            .map_err(|_| ErroPapel::ReservaFalhou)?;
        documento.extend(lote.paginas.iter().flatten().copied().map(Pagina::original));
        let junta = reparto == RepartoAutomatico::PorTrechoDePapel;
        match lotes.last_mut() {
            Some(anterior)
                if junta && anterior.duas_faces && anterior.papel.formato == lote.papel.formato =>
            {
                anterior
                    .paginas
                    .try_reserve(lote.paginas.len())
                    .map_err(|_| ErroPapel::ReservaFalhou)?;
                anterior
                    .passada1
                    .try_reserve(documento.len())
                    .map_err(|_| ErroPapel::ReservaFalhou)?;
                anterior.paginas.extend(lote.paginas);
                anterior.passada1.extend(documento);
            }
            _ => lotes.push(LotePapel {
                papel: lote.papel,
                paginas: lote.paginas,
                passada1: documento,
                passada2: Vec::new(),
                duas_faces: true,
            }),
        }
    }
    Ok(lotes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numeros(paginas: &[Pagina]) -> Vec<Option<usize>> {
        paginas.iter().map(|pagina| pagina.numero).collect()
    }

    const A4: DimensoesPagina = DimensoesPagina::new(210.0, 297.0);
    const A5: DimensoesPagina = DimensoesPagina::new(148.0, 210.0);
    const PAISAGEM: DimensoesPagina = DimensoesPagina::new(297.0, 210.0);

    fn trecho(inicio: usize, fim: usize) -> Option<IntervaloPaginas> {
        Some(IntervaloPaginas { inicio, fim })
    }

    fn plano_do_modelo(
        paginas: &[DimensoesPagina],
        intervalo: Option<IntervaloPaginas>,
        modelo: ModeloImpressora,
    ) -> Result<Vec<LotePapel>, ErroPapel> {
        plano_lotes_do_intervalo(
            paginas,
            intervalo,
            10,
            OrdemPassadas::default(),
            modelo.perfil(),
        )
    }

    const CARTA: DimensoesPagina = DimensoesPagina::new(216.0, 279.0);

    fn automatico(
        paginas: &[DimensoesPagina],
        intervalo: Option<IntervaloPaginas>,
        folhas: usize,
        reparto: RepartoAutomatico,
    ) -> Vec<LotePapel> {
        plano_automatico_do_intervalo(
            paginas,
            intervalo,
            folhas,
            OrdemPassadas::default(),
            ModeloImpressora::JatoDeTinta.perfil(),
            reparto,
        )
        .unwrap()
    }

    fn inteiros(paginas: &[Pagina]) -> Vec<usize> {
        paginas.iter().filter_map(|pagina| pagina.numero).collect()
    }

    #[test]
    fn automatico_sai_em_um_trabalho_na_ordem_do_documento_sem_inverter_nem_girar() {
        let lotes = automatico(&[A4; 8], None, 1, RepartoAutomatico::PorTrechoDePapel);
        assert_eq!(lotes.len(), 1, "folhas por lote não limita o automático");
        assert!(lotes[0].duas_faces);
        assert_eq!(inteiros(&lotes[0].passada1), [1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(lotes[0].passada2.is_empty());
        assert!(lotes[0].passada1.iter().all(|pagina| !pagina.girar_180));
    }

    #[test]
    fn automatico_com_total_impar_nao_recebe_folha_em_branco() {
        let lotes = automatico(&[A4; 5], None, 10, RepartoAutomatico::PorTrechoDePapel);
        assert_eq!(lotes[0].passada1.len(), 5);
        assert!(lotes[0]
            .passada1
            .iter()
            .all(|pagina| pagina.numero.is_some()));
        assert_eq!(
            lotes[0].paginas.last(),
            Some(&None),
            "a folha continua com o verso vazio"
        );
    }

    #[test]
    fn documento_misto_alterna_os_fluxos_em_ordem_de_documento() {
        let paginas = [A4, A4, A4, A4, A5, A5, A5, A5, A5, A5, CARTA, CARTA, A4, A4];
        let lotes = automatico(&paginas, None, 2, RepartoAutomatico::PorTrechoDePapel);
        let resumo: Vec<(FormatoPapel, bool)> = lotes
            .iter()
            .map(|lote| (lote.papel.formato, lote.duas_faces))
            .collect();
        assert_eq!(
            resumo,
            [
                (FormatoPapel::A4, true),
                (FormatoPapel::A5, false),
                (FormatoPapel::A5, false),
                (FormatoPapel::Carta, true),
                (FormatoPapel::A4, true),
            ]
        );
        assert_eq!(inteiros(&lotes[0].passada1), [1, 2, 3, 4]);
        assert_eq!(inteiros(&lotes[3].passada1), [11, 12]);
        assert_eq!(inteiros(&lotes[4].passada1), [13, 14]);
    }

    #[test]
    fn papel_fora_de_a4_e_carta_continua_em_uma_face_com_o_perfil_manual() {
        for papel in [
            A5,
            DimensoesPagina::new(216.0, 330.0),
            DimensoesPagina::new(216.0, 356.0),
            DimensoesPagina::new(100.0, 150.0),
        ] {
            let paginas = [papel; 4];
            let lotes = automatico(&paginas, None, 10, RepartoAutomatico::PorTrechoDePapel);
            let manual = plano_do_modelo(&paginas, None, ModeloImpressora::JatoDeTinta).unwrap();
            assert_eq!(lotes, manual);
            assert!(!lotes[0].duas_faces);
        }
    }

    #[test]
    fn automatico_respeita_o_trecho_com_os_numeros_do_documento() {
        let lotes = automatico(
            &[A4; 12],
            trecho(3, 7),
            2,
            RepartoAutomatico::PorTrechoDePapel,
        );
        assert_eq!(lotes.len(), 1);
        assert_eq!(inteiros(&lotes[0].passada1), [3, 4, 5, 6, 7]);
    }

    #[test]
    fn na_divisao_os_lotes_sao_os_mesmos_do_plano_manual() {
        let lotes = automatico(&[A4; 18], None, 2, RepartoAutomatico::PorLote);
        assert_eq!(lotes.len(), 5);
        assert!(lotes.iter().all(|lote| lote.duas_faces));
        assert_eq!(inteiros(&lotes[1].passada1), [5, 6, 7, 8]);
        assert_eq!(inteiros(&lotes[4].passada1), [17, 18]);
    }

    #[test]
    fn pagina_deitada_fica_no_mesmo_trabalho_do_mesmo_papel() {
        let paginas = [A4, A4, PAISAGEM, PAISAGEM, A4, A4];
        let lotes = automatico(&paginas, None, 10, RepartoAutomatico::PorTrechoDePapel);
        assert_eq!(lotes.len(), 1);
        assert_eq!(inteiros(&lotes[0].passada1), [1, 2, 3, 4, 5, 6]);
        let divisao = automatico(&paginas, None, 10, RepartoAutomatico::PorLote);
        assert_eq!(
            divisao.len(),
            3,
            "a divisão mantém os lotes de todas as impressoras"
        );
    }

    #[test]
    fn par_de_papeis_incompativeis_e_recusado_como_no_plano_manual() {
        let erro = plano_automatico_do_intervalo(
            &[A4, A5],
            None,
            10,
            OrdemPassadas::default(),
            PerfilImpressora::default(),
            RepartoAutomatico::PorLote,
        );
        assert!(matches!(
            erro,
            Err(ErroPapel::ParIncompativel {
                frente: 1,
                verso: 2
            })
        ));
    }

    #[test]
    fn trecho_par_mantem_os_numeros_do_documento_na_ordem_de_cada_modelo() {
        let paginas = [A4; 12];
        let jato = plano_do_modelo(&paginas, trecho(3, 8), ModeloImpressora::JatoDeTinta).unwrap();
        assert_eq!(jato.len(), 1);
        assert_eq!(numeros(&jato[0].passada1), [Some(8), Some(6), Some(4)]);
        assert_eq!(numeros(&jato[0].passada2), [Some(7), Some(5), Some(3)]);
        assert!(jato[0].passada2.iter().all(|pagina| pagina.girar_180));

        let laser = plano_do_modelo(&paginas, trecho(3, 8), ModeloImpressora::Laser).unwrap();
        assert_eq!(numeros(&laser[0].passada1), [Some(8), Some(6), Some(4)]);
        assert_eq!(numeros(&laser[0].passada2), [Some(3), Some(5), Some(7)]);
        assert!(laser[0].passada2.iter().all(|pagina| !pagina.girar_180));
    }

    #[test]
    fn trecho_impar_recebe_o_verso_em_branco_como_o_documento_inteiro() {
        let paginas = [A4; 12];
        let jato = plano_do_modelo(&paginas, trecho(3, 7), ModeloImpressora::JatoDeTinta).unwrap();
        assert_eq!(
            jato[0].paginas,
            [Some(3), Some(4), Some(5), Some(6), Some(7), None]
        );
        assert_eq!(numeros(&jato[0].passada1), [None, Some(6), Some(4)]);
        assert_eq!(numeros(&jato[0].passada2), [Some(7), Some(5), Some(3)]);

        let laser = plano_do_modelo(&paginas, trecho(3, 7), ModeloImpressora::Laser).unwrap();
        assert_eq!(numeros(&laser[0].passada1), [None, Some(6), Some(4)]);
        assert_eq!(numeros(&laser[0].passada2), [Some(3), Some(5), Some(7)]);
    }

    #[test]
    fn trecho_que_comeca_em_pagina_par_forma_as_folhas_a_partir_dele() {
        let lotes = plano_do_modelo(&[A4; 12], trecho(4, 7), ModeloImpressora::Laser).unwrap();
        assert_eq!(lotes[0].paginas, [Some(4), Some(5), Some(6), Some(7)]);
    }

    #[test]
    fn primeira_e_ultima_pagina_sozinhas_viram_uma_folha_com_verso_em_branco() {
        for pagina in [1, 12, 6] {
            let lotes = plano_do_modelo(&[A4; 12], trecho(pagina, pagina), ModeloImpressora::Laser)
                .unwrap();
            assert_eq!(lotes.len(), 1);
            assert_eq!(lotes[0].paginas, [Some(pagina), None]);
            assert_eq!(numeros(&lotes[0].passada1), [None]);
            assert_eq!(numeros(&lotes[0].passada2), [Some(pagina)]);
        }
    }

    #[test]
    fn trecho_do_documento_inteiro_da_o_mesmo_plano_que_nenhum_trecho() {
        for total in [11, 12] {
            let paginas = vec![A4; total];
            for modelo in [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser] {
                let sem =
                    plano_lotes_por_papel(&paginas, 4, OrdemPassadas::default(), modelo.perfil())
                        .unwrap();
                for intervalo in [None, trecho(1, total)] {
                    let com = plano_lotes_do_intervalo(
                        &paginas,
                        intervalo,
                        4,
                        OrdemPassadas::default(),
                        modelo.perfil(),
                    )
                    .unwrap();
                    assert_eq!(com, sem);
                }
            }
        }
    }

    #[test]
    fn trecho_fora_do_documento_e_recusado_sem_estourar() {
        let paginas = [A4; 12];
        let recusados = [
            (8, 3),
            (0, 3),
            (0, 0),
            (3, 0),
            (3, 13),
            (13, 13),
            (13, 20),
            (1, usize::MAX),
            (usize::MAX, usize::MAX),
            (usize::MAX, 1),
        ];
        for (inicio, fim) in recusados {
            let erro = plano_do_modelo(&paginas, trecho(inicio, fim), ModeloImpressora::Laser);
            assert_eq!(
                erro,
                Err(ErroPapel::IntervaloInvalido {
                    inicio,
                    fim,
                    total: 12
                })
            );
            assert!(pares_mistos_do_intervalo(&paginas, trecho(inicio, fim)).is_err());
        }
    }

    #[test]
    fn documento_vazio_nao_aceita_trecho_nenhum() {
        let intervalo = IntervaloPaginas { inicio: 1, fim: 1 };
        assert!(intervalo.validar(0).is_err());
        assert!(intervalo.validar(1).is_ok());
    }

    #[test]
    fn papeis_diferentes_seguem_os_lotes_olhando_so_o_trecho() {
        let paginas = [A4, A4, A4, A4, A5, A5, A5, A5, A4, A4, A4, A4];
        let dois = plano_do_modelo(&paginas, trecho(3, 8), ModeloImpressora::Laser).unwrap();
        assert_eq!(dois.len(), 2);
        assert_eq!(dois[0].papel.formato, FormatoPapel::A4);
        assert_eq!(dois[0].paginas, [Some(3), Some(4)]);
        assert_eq!(dois[1].papel.formato, FormatoPapel::A5);
        assert_eq!(dois[1].paginas, [Some(5), Some(6), Some(7), Some(8)]);

        let um = plano_do_modelo(&paginas, trecho(5, 8), ModeloImpressora::Laser).unwrap();
        assert_eq!(um.len(), 1);
        assert_eq!(um[0].papel.formato, FormatoPapel::A5);
    }

    #[test]
    fn par_incompativel_do_trecho_cita_os_numeros_do_documento() {
        let paginas = [A4, A4, A4, A4, A5, A5, A5, A5];
        assert_eq!(
            plano_do_modelo(&paginas, trecho(4, 7), ModeloImpressora::Laser),
            Err(ErroPapel::ParIncompativel {
                frente: 4,
                verso: 5
            })
        );
    }

    #[test]
    fn pagina_ruim_fora_do_trecho_nao_impede_e_dentro_cita_o_numero_original() {
        let ruim = DimensoesPagina::new(0.0, 297.0);
        let paginas = [ruim, A4, A4, A4, ruim, A4];
        let lotes = plano_do_modelo(&paginas, trecho(2, 4), ModeloImpressora::Laser).unwrap();
        assert_eq!(lotes[0].paginas, [Some(2), Some(3), Some(4), None]);
        assert_eq!(
            plano_do_modelo(&paginas, trecho(4, 6), ModeloImpressora::Laser),
            Err(ErroPapel::DimensaoInvalida { pagina: 5 })
        );
    }

    #[test]
    fn tamanho_do_lote_conta_as_folhas_do_trecho() {
        let lotes = plano_lotes_do_intervalo(
            &[A4; 12],
            trecho(3, 8),
            2,
            OrdemPassadas::default(),
            PerfilImpressora::default(),
        )
        .unwrap();
        assert_eq!(lotes.len(), 2);
        assert_eq!(lotes[0].paginas, [Some(3), Some(4), Some(5), Some(6)]);
        assert_eq!(lotes[1].paginas, [Some(7), Some(8)]);
    }

    #[test]
    fn orientacao_mista_olha_as_folhas_do_trecho_com_os_numeros_originais() {
        let paginas = [A4, A4, A4, PAISAGEM, A4, A4];
        assert_eq!(pares_mistos_do_intervalo(&paginas, None), Ok(vec![(3, 4)]));
        assert_eq!(
            pares_mistos_do_intervalo(&paginas, trecho(3, 4)),
            Ok(vec![(3, 4)])
        );
        assert_eq!(
            pares_mistos_do_intervalo(&paginas, trecho(4, 5)),
            Ok(vec![(4, 5)])
        );
        assert_eq!(
            pares_mistos_do_intervalo(&paginas, trecho(5, 6)),
            Ok(vec![])
        );
    }

    #[test]
    fn quantidade_conta_as_paginas_do_trecho() {
        assert_eq!(IntervaloPaginas { inicio: 3, fim: 8 }.quantidade(), 6);
        assert_eq!(IntervaloPaginas { inicio: 5, fim: 5 }.quantidade(), 1);
        assert_eq!(IntervaloPaginas { inicio: 8, fim: 3 }.quantidade(), 0);
    }

    #[test]
    fn jato_de_tinta_inverte_as_duas_passadas_e_gira_o_verso() {
        let modelo = ModeloImpressora::JatoDeTinta;
        let lotes = plano_documento(6, 3, OrdemPassadas::ParesPrimeiro, modelo.perfil()).unwrap();
        assert_eq!(numeros(&lotes[0].passada1), [Some(6), Some(4), Some(2)]);
        assert_eq!(numeros(&lotes[0].passada2), [Some(5), Some(3), Some(1)]);
        assert!(lotes[0].passada1.iter().all(|pagina| !pagina.girar_180));
        assert!(lotes[0].passada2.iter().all(|pagina| pagina.girar_180));
        assert_eq!(modelo.gesto().virada, ViradaMaco::PelaBordaDeCima);
        assert_eq!(modelo.perfil().gesto_recolocar, modelo.gesto());
    }

    #[test]
    fn laser_inverte_so_a_frente_e_nao_vira_o_maco() {
        let modelo = ModeloImpressora::Laser;
        let lotes = plano_documento(6, 3, OrdemPassadas::ParesPrimeiro, modelo.perfil()).unwrap();
        assert_eq!(numeros(&lotes[0].passada1), [Some(6), Some(4), Some(2)]);
        assert_eq!(numeros(&lotes[0].passada2), [Some(1), Some(3), Some(5)]);
        assert!(lotes[0].passada2.iter().all(|pagina| !pagina.girar_180));
        assert_eq!(modelo.gesto().virada, ViradaMaco::NaoVirar);
        assert!(!modelo.gesto().vira_o_maco());
    }

    #[test]
    fn codigo_do_modelo_vai_e_volta_e_recusa_o_que_nao_conhece() {
        for modelo in [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser] {
            assert_eq!(ModeloImpressora::de_codigo(modelo.codigo()), Some(modelo));
        }
        for codigo in ["", "Jato", "LASER", "traseira", " jato"] {
            assert_eq!(ModeloImpressora::de_codigo(codigo), None, "{codigo:?}");
        }
    }

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
                                virada: ViradaMaco::NaoVirar,
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

    #[test]
    fn pares_com_orientacao_mista_apontam_so_as_folhas_com_retrato_e_paisagem() {
        let paginas = [
            DimensoesPagina::new(210.0, 297.0),
            DimensoesPagina::new(210.0, 297.0),
            DimensoesPagina::new(297.0, 210.0),
            DimensoesPagina::new(210.0, 297.0),
            DimensoesPagina::new(297.0, 210.0),
            DimensoesPagina::new(297.0, 210.0),
        ];
        assert_eq!(pares_com_orientacao_mista(&paginas).unwrap(), vec![(3, 4)]);
        let invertida = [
            DimensoesPagina::new(842.0, 595.0),
            DimensoesPagina::new(595.0, 842.0),
        ];
        assert_eq!(
            pares_com_orientacao_mista(&invertida).unwrap(),
            vec![(1, 2)]
        );
    }

    #[test]
    fn pares_com_orientacao_mista_ignora_quadradas_e_pagina_sem_verso() {
        let quadradas = [
            DimensoesPagina::new(300.0, 300.0),
            DimensoesPagina::new(300.0, 301.0),
            DimensoesPagina::new(301.0, 300.0),
            DimensoesPagina::new(210.0, 297.0),
            DimensoesPagina::new(297.0, 210.0),
            DimensoesPagina::new(210.0, 297.0),
        ];
        assert_eq!(
            pares_com_orientacao_mista(&quadradas).unwrap(),
            vec![(5, 6)]
        );
        let quadrada_com_paisagem = [
            DimensoesPagina::new(300.0, 300.0),
            DimensoesPagina::new(297.0, 210.0),
        ];
        assert!(pares_com_orientacao_mista(&quadrada_com_paisagem)
            .unwrap()
            .is_empty());
        let sem_verso = [DimensoesPagina::new(210.0, 297.0)];
        assert!(pares_com_orientacao_mista(&sem_verso).unwrap().is_empty());
    }

    #[test]
    fn pares_com_orientacao_mista_aceita_vazio_e_rejeita_dimensao_invalida() {
        assert!(pares_com_orientacao_mista(&[]).unwrap().is_empty());
        let invalida = [
            DimensoesPagina::new(210.0, 297.0),
            DimensoesPagina::new(f64::NAN, 297.0),
        ];
        assert_eq!(
            pares_com_orientacao_mista(&invalida),
            Err(ErroPapel::DimensaoInvalida { pagina: 2 })
        );
        let limite = vec![DimensoesPagina::new(210.0, 297.0); LIMITE_PAGINAS + 1];
        assert_eq!(
            pares_com_orientacao_mista(&limite),
            Err(ErroPapel::LimitePaginasExcedido)
        );
    }
}
