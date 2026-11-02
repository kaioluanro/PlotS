//! Tamanho minimo da janela medido na area util.
//!
//! Sem moldura do sistema, a janela ainda reserva bordas invisiveis para o redimensionamento, e o
//! limite que a biblioteca de janelas informa ao sistema e aplicado ao tamanho externo, com essas
//! bordas incluidas. Por isso o limite configurado sozinho deixa a area util menor que o pedido.

use tauri::{PhysicalSize, Window};

/// Menor area util, em pixels independentes da escala.
pub const AREA_UTIL_MINIMA: (f64, f64) = (640.0, 420.0);

/// Tamanho externo em pixels fisicos que garante a area util pedida, dadas as medidas atuais.
pub fn minimo_externo(
    area_util: (f64, f64),
    externa: (u32, u32),
    interna: (u32, u32),
    escala: f64,
) -> (u32, u32) {
    let escala = if escala.is_finite() && escala > 0.0 {
        escala
    } else {
        1.0
    };
    // A conversao de f64 para u32 satura; nenhuma medida estranha vira valor invalido.
    let lado = |area: f64, fora: u32, dentro: u32| {
        ((area * escala).ceil() as u32).saturating_add(fora.saturating_sub(dentro))
    };
    (
        lado(area_util.0, externa.0, interna.0),
        lado(area_util.1, externa.1, interna.1),
    )
}

/// Recalcula o limite pela borda atual. Se o sistema nao responder, o limite anterior continua.
pub fn ajustar_minimo(janela: &Window) {
    // Maximizada ou minimizada, a janela nao tem as bordas que se quer medir.
    if janela.is_maximized().unwrap_or(true) || janela.is_minimized().unwrap_or(true) {
        return;
    }
    let (Ok(externa), Ok(interna), Ok(escala)) = (
        janela.outer_size(),
        janela.inner_size(),
        janela.scale_factor(),
    ) else {
        return;
    };
    let (largura, altura) = minimo_externo(
        AREA_UTIL_MINIMA,
        (externa.width, externa.height),
        (interna.width, interna.height),
        escala,
    );
    let _ = janela.set_min_size(Some(PhysicalSize::new(largura, altura)));
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn soma_as_bordas_invisiveis_ao_minimo() {
        assert_eq!(
            minimo_externo((640.0, 420.0), (1296, 809), (1280, 800), 1.0),
            (656, 429)
        );
    }

    #[test]
    fn aplica_a_escala_a_area_e_mantem_a_borda_fisica() {
        assert_eq!(
            minimo_externo((640.0, 420.0), (1944, 1213), (1920, 1200), 1.5),
            (984, 643)
        );
    }

    #[test]
    fn sem_borda_o_minimo_e_a_propria_area() {
        assert_eq!(
            minimo_externo((640.0, 420.0), (800, 600), (800, 600), 1.25),
            (800, 525)
        );
    }

    #[test]
    fn medida_incoerente_nao_gera_valor_invalido() {
        assert_eq!(
            minimo_externo((640.0, 420.0), (100, 100), (200, 200), 0.0),
            (640, 420)
        );
        assert_eq!(
            minimo_externo((640.0, 420.0), (116, 109), (100, 100), f64::NAN),
            (656, 429)
        );
        assert_eq!(
            minimo_externo((640.0, 420.0), (u32::MAX, 0), (0, 0), f64::MAX),
            (u32::MAX, u32::MAX)
        );
    }
}
