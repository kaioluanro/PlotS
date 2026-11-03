//! A bancada: entrega as folhas recebidas ao modelo físico, cumpre o gesto como veio e confere.
//!
//! Não sabe nada do Windows: quem chama passa as páginas lidas e as mensagens. Os erros de propósito
//! entram aqui, entre o que o PlotS pediu e o que a mão faz.

use crate::matriz::ErroDeProposito;
use plotsim_core::canal::LeituraPedida;
use plotsim_core::marca::{ErroMarca, MarcaLida};
use plotsim_physics::{
    conferir, ler_maco, BordaQueEntra, Entrada, Envio, FaceNaEntrada, Gesto, Giro, Impressora,
    Lado, Leitura, MacoLido, ModoImpressora, Motivo, Pilha,
};
use std::collections::BTreeMap;

/// Converte o que o leitor viu em uma página enviada. Página sem marca é erro, nunca palpite; em
/// branco é folha que passa em branco, e a conferência acusa se ali deveria haver página.
pub fn envio_da_leitura(leitura: &Result<MarcaLida, ErroMarca>) -> Result<Envio, String> {
    match leitura {
        Ok(marca) => {
            let giro = match marca.rotacao {
                0 => Giro::Nenhum,
                90 => Giro::Direita90,
                180 => Giro::Meia180,
                270 => Giro::Esquerda270,
                outra => return Err(format!("rotação de marca desconhecida: {outra}")),
            };
            Ok(Envio {
                pagina: Some(marca.pagina),
                giro,
            })
        }
        Err(ErroMarca::PaginaEmBranco) => Ok(Envio::branco()),
        Err(erro) => Err(erro.to_string()),
    }
}

/// Em que ponto a bancada está.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Etapa {
    /// Nada pendente: o próximo trabalho é uma frente.
    Parada,
    /// O gesto foi cumprido; o próximo trabalho é o verso deste lote.
    EsperandoVerso { lote: u32 },
    /// A última mensagem chegou.
    Encerrada,
}

/// Uma impressora física com o maço de cada lote.
pub struct Bancada {
    modo: ModoImpressora,
    erro: Option<ErroDeProposito>,
    impressora: Option<Impressora>,
    etapa: Etapa,
    lotes: BTreeMap<u32, Pilha>,
    leitura: Option<Leitura>,
    faltas: Vec<String>,
}

/// O mesmo gesto com o erro de propósito aplicado.
fn gesto_adulterado(gesto: Gesto, erro: Option<ErroDeProposito>) -> Gesto {
    match erro {
        Some(ErroDeProposito::GestoComBordaTrocada) => Gesto {
            borda_que_entra: match gesto.borda_que_entra {
                BordaQueEntra::Topo => BordaQueEntra::Base,
                BordaQueEntra::Base => BordaQueEntra::Topo,
            },
            ..gesto
        },
        Some(ErroDeProposito::GestoComFaceTrocada) => Gesto {
            face_impressa: match gesto.face_impressa {
                FaceNaEntrada::ParaFrente => FaceNaEntrada::ParaTras,
                FaceNaEntrada::ParaTras => FaceNaEntrada::ParaFrente,
                FaceNaEntrada::ParaCima => FaceNaEntrada::ParaBaixo,
                FaceNaEntrada::ParaBaixo => FaceNaEntrada::ParaCima,
            },
            ..gesto
        },
        Some(ErroDeProposito::GestoComOrdemInvertida) => Gesto {
            inverter_ordem: !gesto.inverter_ordem,
            ..gesto
        },
        Some(
            ErroDeProposito::FolhaAMenos
            | ErroDeProposito::PaginaGirada180
            | ErroDeProposito::PaginaDeitada,
        )
        | None => gesto,
    }
}

/// Sem modelo, o PlotS não afirma a entrada: a pessoa usa a da máquina, e a face é dita no jeito
/// daquela entrada.
fn gesto_na_entrada_da_maquina(gesto: Gesto, entrada: Entrada) -> Gesto {
    if gesto.entrada != Entrada::Manual {
        return gesto;
    }
    let face_impressa = match (entrada, gesto.face_impressa) {
        (Entrada::Traseira, FaceNaEntrada::ParaCima) => FaceNaEntrada::ParaFrente,
        (Entrada::Traseira, FaceNaEntrada::ParaBaixo) => FaceNaEntrada::ParaTras,
        (_, face) => face,
    };
    Gesto {
        entrada,
        face_impressa,
        ..gesto
    }
}

impl Bancada {
    pub const fn nova(modo: ModoImpressora, erro: Option<ErroDeProposito>) -> Self {
        Self {
            modo,
            erro,
            impressora: None,
            etapa: Etapa::Parada,
            lotes: BTreeMap::new(),
            leitura: None,
            faltas: Vec::new(),
        }
    }

    pub const fn etapa(&self) -> Etapa {
        self.etapa
    }

    /// O que a bancada notou que o modelo físico não conta como motivo da folha.
    pub fn faltas(&self) -> &[String] {
        &self.faltas
    }

    /// A frente de um lote e o gesto que veio junto. Devolve erro para o PlotS quando ele pede fora
    /// de ordem ou um gesto que a máquina não tem.
    pub fn frente_e_gesto(
        &mut self,
        lote: u32,
        frente: &[Envio],
        gesto: Gesto,
    ) -> Result<(), String> {
        if self.etapa != Etapa::Parada {
            return Err(format!("o gesto do lote {lote} chegou fora de ordem"));
        }
        if self.lotes.contains_key(&lote) {
            return Err(format!("o lote {lote} chegou duas vezes"));
        }
        let mut impressora = Impressora::nova(self.modo, frente.len());
        let resumo = impressora.imprimir(frente);
        if !resumo.sem_folha.is_empty() {
            self.faltas.push(format!(
                "lote {lote}: {} páginas da frente sem folha",
                resumo.sem_folha.len()
            ));
        }
        if self.erro == Some(ErroDeProposito::FolhaAMenos) {
            impressora.saida_mut().retirar_de_cima(0);
        }
        let gesto = gesto_adulterado(
            gesto_na_entrada_da_maquina(gesto, self.modo.entrada),
            self.erro,
        );
        impressora
            .recolocar(gesto)
            .map_err(|erro| format!("a máquina não cumpre o gesto do lote {lote}: {erro:?}"))?;
        self.impressora = Some(impressora);
        self.etapa = Etapa::EsperandoVerso { lote };
        Ok(())
    }

    /// O verso do lote em andamento.
    pub fn verso(&mut self, verso: &[Envio]) -> Result<(), String> {
        let Etapa::EsperandoVerso { lote } = self.etapa else {
            return Err("chegou um verso sem frente".to_string());
        };
        let Some(mut impressora) = self.impressora.take() else {
            return Err("chegou um verso sem frente".to_string());
        };
        let resumo = impressora.imprimir(verso);
        if !resumo.sem_folha.is_empty() {
            self.faltas.push(format!(
                "lote {lote}: {} páginas do verso sem folha na entrada",
                resumo.sem_folha.len()
            ));
        }
        self.lotes.insert(lote, impressora.retirar_saida());
        self.etapa = Etapa::Parada;
        Ok(())
    }

    /// O PlotS terminou. `SemInstrucao` é o perfil sem modelo: a pessoa lê como a máquina deixa,
    /// com a face impressa para cima.
    pub fn encerrar(&mut self, lotes: u32, leitura: LeituraPedida) -> Result<(), String> {
        if self.etapa != Etapa::Parada {
            return Err("o fim chegou com um lote em andamento".to_string());
        }
        if usize::try_from(lotes).ok() != Some(self.lotes.len()) {
            self.faltas.push(format!(
                "o PlotS diz {lotes} lotes, e chegaram {}",
                self.lotes.len()
            ));
        }
        self.leitura = Some(match leitura {
            LeituraPedida::ComoSaiu => Leitura::ComoSaiu,
            LeituraPedida::Virando => Leitura::VirandoOMaco,
            LeituraPedida::SemInstrucao => match self.modo.sai_com_face_para {
                Lado::Cima => Leitura::ComoSaiu,
                Lado::Baixo => Leitura::VirandoOMaco,
            },
        });
        self.etapa = Etapa::Encerrada;
        Ok(())
    }

    /// Os pacotes de cada lote, lidos como a pessoa os pega.
    pub fn pacotes(&self) -> BTreeMap<u32, MacoLido> {
        let leitura = self.leitura.unwrap_or(match self.modo.sai_com_face_para {
            Lado::Cima => Leitura::ComoSaiu,
            Lado::Baixo => Leitura::VirandoOMaco,
        });
        self.lotes
            .iter()
            .map(|(lote, pilha)| (*lote, ler_maco(pilha, leitura)))
            .collect()
    }
}

/// Junta os pacotes na ordem do documento: cada lote vai embaixo do anterior.
pub fn juntar(pacotes: impl IntoIterator<Item = MacoLido>) -> MacoLido {
    let mut maco = MacoLido::default();
    for pacote in pacotes {
        maco.folhas.extend(pacote.folhas);
    }
    maco
}

/// O erro de propósito foi acusado do jeito esperado?
pub fn erro_foi_acusado(erro: ErroDeProposito, motivos: &[Motivo], faltas: &[String]) -> bool {
    motivos.iter().any(|motivo| match erro {
        ErroDeProposito::GestoComBordaTrocada | ErroDeProposito::PaginaGirada180 => matches!(
            motivo,
            Motivo::VersoDeCabecaParaBaixo { .. } | Motivo::FolhaDeCabecaParaBaixo { .. }
        ),
        ErroDeProposito::GestoComFaceTrocada => {
            matches!(motivo, Motivo::ImpressaoSobreImpressao { .. })
        }
        ErroDeProposito::GestoComOrdemInvertida => matches!(
            motivo,
            Motivo::FrenteTrocada { .. } | Motivo::VersoTrocado { .. }
        ),
        ErroDeProposito::FolhaAMenos => matches!(motivo, Motivo::FolhaAMenos { .. }),
        ErroDeProposito::PaginaDeitada => matches!(motivo, Motivo::PaginaGirada { .. }),
    }) || (erro == ErroDeProposito::FolhaAMenos && !faltas.is_empty())
}

/// Confere um maço contra o total de páginas.
pub fn conferir_maco(maco: &MacoLido, total: u32) -> Vec<Motivo> {
    conferir(maco, total)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::matriz::{GAVETA_FACE_CIMA, JATO, LASER, TRASEIRA_FACE_BAIXO, TRASEIRA_LIVRO};
    use plotsim_physics::Motivo;

    const GESTO_JATO: Gesto = Gesto {
        entrada: Entrada::Traseira,
        face_impressa: FaceNaEntrada::ParaTras,
        borda_que_entra: BordaQueEntra::Base,
        inverter_ordem: false,
    };
    const GESTO_LASER: Gesto = Gesto {
        entrada: Entrada::Gaveta,
        face_impressa: FaceNaEntrada::ParaBaixo,
        borda_que_entra: BordaQueEntra::Base,
        inverter_ordem: false,
    };

    fn pagina(n: u32) -> Envio {
        Envio::pagina(n)
    }

    fn girada(n: u32) -> Envio {
        Envio::girada(n, Giro::Meia180)
    }

    fn rodar(
        modo: ModoImpressora,
        gesto: Gesto,
        erro: Option<ErroDeProposito>,
        leitura: LeituraPedida,
        frente: &[Envio],
        verso: &[Envio],
    ) -> (Vec<Motivo>, Vec<String>) {
        let mut bancada = Bancada::nova(modo, erro);
        bancada.frente_e_gesto(1, frente, gesto).unwrap();
        bancada.verso(verso).unwrap();
        bancada.encerrar(1, leitura).unwrap();
        let maco = juntar(bancada.pacotes().into_values());
        (conferir_maco(&maco, 6), bancada.faltas().to_vec())
    }

    const FRENTE: [Envio; 3] = [Envio::pagina(6), Envio::pagina(4), Envio::pagina(2)];

    #[test]
    fn jato_certo_passa() {
        let (motivos, faltas) = rodar(
            JATO.modo,
            GESTO_JATO,
            None,
            LeituraPedida::ComoSaiu,
            &FRENTE,
            &[girada(5), girada(3), girada(1)],
        );
        assert_eq!(motivos, vec![]);
        assert!(faltas.is_empty());
    }

    #[test]
    fn laser_certo_passa() {
        let (motivos, _) = rodar(
            LASER.modo,
            GESTO_LASER,
            None,
            LeituraPedida::Virando,
            &FRENTE,
            &[girada(1), girada(3), girada(5)],
        );
        assert_eq!(motivos, vec![]);
    }

    #[test]
    fn perfil_salvo_usa_a_entrada_da_maquina() {
        let gesto = Gesto {
            entrada: Entrada::Manual,
            face_impressa: FaceNaEntrada::ParaBaixo,
            ..GESTO_LASER
        };
        let (motivos, _) = rodar(
            TRASEIRA_FACE_BAIXO.modo,
            gesto,
            None,
            LeituraPedida::SemInstrucao,
            &FRENTE,
            &[girada(1), girada(3), girada(5)],
        );
        assert_eq!(motivos, vec![]);
        let gesto = Gesto {
            entrada: Entrada::Manual,
            ..GESTO_LASER
        };
        let (motivos, _) = rodar(
            GAVETA_FACE_CIMA.modo,
            gesto,
            None,
            LeituraPedida::SemInstrucao,
            &FRENTE,
            &[girada(5), girada(3), girada(1)],
        );
        assert_eq!(motivos, vec![]);
    }

    /// O perfil escrito à mão do modo que vira como livro fecha no modelo físico, sem giro.
    #[test]
    fn perfil_que_vira_como_livro_fecha_sem_giro() {
        let gesto = Gesto {
            entrada: Entrada::Manual,
            face_impressa: FaceNaEntrada::ParaBaixo,
            borda_que_entra: BordaQueEntra::Topo,
            inverter_ordem: false,
        };
        let (motivos, _) = rodar(
            TRASEIRA_LIVRO.modo,
            gesto,
            None,
            LeituraPedida::SemInstrucao,
            &FRENTE,
            &[pagina(5), pagina(3), pagina(1)],
        );
        assert_eq!(motivos, vec![]);
    }

    fn acusa(erro: ErroDeProposito, frente: &[Envio], verso: &[Envio]) {
        let (motivos, faltas) = rodar(
            JATO.modo,
            GESTO_JATO,
            Some(erro),
            LeituraPedida::ComoSaiu,
            frente,
            verso,
        );
        assert!(
            erro_foi_acusado(erro, &motivos, &faltas),
            "{erro:?}: {motivos:?} {faltas:?}"
        );
    }

    #[test]
    fn cada_erro_de_proposito_e_acusado() {
        let verso = [girada(5), girada(3), girada(1)];
        acusa(ErroDeProposito::GestoComBordaTrocada, &FRENTE, &verso);
        acusa(ErroDeProposito::GestoComFaceTrocada, &FRENTE, &verso);
        acusa(ErroDeProposito::GestoComOrdemInvertida, &FRENTE, &verso);
        acusa(ErroDeProposito::FolhaAMenos, &FRENTE, &verso);
        acusa(
            ErroDeProposito::PaginaGirada180,
            &FRENTE,
            &[girada(5), pagina(3), girada(1)],
        );
        acusa(
            ErroDeProposito::PaginaDeitada,
            &FRENTE,
            &[girada(5), Envio::girada(3, Giro::Direita90), girada(1)],
        );
    }

    #[test]
    fn sem_erro_nenhum_acusado_nao_vale() {
        for erro in ErroDeProposito_todos() {
            assert!(!erro_foi_acusado(erro, &[], &[]), "{erro:?}");
        }
    }

    #[allow(non_snake_case)]
    fn ErroDeProposito_todos() -> [ErroDeProposito; 6] {
        ErroDeProposito::TODOS
    }

    #[test]
    fn mensagens_fora_de_ordem_voltam_como_erro_sem_travar() {
        let mut bancada = Bancada::nova(JATO.modo, None);
        assert!(bancada.verso(&FRENTE).is_err());
        bancada.frente_e_gesto(1, &FRENTE, GESTO_JATO).unwrap();
        assert!(bancada.frente_e_gesto(2, &FRENTE, GESTO_JATO).is_err());
        assert!(bancada.encerrar(1, LeituraPedida::ComoSaiu).is_err());
        bancada.verso(&FRENTE).unwrap();
        assert!(bancada.frente_e_gesto(1, &FRENTE, GESTO_JATO).is_err());
    }

    #[test]
    fn gesto_que_a_maquina_nao_tem_e_recusado() {
        let mut bancada = Bancada::nova(JATO.modo, None);
        let erro = bancada.frente_e_gesto(1, &FRENTE, GESTO_LASER).unwrap_err();
        assert!(erro.contains("não cumpre"), "{erro}");
    }

    #[test]
    fn total_de_lotes_diferente_vira_falta() {
        let mut bancada = Bancada::nova(JATO.modo, None);
        bancada.encerrar(3, LeituraPedida::ComoSaiu).unwrap();
        assert_eq!(bancada.faltas().len(), 1);
    }

    #[test]
    fn leitura_da_marca_vira_envio_e_sem_marca_vira_erro() {
        let marca = |rotacao| Ok(MarcaLida { pagina: 7, rotacao });
        assert_eq!(envio_da_leitura(&marca(0)), Ok(pagina(7)));
        assert_eq!(envio_da_leitura(&marca(180)), Ok(girada(7)));
        assert_eq!(
            envio_da_leitura(&marca(90)),
            Ok(Envio::girada(7, Giro::Direita90))
        );
        assert_eq!(
            envio_da_leitura(&marca(270)),
            Ok(Envio::girada(7, Giro::Esquerda270))
        );
        assert!(envio_da_leitura(&marca(45)).is_err());
        assert_eq!(
            envio_da_leitura(&Err(ErroMarca::PaginaEmBranco)),
            Ok(Envio::branco())
        );
        assert!(envio_da_leitura(&Err(ErroMarca::SemMarca)).is_err());
        assert!(envio_da_leitura(&Err(ErroMarca::MarcaInvalida)).is_err());
    }
}
