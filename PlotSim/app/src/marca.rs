//! O que se sabe de uma página recebida. A leitura vem só da marca visual do desenho, pelo núcleo:
//! o número e o giro saem dela. O texto da página não entra, porque o PlotS envia cada página como
//! imagem; e uma página sem marca legível fica sem número, sem palpite.

use plotsim_core::leitor::PaginaMarcada;
use plotsim_core::marca::ErroMarca;
use plotsim_physics::Giro;

/// Uma página recebida como a bandeja a usa.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaginaVista {
    /// Posição no arquivo, a partir de zero.
    pub indice: usize,
    /// Número que a marca traz. `None` quando não há marca legível, e nunca um palpite.
    pub numero: Option<u32>,
    /// Página em branco, como a que completa um total ímpar. Não é uma página sem marca.
    pub branca: bool,
    /// Giro com que a página chegou. O desenho da página já vem com ele aplicado.
    pub giro: Giro,
    pub largura_mm: f64,
    pub altura_mm: f64,
}

fn giro_da_rotacao(graus: u16) -> Giro {
    match graus {
        90 => Giro::Direita90,
        180 => Giro::Meia180,
        270 => Giro::Esquerda270,
        _ => Giro::Nenhum,
    }
}

/// O que a marca disse de uma página. Só a leitura conferida dá número; a página em branco é
/// reconhecida como tal; qualquer outra falha fica sem número.
pub fn ler_pagina(indice: usize, pagina: &PaginaMarcada) -> PaginaVista {
    let (numero, branca, giro) = match &pagina.leitura {
        Ok(marca) => (Some(marca.pagina), false, giro_da_rotacao(marca.rotacao)),
        Err(ErroMarca::PaginaEmBranco) => (None, true, Giro::Nenhum),
        Err(_) => (None, false, Giro::Nenhum),
    };
    PaginaVista {
        indice,
        numero,
        branca,
        giro,
        largura_mm: pagina.largura_mm,
        altura_mm: pagina.altura_mm,
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use plotsim_core::marca::MarcaLida;

    fn pagina(leitura: Result<MarcaLida, ErroMarca>) -> PaginaMarcada {
        PaginaMarcada {
            numero: 1,
            largura_mm: 210.0,
            altura_mm: 297.0,
            leitura,
        }
    }

    #[test]
    fn pagina_com_marca_traz_numero_e_giro() {
        let vista = ler_pagina(
            4,
            &pagina(Ok(MarcaLida {
                pagina: 12,
                rotacao: 180,
            })),
        );
        assert_eq!(vista.numero, Some(12));
        assert_eq!(vista.giro, Giro::Meia180);
        assert!(!vista.branca);
        assert_eq!(vista.indice, 4);
    }

    #[test]
    fn pagina_sem_marca_legivel_fica_sem_numero_e_sem_palpite() {
        for erro in [
            ErroMarca::SemMarca,
            ErroMarca::MarcaInvalida,
            ErroMarca::TintaDemais,
            ErroMarca::Ambigua { paginas: (3, 4) },
        ] {
            let vista = ler_pagina(0, &pagina(Err(erro.clone())));
            assert_eq!(vista.numero, None, "{erro:?}");
            assert!(!vista.branca, "{erro:?}");
            assert_eq!(vista.giro, Giro::Nenhum, "{erro:?}");
        }
    }

    #[test]
    fn pagina_em_branco_nao_e_pagina_sem_marca() {
        let vista = ler_pagina(2, &pagina(Err(ErroMarca::PaginaEmBranco)));
        assert!(vista.branca);
        assert_eq!(vista.numero, None);
    }

    #[test]
    fn rotacao_fora_dos_quatro_quartos_nao_gira_a_pagina() {
        let vista = ler_pagina(
            0,
            &pagina(Ok(MarcaLida {
                pagina: 1,
                rotacao: 45,
            })),
        );
        assert_eq!(vista.giro, Giro::Nenhum);
    }
}
