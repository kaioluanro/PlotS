//! Planejamento puro de passadas para impressão manual frente e verso.

use std::collections::TryReserveError;

pub mod ajuste;
pub mod calibracao;
pub mod distribuicao;

/// Maior quantidade de páginas que o planejador aceita em uma chamada.
pub const LIMITE_PAGINAS: usize = 100_000;

/// Quem uma página é no plano: a cópia e o número de origem (`None` é a face em branco).
pub type IdentidadeDaPagina = (usize, Option<usize>);

/// Uma página original, ou a folha em branco acrescentada ao fim de um documento ímpar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pagina {
    /// Número de página de origem, começando em 1. `None` representa uma página em branco.
    pub numero: Option<usize>,
    /// Indica que o conteúdo deve ser girado em 180 graus ao ser enviado.
    pub girar_180: bool,
    /// Cópia do documento a que a página pertence, a partir de 0. Com o número, identifica a folha
    /// no plano mesmo quando as cópias repetem as mesmas páginas, e acompanha a página em qualquer
    /// reordenação das passadas.
    pub copia: usize,
}

impl Pagina {
    /// Quem a página é no plano: a cópia e o número de origem.
    pub fn identidade(&self) -> IdentidadeDaPagina {
        (self.copia, self.numero)
    }

    fn original(numero: usize) -> Self {
        Self {
            numero: Some(numero),
            girar_180: false,
            copia: 0,
        }
    }

    fn em_branco() -> Self {
        Self {
            numero: None,
            girar_180: false,
            copia: 0,
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
    /// Gaveta: sai com a face impressa para baixo, e o maço volta sem ser virado nem girado.
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
    /// a primeira, e o verso vai girado 180°; só o jato inverte o verso. O jato gira porque vira
    /// o maço pela borda de cima. A laser gira porque a curva em U troca a aresta que lidera, e o
    /// maço volta como saiu.
    pub const fn perfil(self) -> PerfilImpressora {
        PerfilImpressora {
            passada1_inversa: true,
            passada2_inversa: matches!(self, Self::JatoDeTinta),
            verso_girar_180: true,
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
            .try_reserve(paginas.len().div_ceil(2))
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

/// Páginas que formam a frente física de cada folha do lote, para um envio só de frentes.
///
/// A ordem é a da primeira passada, sem o giro que compensa a segunda: isolada, a frente é a
/// primeira passada, e é assim que o maço volta a casar com [`versos_do_lote`] depois do gesto.
/// Num lote que a impressora imprime nas duas faces, a própria `passada1` já é o documento
/// inteiro.
pub fn frentes_do_lote(lote: &Lote) -> Vec<Pagina> {
    if lote.duas_faces {
        return lote.passada1.clone();
    }
    let base = frentes_estruturais(lote);
    let base_invertida = invertida(&base);
    let versos = versos_estruturais(lote);
    let versos_invertidos = invertida(&versos);
    if mesma_sequencia(&lote.passada1, &versos) {
        base
    } else if mesma_sequencia(&lote.passada1, &versos_invertidos) {
        base_invertida
    } else {
        // A primeira passada não descreve as folhas: a segunda, sem o giro, é a reserva.
        sem_giro(&lote.passada2)
    }
}

/// Páginas que formam o verso físico de cada folha do lote, para um envio só de versos.
///
/// A ordem é a da segunda passada, e o giro é o que o perfil aplica a ela: isolado, o verso é a
/// segunda passada e precisa das duas coisas para casar com as frentes. Num lote de duas faces
/// não há verso separado.
pub fn versos_do_lote(lote: &Lote) -> Vec<Pagina> {
    if lote.duas_faces {
        return Vec::new();
    }
    let base = versos_estruturais(lote);
    let base_invertida = invertida(&base);
    let frentes = frentes_estruturais(lote);
    let frentes_invertidas = invertida(&frentes);
    let segunda = sem_giro(&lote.passada2);
    let mut versos = if mesma_sequencia(&segunda, &frentes) {
        base
    } else if mesma_sequencia(&segunda, &frentes_invertidas) {
        base_invertida
    } else if mesma_sequencia(&lote.passada1, &base)
        || mesma_sequencia(&lote.passada1, &base_invertida)
    {
        // A segunda passada não descreve as frentes: a primeira ainda diz a ordem dos versos.
        lote.passada1.clone()
    } else {
        base
    };
    let giro = lote.passada2.iter().any(|pagina| pagina.girar_180);
    for pagina in &mut versos {
        pagina.girar_180 = giro;
    }
    versos
}

fn sem_giro(paginas: &[Pagina]) -> Vec<Pagina> {
    paginas
        .iter()
        .map(|pagina| Pagina {
            girar_180: false,
            ..*pagina
        })
        .collect()
}

fn invertida(paginas: &[Pagina]) -> Vec<Pagina> {
    paginas.iter().rev().copied().collect()
}

fn mesma_sequencia(a: &[Pagina], b: &[Pagina]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.identidade() == b.identidade())
}

/// A primeira página de cada folha, na ordem das folhas.
fn frentes_estruturais(lote: &Lote) -> Vec<Pagina> {
    lote.paginas
        .chunks(2)
        .filter_map(|par| par.first().copied())
        .map(|pagina| Pagina {
            girar_180: false,
            ..pagina
        })
        .collect()
}

/// A segunda página de cada folha, na ordem das folhas. Uma página sem par ganha a face em branco.
fn versos_estruturais(lote: &Lote) -> Vec<Pagina> {
    lote.paginas
        .chunks(2)
        .filter_map(|par| match par {
            [_, verso] => Some(*verso),
            [frente] => Some(Pagina {
                numero: None,
                girar_180: false,
                copia: frente.copia,
            }),
            _ => None,
        })
        .map(|pagina| Pagina {
            girar_180: false,
            ..pagina
        })
        .collect()
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
    LoteZero,
    LimitePaginasExcedido,
    ReservaFalhou,
    IntervaloInvalido {
        inicio: usize,
        fim: usize,
        total: usize,
    },
    CopiasInvalidas {
        copias: usize,
    },
}

impl std::fmt::Display for ErroPapel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CopiasInvalidas { copias } => {
                write!(
                    f,
                    "o número de cópias deve estar entre 1 e {LIMITE_COPIAS}, não {copias}"
                )
            }
            Self::DimensaoInvalida { pagina } => {
                write!(f, "dimensões inválidas na página {pagina}")
            }
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
/// e a página sem verso não entram. As folhas são as de `plano_lotes_por_papel`: páginas de
/// papéis diferentes não formam par.
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
    let mut papeis: Vec<Papel> = Vec::new();
    papeis
        .try_reserve(paginas.len())
        .map_err(|_| ErroPapel::ReservaFalhou)?;
    for (i, dims) in paginas.iter().copied().enumerate() {
        papeis.push(
            reconhecer_papel(dims).map_err(|_| ErroPapel::DimensaoInvalida {
                pagina: antes + i + 1,
            })?,
        );
        orientacoes.push(orientacao_observada(dims));
    }
    let mut pares = Vec::new();
    pares
        .try_reserve(paginas.len() / 2)
        .map_err(|_| ErroPapel::ReservaFalhou)?;
    for (i, verso) in emparelhar(&papeis) {
        let Some(verso) = verso else { continue };
        let frente = orientacoes.get(i).copied().flatten();
        let verso_o = orientacoes.get(verso).copied().flatten();
        if let (Some(f), Some(v)) = (frente, verso_o) {
            if f != v {
                pares.push((antes + i + 1, antes + verso + 1));
            }
        }
    }
    Ok(pares)
}

/// Os pares (frente, verso), com os números do documento, cujas páginas não cabem na mesma
/// folha: cada uma sai na sua, com o verso em branco. Olha só as folhas do trecho.
pub fn pares_separados_do_intervalo(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
) -> Result<Vec<(usize, usize)>, ErroPapel> {
    if paginas.len() > LIMITE_PAGINAS {
        return Err(ErroPapel::LimitePaginasExcedido);
    }
    let (paginas, antes) = recortar_paginas(paginas, intervalo)?;
    let mut papeis: Vec<Papel> = Vec::new();
    papeis
        .try_reserve(paginas.len())
        .map_err(|_| ErroPapel::ReservaFalhou)?;
    for (i, dims) in paginas.iter().copied().enumerate() {
        papeis.push(
            reconhecer_papel(dims).map_err(|_| ErroPapel::DimensaoInvalida {
                pagina: antes + i + 1,
            })?,
        );
    }
    let mut pares = Vec::new();
    pares
        .try_reserve(paginas.len() / 2)
        .map_err(|_| ErroPapel::ReservaFalhou)?;
    for (k, par) in papeis.chunks(2).enumerate() {
        if let [a, b] = par {
            if !cabem_na_mesma_folha(*a, *b) {
                pares.push((antes + k * 2 + 1, antes + k * 2 + 2));
            }
        }
    }
    Ok(pares)
}

/// Duas páginas cabem na mesma folha: o mesmo formato nominal ou a mesma medida dentro da
/// tolerância. A orientação não conta aqui; ela só gera aviso (`pares_com_orientacao_mista`).
fn cabem_na_mesma_folha(frente: Papel, verso: Papel) -> bool {
    (frente.formato != FormatoPapel::Personalizado && frente.formato == verso.formato)
        || ((frente.largura_mm - verso.largura_mm).abs() <= TOLERANCIA_PAPEL_MM
            && (frente.altura_mm - verso.altura_mm).abs() <= TOLERANCIA_PAPEL_MM)
}

/// Monta as folhas como pares (frente, verso) de índices das páginas. Cada página fica no lado em
/// que o documento foi diagramado: as de posição ímpar são frente, as pares são verso. Só o par
/// cujas páginas não cabem na mesma folha se separa: cada uma vira frente de uma folha própria,
/// com o verso em branco. A última página de um total ímpar também fica sem verso.
fn emparelhar(papeis: &[Papel]) -> Vec<(usize, Option<usize>)> {
    let mut folhas = Vec::with_capacity(papeis.len().div_ceil(2));
    for (k, par) in papeis.chunks(2).enumerate() {
        let i = k * 2;
        match par {
            [a, b] if cabem_na_mesma_folha(*a, *b) => folhas.push((i, Some(i + 1))),
            [_, _] => {
                folhas.push((i, None));
                folhas.push((i + 1, None));
            }
            _ => folhas.push((i, None)),
        }
    }
    folhas
}

/// Um conjunto de folhas consecutivas do mesmo papel, limitado ao tamanho
/// solicitado. `paginas` contém índices de página originais (base 1); `None`
/// representa a face em branco final de um documento ímpar.
#[derive(Clone, Debug, PartialEq)]
pub struct LotePapel {
    pub papel: Papel,
    pub paginas: Vec<Option<usize>>,
    /// A cópia de cada item de `paginas`, na mesma ordem.
    pub copias: Vec<usize>,
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
    for (frente, verso) in emparelhar(&papeis) {
        let Some(papel) = papeis.get(frente).copied() else {
            continue;
        };
        folhas.push((
            papel,
            Some(antes + frente + 1),
            verso.map(|v| antes + v + 1),
        ));
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
                copias: Vec::new(),
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
            lote.copias.extend([0, 0]);
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

/// Plano de quem imprime sem parar: um lote por trecho seguido do mesmo papel, na ordem do perfil.
/// Não há limite de folhas: o que separa um lote do outro é a troca de papel.
pub fn plano_em_trechos_do_intervalo(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
    ordem_passadas: OrdemPassadas,
    perfil: PerfilImpressora,
) -> Result<Vec<LotePapel>, ErroPapel> {
    plano_lotes_do_intervalo(
        paginas,
        intervalo,
        paginas.len().max(1),
        ordem_passadas,
        perfil,
    )
}

/// Maior número de cópias que um pedido aceita.
pub const LIMITE_COPIAS: usize = 999;

/// Marca as páginas do lote como da cópia `copia`.
fn da_copia(lote: &mut LotePapel, copia: usize) {
    lote.copias.iter_mut().for_each(|c| *c = copia);
    for pagina in lote.passada1.iter_mut().chain(&mut lote.passada2) {
        pagina.copia = copia;
    }
}

/// Cópias do documento, planejadas como um documento só: cada cópia é planejada sozinha, com o
/// intervalo, e fecha as suas folhas antes da próxima, então a última folha de uma cópia de total
/// ímpar leva o verso em branco dela. Trechos do mesmo papel viram um lote só, com as passadas das
/// cópias em sequência. A ordem das passadas e o giro do perfil valem para o maço inteiro, por isso
/// são aplicados uma vez, depois de juntar as cópias. Com uma cópia, o resultado é o mesmo do plano
/// de trechos.
pub fn plano_copias_do_intervalo(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
    copias: usize,
    ordem_passadas: OrdemPassadas,
    perfil: PerfilImpressora,
) -> Result<Vec<LotePapel>, ErroPapel> {
    plano_copias_do_intervalo_com_modo(paginas, intervalo, copias, ordem_passadas, perfil, true)
}

/// Cópias do documento em lotes separados: cada cópia fica no seu próprio lote, para o envio
/// poder mandar uma de cada vez e esperar a fila confirmar antes da próxima. A ordem das
/// passadas e o giro do perfil valem para cada cópia, como se ela fosse um maço fechado.
pub fn plano_copias_do_intervalo_separadas(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
    copias: usize,
    ordem_passadas: OrdemPassadas,
    perfil: PerfilImpressora,
) -> Result<Vec<LotePapel>, ErroPapel> {
    plano_copias_do_intervalo_com_modo(paginas, intervalo, copias, ordem_passadas, perfil, false)
}

fn plano_copias_do_intervalo_com_modo(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
    copias: usize,
    ordem_passadas: OrdemPassadas,
    perfil: PerfilImpressora,
    juntar: bool,
) -> Result<Vec<LotePapel>, ErroPapel> {
    if !(1..=LIMITE_COPIAS).contains(&copias) {
        return Err(ErroPapel::CopiasInvalidas { copias });
    }
    let total = paginas
        .len()
        .checked_mul(copias)
        .ok_or(ErroPapel::LimitePaginasExcedido)?;
    if total > LIMITE_PAGINAS {
        return Err(ErroPapel::LimitePaginasExcedido);
    }
    let sem_ajuste_do_perfil = PerfilImpressora {
        passada1_inversa: false,
        passada2_inversa: false,
        verso_girar_180: false,
        ..perfil
    };
    let mut lotes: Vec<LotePapel> = Vec::new();
    for copia in 0..copias {
        for mut lote in
            plano_em_trechos_do_intervalo(paginas, intervalo, ordem_passadas, sem_ajuste_do_perfil)?
        {
            da_copia(&mut lote, copia);
            lotes.push(lote);
        }
    }
    if juntar {
        juntar_lotes(&mut lotes)?;
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

/// Junta lotes vizinhos do mesmo papel e do mesmo fluxo, preservando a ordem das páginas e das
/// cópias. É o que transforma várias cópias em um maço só.
fn juntar_lotes(lotes: &mut Vec<LotePapel>) -> Result<(), ErroPapel> {
    let mut juntos: Vec<LotePapel> = Vec::new();
    juntos
        .try_reserve_exact(lotes.len())
        .map_err(|_| ErroPapel::ReservaFalhou)?;
    for lote in lotes.drain(..) {
        match juntos.last_mut() {
            Some(anterior)
                if anterior.papel == lote.papel && anterior.duas_faces == lote.duas_faces =>
            {
                anterior
                    .paginas
                    .try_reserve(lote.paginas.len())
                    .map_err(|_| ErroPapel::ReservaFalhou)?;
                anterior
                    .copias
                    .try_reserve(lote.copias.len())
                    .map_err(|_| ErroPapel::ReservaFalhou)?;
                anterior
                    .passada1
                    .try_reserve(lote.passada1.len())
                    .map_err(|_| ErroPapel::ReservaFalhou)?;
                anterior
                    .passada2
                    .try_reserve(lote.passada2.len())
                    .map_err(|_| ErroPapel::ReservaFalhou)?;
                anterior.paginas.extend(lote.paginas);
                anterior.copias.extend(lote.copias);
                anterior.passada1.extend(lote.passada1);
                anterior.passada2.extend(lote.passada2);
            }
            _ => juntos.push(lote),
        }
    }
    *lotes = juntos;
    Ok(())
}

/// Cópias para a impressora que imprime as duas faces sozinha. Cada cópia sai como um trabalho
/// próprio, com as páginas do trecho em ordem: a impressora fecha a última folha de uma cópia ímpar
/// com o verso em branco, e nenhuma página se junta a outra cópia. Não se usa a cópia do driver.
pub fn plano_automatico_copias_do_intervalo(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
    copias: usize,
    ordem_passadas: OrdemPassadas,
    perfil: PerfilImpressora,
    reparto: RepartoAutomatico,
) -> Result<Vec<LotePapel>, ErroPapel> {
    plano_automatico_copias_do_intervalo_com_modo(
        paginas,
        intervalo,
        copias,
        ordem_passadas,
        perfil,
        reparto,
        false,
    )
}

/// Cópias para a impressora que imprime as duas faces sozinha, todas no mesmo trabalho quando
/// o papel e o fluxo permitem. É o comportamento de quem desmarca "uma cópia por vez".
pub fn plano_automatico_copias_do_intervalo_juntas(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
    copias: usize,
    ordem_passadas: OrdemPassadas,
    perfil: PerfilImpressora,
    reparto: RepartoAutomatico,
) -> Result<Vec<LotePapel>, ErroPapel> {
    plano_automatico_copias_do_intervalo_com_modo(
        paginas,
        intervalo,
        copias,
        ordem_passadas,
        perfil,
        reparto,
        true,
    )
}

fn plano_automatico_copias_do_intervalo_com_modo(
    paginas: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
    copias: usize,
    ordem_passadas: OrdemPassadas,
    perfil: PerfilImpressora,
    reparto: RepartoAutomatico,
    juntar: bool,
) -> Result<Vec<LotePapel>, ErroPapel> {
    if !(1..=LIMITE_COPIAS).contains(&copias) {
        return Err(ErroPapel::CopiasInvalidas { copias });
    }
    let total = paginas
        .len()
        .checked_mul(copias)
        .ok_or(ErroPapel::LimitePaginasExcedido)?;
    if total > LIMITE_PAGINAS {
        return Err(ErroPapel::LimitePaginasExcedido);
    }
    let mut lotes: Vec<LotePapel> = Vec::new();
    for copia in 0..copias {
        let inicio = lotes.len();
        lotes.extend(plano_automatico_do_intervalo(
            paginas,
            intervalo,
            paginas.len().max(1),
            ordem_passadas,
            perfil,
            reparto,
        )?);
        lotes[inicio..]
            .iter_mut()
            .for_each(|lote| da_copia(lote, copia));
    }
    if juntar {
        juntar_lotes(&mut lotes)?;
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
                anterior.copias.extend(lote.copias);
                anterior.passada1.extend(documento);
            }
            _ => lotes.push(LotePapel {
                papel: lote.papel,
                paginas: lote.paginas,
                copias: lote.copias,
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

    #[test]
    fn plano_em_trechos_faz_um_lote_por_papel_sem_limite_de_folhas() {
        let perfil = PerfilImpressora::default();
        let inteiro =
            plano_em_trechos_do_intervalo(&[A4; 101], None, OrdemPassadas::default(), perfil)
                .unwrap();
        assert_eq!(inteiro.len(), 1);
        assert_eq!(inteiro[0].passada1.len(), 51);
        assert_eq!(inteiro[0].passada2.len(), 51);
        let mut misto = vec![A4; 4];
        misto.extend([A5; 4]);
        misto.extend([A4; 2]);
        let trechos =
            plano_em_trechos_do_intervalo(&misto, None, OrdemPassadas::default(), perfil).unwrap();
        assert_eq!(trechos.len(), 3);
        assert_eq!(trechos[1].papel.formato, FormatoPapel::A5);
        let parte = plano_em_trechos_do_intervalo(
            &[A4; 24],
            trecho(3, 12),
            OrdemPassadas::default(),
            perfil,
        )
        .unwrap();
        assert_eq!(parte.len(), 1);
        assert_eq!(parte[0].paginas.first(), Some(&Some(3)));
        assert_eq!(parte[0].paginas.last(), Some(&Some(12)));
    }

    #[test]
    fn automatico_com_copias_faz_um_trabalho_por_copia_e_cada_uma_fecha_as_folhas() {
        let perfil = PerfilImpressora::default();
        let reparto = RepartoAutomatico::PorTrechoDePapel;
        for paginas in [4usize, 3] {
            let doc = vec![A4; paginas];
            for copias in [1usize, 2, 3] {
                let lotes = plano_automatico_copias_do_intervalo(
                    &doc,
                    None,
                    copias,
                    OrdemPassadas::default(),
                    perfil,
                    reparto,
                )
                .unwrap();
                assert_eq!(lotes.len(), copias, "{copias} cópias de {paginas}");
                let folhas_por_copia = paginas.div_ceil(2);
                for lote in &lotes {
                    assert!(lote.duas_faces);
                    let numeros: Vec<Option<usize>> =
                        lote.passada1.iter().map(|p| p.numero).collect();
                    assert_eq!(numeros, (1..=paginas).map(Some).collect::<Vec<_>>());
                    assert_eq!(lote.paginas.len(), folhas_por_copia * 2);
                }
                let folhas: usize = lotes.iter().map(|lote| lote.paginas.len() / 2).sum();
                assert_eq!(folhas, copias * folhas_por_copia);
            }
        }
        let um = plano_automatico_copias_do_intervalo(
            &[A4; 4],
            None,
            1,
            OrdemPassadas::default(),
            perfil,
            reparto,
        )
        .unwrap();
        let esperado = plano_automatico_do_intervalo(
            &[A4; 4],
            None,
            4,
            OrdemPassadas::default(),
            perfil,
            reparto,
        )
        .unwrap();
        assert_eq!(um, esperado);
    }

    #[test]
    fn copias_separadas_ficam_em_um_lote_por_copia() {
        let perfil = PerfilImpressora::default();
        let lotes = plano_copias_do_intervalo_separadas(
            &[A4; 3],
            None,
            3,
            OrdemPassadas::ParesPrimeiro,
            perfil,
        )
        .unwrap();
        assert_eq!(lotes.len(), 3);
        for (indice, lote) in lotes.iter().enumerate() {
            assert_eq!(
                lote.paginas,
                [Some(1), Some(2), Some(3), None],
                "cópia {indice}"
            );
            assert!(
                lote.copias.iter().all(|copia| *copia == indice),
                "cópia {indice}"
            );
            let (passada1, passada2) = numeros_das_passadas(lote);
            assert_eq!(passada1, [Some(2), None], "cópia {indice}");
            assert_eq!(passada2, [Some(1), Some(3)], "cópia {indice}");
        }
    }

    #[test]
    fn copias_separadas_aplicam_a_inversao_do_perfil_em_cada_copia() {
        let mut perfil = PerfilImpressora::default();
        perfil.passada1_inversa = true;
        perfil.passada2_inversa = true;
        let lotes = plano_copias_do_intervalo_separadas(
            &[A4; 4],
            None,
            2,
            OrdemPassadas::default(),
            perfil,
        )
        .unwrap();
        assert_eq!(lotes.len(), 2);
        for lote in &lotes {
            let (passada1, passada2) = numeros_das_passadas(lote);
            assert_eq!(passada1, [Some(4), Some(2)]);
            assert_eq!(passada2, [Some(3), Some(1)]);
        }
    }

    #[test]
    fn automatico_juntas_poe_as_copias_no_mesmo_trabalho() {
        let perfil = PerfilImpressora::default();
        let lotes = plano_automatico_copias_do_intervalo_juntas(
            &[A4; 4],
            None,
            3,
            OrdemPassadas::default(),
            perfil,
            RepartoAutomatico::PorTrechoDePapel,
        )
        .unwrap();
        assert_eq!(lotes.len(), 1);
        assert!(lotes[0].duas_faces);
        assert_eq!(lotes[0].paginas.len(), 12);
        assert_eq!(lotes[0].copias, [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2]);
        let numeros: Vec<Option<usize>> = lotes[0].passada1.iter().map(|p| p.numero).collect();
        let esperado: Vec<Option<usize>> = (0..3).flat_map(|_| (1..=4).map(Some)).collect();
        assert_eq!(numeros, esperado);
    }

    #[test]
    fn automatico_separadas_nao_juntam_copias_de_papeis_diferentes() {
        let perfil = PerfilImpressora::default();
        let mut misto = vec![A4; 2];
        misto.push(A5);
        let separadas = plano_automatico_copias_do_intervalo(
            &misto,
            None,
            2,
            OrdemPassadas::default(),
            perfil,
            RepartoAutomatico::PorTrechoDePapel,
        )
        .unwrap();
        let juntas = plano_automatico_copias_do_intervalo_juntas(
            &misto,
            None,
            2,
            OrdemPassadas::default(),
            perfil,
            RepartoAutomatico::PorTrechoDePapel,
        )
        .unwrap();
        assert_eq!(separadas, juntas);
        assert_eq!(juntas.len(), 4);
    }

    fn numeros_das_passadas(lote: &LotePapel) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
        (
            lote.passada1.iter().map(|p| p.numero).collect(),
            lote.passada2.iter().map(|p| p.numero).collect(),
        )
    }

    #[test]
    fn uma_copia_sai_igual_ao_plano_de_trechos() {
        let perfil = PerfilImpressora::default();
        let mut misto = vec![A4; 4];
        misto.extend([A5; 4]);
        for intervalo in [None, trecho(1, 4), trecho(5, 8)] {
            let um =
                plano_copias_do_intervalo(&misto, intervalo, 1, OrdemPassadas::default(), perfil)
                    .unwrap();
            let trechos =
                plano_em_trechos_do_intervalo(&misto, intervalo, OrdemPassadas::default(), perfil)
                    .unwrap();
            assert_eq!(um, trechos);
        }
    }

    #[test]
    fn cada_copia_de_total_impar_fecha_com_o_verso_em_branco_da_sua_ultima_folha() {
        let perfil = PerfilImpressora::default();
        let copias =
            plano_copias_do_intervalo(&[A4; 3], None, 2, OrdemPassadas::ParesPrimeiro, perfil)
                .unwrap();
        assert_eq!(copias.len(), 1);
        let paginas: Vec<Option<usize>> = copias[0].paginas.clone();
        assert_eq!(
            paginas,
            [
                Some(1),
                Some(2),
                Some(3),
                None,
                Some(1),
                Some(2),
                Some(3),
                None
            ]
        );
        let (passada1, passada2) = numeros_das_passadas(&copias[0]);
        assert_eq!(passada1, [Some(2), None, Some(2), None]);
        assert_eq!(passada2, [Some(1), Some(3), Some(1), Some(3)]);
    }

    #[test]
    fn copias_com_trechos_de_papeis_diferentes_repetem_o_trecho_inteiro() {
        let perfil = PerfilImpressora::default();
        let mut misto = vec![A4; 2];
        misto.push(A5);
        let lotes =
            plano_copias_do_intervalo(&misto, None, 2, OrdemPassadas::default(), perfil).unwrap();
        let trechos: Vec<Vec<Option<usize>>> =
            lotes.iter().map(|lote| lote.paginas.clone()).collect();
        assert_eq!(
            trechos,
            [
                vec![Some(1), Some(2)],
                vec![Some(3), None],
                vec![Some(1), Some(2)],
                vec![Some(3), None],
            ]
        );
    }

    #[test]
    fn copias_fora_de_um_a_novecentos_e_noventa_e_nove_sao_recusadas() {
        let perfil = PerfilImpressora::default();
        let doc = [A4; 2];
        let planejar = |copias| {
            plano_copias_do_intervalo(&doc, None, copias, OrdemPassadas::default(), perfil)
        };
        assert!(matches!(
            planejar(0),
            Err(ErroPapel::CopiasInvalidas { copias: 0 })
        ));
        assert!(planejar(1).is_ok());
        assert!(planejar(LIMITE_COPIAS).is_ok());
        assert!(matches!(
            planejar(LIMITE_COPIAS + 1),
            Err(ErroPapel::CopiasInvalidas { .. })
        ));
    }

    #[test]
    fn copias_que_passam_do_limite_de_paginas_sao_recusadas_sem_planejar() {
        let perfil = PerfilImpressora::default();
        let grande = vec![A4; LIMITE_PAGINAS];
        assert!(matches!(
            plano_copias_do_intervalo(&grande, None, 2, OrdemPassadas::default(), perfil),
            Err(ErroPapel::LimitePaginasExcedido)
        ));
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
    fn papeis_diferentes_vizinhos_nao_sao_recusados_no_automatico_como_no_plano_manual() {
        let lotes = automatico(&[A4, A5], None, 10, RepartoAutomatico::PorLote);
        assert_eq!(lotes.len(), 2);
        assert_eq!(lotes[0].paginas, vec![Some(1), None]);
        assert_eq!(lotes[1].paginas, vec![Some(2), None]);
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
        // A curva em U troca a aresta que lidera: o verso vai girado, como no jato.
        assert!(laser[0].passada2.iter().all(|pagina| pagina.girar_180));
        assert!(laser[0].passada1.iter().all(|pagina| !pagina.girar_180));
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
    fn papeis_diferentes_em_paginas_vizinhas_viram_trechos_cada_um_com_o_verso_em_branco() {
        let a4_pe = DimensoesPagina::new(210.0, 297.0);
        let oficio = DimensoesPagina::new(216.0, 330.0);
        for (outro, formato) in [
            (CARTA, FormatoPapel::Carta),
            (A5, FormatoPapel::A5),
            (oficio, FormatoPapel::Oficio),
        ] {
            let lotes = plano_do_modelo(&[a4_pe, outro], None, ModeloImpressora::Laser)
                .unwrap_or_else(|erro| panic!("A4 + {formato:?} foi recusado: {erro}"));
            assert_eq!(lotes.len(), 2, "A4 + {formato:?}: um trecho por papel");
            assert_eq!(lotes[0].papel.formato, FormatoPapel::A4);
            assert_eq!(lotes[0].paginas, vec![Some(1), None]);
            assert_eq!(lotes[1].papel.formato, formato);
            assert_eq!(lotes[1].paginas, vec![Some(2), None]);
        }
        // Cada página fica no lado do documento: o par A4+Carta se separa, as demais seguem.
        let lotes =
            plano_do_modelo(&[A4, A4, A4, CARTA, CARTA], None, ModeloImpressora::Laser).unwrap();
        assert_eq!(lotes[0].paginas, vec![Some(1), Some(2), Some(3), None]);
        assert_eq!(lotes[1].paginas, vec![Some(4), None, Some(5), None]);
        // Retrato e paisagem do mesmo formato ficam na mesma folha, com o aviso de orientação.
        let lotes = plano_do_modelo(&[A4, PAISAGEM], None, ModeloImpressora::Laser).unwrap();
        assert_eq!(lotes.len(), 1);
        assert_eq!(
            pares_com_orientacao_mista(&[A4, PAISAGEM]).unwrap(),
            [(1, 2)]
        );
        // O aviso de orientação segue as mesmas folhas: A4 + Carta não formam um par.
        assert!(pares_com_orientacao_mista(&[A4, CARTA]).unwrap().is_empty());
    }

    /// Documento de 10 páginas com a `pos` (base 1) de outro papel: o par que a contém se separa
    /// e cada página ganha verso em branco; o resto segue nos lados do documento.
    fn folhas_de(
        total: usize,
        pos: usize,
        outro: DimensoesPagina,
        intervalo: Option<IntervaloPaginas>,
    ) -> Vec<(Option<usize>, Option<usize>)> {
        let paginas: Vec<_> = (1..=total)
            .map(|n| if n == pos { outro } else { A4 })
            .collect();
        plano_lotes_do_intervalo(
            &paginas,
            intervalo,
            100,
            OrdemPassadas::ImparesPrimeiro,
            PerfilImpressora::default(),
        )
        .unwrap()
        .iter()
        .flat_map(|l| l.paginas.chunks(2).map(|c| (c[0], c[1])))
        .collect()
    }

    #[test]
    fn papel_misto_mantem_o_lado_e_so_o_par_diferente_se_separa() {
        let s = Some;
        let oficio = DimensoesPagina::new(216.0, 330.0);
        for outro in [CARTA, A5, oficio] {
            // Exemplo do usuário: a 5 de outro tamanho.
            assert_eq!(
                folhas_de(10, 5, outro, None),
                [
                    (s(1), s(2)),
                    (s(3), s(4)),
                    (s(5), None),
                    (s(6), None),
                    (s(7), s(8)),
                    (s(9), s(10))
                ]
            );
            // No início, no verso de um par, no fim.
            assert_eq!(
                folhas_de(4, 1, outro, None),
                [(s(1), None), (s(2), None), (s(3), s(4))]
            );
            assert_eq!(
                folhas_de(4, 4, outro, None),
                [(s(1), s(2)), (s(3), None), (s(4), None)]
            );
            // Total ímpar: a última página, de outro papel, fica sozinha; no meio, separa o par.
            assert_eq!(
                folhas_de(5, 5, outro, None),
                [(s(1), s(2)), (s(3), s(4)), (s(5), None)]
            );
            assert_eq!(
                folhas_de(5, 2, outro, None),
                [(s(1), None), (s(2), None), (s(3), s(4)), (s(5), None)]
            );
            // Com intervalo, os lados contam a partir da primeira página do trecho.
            assert_eq!(
                folhas_de(10, 5, outro, trecho(3, 8)),
                [(s(3), s(4)), (s(5), None), (s(6), None), (s(7), s(8))]
            );
            assert_eq!(
                folhas_de(10, 5, outro, trecho(2, 7)),
                [(s(2), s(3)), (s(4), None), (s(5), None), (s(6), s(7))]
            );
        }
    }

    #[test]
    fn trecho_que_corta_o_par_de_papeis_cita_os_numeros_do_documento() {
        let paginas = [A4, A4, A4, A4, A5, A5, A5, A5];
        let lotes = plano_do_modelo(&paginas, trecho(4, 7), ModeloImpressora::Laser).unwrap();
        assert_eq!(lotes.len(), 2);
        assert_eq!(lotes[0].paginas, vec![Some(4), None]);
        assert_eq!(lotes[1].paginas, vec![Some(5), None, Some(6), Some(7)]);
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
    fn pares_de_tamanhos_diferentes_saem_cada_um_na_sua_folha() {
        let paginas = [A4, A4, A4, A5, A5, A5, A4];
        assert_eq!(
            pares_separados_do_intervalo(&paginas, None),
            Ok(vec![(3, 4)])
        );
        // O trecho refaz as folhas a partir da sua primeira página, com os números do documento.
        assert_eq!(
            pares_separados_do_intervalo(&paginas, trecho(3, 6)),
            Ok(vec![(3, 4)])
        );
        assert_eq!(
            pares_separados_do_intervalo(&paginas, trecho(4, 7)),
            Ok(vec![(6, 7)])
        );
        assert_eq!(
            pares_separados_do_intervalo(&[A4, A4, A5, A5], None),
            Ok(vec![])
        );
        // Orientação diferente com o mesmo papel cabe na folha: não é par separado.
        assert_eq!(
            pares_separados_do_intervalo(&[A4, PAISAGEM], None),
            Ok(vec![])
        );
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
    fn laser_inverte_so_a_frente_gira_o_verso_e_nao_vira_o_maco() {
        let modelo = ModeloImpressora::Laser;
        let lotes = plano_documento(6, 3, OrdemPassadas::ParesPrimeiro, modelo.perfil()).unwrap();
        assert_eq!(numeros(&lotes[0].passada1), [Some(6), Some(4), Some(2)]);
        assert_eq!(numeros(&lotes[0].passada2), [Some(1), Some(3), Some(5)]);
        assert!(lotes[0].passada1.iter().all(|pagina| !pagina.girar_180));
        assert!(lotes[0].passada2.iter().all(|pagina| pagina.girar_180));
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
    fn faces_de_tamanhos_diferentes_nao_dividem_folha_e_o_impar_ganha_verso_em_branco() {
        let incompatible = [
            DimensoesPagina::new(210.0, 297.0),
            DimensoesPagina::new(148.0, 210.0),
        ];
        let lotes = lotes_por_papel(&incompatible, 10).unwrap();
        assert_eq!(lotes.len(), 2);
        assert_eq!(lotes[0].paginas, vec![Some(1), None]);
        assert_eq!(lotes[1].paginas, vec![Some(2), None]);
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
            vec![(5, 6)],
            "a página 4 fica no verso da folha 3/4, que se separa por papel; o par com orientação é o 5/6"
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

    fn lote_do(total: usize, perfil: PerfilImpressora) -> Lote {
        plano_documento(total, 10, OrdemPassadas::default(), perfil)
            .unwrap()
            .remove(0)
    }

    fn pagina(numero: usize, girar_180: bool) -> Pagina {
        Pagina {
            numero: Some(numero),
            girar_180,
            copia: 0,
        }
    }

    #[test]
    fn a_frente_isolada_tem_as_paginas_impares() {
        for total in [1usize, 2, 3, 4] {
            let lote = lote_do(total, PerfilImpressora::default());
            let frentes = frentes_do_lote(&lote);
            let esperado: Vec<Option<usize>> = (1..=total).step_by(2).map(Some).collect();
            assert_eq!(numeros(&frentes), esperado, "{total} páginas");
            assert!(frentes.iter().all(|pagina| !pagina.girar_180));
        }
    }

    #[test]
    fn o_verso_isolado_tem_as_paginas_pares_e_o_branco_final() {
        for (total, esperado) in [
            (1usize, vec![None]),
            (2, vec![Some(2)]),
            (3, vec![Some(2), None]),
            (4, vec![Some(2), Some(4)]),
        ] {
            let lote = lote_do(total, PerfilImpressora::default());
            assert_eq!(numeros(&versos_do_lote(&lote)), esperado, "{total} páginas");
        }
    }

    #[test]
    fn a_ordem_do_perfil_continua_valendo_nas_faces_isoladas() {
        // Laser: só a primeira passada vai invertida; as frentes seguem a ordem da frente.
        let laser = PerfilImpressora {
            passada1_inversa: true,
            verso_girar_180: true,
            ..Default::default()
        };
        let lote = lote_do(4, laser);
        assert_eq!(numeros(&lote.passada1), [Some(4), Some(2)]);
        let frentes = frentes_do_lote(&lote);
        assert_eq!(numeros(&frentes), [Some(3), Some(1)]);
        assert!(frentes.iter().all(|pagina| !pagina.girar_180));
        assert_eq!(numeros(&versos_do_lote(&lote)), [Some(2), Some(4)]);
        assert!(versos_do_lote(&lote).iter().all(|pagina| pagina.girar_180));

        // Jato de tinta: as duas passadas vão invertidas, e as frentes também.
        let jato = PerfilImpressora {
            passada1_inversa: true,
            passada2_inversa: true,
            verso_girar_180: true,
            ..Default::default()
        };
        let lote = lote_do(4, jato);
        assert_eq!(numeros(&lote.passada1), [Some(4), Some(2)]);
        let frentes = frentes_do_lote(&lote);
        assert_eq!(numeros(&frentes), [Some(3), Some(1)]);
        assert!(frentes.iter().all(|pagina| !pagina.girar_180));
        assert_eq!(numeros(&versos_do_lote(&lote)), [Some(4), Some(2)]);
        assert!(versos_do_lote(&lote).iter().all(|pagina| pagina.girar_180));
    }

    #[test]
    fn o_verso_isolado_recebe_o_giro_da_segunda_passada() {
        let lote = Lote {
            paginas: vec![pagina(1, false), pagina(2, false)],
            passada1: vec![pagina(2, false)],
            passada2: vec![pagina(1, true)],
            duas_faces: false,
        };
        assert!(versos_do_lote(&lote)[0].girar_180);
        assert!(!frentes_do_lote(&lote)[0].girar_180);
        let sem_giro = Lote {
            passada2: vec![pagina(1, false)],
            ..lote
        };
        assert!(!versos_do_lote(&sem_giro)[0].girar_180);
    }

    #[test]
    fn frente_fora_do_par_usa_a_segunda_passada_sem_giro() {
        let lote = Lote {
            paginas: vec![
                pagina(1, false),
                pagina(2, false),
                pagina(3, false),
                pagina(4, false),
            ],
            passada1: vec![pagina(1, false), pagina(3, false)],
            passada2: vec![pagina(2, true), pagina(4, true)],
            duas_faces: false,
        };
        let frentes = frentes_do_lote(&lote);
        assert_eq!(numeros(&frentes), [Some(2), Some(4)]);
        assert!(frentes.iter().all(|pagina| !pagina.girar_180));
    }

    #[test]
    fn lote_de_duas_faces_ja_traz_a_frente_inteira_e_nao_tem_verso_separado() {
        let lote = Lote {
            paginas: vec![
                pagina(1, false),
                pagina(2, false),
                pagina(3, false),
                pagina(4, false),
            ],
            passada1: vec![
                pagina(1, false),
                pagina(2, false),
                pagina(3, false),
                pagina(4, false),
            ],
            passada2: Vec::new(),
            duas_faces: true,
        };
        assert_eq!(
            numeros(&frentes_do_lote(&lote)),
            [Some(1), Some(2), Some(3), Some(4)]
        );
        assert!(versos_do_lote(&lote).is_empty());
    }
}
