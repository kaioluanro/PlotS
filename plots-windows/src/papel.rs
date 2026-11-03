//! Tradução do papel do lote para os campos de papel do `DEVMODE`.

use crate::erro::ErroEnvio;
use plots_core::{FormatoPapel, Papel};

/// Valores de `DMPAPER_*` da documentação do Windows.
const DMPAPER_LETTER: i16 = 1;
const DMPAPER_LEGAL: i16 = 5;
const DMPAPER_A4: i16 = 9;
const DMPAPER_A5: i16 = 11;
const DMPAPER_FOLIO: i16 = 14;
/// Indica que o tamanho vai em `dmPaperWidth` e `dmPaperLength`.
pub const DMPAPER_USER: i16 = 256;

/// Valores que o envio fixa no `DEVMODE`, da documentação do Windows.
pub const UMA_COPIA: i16 = 1;
pub const ESCALA_REAL: i16 = 100;
pub const ORIENTACAO_RETRATO: i16 = 1;
/// `DMNUP_ONEUP`: uma página por folha.
pub const UMA_POR_FOLHA: u32 = 2;
/// `DMNUP_SYSTEM`: o valor que um driver sem suporte a isso costuma deixar.
const NUP_DO_SISTEMA: u32 = 1;

/// Um campo do `DEVMODE` devolvido pelo driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CampoLido {
    /// O bit do campo está ligado em `dmFields`.
    pub marcado: bool,
    pub valor: i64,
}

/// O que o driver devolveu nos campos que o envio fixa.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LeituraDevmode {
    pub copias: CampoLido,
    pub escala: CampoLido,
    pub orientacao: CampoLido,
    /// `None` quando o `DEVMODE` do driver é curto demais para ter o campo.
    pub paginas_por_folha: Option<CampoLido>,
}

/// Bit desligado com valor neutro é um driver que não tem o recurso, e a saída sai como pedida.
/// Qualquer outra diferença é um valor que o driver não aplicou.
fn campo_aplicado(lido: CampoLido, pedido: i64, neutro: i64) -> bool {
    let sem_recurso = !lido.marcado && (lido.valor == 0 || lido.valor == neutro);
    lido.valor == pedido || sem_recurso
}

/// Para campos cujo valor é uma quantidade, que quem lê entende como número.
fn conferir_quantidade(campo: &'static str, lido: CampoLido, pedido: i64) -> Result<(), ErroEnvio> {
    if campo_aplicado(lido, pedido, 0) {
        return Ok(());
    }
    Err(ErroEnvio::ValorNaoAceito {
        campo,
        pedido,
        obtido: lido.valor,
    })
}

/// Para campos cujo valor é um código do Windows, que não diria nada a quem lê.
fn conferir_opcao(
    texto: &'static str,
    lido: CampoLido,
    pedido: i64,
    neutro: i64,
) -> Result<(), ErroEnvio> {
    if campo_aplicado(lido, pedido, neutro) {
        return Ok(());
    }
    Err(ErroEnvio::ValorNaoConfirmado { pedido: texto })
}

/// Confere o que o driver devolveu depois de mesclar o pedido. Uma cópia a mais, uma escala ou
/// várias páginas por folha estragariam o alinhamento entre frente e verso.
pub fn conferir_devmode(leitura: &LeituraDevmode) -> Result<(), ErroEnvio> {
    conferir_quantidade("o número de cópias", leitura.copias, UMA_COPIA.into())?;
    conferir_quantidade("a escala", leitura.escala, ESCALA_REAL.into())?;
    conferir_opcao(
        "a orientação em retrato",
        leitura.orientacao,
        ORIENTACAO_RETRATO.into(),
        0,
    )?;
    if let Some(lido) = leitura.paginas_por_folha {
        conferir_opcao(
            "uma página por folha",
            lido,
            UMA_POR_FOLHA.into(),
            NUP_DO_SISTEMA.into(),
        )?;
    }
    Ok(())
}

/// Medida nominal do papel, em retrato: é o que o driver aplica ao receber a constante do
/// formato, e a página do documento pode diferir dela dentro da tolerância. Fora da tabela, é a
/// própria medida da página.
pub fn medida_nominal(papel: &Papel) -> (f64, f64) {
    match papel.formato {
        FormatoPapel::A4 => (210.0, 297.0),
        FormatoPapel::A5 => (148.0, 210.0),
        FormatoPapel::Carta => (216.0, 279.0),
        FormatoPapel::Oficio => (216.0, 330.0),
        FormatoPapel::Legal => (216.0, 356.0),
        FormatoPapel::Personalizado => (papel.largura_mm, papel.altura_mm),
    }
}

/// O que informar ao driver sobre o papel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AjustePapel {
    /// Formato com constante própria.
    Formato(i16),
    /// Medidas em décimos de milímetro, para `DMPAPER_USER`.
    Medidas { largura: i16, comprimento: i16 },
}

fn decimos_de_mm(mm: f64) -> Option<i16> {
    let decimos = (mm * 10.0).round();
    if decimos.is_finite() && (1.0..=f64::from(i16::MAX)).contains(&decimos) {
        Some(decimos as i16)
    } else {
        None
    }
}

/// Escolhe a constante do formato. Fora da tabela, usa as medidas da página; quem confere se o
/// driver aceitou é a leitura do papel depois de criar o contexto da fila.
pub fn ajuste_do_papel(papel: &Papel) -> Result<AjustePapel, ErroEnvio> {
    let formato = match papel.formato {
        FormatoPapel::A4 => DMPAPER_A4,
        FormatoPapel::A5 => DMPAPER_A5,
        FormatoPapel::Carta => DMPAPER_LETTER,
        FormatoPapel::Oficio => DMPAPER_FOLIO,
        FormatoPapel::Legal => DMPAPER_LEGAL,
        FormatoPapel::Personalizado => {
            let grande = ErroEnvio::PapelGrandeDemais {
                largura_mm: papel.largura_mm,
                altura_mm: papel.altura_mm,
            };
            return Ok(AjustePapel::Medidas {
                largura: decimos_de_mm(papel.largura_mm).ok_or_else(|| grande.clone())?,
                comprimento: decimos_de_mm(papel.altura_mm).ok_or(grande)?,
            });
        }
    };
    Ok(AjustePapel::Formato(formato))
}

/// Quantas faces o envio pede ao driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FacesDoEnvio {
    /// Uma face só: o fluxo manual, sempre.
    Uma,
    /// As duas faces, com a virada pela borda longa.
    Duas,
}

impl FacesDoEnvio {
    /// Só o trabalho em duas faces pede o recurso ao driver.
    pub const fn da_passada(passada: plots_print::job::Passada) -> Self {
        match passada {
            plots_print::job::Passada::DuasFaces => Self::Duas,
            plots_print::job::Passada::Frente | plots_print::job::Passada::Verso => Self::Uma,
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn so_o_trabalho_de_duas_faces_pede_o_recurso_ao_driver() {
        use plots_print::job::Passada;
        assert_eq!(
            FacesDoEnvio::da_passada(Passada::DuasFaces),
            FacesDoEnvio::Duas
        );
        assert_eq!(FacesDoEnvio::da_passada(Passada::Frente), FacesDoEnvio::Uma);
        assert_eq!(FacesDoEnvio::da_passada(Passada::Verso), FacesDoEnvio::Uma);
    }
    use plots_core::{reconhecer_papel, DimensoesPagina};

    fn papel(largura: f64, altura: f64) -> Papel {
        reconhecer_papel(DimensoesPagina::new(largura, altura)).unwrap()
    }

    fn campo(valor: i64) -> CampoLido {
        CampoLido {
            marcado: true,
            valor,
        }
    }

    fn leitura_certa() -> LeituraDevmode {
        LeituraDevmode {
            copias: campo(1),
            escala: campo(100),
            orientacao: campo(1),
            paginas_por_folha: Some(campo(2)),
        }
    }

    #[test]
    fn devmode_com_os_valores_pedidos_passa() {
        assert_eq!(conferir_devmode(&leitura_certa()), Ok(()));
        let sem_campo = LeituraDevmode {
            paginas_por_folha: None,
            ..leitura_certa()
        };
        assert_eq!(conferir_devmode(&sem_campo), Ok(()));
    }

    #[test]
    fn cada_valor_recusado_diz_qual_foi_e_que_nada_foi_enviado() {
        let casos = [
            (
                LeituraDevmode {
                    copias: campo(3),
                    ..leitura_certa()
                },
                "cópias",
            ),
            (
                LeituraDevmode {
                    escala: campo(50),
                    ..leitura_certa()
                },
                "escala",
            ),
            (
                LeituraDevmode {
                    orientacao: campo(2),
                    ..leitura_certa()
                },
                "orientação",
            ),
            (
                LeituraDevmode {
                    paginas_por_folha: Some(campo(4)),
                    ..leitura_certa()
                },
                "por folha",
            ),
        ];
        for (leitura, trecho) in casos {
            let texto = conferir_devmode(&leitura).unwrap_err().to_string();
            assert!(texto.contains(trecho), "{texto}");
            assert!(texto.contains("nada foi enviado"), "{texto}");
        }
    }

    #[test]
    fn nup_e_orientacao_recusados_dizem_o_pedido_em_palavras_e_nao_em_codigos() {
        let nup = LeituraDevmode {
            paginas_por_folha: Some(campo(1)),
            ..leitura_certa()
        };
        assert_eq!(
            conferir_devmode(&nup).unwrap_err().to_string(),
            "pedi uma página por folha, e o driver não confirmou; nada foi enviado"
        );
        let orientacao = LeituraDevmode {
            orientacao: campo(2),
            ..leitura_certa()
        };
        assert_eq!(
            conferir_devmode(&orientacao).unwrap_err().to_string(),
            "pedi a orientação em retrato, e o driver não confirmou; nada foi enviado"
        );
    }

    #[test]
    fn campo_marcado_com_zero_nao_foi_aplicado() {
        let leitura = LeituraDevmode {
            copias: campo(0),
            ..leitura_certa()
        };
        assert!(conferir_devmode(&leitura).is_err());
    }

    #[test]
    fn driver_sem_o_recurso_desliga_o_bit_e_passa_so_com_valor_neutro() {
        let desligado = |valor| CampoLido {
            marcado: false,
            valor,
        };
        let sem_recurso = LeituraDevmode {
            copias: desligado(0),
            escala: desligado(100),
            orientacao: desligado(0),
            paginas_por_folha: Some(desligado(1)),
        };
        assert_eq!(conferir_devmode(&sem_recurso), Ok(()));
        let copias_escondidas = LeituraDevmode {
            copias: desligado(3),
            ..leitura_certa()
        };
        assert!(conferir_devmode(&copias_escondidas).is_err());
        let varias_por_folha = LeituraDevmode {
            paginas_por_folha: Some(desligado(4)),
            ..leitura_certa()
        };
        assert!(conferir_devmode(&varias_por_folha).is_err());
    }

    #[test]
    fn medida_nominal_e_a_do_formato_reconhecido_e_nao_a_da_pagina() {
        for (largura, altura) in [
            (210.0, 297.0),
            (148.0, 210.0),
            (216.0, 279.0),
            (216.0, 330.0),
            (216.0, 356.0),
        ] {
            let quase = papel(largura + 1.5, altura - 1.5);
            assert_eq!(medida_nominal(&quase), (largura, altura));
            // A tabela daqui não pode se afastar da que reconhece os formatos.
            assert_eq!(quase.formato, papel(largura, altura).formato);
            assert_ne!(quase.formato, FormatoPapel::Personalizado);
        }
        assert_eq!(medida_nominal(&papel(100.0, 150.5)), (100.0, 150.5));
    }

    #[test]
    fn cada_formato_da_tabela_usa_a_constante_do_windows() {
        for (largura, altura, esperado) in [
            (210.0, 297.0, 9),
            (148.0, 210.0, 11),
            (216.0, 279.0, 1),
            (216.0, 330.0, 14),
            (216.0, 356.0, 5),
        ] {
            assert_eq!(
                ajuste_do_papel(&papel(largura, altura)).unwrap(),
                AjustePapel::Formato(esperado),
                "{largura} × {altura}"
            );
        }
    }

    #[test]
    fn pagina_deitada_usa_o_mesmo_formato() {
        assert_eq!(
            ajuste_do_papel(&papel(297.0, 210.0)).unwrap(),
            AjustePapel::Formato(9)
        );
    }

    #[test]
    fn tamanho_fora_da_tabela_vai_em_decimos_de_milimetro() {
        assert_eq!(
            ajuste_do_papel(&papel(100.0, 150.5)).unwrap(),
            AjustePapel::Medidas {
                largura: 1000,
                comprimento: 1505
            }
        );
    }

    #[test]
    fn papel_que_nao_cabe_no_campo_do_driver_da_erro_claro() {
        let erro = ajuste_do_papel(&papel(100.0, 4000.0)).unwrap_err();
        assert!(
            matches!(erro, ErroEnvio::PapelGrandeDemais { .. }),
            "{erro:?}"
        );
        assert!(erro.to_string().contains("4000"), "{erro}");
    }
}
