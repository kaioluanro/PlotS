//! O que a calibração guiada aprendeu de cada fila, além dos valores do perfil: de que lado a face
//! impressa entra e se o maço final é lido virado.
//!
//! Fica ao lado da entrada do perfil, e não dentro dela: um arquivo antigo continua igual, e um valor
//! ruim aqui não recusa o arquivo. Quem substitui o perfil da fila apaga o que está aqui, porque a
//! frase de recolocação passaria a descrever outro perfil.

use std::collections::BTreeMap;

use plots_core::calibracao::PerfilEmTeste;
use serde::{Deserialize, Serialize};

use crate::perfis::{EntradaPerfil, Perfis};

/// O que a fila aprendeu nas respostas da calibração.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Calibracao {
    /// A face impressa entra voltada para cima.
    pub face_impressa_entra_para_cima: bool,
    /// O maço final é lido depois de virado como página de livro.
    pub le_virando: bool,
}

const LADO_CIMA: &str = "cima";
const LADO_BAIXO: &str = "baixo";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CalibracaoArquivo {
    lado_na_entrada: String,
    le_virando: bool,
    /// Arquivo de antes desta chave não a traz: a calibração abre, sem valor para comparar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    driver_inverte: Option<bool>,
}

impl Perfis {
    /// O que a calibração guiada aprendeu desta fila, se o perfil dela veio de uma.
    pub fn calibracao(&self, fila: &str) -> Option<Calibracao> {
        self.calibracoes.get(fila).copied()
    }

    /// A compensacao do driver que estava gravada quando a calibracao foi feita. `None` sem
    /// calibracao, e em arquivo antigo que nao a guardou.
    pub fn driver_da_calibracao(&self, fila: &str) -> Option<bool> {
        if !self.calibracoes.contains_key(fila) {
            return None;
        }
        self.drivers_calibrados.get(fila).copied()
    }

    /// Grava o resultado de uma calibração. O apelido fica; o modelo, o texto do gesto e o modo
    /// automático saem, porque os valores deixaram de ser os deles.
    pub fn gravar_calibracao(&mut self, fila: &str, teste: PerfilEmTeste, calibracao: Calibracao) {
        let driver_inverte = teste.driver_inverte;
        let apelido = self.obter(fila).and_then(|entrada| entrada.apelido.clone());
        self.inserir(
            fila,
            EntradaPerfil {
                apelido,
                perfil: teste.perfil,
                gesto_texto: None,
                modelo: None,
                automatico: false,
                driver_inverte: teste.driver_inverte,
            },
        );
        self.calibracoes.insert(fila.to_owned(), calibracao);
        self.drivers_calibrados
            .insert(fila.to_owned(), driver_inverte);
    }
}

/// Lê o que estava no arquivo. Só vale para fila com perfil medido: um modelo ou o automático dizem
/// outro gesto, e quem discorda perde a calibração, não o perfil.
pub(crate) fn ler_do_arquivo(perfis: &mut Perfis, valores: BTreeMap<String, toml::Value>) {
    for (fila, valor) in valores {
        let Ok(lido) = valor.try_into::<CalibracaoArquivo>() else {
            continue;
        };
        let face_impressa_entra_para_cima = match lido.lado_na_entrada.as_str() {
            LADO_CIMA => true,
            LADO_BAIXO => false,
            _ => continue,
        };
        let medido = perfis
            .obter(&fila)
            .is_some_and(|entrada| entrada.modelo.is_none() && !entrada.automatico);
        if medido {
            if let Some(driver) = lido.driver_inverte {
                perfis.drivers_calibrados.insert(fila.clone(), driver);
            }
            perfis.calibracoes.insert(
                fila,
                Calibracao {
                    face_impressa_entra_para_cima,
                    le_virando: lido.le_virando,
                },
            );
        }
    }
}

pub(crate) fn para_o_arquivo(perfis: &Perfis) -> BTreeMap<String, toml::Value> {
    perfis
        .calibracoes
        .iter()
        .filter_map(|(fila, calibracao)| {
            let arquivo = CalibracaoArquivo {
                lado_na_entrada: if calibracao.face_impressa_entra_para_cima {
                    LADO_CIMA
                } else {
                    LADO_BAIXO
                }
                .to_owned(),
                le_virando: calibracao.le_virando,
                driver_inverte: perfis.drivers_calibrados.get(fila).copied(),
            };
            toml::Value::try_from(arquivo)
                .ok()
                .map(|valor| (fila.clone(), valor))
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::formato::{codificar, decodificar};
    use plots_core::{ModeloImpressora, ViradaMaco};
    use std::path::Path;

    const LIDA: Calibracao = Calibracao {
        face_impressa_entra_para_cima: true,
        le_virando: true,
    };

    fn testado() -> PerfilEmTeste {
        let mut teste = PerfilEmTeste::do_modelo(ModeloImpressora::Laser, true);
        teste.perfil.verso_girar_180 = !teste.perfil.verso_girar_180;
        teste.perfil.gesto_recolocar.virada = ViradaMaco::PelaLateral;
        teste
    }

    fn reler(perfis: &Perfis) -> Perfis {
        decodificar(&codificar(perfis).unwrap(), Path::new("perfis.toml")).unwrap()
    }

    #[test]
    fn o_resultado_vai_ao_arquivo_com_lado_virada_e_driver_e_volta_igual() {
        let mut perfis = Perfis::new();
        perfis.gravar_calibracao("Sala", testado(), LIDA);
        let lido = reler(&perfis);
        assert_eq!(lido.calibracao("Sala"), Some(LIDA));
        let entrada = lido.obter("Sala").unwrap();
        assert_eq!(entrada.perfil, testado().perfil);
        assert!(entrada.driver_inverte);
        assert_eq!(entrada.modelo, None);
        assert!(!entrada.automatico);
    }

    #[test]
    fn calibrar_tira_o_modelo_e_fica_com_o_apelido() {
        let mut perfis = Perfis::new();
        perfis.definir_automatico("Sala", Some(ModeloImpressora::Laser));
        let mut entrada = perfis.obter("Sala").unwrap().clone();
        entrada.apelido = Some("Do corredor".to_owned());
        perfis.inserir("Sala", entrada);
        perfis.gravar_calibracao("Sala", testado(), LIDA);
        let entrada = perfis.obter("Sala").unwrap();
        assert_eq!(entrada.apelido.as_deref(), Some("Do corredor"));
        assert_eq!(entrada.modelo, None);
        assert!(!entrada.automatico);
    }

    #[test]
    fn trocar_a_compensacao_do_driver_nao_apaga_a_calibracao() {
        let mut perfis = Perfis::new();
        perfis.gravar_calibracao("Sala", testado(), LIDA);
        assert!(perfis.definir_driver_inverte("Sala", false));
        let lido = reler(&perfis);
        assert_eq!(lido.calibracao("Sala"), Some(LIDA));
        assert!(!lido.obter("Sala").unwrap().driver_inverte);
    }

    #[test]
    fn escolher_um_modelo_depois_apaga_a_calibracao() {
        let mut perfis = Perfis::new();
        perfis.gravar_calibracao("Sala", testado(), LIDA);
        perfis.definir_modelo("Sala", ModeloImpressora::JatoDeTinta);
        assert_eq!(perfis.calibracao("Sala"), None);
        assert_eq!(reler(&perfis).calibracao("Sala"), None);
    }

    #[test]
    fn substituir_ou_remover_o_perfil_apaga_a_calibracao() {
        let mut perfis = Perfis::new();
        perfis.gravar_calibracao("Sala", testado(), LIDA);
        let entrada = perfis.obter("Sala").unwrap().clone();
        perfis.inserir("Sala", entrada);
        assert_eq!(perfis.calibracao("Sala"), None);

        perfis.gravar_calibracao("Sala", testado(), LIDA);
        perfis.remover("Sala");
        assert_eq!(perfis.calibracao("Sala"), None);
        assert!(!codificar(&perfis).unwrap().contains("calibracoes"));
    }

    #[test]
    fn sem_calibracao_o_arquivo_nao_ganha_a_secao() {
        let mut perfis = Perfis::new();
        perfis.definir_modelo("Sala", ModeloImpressora::Laser);
        assert!(!codificar(&perfis).unwrap().contains("calibracoes"));
    }

    #[test]
    fn valor_ruim_some_sem_recusar_o_arquivo_e_o_perfil_fica() {
        let mut perfis = Perfis::new();
        perfis.gravar_calibracao("Sala", testado(), LIDA);
        let texto = codificar(&perfis).unwrap();
        for ruim in [
            "lado_na_entrada = \"meio\"\nle_virando = true",
            "lado_na_entrada = 7\nle_virando = true",
            "lado_na_entrada = \"cima\"",
            "lado_na_entrada = \"cima\"\nle_virando = true\nextra = 1",
        ] {
            let trocado = texto.replace("lado_na_entrada = \"cima\"\nle_virando = true", ruim);
            assert_ne!(trocado, texto, "{ruim}");
            let lido = decodificar(&trocado, Path::new("perfis.toml")).unwrap();
            assert_eq!(lido.calibracao("Sala"), None, "{ruim}");
            assert!(lido.obter("Sala").is_some(), "{ruim}");
        }
    }

    #[test]
    fn calibracao_de_fila_com_modelo_ou_sem_perfil_nao_vale() {
        let mut perfis = Perfis::new();
        perfis.gravar_calibracao("Sala", testado(), LIDA);
        let sem_a_fila = codificar(&perfis)
            .unwrap()
            .replace("[filas.Sala]", "[filas.Outra]");
        let lido = decodificar(&sem_a_fila, Path::new("perfis.toml")).unwrap();
        assert_eq!(lido.calibracao("Sala"), None);

        let mut com_modelo = Perfis::new();
        com_modelo.definir_modelo("Sala", ModeloImpressora::Laser);
        let texto = format!(
            "{}\n[calibracoes.Sala]\nlado_na_entrada = \"cima\"\nle_virando = true\n",
            codificar(&com_modelo).unwrap()
        );
        let lido = decodificar(&texto, Path::new("perfis.toml")).unwrap();
        assert_eq!(lido.calibracao("Sala"), None);
        assert_eq!(
            lido.obter("Sala").unwrap().modelo,
            Some(ModeloImpressora::Laser)
        );
    }

    /// Arquivo de antes da compensacao guardada: a calibracao abre, e nao ha valor para comparar.
    const FILA_SALA: &str = "versao = 1\n\n[filas.Sala]\npassada1_inversa = true\npassada2_inversa = false\nverso_girar_180 = false\nvirar_maco = true\nborda_primeiro = \"curta\"\n";

    #[test]
    fn arquivo_antigo_abre_a_calibracao_sem_valor_do_driver() {
        let texto = format!(
            "{FILA_SALA}[calibracoes.Sala]\nlado_na_entrada = \"cima\"\nle_virando = true\n"
        );
        let lido = decodificar(&texto, Path::new("perfis.toml")).unwrap();
        assert_eq!(lido.calibracao("Sala"), Some(LIDA));
        assert_eq!(lido.driver_da_calibracao("Sala"), None);
    }

    #[test]
    fn o_driver_da_calibracao_vai_ao_arquivo_e_volta() {
        let mut perfis = Perfis::new();
        perfis.gravar_calibracao("Sala", testado(), LIDA);
        let lido = reler(&perfis);
        assert_eq!(lido.driver_da_calibracao("Sala"), Some(true));
        assert!(codificar(&perfis)
            .unwrap()
            .contains("driver_inverte = true"));
    }

    #[test]
    fn valor_ruim_do_driver_some_com_a_calibracao_sem_recusar_o_arquivo() {
        let texto = format!(
            "{FILA_SALA}[calibracoes.Sala]\nlado_na_entrada = \"cima\"\nle_virando = true\ndriver_inverte = \"sim\"\n"
        );
        let lido = decodificar(&texto, Path::new("perfis.toml")).unwrap();
        assert_eq!(lido.calibracao("Sala"), None);
        assert!(lido.obter("Sala").is_some());
    }

    #[test]
    fn calibrar_de_novo_troca_o_driver_guardado() {
        let mut perfis = Perfis::new();
        perfis.gravar_calibracao("Sala", testado(), LIDA);
        let mut outro = testado();
        outro.driver_inverte = false;
        perfis.gravar_calibracao("Sala", outro, LIDA);
        assert_eq!(perfis.driver_da_calibracao("Sala"), Some(false));
    }

    #[test]
    fn sem_calibracao_nao_ha_driver_guardado() {
        let mut perfis = Perfis::new();
        perfis.gravar_calibracao("Sala", testado(), LIDA);
        perfis.definir_modelo("Sala", ModeloImpressora::JatoDeTinta);
        assert_eq!(perfis.driver_da_calibracao("Sala"), None);
    }
}
