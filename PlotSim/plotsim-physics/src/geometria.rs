//! Rotações rígidas de uma folha no espaço, sem tabela de casos.
//!
//! Referencial do mundo, visto de cima da impressora: `x` para a direita, `y` no sentido em que
//! a folha anda ao entrar, `z` para cima. Referencial do material: a folha parada na entrada, com
//! a face A para cima e a mesma orientação do mundo. Toda rotação usada aqui é múltiplo de 90°
//! em torno de um eixo, portanto os componentes ficam sempre em -1, 0 ou 1.

/// Vetor de componentes inteiros.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Vetor {
    pub x: i8,
    pub y: i8,
    pub z: i8,
}

impl Vetor {
    pub const X: Vetor = Vetor::novo(1, 0, 0);
    pub const Y: Vetor = Vetor::novo(0, 1, 0);
    pub const Z: Vetor = Vetor::novo(0, 0, 1);

    pub const fn novo(x: i8, y: i8, z: i8) -> Self {
        Self { x, y, z }
    }

    /// Sentido contrário.
    pub const fn oposto(self) -> Self {
        Self::novo(
            self.x.saturating_neg(),
            self.y.saturating_neg(),
            self.z.saturating_neg(),
        )
    }

    /// Produto escalar. Satura em vez de transbordar, embora os vetores daqui sejam unitários.
    pub const fn escalar(self, o: Vetor) -> i8 {
        soma3(
            self.x.saturating_mul(o.x),
            self.y.saturating_mul(o.y),
            self.z.saturating_mul(o.z),
        )
    }

    /// Produto vetorial.
    pub const fn vetorial(self, o: Vetor) -> Self {
        Self::novo(
            self.y
                .saturating_mul(o.z)
                .saturating_sub(self.z.saturating_mul(o.y)),
            self.z
                .saturating_mul(o.x)
                .saturating_sub(self.x.saturating_mul(o.z)),
            self.x
                .saturating_mul(o.y)
                .saturating_sub(self.y.saturating_mul(o.x)),
        )
    }
}

const fn soma3(a: i8, b: i8, c: i8) -> i8 {
    a.saturating_add(b).saturating_add(c)
}

/// Rotação rígida guardada pelas imagens dos três eixos do material.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rotacao {
    x: Vetor,
    y: Vetor,
    z: Vetor,
}

impl Rotacao {
    pub const IDENTIDADE: Rotacao = Rotacao {
        x: Vetor::X,
        y: Vetor::Y,
        z: Vetor::Z,
    };

    /// Meia volta em torno do eixo `x`: inverte `y` e `z`. É a virada por uma aresta paralela ao
    /// rolo, e é também o que a curva em U do caminho do papel faz com a folha.
    pub const MEIA_VOLTA_X: Rotacao = Rotacao {
        x: Vetor::X,
        y: Vetor::novo(0, -1, 0),
        z: Vetor::novo(0, 0, -1),
    };

    /// Meia volta em torno do eixo `y`: inverte `x` e `z`.
    pub const MEIA_VOLTA_Y: Rotacao = Rotacao {
        x: Vetor::novo(-1, 0, 0),
        y: Vetor::Y,
        z: Vetor::novo(0, 0, -1),
    };

    /// Meia volta em torno do eixo `z`: giro no plano da folha.
    pub const MEIA_VOLTA_Z: Rotacao = Rotacao {
        x: Vetor::novo(-1, 0, 0),
        y: Vetor::novo(0, -1, 0),
        z: Vetor::Z,
    };

    /// Leva um vetor do material para o mundo.
    pub const fn aplicar(self, v: Vetor) -> Vetor {
        Vetor::novo(
            soma3(
                self.x.x.saturating_mul(v.x),
                self.y.x.saturating_mul(v.y),
                self.z.x.saturating_mul(v.z),
            ),
            soma3(
                self.x.y.saturating_mul(v.x),
                self.y.y.saturating_mul(v.y),
                self.z.y.saturating_mul(v.z),
            ),
            soma3(
                self.x.z.saturating_mul(v.x),
                self.y.z.saturating_mul(v.y),
                self.z.z.saturating_mul(v.z),
            ),
        )
    }

    /// Leva um vetor do mundo de volta ao material. A inversa de uma rotação é a sua transposta.
    pub const fn desfazer(self, v: Vetor) -> Vetor {
        Vetor::novo(self.x.escalar(v), self.y.escalar(v), self.z.escalar(v))
    }

    /// Aplica `depois` em seguida a esta rotação.
    pub const fn seguida_de(self, depois: Rotacao) -> Rotacao {
        Rotacao {
            x: depois.aplicar(self.x),
            y: depois.aplicar(self.y),
            z: depois.aplicar(self.z),
        }
    }

    /// Direção da normal da face A no mundo.
    pub const fn normal_da_face_a(self) -> Vetor {
        self.z
    }
}

/// Giro de uma imagem no plano da página, no sentido horário de quem lê a face.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Giro {
    #[default]
    Nenhum,
    Direita90,
    Meia180,
    Esquerda270,
}

impl Giro {
    /// Gira `topo` em torno da normal que aponta para quem lê a face.
    pub const fn aplicar(self, topo: Vetor, normal_para_o_leitor: Vetor) -> Vetor {
        let direita = topo.vetorial(normal_para_o_leitor);
        match self {
            Giro::Nenhum => topo,
            Giro::Direita90 => direita,
            Giro::Meia180 => topo.oposto(),
            Giro::Esquerda270 => direita.oposto(),
        }
    }
}
