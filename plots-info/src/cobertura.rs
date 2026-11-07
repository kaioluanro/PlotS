//! Cobertura média de cada canal de tinta, em percentual, a partir de pixels BGRA ou de um PAM
//! CMYK.
//!
//! A conversão de RGB para CMYK é a simples: o preto é o que sobra do maior componente e os
//! outros canais dividem o restante. "Cinzas só com preto" torna explícito que um pixel com os
//! três componentes iguais usa apenas o canal preto.

use thiserror::Error;

/// Quantidade de canais de tinta: ciano, magenta, amarelo e preto.
pub const CANAIS: usize = 4;

/// Cobertura média de uma página, em percentual de 0 a 100, por canal.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Cobertura {
    pub c: f64,
    pub m: f64,
    pub y: f64,
    pub k: f64,
}

impl Cobertura {
    pub fn new(c: f64, m: f64, y: f64, k: f64) -> Self {
        Self { c, m, y, k }
    }

    /// Soma dos quatro canais, em pontos percentuais.
    pub fn total(&self) -> f64 {
        self.c + self.m + self.y + self.k
    }

    /// Os quatro canais na ordem C, M, Y, K.
    pub fn valores(&self) -> [f64; CANAIS] {
        [self.c, self.m, self.y, self.k]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErroCobertura {
    #[error("a largura e a altura da página precisam ser positivas")]
    DimensaoInvalida,
    #[error("o passo {passo} é menor que a linha de {minimo} bytes")]
    PassoInvalido { passo: usize, minimo: usize },
    #[error("os pixels têm {recebido} bytes e a página precisa de {esperado}")]
    DadosCurtos { esperado: usize, recebido: usize },
    #[error("os dados da imagem passam do tamanho representável")]
    Overflow,
    #[error("o cabeçalho do PAM está incompleto ou malformado")]
    CabecalhoInvalido,
    #[error("o arquivo não começa com o marcador P7 de um PAM")]
    MagicInvalido,
    #[error("a profundidade {profundidade} não é suportada; o PAM precisa ter DEPTH 4")]
    ProfundidadeNaoSuportada { profundidade: u32 },
    #[error("o valor máximo {maxval} não é suportado; o PAM precisa ter MAXVAL 255")]
    MaxvalNaoSuportado { maxval: u32 },
    #[error("o PAM tem {recebido} bytes de dados e precisa de {esperado}")]
    TamanhoInvalido { esperado: usize, recebido: usize },
}

/// Cobertura média de uma página descrita em BGRA de 8 bits, com um passo de linha próprio.
///
/// O passo é a distância entre o começo de duas linhas; a última linha pode terminar antes do
/// passo completo. Canais fora dos pixels da página, como o preenchimento no fim da linha, não
/// entram na média.
pub fn cobertura_de_bgra(
    pixels: &[u8],
    largura: usize,
    altura: usize,
    passo: usize,
    cinzas_so_preto: bool,
) -> Result<Cobertura, ErroCobertura> {
    if largura == 0 || altura == 0 {
        return Err(ErroCobertura::DimensaoInvalida);
    }
    let linha = largura.checked_mul(4).ok_or(ErroCobertura::Overflow)?;
    if passo < linha {
        return Err(ErroCobertura::PassoInvalido {
            passo,
            minimo: linha,
        });
    }
    let ultima_linha = passo
        .checked_mul(altura - 1)
        .and_then(|inicio| inicio.checked_add(linha))
        .ok_or(ErroCobertura::Overflow)?;
    if pixels.len() < ultima_linha {
        return Err(ErroCobertura::DadosCurtos {
            esperado: ultima_linha,
            recebido: pixels.len(),
        });
    }
    let total_de_pixels = largura.checked_mul(altura).ok_or(ErroCobertura::Overflow)?;

    let mut soma = [0.0f64; CANAIS];
    for y in 0..altura {
        let base = y.checked_mul(passo).ok_or(ErroCobertura::Overflow)?;
        for x in 0..largura {
            let inicio = x
                .checked_mul(4)
                .and_then(|deslocamento| base.checked_add(deslocamento))
                .ok_or(ErroCobertura::Overflow)?;
            let fim = inicio.checked_add(4).ok_or(ErroCobertura::Overflow)?;
            let pixel = pixels.get(inicio..fim).ok_or(ErroCobertura::DadosCurtos {
                esperado: ultima_linha,
                recebido: pixels.len(),
            })?;
            let canais = cmyk_do_pixel(pixel[2], pixel[1], pixel[0], cinzas_so_preto);
            for (acumulado, valor) in soma.iter_mut().zip(canais) {
                *acumulado += valor;
            }
        }
    }

    let divisor = total_de_pixels as f64;
    Ok(Cobertura {
        c: percentual(soma[0], divisor),
        m: percentual(soma[1], divisor),
        y: percentual(soma[2], divisor),
        k: percentual(soma[3], divisor),
    })
}

fn percentual(soma: f64, divisor: f64) -> f64 {
    if divisor <= 0.0 {
        return 0.0;
    }
    (soma / divisor * 100.0).clamp(0.0, 100.0)
}

fn cmyk_do_pixel(r: u8, g: u8, b: u8, cinzas_so_preto: bool) -> [f64; CANAIS] {
    let r = f64::from(r) / 255.0;
    let g = f64::from(g) / 255.0;
    let b = f64::from(b) / 255.0;
    let maior = r.max(g).max(b);
    let k = 1.0 - maior;
    if cinzas_so_preto && r == g && g == b {
        return [0.0, 0.0, 0.0, k];
    }
    if k >= 1.0 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let restante = 1.0 - k;
    [
        (1.0 - r - k) / restante,
        (1.0 - g - k) / restante,
        (1.0 - b - k) / restante,
        k,
    ]
}

/// Cobertura média de um PAM CMYK gerado pelo conversor: `P7`, `DEPTH 4` e `MAXVAL 255`.
///
/// Os quatro bytes de cada pixel são C, M, Y e K, e a cobertura é a média de cada byte dividida
/// pelo valor máximo. Um PAM com outra profundidade, outro `MAXVAL` ou dados de tamanho diferente
/// é recusado antes de qualquer média.
pub fn cobertura_de_pam(bytes: &[u8]) -> Result<Cobertura, ErroCobertura> {
    let Some((primeira, mut posicao)) = proxima_linha(bytes, 0) else {
        return Err(ErroCobertura::CabecalhoInvalido);
    };
    if texto(primeira)?.trim() != "P7" {
        return Err(ErroCobertura::MagicInvalido);
    }

    let mut largura = None;
    let mut altura = None;
    let mut profundidade = None;
    let mut maxval = None;
    let inicio_dos_dados;
    loop {
        let Some((linha, proximo)) = proxima_linha(bytes, posicao) else {
            return Err(ErroCobertura::CabecalhoInvalido);
        };
        posicao = proximo;
        let linha = texto(linha)?.trim();
        if linha.is_empty() || linha.starts_with('#') {
            if posicao >= bytes.len() {
                return Err(ErroCobertura::CabecalhoInvalido);
            }
            continue;
        }
        if linha == "ENDHDR" {
            inicio_dos_dados = posicao;
            break;
        }
        let mut partes = linha.split_ascii_whitespace();
        let chave = partes.next().ok_or(ErroCobertura::CabecalhoInvalido)?;
        let valor = partes.next().ok_or(ErroCobertura::CabecalhoInvalido)?;
        match chave {
            "WIDTH" => largura = Some(inteiro(valor)?),
            "HEIGHT" => altura = Some(inteiro(valor)?),
            "DEPTH" => profundidade = Some(inteiro_u32(valor)?),
            "MAXVAL" => maxval = Some(inteiro_u32(valor)?),
            _ => {}
        }
    }

    let largura = largura.ok_or(ErroCobertura::CabecalhoInvalido)?;
    let altura = altura.ok_or(ErroCobertura::CabecalhoInvalido)?;
    if largura == 0 || altura == 0 {
        return Err(ErroCobertura::DimensaoInvalida);
    }
    let profundidade = profundidade.ok_or(ErroCobertura::CabecalhoInvalido)?;
    if profundidade != 4 {
        return Err(ErroCobertura::ProfundidadeNaoSuportada { profundidade });
    }
    let maxval = maxval.ok_or(ErroCobertura::CabecalhoInvalido)?;
    if maxval != 255 {
        return Err(ErroCobertura::MaxvalNaoSuportado { maxval });
    }

    let pixels = largura.checked_mul(altura).ok_or(ErroCobertura::Overflow)?;
    let esperado = pixels.checked_mul(4).ok_or(ErroCobertura::Overflow)?;
    let dados = bytes
        .get(inicio_dos_dados..)
        .ok_or(ErroCobertura::CabecalhoInvalido)?;
    if dados.len() != esperado {
        return Err(ErroCobertura::TamanhoInvalido {
            esperado,
            recebido: dados.len(),
        });
    }

    let mut soma = [0u128; CANAIS];
    for pixel in dados.chunks_exact(4) {
        for (acumulado, valor) in soma.iter_mut().zip(pixel) {
            *acumulado += u128::from(*valor);
        }
    }
    let divisor = pixels as f64 * 255.0;
    Ok(Cobertura {
        c: percentual(soma[0] as f64, divisor),
        m: percentual(soma[1] as f64, divisor),
        y: percentual(soma[2] as f64, divisor),
        k: percentual(soma[3] as f64, divisor),
    })
}

fn proxima_linha(bytes: &[u8], inicio: usize) -> Option<(&[u8], usize)> {
    let resto = bytes.get(inicio..)?;
    if resto.is_empty() {
        return None;
    }
    match resto.iter().position(|byte| *byte == b'\n') {
        Some(posicao) => Some((&resto[..posicao], inicio + posicao + 1)),
        None => Some((resto, bytes.len())),
    }
}

fn texto(linha: &[u8]) -> Result<&str, ErroCobertura> {
    std::str::from_utf8(linha).map_err(|_| ErroCobertura::CabecalhoInvalido)
}

fn inteiro(valor: &str) -> Result<usize, ErroCobertura> {
    valor
        .parse::<usize>()
        .map_err(|_| ErroCobertura::CabecalhoInvalido)
}

fn inteiro_u32(valor: &str) -> Result<u32, ErroCobertura> {
    valor
        .parse::<u32>()
        .map_err(|_| ErroCobertura::CabecalhoInvalido)
}
