//! Contas de imprimir um documento em outro papel, sem tocar em PDF nem em fila.

use crate::{recortar_paginas, DimensoesPagina, ErroPapel, FormatoPapel, IntervaloPaginas};

/// Um papel da tabela em que o documento pode ser ajustado. Só existe para formato com medida
/// nominal: "Outro" não tem medida para pedir ao driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PapelDeAjuste(FormatoPapel);

impl PapelDeAjuste {
    /// Papel com este código exato, o mesmo que a interface e a linha de comando usam.
    pub fn de_codigo(codigo: &str) -> Option<Self> {
        match codigo {
            "a4" => Some(Self(FormatoPapel::A4)),
            "a5" => Some(Self(FormatoPapel::A5)),
            "carta" => Some(Self(FormatoPapel::Carta)),
            "oficio" => Some(Self(FormatoPapel::Oficio)),
            "legal" => Some(Self(FormatoPapel::Legal)),
            _ => None,
        }
    }

    pub const fn formato(self) -> FormatoPapel {
        self.0
    }

    /// Largura e altura em retrato, em milímetros.
    pub const fn medida_mm(self) -> (f64, f64) {
        match self.0 {
            FormatoPapel::A4 | FormatoPapel::Personalizado => (210.0, 297.0),
            FormatoPapel::A5 => (148.0, 210.0),
            FormatoPapel::Carta => (216.0, 279.0),
            FormatoPapel::Oficio => (216.0, 330.0),
            FormatoPapel::Legal => (216.0, 356.0),
        }
    }

    /// Fator proporcional que faz a página caber inteira, com o papel na orientação dela. Medida
    /// inválida ou que estouraria a conta não tem fator.
    pub fn escala(self, pagina: DimensoesPagina) -> Option<f64> {
        let (largura, altura) = (pagina.largura_mm, pagina.altura_mm);
        if !largura.is_finite() || !altura.is_finite() || largura <= 0.0 || altura <= 0.0 {
            return None;
        }
        let (papel_menor, papel_maior) = self.medida_mm();
        let fator = (papel_menor / largura.min(altura)).min(papel_maior / largura.max(altura));
        (fator.is_finite() && fator > 0.0).then_some(fator)
    }

    /// As medidas com que o plano enxerga o documento: toda página válida fica no papel escolhido,
    /// e os lotes não se separam por papel nem por orientação. Página inválida fica como veio, para
    /// o plano recusá-la pelo número quando ela estiver no trecho enviado.
    pub fn dimensoes_no_papel(self, paginas: &[DimensoesPagina]) -> Vec<DimensoesPagina> {
        let (largura, altura) = self.medida_mm();
        paginas
            .iter()
            .map(|pagina| {
                if self.escala(*pagina).is_some() {
                    DimensoesPagina::new(largura, altura)
                } else {
                    *pagina
                }
            })
            .collect()
    }

    /// A menor e a maior escala entre as páginas do trecho, para a confirmação dizer. Sem página no
    /// trecho, não há o que dizer.
    pub fn faixa_de_escala(
        self,
        paginas: &[DimensoesPagina],
        intervalo: Option<IntervaloPaginas>,
    ) -> Result<Option<(f64, f64)>, ErroPapel> {
        let (recorte, antes) = recortar_paginas(paginas, intervalo)?;
        let mut faixa: Option<(f64, f64)> = None;
        for (indice, pagina) in recorte.iter().enumerate() {
            let escala = self.escala(*pagina).ok_or(ErroPapel::DimensaoInvalida {
                pagina: antes.saturating_add(indice).saturating_add(1),
            })?;
            faixa = Some(faixa.map_or((escala, escala), |(menor, maior)| {
                (menor.min(escala), maior.max(escala))
            }));
        }
        Ok(faixa)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::{plano_lotes_do_intervalo, OrdemPassadas, PerfilImpressora};

    const A4: DimensoesPagina = DimensoesPagina::new(210.0, 297.0);
    const A5: DimensoesPagina = DimensoesPagina::new(148.0, 210.0);
    const CARTA: DimensoesPagina = DimensoesPagina::new(216.0, 279.0);

    fn papel(codigo: &str) -> PapelDeAjuste {
        PapelDeAjuste::de_codigo(codigo).unwrap()
    }

    fn percentual(fator: f64) -> i64 {
        (fator * 100.0).round() as i64
    }

    #[test]
    fn escala_e_proporcional_nos_casos_comuns() {
        assert_eq!(percentual(papel("a4").escala(CARTA).unwrap()), 97);
        assert_eq!(percentual(papel("carta").escala(A4).unwrap()), 94);
        assert_eq!(percentual(papel("a4").escala(A5).unwrap()), 141);
        assert_eq!(percentual(papel("a4").escala(A4).unwrap()), 100);
    }

    #[test]
    fn a_pagina_cabe_inteira_em_qualquer_orientacao() {
        for pagina in [
            A4,
            A5,
            CARTA,
            DimensoesPagina::new(297.0, 210.0),
            DimensoesPagina::new(500.0, 20.0),
            DimensoesPagina::new(20.0, 500.0),
            DimensoesPagina::new(300.0, 300.0),
        ] {
            let alvo = papel("a4");
            let fator = alvo.escala(pagina).unwrap();
            let (menor, maior) = alvo.medida_mm();
            let lado_menor = pagina.largura_mm.min(pagina.altura_mm) * fator;
            let lado_maior = pagina.largura_mm.max(pagina.altura_mm) * fator;
            assert!(lado_menor <= menor + 1e-9, "{pagina:?}");
            assert!(lado_maior <= maior + 1e-9, "{pagina:?}");
            assert!(
                (lado_menor - menor).abs() < 1e-9 || (lado_maior - maior).abs() < 1e-9,
                "um dos lados encosta no papel: {pagina:?}"
            );
        }
    }

    #[test]
    fn medidas_extremas_nao_estouram_nem_viram_escala() {
        for (largura, altura) in [
            (0.0, 297.0),
            (-1.0, 297.0),
            (f64::NAN, 297.0),
            (f64::INFINITY, 297.0),
            (210.0, f64::NEG_INFINITY),
            (f64::MIN_POSITIVE / 8.0, f64::MIN_POSITIVE / 8.0),
        ] {
            assert_eq!(
                papel("a4").escala(DimensoesPagina::new(largura, altura)),
                None,
                "{largura} {altura}"
            );
        }
        let gigante = papel("a4")
            .escala(DimensoesPagina::new(f64::MAX, f64::MAX))
            .unwrap();
        assert!(gigante > 0.0 && gigante.is_finite());
    }

    #[test]
    fn codigo_desconhecido_nao_e_papel() {
        for codigo in ["", "A4", "outro", "personalizado", "a4 "] {
            assert_eq!(PapelDeAjuste::de_codigo(codigo), None, "{codigo:?}");
        }
    }

    #[test]
    fn papeis_mistos_saem_em_um_so_fluxo_e_sem_recusa_de_par() {
        let documento = [A4, A5, CARTA, DimensoesPagina::new(297.0, 210.0), A5, A5];
        assert!(
            plano_lotes_do_intervalo(
                &documento,
                None,
                10,
                OrdemPassadas::default(),
                PerfilImpressora::default()
            )
            .is_err(),
            "sem a opção, frente e verso de tamanhos diferentes continuam recusados"
        );
        let ajustadas = papel("a4").dimensoes_no_papel(&documento);
        let lotes = plano_lotes_do_intervalo(
            &ajustadas,
            None,
            10,
            OrdemPassadas::default(),
            PerfilImpressora::default(),
        )
        .unwrap();
        assert_eq!(lotes.len(), 1);
        assert_eq!(lotes[0].papel.formato, FormatoPapel::A4);
    }

    #[test]
    fn a_faixa_olha_so_o_trecho_e_diz_a_pagina_invalida() {
        let documento = [A4, A5, DimensoesPagina::new(0.0, 0.0)];
        let alvo = papel("a4");
        let so_a4 = alvo
            .faixa_de_escala(&documento, Some(IntervaloPaginas { inicio: 1, fim: 1 }))
            .unwrap()
            .unwrap();
        assert_eq!(percentual(so_a4.0), 100);
        let misto = alvo
            .faixa_de_escala(&documento, Some(IntervaloPaginas { inicio: 1, fim: 2 }))
            .unwrap()
            .unwrap();
        assert_eq!((percentual(misto.0), percentual(misto.1)), (100, 141));
        assert_eq!(
            alvo.faixa_de_escala(&documento, None),
            Err(ErroPapel::DimensaoInvalida { pagina: 3 })
        );
        assert_eq!(alvo.faixa_de_escala(&[], None), Ok(None));
        let fora = IntervaloPaginas { inicio: 2, fim: 9 };
        assert!(alvo.faixa_de_escala(&documento, Some(fora)).is_err());
    }
}
