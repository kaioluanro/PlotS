//! Contas de tamanho e de posição de uma página na fila, sem tocar no sistema.
//!
//! A página sai sempre em tamanho real: os pixels vêm dos pontos do PDF e da resolução da
//! fila. O alinhamento entre frente e verso depende disso, então nada é ampliado para caber
//! na área imprimível. O que passar dela é cortado pelo próprio dispositivo.

use crate::erro::ErroEnvio;
use crate::papel::medida_nominal;
use plots_core::{DimensoesPagina, Papel, TOLERANCIA_PAPEL_MM};

/// Maior bitmap aceito para uma página (1 GiB). O pico de memória é o dobro, porque a
/// biblioteca devolve uma cópia dos pixels.
pub const LIMITE_BYTES_PAGINA: u64 = 1024 * 1024 * 1024;
/// Cada pixel usa 4 bytes (BGRA).
const BYTES_POR_PIXEL: u64 = 4;
const PONTOS_POR_POLEGADA: f64 = 72.0;
const MM_POR_POLEGADA: f64 = 25.4;

/// Medidas da fila, vindas de `GetDeviceCaps`. Os valores são tratados como hostis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dispositivo {
    pub dpi_x: i32,
    pub dpi_y: i32,
    /// Largura e altura do papel inteiro, em pixels.
    pub papel_largura: i32,
    pub papel_altura: i32,
    /// Distância da borda do papel até a origem da área imprimível, em pixels.
    pub margem_x: i32,
    pub margem_y: i32,
    /// Largura e altura da área imprimível, em pixels.
    pub area_largura: i32,
    pub area_altura: i32,
}

impl Dispositivo {
    /// Recusa medidas nulas ou negativas, que deixariam as contas sem sentido.
    pub fn validar(&self) -> Result<(), ErroEnvio> {
        let positivos = [
            self.dpi_x,
            self.dpi_y,
            self.papel_largura,
            self.papel_altura,
            self.area_largura,
            self.area_altura,
        ];
        if positivos.iter().any(|valor| *valor <= 0) || self.margem_x < 0 || self.margem_y < 0 {
            return Err(ErroEnvio::Resposta("medidas da fila inválidas"));
        }
        Ok(())
    }

    /// Tamanho do papel em milímetros, como o driver o informa.
    fn papel_mm(&self) -> (f64, f64) {
        (
            f64::from(self.papel_largura) / f64::from(self.dpi_x) * MM_POR_POLEGADA,
            f64::from(self.papel_altura) / f64::from(self.dpi_y) * MM_POR_POLEGADA,
        )
    }
}

/// Como desenhar uma página na fila.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanoPagina {
    /// Tamanho final do bitmap, já considerado o giro.
    pub largura_px: i32,
    pub altura_px: i32,
    /// A página é paisagem e o papel entra em retrato, ou o contrário.
    pub girar_90: bool,
    /// Posição do bitmap em relação à origem da área imprimível; pode ser negativa.
    pub x: i32,
    pub y: i32,
}

/// Fator entre o tamanho físico da página (`medida`, que já considera `/UserUnit`) e a caixa de
/// mídia em pontos. A biblioteca de rasterização informa só os pontos, sem a unidade: sem este
/// fator, uma página com `/UserUnit` 2 sairia com metade do tamanho.
///
/// Compara os lados maiores para não depender da rotação da página. Uma diferença de até 0,5%
/// é só arredondamento e vale como 1.
pub fn fator_de_unidade(
    medida: DimensoesPagina,
    caixa_largura_pt: f64,
    caixa_altura_pt: f64,
) -> Result<f64, ErroEnvio> {
    let fisico = medida.largura_mm.max(medida.altura_mm) / MM_POR_POLEGADA * PONTOS_POR_POLEGADA;
    let caixa = caixa_largura_pt.max(caixa_altura_pt);
    let fator = fisico / caixa;
    if !fator.is_finite() || fator <= 0.0 {
        return Err(ErroEnvio::TamanhoInvalido);
    }
    Ok(if (fator - 1.0).abs() <= 0.005 {
        1.0
    } else {
        fator
    })
}

/// Bytes do bitmap, ou erro se passar do limite. Em aritmética checada.
pub fn bytes_do_bitmap(largura: u64, altura: u64) -> Result<u64, ErroEnvio> {
    largura
        .checked_mul(altura)
        .and_then(|pixels| pixels.checked_mul(BYTES_POR_PIXEL))
        .filter(|bytes| *bytes <= LIMITE_BYTES_PAGINA)
        .ok_or(ErroEnvio::MemoriaExcedida { largura, altura })
}

fn para_pixels(pontos: f64, dpi: i32) -> Result<u64, ErroEnvio> {
    let pixels = (pontos / PONTOS_POR_POLEGADA * f64::from(dpi)).round();
    // O limite de bytes recusa qualquer valor próximo deste; aqui só se evita a conversão
    // saturada de um número absurdo.
    if !pixels.is_finite() || !(1.0..=(u32::MAX as f64)).contains(&pixels) {
        return Err(ErroEnvio::TamanhoInvalido);
    }
    Ok(pixels as u64)
}

/// Centraliza o bitmap no papel e converte para a origem da área imprimível.
fn posicao(papel: i32, bitmap: i32, margem: i32) -> Result<i32, ErroEnvio> {
    let centro = (i64::from(papel) - i64::from(bitmap)) / 2;
    i32::try_from(centro - i64::from(margem)).map_err(|_| ErroEnvio::TamanhoInvalido)
}

/// Planeja uma página de `largura_pt` × `altura_pt` pontos, já com a rotação da própria página.
pub fn planejar_pagina(
    largura_pt: f64,
    altura_pt: f64,
    dispositivo: &Dispositivo,
) -> Result<PlanoPagina, ErroEnvio> {
    dispositivo.validar()?;
    if !largura_pt.is_finite() || !altura_pt.is_finite() || largura_pt <= 0.0 || altura_pt <= 0.0 {
        return Err(ErroEnvio::TamanhoInvalido);
    }
    let pagina_deitada = largura_pt > altura_pt;
    let papel_deitado = dispositivo.papel_largura > dispositivo.papel_altura;
    let quadrada = largura_pt == altura_pt;
    let girar_90 = !quadrada && pagina_deitada != papel_deitado;
    let (na_largura, na_altura) = if girar_90 {
        (altura_pt, largura_pt)
    } else {
        (largura_pt, altura_pt)
    };
    let largura = para_pixels(na_largura, dispositivo.dpi_x)?;
    let altura = para_pixels(na_altura, dispositivo.dpi_y)?;
    bytes_do_bitmap(largura, altura)?;
    let excedida = ErroEnvio::MemoriaExcedida { largura, altura };
    let largura_px = i32::try_from(largura).map_err(|_| excedida.clone())?;
    let altura_px = i32::try_from(altura).map_err(|_| excedida)?;
    Ok(PlanoPagina {
        largura_px,
        altura_px,
        girar_90,
        x: posicao(dispositivo.papel_largura, largura_px, dispositivo.margem_x)?,
        y: posicao(dispositivo.papel_altura, altura_px, dispositivo.margem_y)?,
    })
}

/// Confere se o driver aplicou o papel pedido, com a mesma tolerância que reconhece os formatos.
/// O papel pode estar deitado no driver; as medidas são comparadas sem orientação.
pub fn conferir_papel(dispositivo: &Dispositivo, papel: &Papel) -> Result<(), ErroEnvio> {
    dispositivo.validar()?;
    let (largura, altura) = dispositivo.papel_mm();
    let (menor, maior) = if largura <= altura {
        (largura, altura)
    } else {
        (altura, largura)
    };
    let (nominal_menor, nominal_maior) = medida_nominal(papel);
    if (menor - nominal_menor).abs() > TOLERANCIA_PAPEL_MM
        || (maior - nominal_maior).abs() > TOLERANCIA_PAPEL_MM
    {
        return Err(ErroEnvio::PapelNaoAceito {
            pedido_mm: (nominal_menor, nominal_maior),
            obtido_mm: (menor, maior),
        });
    }
    Ok(())
}

/// Cor do ponto da página em branco, em BGRA. Preto puro: um cinza claro some no meio-tom do
/// driver, e a folha em branco poderia nem sair.
const COR_DO_PONTO: [u8; 4] = [0, 0, 0, 255];

/// Onde fica o ponto da página em branco, em pixels do bitmap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PontoDiscreto {
    pub x: usize,
    pub y: usize,
    pub largura: usize,
    pub altura: usize,
}

/// Parte do bitmap, em um eixo, que cai dentro da área imprimível. `origem` é a posição do
/// bitmap em relação ao início da área.
fn trecho_imprimivel(origem: i32, bitmap: i32, area: i32) -> Result<(i64, i64), ErroEnvio> {
    let inicio = i64::max(0, -i64::from(origem));
    let fim = i64::min(i64::from(bitmap), i64::from(area) - i64::from(origem));
    if inicio >= fim {
        return Err(ErroEnvio::PaginaForaDaArea);
    }
    Ok((inicio, fim))
}

fn para_indice(valor: i64) -> Result<usize, ErroEnvio> {
    usize::try_from(valor).map_err(|_| ErroEnvio::PaginaForaDaArea)
}

/// Escolhe o ponto da página em branco: perto do canto inferior esquerdo da área imprimível, a
/// cerca de 2,5 mm das bordas dela quando há espaço, com cerca de 0,08 mm de lado e nunca menos
/// de um pixel. Recusa quando o bitmap não alcança a área, porque a folha sairia vazia.
pub fn ponto_da_pagina_em_branco(
    plano: &PlanoPagina,
    dispositivo: &Dispositivo,
) -> Result<PontoDiscreto, ErroEnvio> {
    dispositivo.validar()?;
    let (x0, x1) = trecho_imprimivel(plano.x, plano.largura_px, dispositivo.area_largura)?;
    let (y0, y1) = trecho_imprimivel(plano.y, plano.altura_px, dispositivo.area_altura)?;
    let largura = i64::from((dispositivo.dpi_x / 300).max(1)).min(x1 - x0);
    let altura = i64::from((dispositivo.dpi_y / 300).max(1)).min(y1 - y0);
    let recuo_x = i64::from(dispositivo.dpi_x / 10);
    let recuo_y = i64::from(dispositivo.dpi_y / 10);
    Ok(PontoDiscreto {
        x: para_indice((x0 + recuo_x).min(x1 - largura))?,
        y: para_indice((y1 - recuo_y - altura).max(y0))?,
        largura: para_indice(largura)?,
        altura: para_indice(altura)?,
    })
}

/// Marca o ponto da página em branco em um bitmap BGRA de cima para baixo.
pub fn marcar_ponto_discreto(
    pixels: &mut [u8],
    plano: &PlanoPagina,
    dispositivo: &Dispositivo,
) -> Result<(), ErroEnvio> {
    let ponto = ponto_da_pagina_em_branco(plano, dispositivo)?;
    let por_linha = usize::try_from(plano.largura_px).map_err(|_| ErroEnvio::TamanhoInvalido)?;
    let fora = || ErroEnvio::Rasterizacao("o bitmap não tem o tamanho pedido".to_string());
    for linha in 0..ponto.altura {
        for coluna in 0..ponto.largura {
            let inicio = ponto
                .y
                .checked_add(linha)
                .and_then(|y| y.checked_mul(por_linha))
                .and_then(|base| base.checked_add(ponto.x))
                .and_then(|base| base.checked_add(coluna))
                .and_then(|pixel| pixel.checked_mul(COR_DO_PONTO.len()))
                .ok_or_else(fora)?;
            let fim = inicio.checked_add(COR_DO_PONTO.len()).ok_or_else(fora)?;
            pixels
                .get_mut(inicio..fim)
                .ok_or_else(fora)?
                .copy_from_slice(&COR_DO_PONTO);
        }
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use plots_core::reconhecer_papel;

    #[test]
    fn unidade_da_pagina_vira_fator_em_qualquer_rotacao() {
        let a4 = DimensoesPagina::new(210.0, 297.0);
        assert_eq!(fator_de_unidade(a4, 595.0, 842.0).unwrap(), 1.0);
        assert_eq!(fator_de_unidade(a4, 842.0, 595.0).unwrap(), 1.0);
        let dobro = fator_de_unidade(a4, 297.5, 421.0).unwrap();
        assert!((dobro - 2.0).abs() < 0.01, "{dobro}");
        assert_eq!(
            fator_de_unidade(a4, 0.0, 0.0).unwrap_err(),
            ErroEnvio::TamanhoInvalido
        );
        assert_eq!(
            fator_de_unidade(a4, f64::NAN, f64::NAN).unwrap_err(),
            ErroEnvio::TamanhoInvalido
        );
    }

    /// A4 em retrato, a 600 dpi, com margem imprimível de 100 pixels.
    fn a4_600() -> Dispositivo {
        Dispositivo {
            dpi_x: 600,
            dpi_y: 600,
            papel_largura: 4960,
            papel_altura: 7016,
            margem_x: 100,
            margem_y: 100,
            area_largura: 4760,
            area_altura: 6816,
        }
    }

    /// Fila a 72 dpi com margens diferentes em cada lado do papel de 200 × 300.
    fn fila_com_margem() -> Dispositivo {
        Dispositivo {
            dpi_x: 72,
            dpi_y: 72,
            papel_largura: 200,
            papel_altura: 300,
            margem_x: 15,
            margem_y: 20,
            area_largura: 170,
            area_altura: 265,
        }
    }

    /// Pixels marcados, em coordenadas do bitmap.
    fn marcados(plano: &PlanoPagina, pixels: &[u8]) -> Vec<(i32, i32)> {
        pixels
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, cor)| *cor != [255u8; 4])
            .map(|(indice, _)| {
                let indice = indice as i32;
                (indice % plano.largura_px, indice / plano.largura_px)
            })
            .collect()
    }

    #[test]
    fn ponto_da_pagina_em_branco_fica_dentro_da_area_e_nao_e_branco() {
        let fila = fila_com_margem();
        let plano = planejar_pagina(200.0, 300.0, &fila).unwrap();
        let mut pixels = vec![255u8; 200 * 300 * 4];
        marcar_ponto_discreto(&mut pixels, &plano, &fila).unwrap();
        let pontos = marcados(&plano, &pixels);
        assert!(!pontos.is_empty());
        for (x, y) in pontos {
            let no_dispositivo = (plano.x + x, plano.y + y);
            assert!(
                (0..fila.area_largura).contains(&no_dispositivo.0)
                    && (0..fila.area_altura).contains(&no_dispositivo.1),
                "{no_dispositivo:?}"
            );
            let inicio = ((y * plano.largura_px + x) * 4) as usize;
            assert_eq!(pixels[inicio..inicio + 4], COR_DO_PONTO);
        }
    }

    #[test]
    fn ponto_em_alta_resolucao_e_pequeno_e_fica_na_area() {
        let fila = a4_600();
        let plano = planejar_pagina(595.0, 842.0, &fila).unwrap();
        let ponto = ponto_da_pagina_em_branco(&plano, &fila).unwrap();
        assert_eq!((ponto.largura, ponto.altura), (2, 2));
        let x = plano.x + ponto.x as i32;
        let y = plano.y + ponto.y as i32;
        assert!((0..fila.area_largura).contains(&x), "{x}");
        assert!((0..fila.area_altura).contains(&y), "{y}");
    }

    #[test]
    fn bitmap_menor_que_o_recuo_ainda_recebe_o_ponto() {
        let fila = fila_com_margem();
        let plano = PlanoPagina {
            largura_px: 1,
            altura_px: 1,
            girar_90: false,
            x: 3,
            y: 3,
        };
        let mut pixels = vec![255u8; 4];
        marcar_ponto_discreto(&mut pixels, &plano, &fila).unwrap();
        assert_eq!(pixels, COR_DO_PONTO);
    }

    #[test]
    fn bitmap_fora_da_area_imprimivel_e_recusado_e_nada_e_marcado() {
        let fila = fila_com_margem();
        for (x, y) in [(170, 0), (0, 265), (-200, 0), (0, -300)] {
            let plano = PlanoPagina {
                largura_px: 200,
                altura_px: 300,
                girar_90: false,
                x,
                y,
            };
            let mut pixels = vec![255u8; 200 * 300 * 4];
            assert_eq!(
                marcar_ponto_discreto(&mut pixels, &plano, &fila),
                Err(ErroEnvio::PaginaForaDaArea),
                "{x}, {y}"
            );
            assert!(pixels.iter().all(|byte| *byte == 255));
        }
    }

    #[test]
    fn bitmap_menor_que_o_plano_vira_erro_sem_estourar() {
        let fila = fila_com_margem();
        let plano = planejar_pagina(200.0, 300.0, &fila).unwrap();
        let mut pixels = vec![255u8; 8];
        assert!(matches!(
            marcar_ponto_discreto(&mut pixels, &plano, &fila),
            Err(ErroEnvio::Rasterizacao(_))
        ));
    }

    #[test]
    fn medidas_extremas_nao_estouram_o_ponto() {
        let fila = Dispositivo {
            dpi_x: i32::MAX,
            dpi_y: i32::MAX,
            area_largura: i32::MAX,
            area_altura: i32::MAX,
            ..a4_600()
        };
        for origem in [i32::MIN, i32::MAX, 0] {
            let plano = PlanoPagina {
                largura_px: i32::MAX,
                altura_px: i32::MAX,
                girar_90: false,
                x: origem,
                y: origem,
            };
            let _ = ponto_da_pagina_em_branco(&plano, &fila);
        }
    }

    #[test]
    fn a4_retrato_vira_pixels_da_resolucao_da_fila_sem_giro() {
        let plano = planejar_pagina(595.0, 842.0, &a4_600()).unwrap();
        assert_eq!(plano.largura_px, 4958);
        assert_eq!(plano.altura_px, 7017);
        assert!(!plano.girar_90);
        assert_eq!(plano.x, -99);
        // (7016 - 7017) / 2 trunca para zero: a diferença de um pixel não desloca a página.
        assert_eq!(plano.y, -100);
    }

    #[test]
    fn paisagem_em_papel_retrato_gira_e_troca_os_lados() {
        let plano = planejar_pagina(842.0, 595.0, &a4_600()).unwrap();
        assert!(plano.girar_90);
        assert_eq!(plano.largura_px, 4958);
        assert_eq!(plano.altura_px, 7017);
    }

    #[test]
    fn paisagem_em_papel_deitado_nao_gira() {
        let deitado = Dispositivo {
            papel_largura: 7016,
            papel_altura: 4960,
            ..a4_600()
        };
        let plano = planejar_pagina(842.0, 595.0, &deitado).unwrap();
        assert!(!plano.girar_90);
        assert_eq!(plano.largura_px, 7017);
        assert_eq!(plano.altura_px, 4958);
    }

    #[test]
    fn pagina_quadrada_nunca_gira() {
        let plano = planejar_pagina(500.0, 500.0, &a4_600()).unwrap();
        assert!(!plano.girar_90);
    }

    #[test]
    fn resolucoes_diferentes_nos_dois_eixos_valem_para_cada_lado() {
        let fila = Dispositivo {
            dpi_x: 300,
            dpi_y: 600,
            papel_largura: 2480,
            ..a4_600()
        };
        let plano = planejar_pagina(595.0, 842.0, &fila).unwrap();
        assert_eq!(plano.largura_px, 2479);
        assert_eq!(plano.altura_px, 7017);
    }

    #[test]
    fn pagina_menor_que_o_papel_fica_centralizada() {
        let plano = planejar_pagina(297.5, 421.0, &a4_600()).unwrap();
        assert_eq!(plano.largura_px, 2479);
        assert_eq!(plano.x, (4960 - 2479) / 2 - 100);
    }

    #[test]
    fn pagina_gigante_recusa_com_erro_de_memoria_e_nao_estoura() {
        let erro = planejar_pagina(14_400.0, 14_400.0, &a4_600()).unwrap_err();
        assert!(
            matches!(erro, ErroEnvio::MemoriaExcedida { .. }),
            "{erro:?}"
        );
        let erro = planejar_pagina(1.0e30, 1.0e30, &a4_600()).unwrap_err();
        assert_eq!(erro, ErroEnvio::TamanhoInvalido);
    }

    #[test]
    fn limite_de_bytes_aceita_o_exato_e_recusa_o_seguinte() {
        assert_eq!(bytes_do_bitmap(16384, 16384).unwrap(), LIMITE_BYTES_PAGINA);
        assert!(bytes_do_bitmap(16384, 16385).is_err());
        assert!(bytes_do_bitmap(u64::MAX, u64::MAX).is_err());
    }

    #[test]
    fn dimensoes_e_medidas_invalidas_viram_erro() {
        for (largura, altura) in [
            (0.0, 842.0),
            (595.0, -1.0),
            (f64::NAN, 842.0),
            (595.0, f64::INFINITY),
        ] {
            assert_eq!(
                planejar_pagina(largura, altura, &a4_600()).unwrap_err(),
                ErroEnvio::TamanhoInvalido
            );
        }
        let sem_dpi = Dispositivo {
            dpi_x: 0,
            ..a4_600()
        };
        assert!(matches!(
            planejar_pagina(595.0, 842.0, &sem_dpi).unwrap_err(),
            ErroEnvio::Resposta(_)
        ));
        let margem_negativa = Dispositivo {
            margem_x: -1,
            ..a4_600()
        };
        assert!(margem_negativa.validar().is_err());
    }

    #[test]
    fn medidas_extremas_do_dispositivo_nao_estouram_a_posicao() {
        let fila = Dispositivo {
            papel_largura: i32::MAX,
            papel_altura: i32::MAX,
            margem_x: i32::MAX,
            margem_y: i32::MAX,
            ..a4_600()
        };
        assert!(planejar_pagina(595.0, 842.0, &fila).is_ok());
    }

    #[test]
    fn papel_e_conferido_com_a_medida_nominal_do_formato_e_nao_com_a_da_pagina() {
        // A página fica a 1,9 mm do A4 nominal para um lado; o driver, a 1,5 mm para o outro.
        let pagina = reconhecer_papel(DimensoesPagina::new(211.9, 298.9)).unwrap();
        assert_eq!(pagina.formato, plots_core::FormatoPapel::A4);
        let driver = Dispositivo {
            papel_largura: 4925,
            papel_altura: 6980,
            ..a4_600()
        };
        assert!(conferir_papel(&driver, &pagina).is_ok());
    }

    #[test]
    fn papel_aplicado_pelo_driver_e_conferido_com_tolerancia() {
        let a4 = reconhecer_papel(DimensoesPagina::new(210.0, 297.0)).unwrap();
        assert!(conferir_papel(&a4_600(), &a4).is_ok());
        let carta = reconhecer_papel(DimensoesPagina::new(216.0, 279.0)).unwrap();
        let erro = conferir_papel(&a4_600(), &carta).unwrap_err();
        assert!(matches!(erro, ErroEnvio::PapelNaoAceito { .. }), "{erro:?}");
        let deitado = Dispositivo {
            papel_largura: 7016,
            papel_altura: 4960,
            ..a4_600()
        };
        assert!(conferir_papel(&deitado, &a4).is_ok());
    }
}
