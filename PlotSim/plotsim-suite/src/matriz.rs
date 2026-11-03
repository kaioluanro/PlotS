//! A matriz de cenários: modos da impressora, páginas, lotes, formatos, filas e erros de propósito.
//!
//! O PlotS é a entrada: nada aqui usa o planejador dele. Os modos são hipóteses físicas, e o perfil
//! de cada um está escrito à mão.

use crate::documento::{Formato, GiroNoPdf, PaginaDeTeste};
use plotsim_core::{FaceDeSaida, ModoFisico, Saida};
use plotsim_physics::{Entrada, Lado, ModoImpressora, TopoDaImagem};

/// Páginas da matriz.
pub const PAGINAS: [u32; 8] = [1, 2, 3, 9, 10, 11, 24, 101];
/// Folhas por lote; `None` é o documento inteiro.
pub const LOTES: [Option<u32>; 4] = [Some(1), Some(5), Some(10), None];

/// Como a fila ganha o perfil.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PerfilDaFila {
    /// `plots perfil --modelo`.
    Modelo(&'static str),
    /// Um perfil salvo, sem modelo: é o que a calibração grava.
    Salvo {
        passada1_inversa: bool,
        passada2_inversa: bool,
        verso_girar_180: bool,
        virada: ViradaDoPerfil,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViradaDoPerfil {
    NaoVirar,
    PelaLateral,
    PelaBordaDeCima,
}

impl PerfilDaFila {
    /// O arquivo de perfis com esta fila, para o caso sem modelo.
    pub fn toml(&self, fila: &str) -> Option<String> {
        let Self::Salvo {
            passada1_inversa,
            passada2_inversa,
            verso_girar_180,
            virada,
        } = self
        else {
            return None;
        };
        let (virar, borda) = match virada {
            ViradaDoPerfil::NaoVirar => (false, ""),
            ViradaDoPerfil::PelaLateral => (true, ""),
            ViradaDoPerfil::PelaBordaDeCima => (true, "borda_virada = \"cima\"\n"),
        };
        Some(format!(
            "versao = 1\n\n[filas.\"{fila}\"]\npassada1_inversa = {passada1_inversa}\npassada2_inversa = {passada2_inversa}\nverso_girar_180 = {verso_girar_180}\nvirar_maco = {virar}\nborda_primeiro = \"curta\"\n{borda}"
        ))
    }
}

/// Um modo da impressora: como o papel anda e que perfil o PlotS recebe para ele.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModoDaFila {
    /// Entra no nome da fila.
    pub codigo: &'static str,
    pub nome: &'static str,
    pub modo: ModoImpressora,
    pub perfil: PerfilDaFila,
}

impl ModoDaFila {
    /// O que fica guardado no registro das filas.
    pub const fn modo_fisico(&self) -> ModoFisico {
        ModoFisico {
            saida: match self.modo.entrada {
                Entrada::Gaveta => Saida::Gaveta,
                Entrada::Traseira | Entrada::Manual => Saida::Traseira,
            },
            face: match self.modo.sai_com_face_para {
                Lado::Cima => FaceDeSaida::ParaCima,
                Lado::Baixo => FaceDeSaida::ParaBaixo,
            },
        }
    }
}

const fn modo(entrada: Entrada, sai: Lado) -> ModoImpressora {
    ModoImpressora {
        entrada,
        puxa: Lado::Cima,
        imprime_na_face_para: Lado::Cima,
        sai_com_face_para: sai,
        cai: Lado::Cima,
        topo_da_imagem: TopoDaImagem::NaArestaQueEntra,
    }
}

pub const JATO: ModoDaFila = ModoDaFila {
    codigo: "Jato",
    nome: "Traseira, face para cima (jato)",
    modo: modo(Entrada::Traseira, Lado::Cima),
    perfil: PerfilDaFila::Modelo("jato"),
};

pub const LASER: ModoDaFila = ModoDaFila {
    codigo: "Laser",
    nome: "Gaveta, face para baixo (laser)",
    modo: modo(Entrada::Gaveta, Lado::Baixo),
    perfil: PerfilDaFila::Modelo("laser"),
};

/// O caminho da laser, em uma entrada traseira, com o perfil que a calibração gravaria.
pub const TRASEIRA_FACE_BAIXO: ModoDaFila = ModoDaFila {
    codigo: "T-baixo",
    nome: "Traseira, face para baixo, perfil salvo",
    modo: modo(Entrada::Traseira, Lado::Baixo),
    perfil: PerfilDaFila::Salvo {
        passada1_inversa: true,
        passada2_inversa: false,
        verso_girar_180: true,
        virada: ViradaDoPerfil::NaoVirar,
    },
};

/// O caminho do jato, em uma gaveta, com o perfil que a calibração gravaria.
pub const GAVETA_FACE_CIMA: ModoDaFila = ModoDaFila {
    codigo: "G-cima",
    nome: "Gaveta, face para cima, perfil salvo",
    modo: modo(Entrada::Gaveta, Lado::Cima),
    perfil: PerfilDaFila::Salvo {
        passada1_inversa: true,
        passada2_inversa: true,
        verso_girar_180: true,
        virada: ViradaDoPerfil::PelaBordaDeCima,
    },
};

/// Caminho reto, virando como página de livro: o verso não precisa de giro.
pub const TRASEIRA_LIVRO: ModoDaFila = ModoDaFila {
    codigo: "T-livro",
    nome: "Traseira, face para cima, vira pela lateral, sem giro",
    modo: modo(Entrada::Traseira, Lado::Cima),
    perfil: PerfilDaFila::Salvo {
        passada1_inversa: true,
        passada2_inversa: true,
        verso_girar_180: false,
        virada: ViradaDoPerfil::PelaLateral,
    },
};

pub const TODOS_OS_MODOS: [ModoDaFila; 5] = [
    JATO,
    LASER,
    TRASEIRA_FACE_BAIXO,
    GAVETA_FACE_CIMA,
    TRASEIRA_LIVRO,
];

/// Papel do documento.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Papel {
    Um(Formato),
    /// A cada folha, o próximo formato da lista: uma folha nunca mistura dois papéis.
    Misto,
}

impl Papel {
    pub const fn nome(self) -> &'static str {
        match self {
            Self::Um(formato) => formato.nome(),
            Self::Misto => "misto",
        }
    }
}

/// Os formatos que a fila virtual aceita no misto. O Ofício de 216 × 330 mm não entra aqui: a fila
/// não tem esse papel, e ele fica só nos cenários de papel único.
const SEQUENCIA_DO_MISTO: [Formato; 3] = [Formato::A4, Formato::A5, Formato::Carta];
pub const PAPEIS: [Papel; 5] = [
    Papel::Um(Formato::A4),
    Papel::Um(Formato::A5),
    Papel::Um(Formato::Carta),
    Papel::Um(Formato::Oficio),
    Papel::Misto,
];

/// Como as filas do cenário dividem o documento.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reparto {
    Uma,
    /// Cada fila imprime o documento inteiro.
    Espelho,
    /// Cada fila imprime parte dos lotes, e os pacotes se juntam no fim.
    Divisao,
}

/// Erros de propósito: o PlotSim tem de acusar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErroDeProposito {
    /// O gesto cumprido troca a borda que entra.
    GestoComBordaTrocada,
    /// O gesto cumprido troca a face impressa.
    GestoComFaceTrocada,
    /// O gesto cumprido inverte a ordem do maço.
    GestoComOrdemInvertida,
    /// Uma folha some antes de o maço voltar à entrada.
    FolhaAMenos,
    /// A página 3 do PDF declara 180° de rotação.
    PaginaGirada180,
    /// A página 3 do PDF tem o desenho deitado.
    PaginaDeitada,
}

impl ErroDeProposito {
    pub const TODOS: [Self; 6] = [
        Self::GestoComBordaTrocada,
        Self::GestoComFaceTrocada,
        Self::GestoComOrdemInvertida,
        Self::FolhaAMenos,
        Self::PaginaGirada180,
        Self::PaginaDeitada,
    ];

    pub const fn nome(self) -> &'static str {
        match self {
            Self::GestoComBordaTrocada => "gesto errado: borda trocada",
            Self::GestoComFaceTrocada => "gesto errado: face trocada",
            Self::GestoComOrdemInvertida => "gesto errado: ordem invertida",
            Self::FolhaAMenos => "folha a menos",
            Self::PaginaGirada180 => "página girada 180° no PDF",
            Self::PaginaDeitada => "página deitada no PDF",
        }
    }
}

/// Página que recebe o erro nos documentos com página girada.
const PAGINA_COM_ERRO: u32 = 3;

/// Um cenário da matriz.
#[derive(Clone, Debug, PartialEq)]
pub struct Cenario {
    pub modos: Vec<ModoDaFila>,
    pub reparto: Reparto,
    pub paginas: u32,
    /// Folhas por lote; `None` é o documento inteiro.
    pub lote: Option<u32>,
    pub papel: Papel,
    pub erro: Option<ErroDeProposito>,
}

impl Cenario {
    /// Folhas por lote, como o PlotS recebe.
    pub const fn lote_pedido(&self) -> u32 {
        match self.lote {
            Some(folhas) => folhas,
            None => self.paginas.div_ceil(2),
        }
    }

    pub fn nome(&self) -> String {
        let modos: Vec<&str> = self.modos.iter().map(|modo| modo.codigo).collect();
        let lote = self
            .lote
            .map_or_else(|| "inteiro".to_string(), |folhas| folhas.to_string());
        let reparto = match self.reparto {
            Reparto::Uma => "",
            Reparto::Espelho => ", espelho",
            Reparto::Divisao => ", divisão",
        };
        let erro = self
            .erro
            .map_or_else(String::new, |erro| format!(", {}", erro.nome()));
        format!(
            "{}{reparto}, {} pág., lote {lote}, {}{erro}",
            modos.join(" + "),
            self.paginas,
            self.papel.nome(),
        )
    }

    /// O formato da página, em folhas de duas páginas.
    pub fn formato_da_pagina(&self, numero: u32) -> Formato {
        match self.papel {
            Papel::Um(formato) => formato,
            Papel::Misto => {
                let folha = usize::try_from(numero.saturating_sub(1) / 2).unwrap_or(0);
                SEQUENCIA_DO_MISTO
                    .get(folha % SEQUENCIA_DO_MISTO.len())
                    .copied()
                    .unwrap_or(Formato::A4)
            }
        }
    }

    /// As páginas do documento deste cenário.
    pub fn paginas_do_documento(&self) -> Vec<PaginaDeTeste> {
        (1..=self.paginas)
            .map(|numero| PaginaDeTeste {
                numero,
                formato: self.formato_da_pagina(numero),
                giro: match self.erro {
                    Some(ErroDeProposito::PaginaGirada180) if numero == PAGINA_COM_ERRO => {
                        GiroNoPdf::Declarado180
                    }
                    Some(ErroDeProposito::PaginaDeitada) if numero == PAGINA_COM_ERRO => {
                        GiroNoPdf::ConteudoDeitado
                    }
                    _ => GiroNoPdf::Nenhum,
                },
            })
            .collect()
    }
}

fn simples(modo: ModoDaFila, paginas: u32, lote: Option<u32>, papel: Papel) -> Cenario {
    Cenario {
        modos: vec![modo],
        reparto: Reparto::Uma,
        paginas,
        lote,
        papel,
        erro: None,
    }
}

fn com_erro(modo: ModoDaFila, erro: ErroDeProposito) -> Cenario {
    Cenario {
        erro: Some(erro),
        ..simples(modo, 6, Some(3), Papel::Um(Formato::A4))
    }
}

fn pares_de_filas() -> [[ModoDaFila; 2]; 3] {
    [[JATO, JATO], [LASER, LASER], [JATO, LASER]]
}

/// Recorte para o uso comum: cada dimensão aparece, sem o produto entre elas.
pub fn matriz_rapida() -> Vec<Cenario> {
    let a4 = Papel::Um(Formato::A4);
    let mut cenarios = Vec::new();
    for modo in [JATO, LASER] {
        cenarios.push(simples(modo, 1, Some(1), a4));
        cenarios.push(simples(modo, 2, None, a4));
        cenarios.push(simples(modo, 3, Some(1), a4));
        cenarios.push(simples(modo, 9, Some(5), a4));
        cenarios.push(simples(modo, 10, None, a4));
        cenarios.push(simples(modo, 11, Some(10), a4));
    }
    cenarios.push(simples(JATO, 24, Some(5), a4));
    cenarios.push(simples(LASER, 24, Some(10), a4));
    cenarios.push(simples(LASER, 101, Some(10), a4));
    for modo in [TRASEIRA_FACE_BAIXO, GAVETA_FACE_CIMA, TRASEIRA_LIVRO] {
        cenarios.push(simples(modo, 6, Some(2), a4));
        cenarios.push(simples(modo, 3, None, a4));
    }
    for papel in [
        Papel::Um(Formato::A5),
        Papel::Um(Formato::Carta),
        Papel::Um(Formato::Oficio),
        Papel::Misto,
    ] {
        cenarios.push(simples(JATO, 6, Some(2), papel));
        // Um misto por modelo: seis páginas bastam para os três formatos que ele usa.
        let paginas_da_laser = if papel == Papel::Misto { 6 } else { 7 };
        cenarios.push(simples(LASER, paginas_da_laser, Some(2), papel));
    }
    for reparto in [Reparto::Espelho, Reparto::Divisao] {
        cenarios.push(Cenario {
            modos: vec![JATO, LASER],
            reparto,
            ..simples(JATO, 6, Some(1), a4)
        });
    }
    cenarios.push(Cenario {
        modos: vec![LASER, LASER],
        reparto: Reparto::Divisao,
        ..simples(LASER, 9, Some(2), a4)
    });
    for erro in ErroDeProposito::TODOS {
        cenarios.push(com_erro(JATO, erro));
        cenarios.push(com_erro(LASER, erro));
    }
    cenarios
}

/// A matriz inteira: o produto de modo, páginas, lote e papel em uma fila; o de pares de filas,
/// páginas e lote em A4; e cada erro de propósito em cada modo. Um lote que já cabe o documento
/// inteiro repete o caso "inteiro", e fica só uma vez.
pub fn matriz_completa() -> Vec<Cenario> {
    let mut cenarios = Vec::new();
    let distintos = |paginas: u32| {
        let folhas = paginas.div_ceil(2);
        LOTES
            .into_iter()
            .filter(move |lote| lote.map_or(true, |tamanho| tamanho < folhas))
    };
    for modo in TODOS_OS_MODOS {
        for papel in PAPEIS {
            for paginas in PAGINAS {
                for lote in distintos(paginas) {
                    cenarios.push(simples(modo, paginas, lote, papel));
                }
            }
        }
    }
    for par in pares_de_filas() {
        for reparto in [Reparto::Espelho, Reparto::Divisao] {
            for paginas in PAGINAS {
                for lote in distintos(paginas) {
                    cenarios.push(Cenario {
                        modos: par.to_vec(),
                        reparto,
                        ..simples(JATO, paginas, lote, Papel::Um(Formato::A4))
                    });
                }
            }
        }
    }
    for modo in TODOS_OS_MODOS {
        for erro in ErroDeProposito::TODOS {
            cenarios.push(com_erro(modo, erro));
        }
    }
    cenarios
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn a_completa_cobre_as_dimensoes_e_nao_repete_cenario() {
        let cenarios = matriz_completa();
        let nomes: HashSet<String> = cenarios.iter().map(Cenario::nome).collect();
        assert_eq!(nomes.len(), cenarios.len());
        for paginas in PAGINAS {
            assert!(cenarios.iter().any(|c| c.paginas == paginas), "{paginas}");
        }
        for papel in PAPEIS {
            assert!(cenarios.iter().any(|c| c.papel == papel), "{papel:?}");
        }
        for lote in LOTES {
            assert!(cenarios.iter().any(|c| c.lote == lote), "{lote:?}");
        }
        for modo in TODOS_OS_MODOS {
            assert!(cenarios.iter().any(|c| c.modos.contains(&modo)), "{modo:?}");
        }
        for erro in ErroDeProposito::TODOS {
            assert!(cenarios.iter().any(|c| c.erro == Some(erro)), "{erro:?}");
        }
        for reparto in [Reparto::Uma, Reparto::Espelho, Reparto::Divisao] {
            assert!(cenarios.iter().any(|c| c.reparto == reparto), "{reparto:?}");
        }
    }

    #[test]
    fn a_rapida_passa_por_cada_dimensao_uma_vez_ao_menos_e_e_bem_menor() {
        let rapida = matriz_rapida();
        let completa = matriz_completa();
        assert!(rapida.len() * 10 < completa.len());
        let nomes: HashSet<String> = rapida.iter().map(Cenario::nome).collect();
        assert_eq!(nomes.len(), rapida.len());
        for paginas in PAGINAS {
            assert!(rapida.iter().any(|c| c.paginas == paginas), "{paginas}");
        }
        for papel in PAPEIS {
            assert!(rapida.iter().any(|c| c.papel == papel), "{papel:?}");
        }
        for modo in TODOS_OS_MODOS {
            assert!(rapida.iter().any(|c| c.modos.contains(&modo)), "{modo:?}");
        }
        for erro in ErroDeProposito::TODOS {
            assert!(rapida.iter().any(|c| c.erro == Some(erro)), "{erro:?}");
        }
        for reparto in [Reparto::Uma, Reparto::Espelho, Reparto::Divisao] {
            assert!(rapida.iter().any(|c| c.reparto == reparto), "{reparto:?}");
        }
    }

    #[test]
    fn no_misto_uma_folha_nunca_junta_dois_papeis() {
        let cenario = simples(JATO, 24, Some(5), Papel::Misto);
        for frente in (1..=23).step_by(2) {
            assert_eq!(
                cenario.formato_da_pagina(frente),
                cenario.formato_da_pagina(frente + 1)
            );
        }
        let formatos: HashSet<&str> = (1..=24)
            .map(|n| cenario.formato_da_pagina(n).nome())
            .collect();
        assert_eq!(formatos.len(), 3);
    }

    #[test]
    fn o_misto_nao_usa_o_oficio() {
        let cenario = simples(JATO, 40, Some(5), Papel::Misto);
        for numero in 1..=40 {
            assert_ne!(
                cenario.formato_da_pagina(numero),
                Formato::Oficio,
                "{numero}"
            );
        }
    }

    #[test]
    fn a_rapida_tem_um_misto_por_modelo() {
        let rapida = matriz_rapida();
        let mistos_de = |modo: ModoDaFila| {
            rapida
                .iter()
                .filter(|c| c.papel == Papel::Misto && c.modos == vec![modo])
                .count()
        };
        assert_eq!(mistos_de(JATO), 1);
        assert_eq!(mistos_de(LASER), 1);
    }

    #[test]
    fn o_erro_de_pagina_so_toca_a_pagina_combinada() {
        let cenario = com_erro(JATO, ErroDeProposito::PaginaDeitada);
        let giros: Vec<GiroNoPdf> = cenario
            .paginas_do_documento()
            .iter()
            .map(|pagina| pagina.giro)
            .collect();
        assert_eq!(giros[2], GiroNoPdf::ConteudoDeitado);
        assert_eq!(
            giros
                .iter()
                .filter(|giro| **giro == GiroNoPdf::Nenhum)
                .count(),
            5
        );
    }

    #[test]
    fn o_arquivo_de_perfil_salvo_tem_a_virada_certa() {
        let toml = GAVETA_FACE_CIMA.perfil.toml("PlotSim — X").unwrap();
        assert!(toml.contains("[filas.\"PlotSim — X\"]"));
        assert!(toml.contains("virar_maco = true"));
        assert!(toml.contains("borda_virada = \"cima\""));
        let toml = TRASEIRA_LIVRO.perfil.toml("PlotSim — X").unwrap();
        assert!(toml.contains("virar_maco = true"));
        assert!(!toml.contains("borda_virada"));
        assert!(JATO.perfil.toml("PlotSim — X").is_none());
    }
}
