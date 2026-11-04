//! Erros como a tela os recebe: um código traduzido por ela e, quando há, o detalhe que veio de
//! fora, como o texto do próprio Windows.

use plotsim_core::leitor::ErroLeitura;
use plotsim_core::recepcao::ErroRecepcao;
use plotsim_core::{ErroFilas, ErroNome, ErroSistema};
use plotsim_physics::{Entrada, ErroGesto};
use serde::Serialize;

use crate::bandeja::{Aviso, RecusaDoGesto};
use crate::desenho::ErroDesenho;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ErroTela {
    pub codigo: &'static str,
    pub detalhe: Option<String>,
}

impl ErroTela {
    pub fn so(codigo: &'static str) -> Self {
        Self {
            codigo,
            detalhe: None,
        }
    }

    pub fn com(codigo: &'static str, detalhe: impl Into<String>) -> Self {
        Self {
            codigo,
            detalhe: Some(detalhe.into()),
        }
    }
}

fn do_sistema(erro: &ErroSistema) -> ErroTela {
    match erro {
        ErroSistema::PrecisaAdministrador => ErroTela::so("sem_permissao"),
        ErroSistema::Recusado { mensagem, .. } => ErroTela::com("windows_recusou", mensagem),
        ErroSistema::TempoEsgotado => ErroTela::so("tempo_esgotado"),
        ErroSistema::Processo(detalhe) => ErroTela::com("processo", detalhe),
        ErroSistema::Leitura(detalhe) => ErroTela::com("leitura_filas", detalhe),
        ErroSistema::NaoSuportado => ErroTela::so("nao_suportado"),
    }
}

fn do_nome(erro: &ErroNome) -> ErroTela {
    match erro {
        ErroNome::Vazio => ErroTela::so("nome_vazio"),
        ErroNome::Longo => ErroTela::so("nome_longo"),
        ErroNome::Caractere(c) => ErroTela::com("nome_caractere", c.to_string()),
        ErroNome::Pontas => ErroTela::so("nome_pontas"),
        ErroNome::SemPrefixo => ErroTela::so("nome_vazio"),
    }
}

pub fn das_filas(erro: &ErroFilas) -> ErroTela {
    match erro {
        ErroFilas::Nome(nome) => do_nome(nome),
        ErroFilas::Pasta(pasta) => ErroTela::com("pasta", pasta.to_string()),
        ErroFilas::Registro(registro) => ErroTela::com("registro", registro.to_string()),
        ErroFilas::Sistema(sistema) => do_sistema(sistema),
        ErroFilas::FilaAlheia { fila, .. } => ErroTela::com("fila_alheia", fila),
        ErroFilas::PastaDaPorta(pasta, _) => {
            ErroTela::com("pasta_da_porta", pasta.display().to_string())
        }
        ErroFilas::SemEspaco => ErroTela::so("sem_espaco"),
    }
}

fn nome_da_entrada(entrada: Entrada) -> &'static str {
    match entrada {
        Entrada::Traseira => "entrada_diferente_traseira",
        Entrada::Gaveta => "entrada_diferente_gaveta",
        Entrada::Manual => "entrada_diferente_manual",
    }
}

pub fn do_gesto(erro: RecusaDoGesto) -> ErroTela {
    match erro {
        RecusaDoGesto::NadaParaRecolocar => ErroTela::so("nada_para_recolocar"),
        RecusaDoGesto::Fisica(ErroGesto::EntradaDiferente { da_maquina, .. }) => {
            ErroTela::so(nome_da_entrada(da_maquina))
        }
        RecusaDoGesto::Fisica(ErroGesto::FaceSemSentido { .. }) => ErroTela::so("face_sem_sentido"),
    }
}

pub fn aviso_de_leitura(erro: &ErroLeitura) -> Aviso {
    let (codigo, detalhe) = match erro {
        ErroLeitura::Grande { .. } | ErroLeitura::TextoGrande { .. } => ("pdf_grande", None),
        ErroLeitura::Criptografado => ("pdf_protegido", None),
        ErroLeitura::Biblioteca(_) => ("biblioteca_pdf", None),
        outro => ("pdf_ilegivel", Some(outro.to_string())),
    };
    Aviso::novo(codigo, detalhe)
}

pub fn aviso_de_recepcao(erro: &str) -> Aviso {
    Aviso::novo("erro_de_fila", Some(erro.to_string()))
}

pub fn da_recepcao(erro: &ErroRecepcao) -> ErroTela {
    ErroTela::com("erro_de_fila", erro.to_string())
}

pub fn do_desenho(erro: &ErroDesenho) -> ErroTela {
    match erro {
        ErroDesenho::Biblioteca(_) => ErroTela::so("biblioteca_pdf"),
        ErroDesenho::Pdf(detalhe) => ErroTela::com("pdf_ilegivel", detalhe),
        ErroDesenho::PaginaAusente | ErroDesenho::Medidas => ErroTela::so("pdf_ilegivel"),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use plotsim_physics::FaceNaEntrada;

    #[test]
    fn falta_de_permissao_tem_codigo_proprio() {
        let erro = ErroFilas::Sistema(ErroSistema::PrecisaAdministrador);
        assert_eq!(das_filas(&erro).codigo, "sem_permissao");
    }

    #[test]
    fn texto_do_windows_segue_como_detalhe() {
        let erro = ErroFilas::Sistema(ErroSistema::Recusado {
            codigo: "x".to_string(),
            mensagem: "spooler parado".to_string(),
        });
        assert_eq!(
            das_filas(&erro),
            ErroTela::com("windows_recusou", "spooler parado")
        );
    }

    #[test]
    fn fila_alheia_diz_qual_e() {
        let erro = ErroFilas::FilaAlheia {
            fila: "PlotSim — X".to_string(),
            driver: "d".to_string(),
            porta: "p".to_string(),
        };
        assert_eq!(das_filas(&erro).detalhe.as_deref(), Some("PlotSim — X"));
    }

    #[test]
    fn nome_recusado_diz_o_caractere() {
        let erro = ErroFilas::Nome(ErroNome::Caractere('/'));
        assert_eq!(das_filas(&erro), ErroTela::com("nome_caractere", "/"));
    }

    #[test]
    fn gesto_recusado_diz_a_entrada_da_maquina() {
        let erro = RecusaDoGesto::Fisica(ErroGesto::EntradaDiferente {
            pedida: Entrada::Gaveta,
            da_maquina: Entrada::Traseira,
        });
        assert_eq!(do_gesto(erro).codigo, "entrada_diferente_traseira");
        let face = RecusaDoGesto::Fisica(ErroGesto::FaceSemSentido {
            entrada: Entrada::Traseira,
            face: FaceNaEntrada::ParaCima,
        });
        assert_eq!(do_gesto(face).codigo, "face_sem_sentido");
    }

    #[test]
    fn pdf_ruim_tem_aviso_proprio_e_protegido_nao_passa_por_ilegivel() {
        let ruim = aviso_de_leitura(&ErroLeitura::Pdf("sem xref".to_string()));
        assert_eq!(ruim.codigo, "pdf_ilegivel");
        assert_eq!(
            aviso_de_leitura(&ErroLeitura::Criptografado).codigo,
            "pdf_protegido"
        );
    }
}
