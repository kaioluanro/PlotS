//! Como uma folha aparece para quem olha: tudo sai da pose da folha no espaço, sem tabela.

use plotsim_physics::{Face, Folha, Giro, Lado, Vetor};

/// De onde se olha a folha e o que aparece como "para cima" na tela.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quadro {
    /// Aponta da folha para o olho: `+z` ou `-z`.
    pub olho: Vetor,
    pub cima: Vetor,
}

impl Quadro {
    pub const fn novo(olho: Vetor, cima: Vetor) -> Self {
        Self { olho, cima }
    }

    /// O mesmo olhar pelo outro lado da folha, virando-a como página de livro.
    pub const fn do_outro_lado(self) -> Self {
        Self {
            olho: self.olho.oposto(),
            cima: self.cima,
        }
    }

    /// Sentido horário na tela, em graus, de uma direção do mundo.
    fn graus(self, direcao: Vetor) -> i32 {
        let direita = self.cima.vetorial(self.olho);
        match (direcao.escalar(direita), direcao.escalar(self.cima)) {
            (1, 0) => 90,
            (0, -1) => 180,
            (-1, 0) => 270,
            _ => 0,
        }
    }

    fn lado_do_olho(self) -> Lado {
        if self.olho.z < 0 {
            Lado::Baixo
        } else {
            Lado::Cima
        }
    }
}

/// Uma imagem desenhada na folha, já com a transformação da tela.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImagemNaTela {
    pub pagina: u32,
    /// Giro a aplicar no desenho, em graus, no sentido horário.
    pub graus: u16,
    /// Vista através do papel: espelhada e tênue.
    pub atras: bool,
}

fn giro_em_graus(giro: Giro) -> i32 {
    match giro {
        Giro::Nenhum => 0,
        Giro::Direita90 => 90,
        Giro::Meia180 => 180,
        Giro::Esquerda270 => 270,
    }
}

/// Giro a aplicar no desenho. O desenho da página já traz o giro com que ela chegou, então só resta
/// a diferença até o topo cair onde a geometria diz. Do outro lado do papel o desenho é espelhado,
/// e o espelho inverte o sentido do giro já embutido.
fn giro_do_desenho(quadro: Quadro, topo: Vetor, embutido: i32, atras: bool) -> u16 {
    let alvo = quadro.graus(topo);
    let graus = if atras {
        alvo.saturating_add(embutido)
    } else {
        alvo.saturating_sub(embutido)
    };
    u16::try_from(graus.rem_euclid(360)).unwrap_or(0)
}

fn imagens_da_face(
    folha: &Folha,
    face: Face,
    quadro: Quadro,
    atras: bool,
    giro_da_pagina: &impl Fn(u32) -> Giro,
    saida: &mut Vec<ImagemNaTela>,
) {
    for impressao in folha.impressoes(face) {
        let topo = folha.topo_no_mundo(*impressao);
        let embutido = giro_em_graus(giro_da_pagina(impressao.pagina));
        saida.push(ImagemNaTela {
            pagina: impressao.pagina,
            graus: giro_do_desenho(quadro, topo, embutido, atras),
            atras,
        });
    }
}

/// O que se vê da folha por este quadro: primeiro o que fica atrás do papel, depois o da frente.
pub fn imagens_da_folha(
    folha: &Folha,
    quadro: Quadro,
    giro_da_pagina: &impl Fn(u32) -> Giro,
) -> Vec<ImagemNaTela> {
    let perto = folha.face_para(quadro.lado_do_olho());
    let longe = match perto {
        Face::A => Face::B,
        Face::B => Face::A,
    };
    let mut imagens = Vec::new();
    imagens_da_face(folha, longe, quadro, true, giro_da_pagina, &mut imagens);
    imagens_da_face(folha, perto, quadro, false, giro_da_pagina, &mut imagens);
    imagens
}
