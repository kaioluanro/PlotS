//! Marca visual das páginas de teste: número da página e orientação, lidos só do desenho.
//!
//! O PlotS envia cada página como imagem, então o texto não volta. A marca é um quadrado de 11 por
//! 11 módulos: moldura preta, um anel branco e, dentro, 7 por 7 módulos. Dos quatro cantos internos
//! só um é preto, e ele diz a rotação. Os outros módulos guardam o número, uma soma de conferência
//! e a cópia invertida dos primeiros bits. Ela não depende de escala, de posição nem de DPI: o
//! leitor acha a moldura pela tinta e lê os módulos em relação a ela.

use std::fmt;

/// Módulos de cada lado da marca.
pub const MODULOS: usize = 11;
/// Maior número de página que cabe na marca.
pub const MAIOR_PAGINA: u32 = u16::MAX as u32;
/// Menor módulo que dá para ler, em pontos do bitmap.
const MENOR_MODULO: usize = 3;
/// Maior bitmap aceito, em pontos.
const LIMITE_PONTOS: usize = 100_000_000;
/// Abaixo disto o ponto é tinta.
const LIMIAR_DE_TINTA: u8 = 128;
/// Até esta fração de tinta, a página é considerada em branco: uma em dez mil. O PlotS põe um ponto
/// discreto na página em branco, para o driver não a descartar.
const TINTA_DE_PAGINA_EM_BRANCO: usize = 10_000;
/// Mais candidatos que isto não é uma página de teste.
const LIMITE_MANCHAS: usize = 20_000;
/// A soma de conferência começa aqui, para uma mancha sem sentido não passar com zeros.
const INICIO_DA_SOMA: u8 = 0x5A;
const BITS_DE_NUMERO: usize = 16;
const BITS_DE_SOMA: usize = 8;
/// Módulos internos menos os quatro cantos.
const BITS_NA_MARCA: usize = 45;

/// Os módulos, por linha e coluna. Verdadeiro é preto.
pub type Matriz = [[bool; MODULOS]; MODULOS];

/// O que a marca disse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarcaLida {
    pub pagina: u32,
    /// Quanto o topo da marca está girado no desenho, no sentido horário: 0, 90, 180 ou 270.
    pub rotacao: u16,
}

/// Por que uma página não deu leitura. Nunca há palpite: sem marca certa, é erro.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErroMarca {
    /// As medidas não fecham com os dados, ou passam do limite.
    BitmapInvalido,
    /// Quase sem tinta.
    PaginaEmBranco,
    /// Tem tinta, mas nenhuma moldura.
    SemMarca,
    /// Há moldura, mas o conteúdo não confere.
    MarcaInvalida,
    /// A moldura é pequena demais para ler com segurança.
    MarcaPequena { modulo: usize },
    /// Duas marcas válidas dizem coisas diferentes.
    Ambigua { paginas: (u32, u32) },
    /// Manchas demais para uma página de teste.
    TintaDemais,
}

impl fmt::Display for ErroMarca {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BitmapInvalido => f.write_str("o desenho da página veio com medidas inválidas"),
            Self::PaginaEmBranco => f.write_str("a página está em branco"),
            Self::SemMarca => f.write_str("a página tem tinta, mas nenhuma marca"),
            Self::MarcaInvalida => {
                f.write_str("a página tem uma moldura de marca, mas o conteúdo não confere")
            }
            Self::MarcaPequena { modulo } => write!(
                f,
                "a marca da página tem módulos de {modulo} pontos, pequena demais para ler"
            ),
            Self::Ambigua { paginas: (a, b) } => {
                write!(f, "a página tem duas marcas diferentes: {a} e {b}")
            }
            Self::TintaDemais => f.write_str("a página tem manchas demais para ser de teste"),
        }
    }
}

impl std::error::Error for ErroMarca {}

/// Soma de conferência do número.
fn soma_do_numero(numero: u16) -> u8 {
    numero
        .to_be_bytes()
        .iter()
        .fold(INICIO_DA_SOMA, |soma, byte| {
            (0..8).fold(soma ^ byte, |crc, _| {
                if crc & 0x80 != 0 {
                    (crc << 1) ^ 0x07
                } else {
                    crc << 1
                }
            })
        })
}

/// Os bits da marca: número, soma e a cópia invertida dos primeiros.
fn bits_da_marca(numero: u16) -> [bool; BITS_NA_MARCA] {
    let mut bits = [false; BITS_NA_MARCA];
    let soma = soma_do_numero(numero);
    let dados = (0..BITS_DE_NUMERO)
        .map(|i| numero >> (BITS_DE_NUMERO - 1 - i) & 1 == 1)
        .chain((0..BITS_DE_SOMA).map(|i| soma >> (BITS_DE_SOMA - 1 - i) & 1 == 1));
    let mut quantos = 0;
    for (lugar, bit) in bits.iter_mut().zip(dados) {
        *lugar = bit;
        quantos += 1;
    }
    let (primeiros, resto) = bits.split_at_mut(quantos);
    for (lugar, bit) in resto.iter_mut().zip(primeiros.iter()) {
        *lugar = !*bit;
    }
    bits
}

/// Módulos internos que guardam bits, em ordem de leitura.
fn lugares_dos_bits() -> impl Iterator<Item = (usize, usize)> {
    (2..=8usize)
        .flat_map(|linha| (2..=8usize).map(move |coluna| (linha, coluna)))
        .filter(|(linha, coluna)| !(matches!(*linha, 2 | 8) && matches!(*coluna, 2 | 8)))
}

/// O desenho da marca de uma página, em pé. Número fora do que cabe não tem marca.
pub fn matriz_da_marca(pagina: u32) -> Option<Matriz> {
    if pagina == 0 {
        return None;
    }
    let numero = u16::try_from(pagina).ok()?;
    let mut matriz = [[false; MODULOS]; MODULOS];
    for (l, linha) in matriz.iter_mut().enumerate() {
        for (c, modulo) in linha.iter_mut().enumerate() {
            *modulo = matches!(l, 0 | 10) || matches!(c, 0 | 10);
        }
    }
    *matriz.get_mut(2)?.get_mut(2)? = true;
    for ((l, c), bit) in lugares_dos_bits().zip(bits_da_marca(numero)) {
        *matriz.get_mut(l)?.get_mut(c)? = bit;
    }
    Some(matriz)
}

fn girar_horario(m: &Matriz) -> Matriz {
    let mut girada = [[false; MODULOS]; MODULOS];
    for (l, linha) in m.iter().enumerate() {
        for (c, modulo) in linha.iter().enumerate() {
            if let Some(lugar) = girada
                .get_mut(c)
                .and_then(|nova| nova.get_mut(MODULOS - 1 - l))
            {
                *lugar = *modulo;
            }
        }
    }
    girada
}

fn modulo(m: &Matriz, linha: usize, coluna: usize) -> bool {
    m.get(linha)
        .and_then(|l| l.get(coluna))
        .copied()
        .unwrap_or(false)
}

/// Lê uma matriz como ela veio do desenho. Só devolve o que confere inteiro.
pub fn ler_matriz(lida: &Matriz) -> Option<MarcaLida> {
    for l in 0..MODULOS {
        for c in 0..MODULOS {
            let moldura = matches!(l, 0 | 10) || matches!(c, 0 | 10);
            let anel = !moldura && (matches!(l, 1 | 9) || matches!(c, 1 | 9));
            if (moldura && !modulo(lida, l, c)) || (anel && modulo(lida, l, c)) {
                return None;
            }
        }
    }
    let cantos = [(2, 2, 0u16), (2, 8, 90), (8, 8, 180), (8, 2, 270)];
    let mut pretos = cantos.iter().filter(|(l, c, _)| modulo(lida, *l, *c));
    let (_, _, rotacao) = pretos.next()?;
    if pretos.next().is_some() {
        return None;
    }
    // Desfaz a rotação: girar no sentido horário o que falta para completar a volta.
    let mut em_pe = *lida;
    for _ in 0..((360 - *rotacao) % 360) / 90 {
        em_pe = girar_horario(&em_pe);
    }
    let mut lidos = [false; BITS_NA_MARCA];
    for (lugar, (l, c)) in lidos.iter_mut().zip(lugares_dos_bits()) {
        *lugar = modulo(&em_pe, l, c);
    }
    let numero = lidos
        .iter()
        .take(BITS_DE_NUMERO)
        .fold(0u16, |n, bit| (n << 1) | u16::from(*bit));
    if numero == 0 || bits_da_marca(numero) != lidos {
        return None;
    }
    Some(MarcaLida {
        pagina: u32::from(numero),
        rotacao: *rotacao,
    })
}

/// Caixa de uma mancha de tinta.
#[derive(Clone, Copy, Debug)]
struct Caixa {
    esquerda: usize,
    topo: usize,
    direita: usize,
    base: usize,
}

impl Caixa {
    const fn largura(self) -> usize {
        self.direita.saturating_sub(self.esquerda).saturating_add(1)
    }
    const fn altura(self) -> usize {
        self.base.saturating_sub(self.topo).saturating_add(1)
    }
    /// A moldura é quadrada, com alguns pontos de folga por causa do arredondamento.
    fn quadrada(self) -> bool {
        let (a, b) = (self.largura(), self.altura());
        a.abs_diff(b) <= (a.max(b) / 16).max(1)
    }
}

/// Bitmap em tons de cinza, de cima para baixo e da esquerda para a direita.
#[derive(Clone, Copy, Debug)]
pub struct Cinza<'a> {
    pub largura: usize,
    pub altura: usize,
    pub pontos: &'a [u8],
}

impl Cinza<'_> {
    fn tinta(&self, x: usize, y: usize) -> bool {
        y.checked_mul(self.largura)
            .and_then(|linha| linha.checked_add(x))
            .and_then(|indice| self.pontos.get(indice))
            .is_some_and(|ponto| *ponto < LIMIAR_DE_TINTA)
    }
}

/// Lê a marca de um bitmap. Entrada hostil dá erro, nunca pânico.
pub fn ler_marca(cinza: Cinza<'_>) -> Result<MarcaLida, ErroMarca> {
    let total = cinza
        .largura
        .checked_mul(cinza.altura)
        .filter(|total| *total > 0 && *total <= LIMITE_PONTOS && *total == cinza.pontos.len())
        .ok_or(ErroMarca::BitmapInvalido)?;
    let tinta = cinza
        .pontos
        .iter()
        .filter(|ponto| **ponto < LIMIAR_DE_TINTA)
        .count();
    if tinta <= total / TINTA_DE_PAGINA_EM_BRANCO {
        return Err(ErroMarca::PaginaEmBranco);
    }
    let caixas = manchas(cinza)?;
    let mut achada: Option<MarcaLida> = None;
    let mut moldura_sem_leitura = false;
    let mut menor_modulo = None;
    for caixa in caixas.iter().filter(|caixa| caixa.quadrada()) {
        let lado = caixa.largura().min(caixa.altura());
        if lado < MODULOS {
            continue;
        }
        if !moldura_fechada(cinza, *caixa) {
            continue;
        }
        let modulo = lado / MODULOS;
        if modulo < MENOR_MODULO {
            menor_modulo = Some(modulo);
            continue;
        }
        match ler_matriz(&amostrar(cinza, *caixa)) {
            Some(marca) => match achada {
                Some(outra) if outra != marca => {
                    return Err(ErroMarca::Ambigua {
                        paginas: (outra.pagina, marca.pagina),
                    })
                }
                _ => achada = Some(marca),
            },
            None => moldura_sem_leitura = true,
        }
    }
    match (achada, moldura_sem_leitura, menor_modulo) {
        (Some(marca), _, _) => Ok(marca),
        (None, true, _) => Err(ErroMarca::MarcaInvalida),
        (None, false, Some(modulo)) => Err(ErroMarca::MarcaPequena { modulo }),
        (None, false, None) => Err(ErroMarca::SemMarca),
    }
}

/// As quatro laterais da caixa têm tinta na maior parte: é o que separa uma moldura de um desenho
/// qualquer antes de amostrar. A folga cobre os cantos e o serrilhado.
fn moldura_fechada(cinza: Cinza<'_>, caixa: Caixa) -> bool {
    let margem = caixa.largura().min(caixa.altura()) / (MODULOS * 2);
    let horizontal = |y: usize| {
        let cheios = (caixa.esquerda..=caixa.direita)
            .filter(|x| cinza.tinta(*x, y))
            .count();
        cheios.saturating_mul(10) >= caixa.largura().saturating_mul(9)
    };
    let vertical = |x: usize| {
        let cheios = (caixa.topo..=caixa.base)
            .filter(|y| cinza.tinta(x, *y))
            .count();
        cheios.saturating_mul(10) >= caixa.altura().saturating_mul(9)
    };
    horizontal(caixa.topo.saturating_add(margem))
        && horizontal(caixa.base.saturating_sub(margem))
        && vertical(caixa.esquerda.saturating_add(margem))
        && vertical(caixa.direita.saturating_sub(margem))
}

/// Lê cada módulo pela maioria de nove pontos em volta do centro.
fn amostrar(cinza: Cinza<'_>, caixa: Caixa) -> Matriz {
    let mut matriz = [[false; MODULOS]; MODULOS];
    let (largura, altura) = (caixa.largura(), caixa.altura());
    let meio = |lado: usize, n: usize| {
        n.saturating_mul(2).saturating_add(1).saturating_mul(lado) / (MODULOS * 2)
    };
    let passo = (largura.min(altura) / MODULOS / 4).max(1);
    for (l, linha) in matriz.iter_mut().enumerate() {
        for (c, lugar) in linha.iter_mut().enumerate() {
            let x = caixa.esquerda.saturating_add(meio(largura, c));
            let y = caixa.topo.saturating_add(meio(altura, l));
            let mut pretos = 0;
            for dy in [0, passo, passo.saturating_mul(2)] {
                for dx in [0, passo, passo.saturating_mul(2)] {
                    let px = x.saturating_add(dx).saturating_sub(passo);
                    let py = y.saturating_add(dy).saturating_sub(passo);
                    if cinza.tinta(px, py) {
                        pretos += 1;
                    }
                }
            }
            *lugar = pretos >= 5;
        }
    }
    matriz
}

/// Caixas das manchas de tinta ligadas. A pilha é explícita: o tamanho da página não vira recursão.
fn manchas(cinza: Cinza<'_>) -> Result<Vec<Caixa>, ErroMarca> {
    let mut visto = vec![false; cinza.pontos.len()];
    let mut caixas = Vec::new();
    let mut pilha = Vec::new();
    for y in 0..cinza.altura {
        for x in 0..cinza.largura {
            let indice = y.saturating_mul(cinza.largura).saturating_add(x);
            if !cinza.tinta(x, y) || visto.get(indice).copied().unwrap_or(true) {
                continue;
            }
            if caixas.len() >= LIMITE_MANCHAS {
                return Err(ErroMarca::TintaDemais);
            }
            let mut caixa = Caixa {
                esquerda: x,
                topo: y,
                direita: x,
                base: y,
            };
            if let Some(lugar) = visto.get_mut(indice) {
                *lugar = true;
            }
            pilha.push((x, y));
            while let Some((px, py)) = pilha.pop() {
                caixa.esquerda = caixa.esquerda.min(px);
                caixa.direita = caixa.direita.max(px);
                caixa.topo = caixa.topo.min(py);
                caixa.base = caixa.base.max(py);
                let vizinhos = [
                    (px.checked_sub(1), Some(py)),
                    (px.checked_add(1), Some(py)),
                    (Some(px), py.checked_sub(1)),
                    (Some(px), py.checked_add(1)),
                ];
                for vizinho in vizinhos {
                    let (Some(vx), Some(vy)) = vizinho else {
                        continue;
                    };
                    if vx >= cinza.largura || vy >= cinza.altura || !cinza.tinta(vx, vy) {
                        continue;
                    }
                    let lugar = vy.saturating_mul(cinza.largura).saturating_add(vx);
                    if let Some(marcado) = visto.get_mut(lugar) {
                        if !*marcado {
                            *marcado = true;
                            pilha.push((vx, vy));
                        }
                    }
                }
            }
            caixas.push(caixa);
        }
    }
    Ok(caixas)
}

#[cfg(test)]
mod testes {
    use super::*;

    struct Tela {
        largura: usize,
        altura: usize,
        pontos: Vec<u8>,
    }

    impl Tela {
        fn branca(largura: usize, altura: usize) -> Self {
            Self {
                largura,
                altura,
                pontos: vec![255; largura * altura],
            }
        }

        fn retangulo(&mut self, x: usize, y: usize, l: usize, a: usize, tom: u8) {
            for py in y..(y + a).min(self.altura) {
                for px in x..(x + l).min(self.largura) {
                    self.pontos[py * self.largura + px] = tom;
                }
            }
        }

        fn marca(&mut self, matriz: &Matriz, x: usize, y: usize, modulo: usize) {
            for (l, linha) in matriz.iter().enumerate() {
                for (c, preto) in linha.iter().enumerate() {
                    if *preto {
                        self.retangulo(x + c * modulo, y + l * modulo, modulo, modulo, 0);
                    }
                }
            }
        }

        fn cinza(&self) -> Cinza<'_> {
            Cinza {
                largura: self.largura,
                altura: self.altura,
                pontos: &self.pontos,
            }
        }

        /// A mesma tela girada 90° no sentido horário.
        fn girada(&self) -> Self {
            let mut nova = Self::branca(self.altura, self.largura);
            for y in 0..self.altura {
                for x in 0..self.largura {
                    let (nx, ny) = (self.altura - 1 - y, x);
                    nova.pontos[ny * nova.largura + nx] = self.pontos[y * self.largura + x];
                }
            }
            nova
        }
    }

    fn pagina_de_teste(numero: u32, modulo: usize) -> Tela {
        let mut tela = Tela::branca(modulo * 60, modulo * 85);
        // Um número grande e uma seta: tinta que não é a marca, e não deve atrapalhar.
        tela.retangulo(modulo * 20, modulo * 20, modulo * 20, modulo * 4, 0);
        tela.retangulo(modulo * 28, modulo * 5, modulo * 4, modulo * 14, 0);
        tela.marca(
            &matriz_da_marca(numero).unwrap(),
            modulo * 25,
            modulo * 60,
            modulo,
        );
        tela
    }

    #[test]
    fn le_o_numero_e_a_rotacao_em_todos_os_giros() {
        for numero in [1, 2, 9, 10, 11, 24, 101, 4_096, 65_535] {
            let mut tela = pagina_de_teste(numero, 8);
            for rotacao in [0, 90, 180, 270] {
                assert_eq!(
                    ler_marca(tela.cinza()),
                    Ok(MarcaLida {
                        pagina: numero,
                        rotacao
                    }),
                    "página {numero}, giro {rotacao}"
                );
                tela = tela.girada();
            }
        }
    }

    #[test]
    fn le_em_qualquer_escala_e_posicao() {
        for modulo in [3, 4, 5, 7, 16, 40] {
            let mut tela = Tela::branca(40 * modulo + 13, 50 * modulo + 7);
            tela.marca(
                &matriz_da_marca(77).unwrap(),
                5 * modulo + 3,
                11 * modulo + 1,
                modulo,
            );
            assert_eq!(
                ler_marca(tela.cinza()),
                Ok(MarcaLida {
                    pagina: 77,
                    rotacao: 0
                }),
                "módulo {modulo}"
            );
        }
    }

    #[test]
    fn serrilhado_e_tons_nao_mudam_a_leitura() {
        let mut tela = pagina_de_teste(33, 6);
        // Borda clara em volta de cada ponto preto, como sai de uma imagem reamostrada.
        let copia = tela.pontos.clone();
        for (i, ponto) in copia.iter().enumerate() {
            if *ponto == 0 && i + 1 < copia.len() && copia[i + 1] == 255 {
                tela.pontos[i + 1] = 180;
            }
        }
        for ponto in &mut tela.pontos {
            if *ponto == 0 {
                *ponto = 60;
            }
        }
        assert_eq!(ler_marca(tela.cinza()).map(|m| m.pagina), Ok(33));
    }

    #[test]
    fn em_branco_e_sem_marca_sao_erros_diferentes() {
        let branca = Tela::branca(600, 850);
        assert_eq!(ler_marca(branca.cinza()), Err(ErroMarca::PaginaEmBranco));
        let mut com_ponto = Tela::branca(600, 850);
        com_ponto.retangulo(10, 10, 2, 2, 0);
        assert_eq!(ler_marca(com_ponto.cinza()), Err(ErroMarca::PaginaEmBranco));
        let mut sem_marca = Tela::branca(600, 850);
        sem_marca.retangulo(100, 100, 300, 40, 0);
        assert_eq!(ler_marca(sem_marca.cinza()), Err(ErroMarca::SemMarca));
    }

    #[test]
    fn marca_adulterada_nunca_vira_palpite() {
        let base = matriz_da_marca(24).unwrap();
        for (l, c) in lugares_dos_bits() {
            let mut errada = base;
            errada[l][c] = !errada[l][c];
            let mut tela = Tela::branca(300, 300);
            tela.marca(&errada, 40, 40, 10);
            assert_eq!(
                ler_marca(tela.cinza()),
                Err(ErroMarca::MarcaInvalida),
                "bit em {l},{c}"
            );
        }
    }

    #[test]
    fn marca_espelhada_nao_passa() {
        let base = matriz_da_marca(24).unwrap();
        let mut espelhada = base;
        for linha in &mut espelhada {
            linha.reverse();
        }
        assert_eq!(ler_matriz(&espelhada), None);
    }

    #[test]
    fn moldura_cheia_nao_passa_por_marca() {
        let mut tela = Tela::branca(400, 400);
        tela.retangulo(50, 50, 110, 110, 0);
        assert_eq!(ler_marca(tela.cinza()), Err(ErroMarca::MarcaInvalida));
    }

    #[test]
    fn marca_pequena_demais_e_erro_claro() {
        let mut tela = Tela::branca(800, 800);
        tela.marca(&matriz_da_marca(5).unwrap(), 100, 100, 2);
        assert_eq!(
            ler_marca(tela.cinza()),
            Err(ErroMarca::MarcaPequena { modulo: 2 })
        );
    }

    #[test]
    fn duas_marcas_diferentes_sao_ambiguas() {
        let mut tela = Tela::branca(800, 800);
        tela.marca(&matriz_da_marca(5).unwrap(), 50, 50, 10);
        tela.marca(&matriz_da_marca(6).unwrap(), 400, 400, 10);
        assert_eq!(
            ler_marca(tela.cinza()),
            Err(ErroMarca::Ambigua { paginas: (5, 6) })
        );
    }

    #[test]
    fn medidas_hostis_nao_travam() {
        let pontos = [255u8; 16];
        for (largura, altura) in [
            (0, 0),
            (5, 5),
            (usize::MAX, 2),
            (2, usize::MAX),
            (usize::MAX, usize::MAX),
        ] {
            assert_eq!(
                ler_marca(Cinza {
                    largura,
                    altura,
                    pontos: &pontos
                }),
                Err(ErroMarca::BitmapInvalido)
            );
        }
    }

    #[test]
    fn xadrez_de_pontos_nao_estoura_memoria_nem_acha_marca() {
        let mut tela = Tela::branca(1000, 1000);
        for y in (0..1000).step_by(2) {
            for x in (0..1000).step_by(2) {
                tela.pontos[y * 1000 + x] = 0;
            }
        }
        assert_eq!(ler_marca(tela.cinza()), Err(ErroMarca::TintaDemais));
    }

    #[test]
    fn numero_fora_do_que_cabe_nao_tem_marca() {
        assert!(matriz_da_marca(0).is_none());
        assert!(matriz_da_marca(MAIOR_PAGINA + 1).is_none());
        assert!(matriz_da_marca(MAIOR_PAGINA).is_some());
    }
}
