//! Comandos que a interface usa para conduzir os trabalhos de impressao.

use crate::documento::{self, ErroDocumento};
use crate::historico::{self, CabecalhoDoEnvio, Diario, Diarios};
use crate::impressoras;
use crate::inversao;
use crate::tempo_de_impressao::RelogioDoSistema;
use crate::trabalhos::{
    self, Comando, Emissor, Gerente, GestoDaImpressora, Identidade, Motivo, Pedido, Recolocar,
    ResumoDoEnvio,
};
use plots_core::{IntervaloPaginas, ModeloImpressora};
use plots_perfis::historico::FaceDoRegistro;
use plots_perfis::{ErroPerfis, Perfis};
use plots_windows::plots_print::job::Alcance;
use plots_windows::plots_print::opcoes::IdiomaDaMarca;
use serde::Deserialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// O trecho como a interface manda. O nucleo nao conhece o formato da mensagem, e numero negativo,
/// fracionario ou grande demais ja para aqui, sem chegar ao plano.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct IntervaloRecebido {
    inicio: usize,
    fim: usize,
}

impl From<IntervaloRecebido> for IntervaloPaginas {
    fn from(recebido: IntervaloRecebido) -> Self {
        Self {
            inicio: recebido.inicio,
            fim: recebido.fim,
        }
    }
}

/// Em que idioma a marca da folha de controle sai no papel; ausente, nao ha marca.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum MarcaRecebida {
    #[serde(rename = "pt-BR")]
    Portugues,
    #[serde(rename = "en")]
    Ingles,
}

impl From<MarcaRecebida> for IdiomaDaMarca {
    fn from(marca: MarcaRecebida) -> Self {
        match marca {
            MarcaRecebida::Portugues => Self::Portugues,
            MarcaRecebida::Ingles => Self::Ingles,
        }
    }
}

/// Le o PDF para a confirmacao dizer paginas, lotes e papeis antes de qualquer envio.
#[tauri::command]
pub async fn resumo_do_envio(
    caminho: String,
    intervalo: Option<IntervaloRecebido>,
    impressoras: Vec<String>,
    papel_de_ajuste: Option<String>,
    copias: Option<u32>,
) -> Result<ResumoDoEnvio, ErroDocumento> {
    let falha = documento::falha_inesperada(&caminho);
    tauri::async_runtime::spawn_blocking(move || {
        let papel = papel_de_ajuste
            .as_deref()
            .map(|codigo| {
                plots_core::ajuste::PapelDeAjuste::de_codigo(codigo)
                    .ok_or_else(|| documento::falha_inesperada(&caminho))
            })
            .transpose()?;
        let copias = copias.map_or(1, |valor| usize::try_from(valor).unwrap_or(usize::MAX));
        let mut resumo = trabalhos::resumo_do_envio_com_copias(
            &caminho,
            intervalo.map(Into::into),
            papel,
            copias,
        )?;
        let perfis = impressoras::ler_perfis_do_sistema();
        resumo.comparacao = rota_comum(&impressoras, perfis.as_ref().ok(), &resumo.fluxos)
            .and_then(|automatico| historico::comparacao_do_envio(&impressoras, automatico));
        resumo.gestos = impressoras
            .iter()
            .map(|impressora| GestoDaImpressora {
                impressora: impressora.clone(),
                recolocar: gesto_da_impressora(impressora, &perfis),
            })
            .collect();
        Ok(resumo)
    })
    .await
    .unwrap_or(Err(falha))
}

/// O gesto que o cartao diria a esta impressora. Perfil sem modelo, ou nao lido, nao afirma lado: vale
/// o que foi feito na calibracao.
fn gesto_da_impressora(impressora: &str, perfis: &Result<Perfis, ErroPerfis>) -> Recolocar {
    let (simulada, nome) = identificar(impressora);
    if simulada.is_some() {
        return trabalhos::recolocar_do_perfil(simulada, None);
    }
    let perfis = perfis.as_ref().ok();
    let Some(entrada) = perfis.and_then(|perfis| perfis.obter(&nome)) else {
        return trabalhos::recolocar_do_perfil(None, None);
    };
    let calibracao = perfis.and_then(|perfis| perfis.calibracao(&nome));
    trabalhos::gesto_e_leitura(entrada, calibracao).0
}

/// A rota em que todas as impressoras do envio imprimem, para comparar so o que e comparavel. Quem
/// nao e automatica so tem a manual; a automatica segue os papeis deste envio.
fn rota_comum(
    quem: &[String],
    perfis: Option<&Perfis>,
    fluxos: &[trabalhos::TrechoDoFluxo],
) -> Option<bool> {
    let tem = |automatico: bool| fluxos.iter().any(|fluxo| fluxo.automatico == automatico);
    let e_automatica = |id: &String| {
        id == impressoras::ID_SIMULADA_AUTOMATICA
            || id
                .strip_prefix("fila:")
                .and_then(|fila| perfis?.obter(fila))
                .is_some_and(|entrada| entrada.automatico)
    };
    if tem(true) && quem.iter().all(e_automatica) {
        Some(true)
    } else if tem(false) || !quem.iter().any(e_automatica) {
        Some(false)
    } else {
        None
    }
}

/// O identificador diz o tipo: so os nomes fixos sao simulados, e o resto e o nome exato da fila. Um
/// nome sem prefixo nunca cai em uma impressora simulada. O modelo da simulada automatica e o gesto
/// dos papeis que ela nao imprime sozinha.
fn identificar(impressora: &str) -> (Option<ModeloImpressora>, String) {
    match impressora {
        "simulada:traseira" => (Some(ModeloImpressora::JatoDeTinta), impressora.to_owned()),
        "simulada:gaveta" => (Some(ModeloImpressora::Laser), impressora.to_owned()),
        impressoras::ID_SIMULADA_AUTOMATICA => {
            (Some(ModeloImpressora::JatoDeTinta), impressora.to_owned())
        }
        outro => (
            None,
            outro.strip_prefix("fila:").unwrap_or(outro).to_owned(),
        ),
    }
}

fn iniciar_um(
    gerente: &Gerente,
    emitir: Emissor,
    aba: u64,
    impressora: &str,
    pedido: Result<Pedido, Motivo>,
    diario: &Arc<Diario>,
) -> bool {
    let (simulada, nome) = identificar(impressora);
    let identidade = Identidade {
        aba,
        impressora: impressora.to_owned(),
        nome: nome.clone(),
        simulada: simulada.is_some(),
    };
    let fim = Diario::fim_da_parte(diario, impressora, simulada.is_some());
    let relogio = Arc::new(RelogioDoSistema::default());
    if let Some(modelo) = simulada {
        let automatica = impressora == impressoras::ID_SIMULADA_AUTOMATICA;
        let id = impressora.to_owned();
        return gerente.iniciar_medido(identidade, emitir, relogio, fim, move || {
            let pedido = pedido?;
            if automatica {
                trabalhos::preparar_simulada_automatica(modelo, &pedido)
            } else {
                let driver = inversao::driver_da_simulada(&id);
                trabalhos::preparar_simulada(modelo, &pedido.com_driver_da_simulada(driver))
            }
        });
    }
    iniciar_fila(gerente, identidade, emitir, nome, pedido, fim)
}

#[cfg(windows)]
fn iniciar_fila(
    gerente: &Gerente,
    identidade: Identidade,
    emitir: Emissor,
    nome: String,
    pedido: Result<Pedido, Motivo>,
    fim: trabalhos::FimDoTrabalho,
) -> bool {
    let relogio = Arc::new(RelogioDoSistema::default());
    gerente.iniciar_medido(identidade, emitir, relogio, fim, move || {
        trabalhos::preparar_fila(&nome, &pedido?)
    })
}

/// Fora do Windows nao ha fila: o cartao mostra a recusa, e nada e enviado.
#[cfg(not(windows))]
fn iniciar_fila(
    gerente: &Gerente,
    identidade: Identidade,
    emitir: Emissor,
    _nome: String,
    _pedido: Result<Pedido, Motivo>,
    fim: trabalhos::FimDoTrabalho,
) -> bool {
    let relogio = Arc::new(RelogioDoSistema::default());
    gerente.iniciar_medido::<plots_windows::plots_print::fluxo::SimGuiado, _>(
        identidade,
        emitir,
        relogio,
        fim,
        || Err(trabalhos::fila_indisponivel()),
    )
}

/// A escolha do rodape da janela: so as frentes ou so os versos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaceRecebida {
    Frente,
    Verso,
}

impl From<FaceRecebida> for Alcance {
    fn from(face: FaceRecebida) -> Self {
        match face {
            FaceRecebida::Frente => Self::SoFrentes,
            FaceRecebida::Verso => Self::SoVersos,
        }
    }
}

impl From<FaceRecebida> for FaceDoRegistro {
    fn from(face: FaceRecebida) -> Self {
        match face {
            FaceRecebida::Frente => Self::Frente,
            FaceRecebida::Verso => Self::Verso,
        }
    }
}

/// Inicia um trabalho por impressora, todas com o mesmo pedido. Cada um roda na propria thread e
/// publica o proprio cartao.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Cada argumento e um campo do pedido que a janela envia.
pub fn iniciar_impressao(
    app: AppHandle,
    gerente: State<'_, Gerente>,
    diarios: State<'_, Diarios>,
    aba: u64,
    caminho: String,
    impressoras: Vec<String>,
    intervalo: Option<IntervaloRecebido>,
    papel_de_ajuste: Option<String>,
    marca_controle: Option<MarcaRecebida>,
    face: FaceRecebida,
    copias: Option<u32>,
    uma_copia_por_vez: Option<bool>,
) -> Vec<String> {
    let emitir: Emissor = Arc::new(move |cartao| {
        // Sem janela nao ha quem mostrar; o trabalho segue e termina sozinho.
        let _ = app.emit(trabalhos::EVENTO_DO_CARTAO, cartao);
    });
    let diario = diarios.abrir(
        CabecalhoDoEnvio {
            arquivo: caminho.clone(),
            intervalo: intervalo.map(|trecho| (trecho.inicio, trecho.fim)),
            divisao: false,
            lotes_confirmados: 0,
            face: Some(face.into()),
        },
        impressoras.clone(),
    );
    let alcance: Alcance = face.into();
    let impressao = match alcance {
        Alcance::SoVersos => trabalhos::impressao_guardada(aba),
        _ => None,
    };
    // A frente guarda o que a própria thread leu, depois que o preparo dela é aceito.
    let iniciadas: Vec<String> = impressoras
        .into_iter()
        .filter(|impressora| {
            let aceito =
                iniciar_um(
                    &gerente,
                    Arc::clone(&emitir),
                    aba,
                    impressora,
                    papel_de_ajuste
                        .as_deref()
                        .map(|codigo| {
                            plots_core::ajuste::PapelDeAjuste::de_codigo(codigo)
                                .ok_or_else(|| Motivo::novo("pdf", "papel de destino inválido"))
                        })
                        .transpose()
                        .map(|papel| {
                            Pedido::novo(caminho.clone(), intervalo.map(Into::into))
                                .com_alcance(face.into())
                                .com_papel_de_ajuste(papel)
                                .com_controle(marca_controle.map(Into::into))
                                .com_impressao_esperada(impressao.clone())
                                .com_aba(aba)
                                .com_uma_copia_por_vez(uma_copia_por_vez.unwrap_or(false))
                                .com_copias(copias.map_or(1, |valor| {
                                    usize::try_from(valor).unwrap_or(usize::MAX)
                                }))
                        }),
                    &diario,
                );
            // A impressora ja tinha um trabalho: este nao comecou e nao entra no registro.
            if !aceito {
                diario.desistir(impressora);
            }
            aceito
        })
        .collect();
    iniciadas
}

/// Entrega um pedido ao trabalho desta impressora. A recusa quer dizer que a thread nao existe mais.
#[tauri::command]
pub fn comandar_trabalho(
    gerente: State<'_, Gerente>,
    impressora: String,
    comando: Comando,
) -> Result<(), ()> {
    gerente.comandar(&impressora, comando)
}

/// O documento fechou: os trabalhos dele param de ser acompanhados, e nada sai da fila.
#[tauri::command]
pub fn soltar_trabalhos(gerente: State<'_, Gerente>, aba: u64) {
    gerente.soltar_aba(aba);
    trabalhos::esquecer_impressao(aba);
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde::de::value::{Error as ErroDeLeitura, StrDeserializer};

    fn ids(nomes: &[&str]) -> Vec<String> {
        nomes.iter().map(|nome| (*nome).to_owned()).collect()
    }

    #[test]
    fn simulada_automatica_e_reconhecida_com_o_gesto_dos_papeis_feitos_a_mao() {
        assert_eq!(
            identificar("simulada:automatica").0,
            Some(ModeloImpressora::JatoDeTinta)
        );
        let gesto = gesto_da_impressora("simulada:automatica", &Ok(Perfis::new()));
        assert!(gesto.lado.is_some());
    }

    #[test]
    fn fila_calibrada_le_o_maco_e_recoloca_como_as_respostas_disseram() {
        for le_virando in [true, false] {
            let mut perfis = Perfis::new();
            perfis.gravar_calibracao(
                "Sala",
                plots_core::calibracao::PerfilEmTeste::do_modelo(
                    ModeloImpressora::JatoDeTinta,
                    false,
                ),
                plots_perfis::Calibracao {
                    face_impressa_entra_para_cima: true,
                    le_virando,
                },
            );
            let entrada = perfis.obter("Sala").unwrap();
            let (recolocar, le) = trabalhos::gesto_e_leitura(entrada, perfis.calibracao("Sala"));
            assert_eq!(le, le_virando);
            assert_eq!(gesto_da_impressora("fila:Sala", &Ok(perfis)), recolocar);
        }
    }

    #[test]
    fn o_gesto_da_confirmacao_e_o_do_modelo_e_sem_modelo_nao_afirma_lado() {
        let laser = gesto_da_impressora("simulada:gaveta", &Ok(Perfis::new()));
        let jato = gesto_da_impressora("simulada:traseira", &Ok(Perfis::new()));
        assert_ne!(laser, jato);
        let sem_perfil = gesto_da_impressora("fila:Sala", &Ok(Perfis::new()));
        assert_eq!(sem_perfil.lado, None);
        assert_eq!(sem_perfil.virada, None);
        let mut perfis = Perfis::new();
        perfis.definir_automatico("Sala", Some(ModeloImpressora::Laser));
        assert_eq!(gesto_da_impressora("fila:Sala", &Ok(perfis)), laser);
    }

    #[test]
    fn a_face_recebida_escolhe_o_alcance_e_outro_texto_para_na_mensagem() {
        let ler = |texto: &'static str| {
            FaceRecebida::deserialize(StrDeserializer::<ErroDeLeitura>::new(texto))
        };
        assert_eq!(Alcance::from(ler("frente").unwrap()), Alcance::SoFrentes);
        assert_eq!(Alcance::from(ler("verso").unwrap()), Alcance::SoVersos);
        assert!(ler("as_duas").is_err());
    }

    #[test]
    fn a_comparacao_so_vale_na_rota_em_que_todas_imprimem() {
        let fluxo = |automatico| trabalhos::TrechoDoFluxo {
            automatico,
            inicio: 1,
            fim: 2,
        };
        let mut perfis = Perfis::new();
        perfis.definir_automatico("Auto", None);
        perfis.definir_automatico("Outra", None);
        perfis.definir_modelo("Manual", ModeloImpressora::Laser);
        let so_duas_faces = [fluxo(true)];
        let so_manual = [fluxo(false)];
        let misto = [fluxo(true), fluxo(false)];
        let rota = |nomes: &[&str], fluxos: &[trabalhos::TrechoDoFluxo]| {
            rota_comum(&ids(nomes), Some(&perfis), fluxos)
        };
        assert_eq!(
            rota(&["fila:Auto", "fila:Outra"], &so_duas_faces),
            Some(true)
        );
        assert_eq!(rota(&["fila:Auto", "fila:Outra"], &misto), Some(true));
        assert_eq!(rota(&["fila:Auto", "fila:Outra"], &so_manual), Some(false));
        assert_eq!(rota(&["fila:Auto", "fila:Manual"], &so_duas_faces), None);
        assert_eq!(rota(&["fila:Auto", "fila:Manual"], &misto), Some(false));
        assert_eq!(
            rota(&["fila:Manual", "simulada:gaveta"], &so_duas_faces),
            Some(false)
        );
    }

    #[test]
    fn idioma_da_marca_desconhecido_para_na_mensagem() {
        let ler = |texto: &str| {
            MarcaRecebida::deserialize(
                serde::de::value::StrDeserializer::<serde::de::value::Error>::new(texto),
            )
        };
        assert_eq!(ler("pt-BR").unwrap(), MarcaRecebida::Portugues);
        assert_eq!(ler("en").unwrap(), MarcaRecebida::Ingles);
        assert!(ler("<script>").is_err());
    }

    #[test]
    fn trecho_recebido_vira_o_do_nucleo_e_numero_hostil_para_na_mensagem() {
        use serde::de::value::{Error, MapDeserializer};
        fn ler_inteiros(inicio: i64, fim: i64) -> Result<IntervaloRecebido, Error> {
            IntervaloRecebido::deserialize(MapDeserializer::new(
                [("inicio", inicio), ("fim", fim)].into_iter(),
            ))
        }
        fn ler_fracoes(inicio: f64, fim: f64) -> Result<IntervaloRecebido, Error> {
            IntervaloRecebido::deserialize(MapDeserializer::new(
                [("inicio", inicio), ("fim", fim)].into_iter(),
            ))
        }
        let lido: IntervaloPaginas = ler_inteiros(3, 8).unwrap().into();
        assert_eq!(lido, IntervaloPaginas { inicio: 3, fim: 8 });
        assert!(ler_inteiros(-1, 8).is_err());
        assert!(ler_inteiros(3, i64::MIN).is_err());
        assert!(ler_fracoes(3.5, 8.0).is_err());
        assert!(ler_fracoes(1e300, 1e300).is_err());
    }

    #[test]
    fn so_os_dois_identificadores_fixos_sao_simulados() {
        assert_eq!(
            identificar("simulada:gaveta").0,
            Some(ModeloImpressora::Laser)
        );
        assert_eq!(
            identificar("simulada:traseira").0,
            Some(ModeloImpressora::JatoDeTinta)
        );
        let (modelo, nome) = identificar("fila:Simulada — Gaveta");
        assert_eq!((modelo, nome.as_str()), (None, "Simulada — Gaveta"));
        assert_eq!(identificar("simulada:outra").0, None);
    }
}

#[cfg(test)]
mod restaurados_do_automatico {
    use super::*;
    use plots_core::{DimensoesPagina, PerfilImpressora};

    #[test]
    fn simulada_automatica_e_reconhecida_e_nao_afirma_como_o_maco_sai() {
        assert_eq!(
            identificar(impressoras::ID_SIMULADA_AUTOMATICA).0,
            Some(ModeloImpressora::JatoDeTinta)
        );
        let dimensoes = vec![DimensoesPagina::new(210.0, 297.0); 4];
        let pedido = Pedido::novo("x.pdf".to_owned(), None);
        let lotes = crate::trabalhos::planejar_pedido(
            &dimensoes,
            &pedido,
            PerfilImpressora::default(),
            true,
        )
        .unwrap();
        assert!(lotes.iter().all(|item| item.lote.duas_faces));
    }

    #[test]
    fn fila_automatica_nao_afirma_como_o_maco_sai_nem_com_gesto_de_laser() {
        let mut perfis = Perfis::new();
        perfis.definir_automatico("Sala", Some(ModeloImpressora::Laser));
        let entrada = perfis.obter("Sala").expect("a fila tem perfil");
        assert!(entrada.automatico);
        let dimensoes = vec![DimensoesPagina::new(210.0, 297.0); 4];
        let lotes = crate::trabalhos::planejar_pedido(
            &dimensoes,
            &Pedido::novo("x.pdf".to_owned(), None),
            PerfilImpressora::default(),
            true,
        )
        .unwrap();
        // A decisão real está em preparar_fila e precisa do spooler: aqui vale a mesma regra, com
        // o plano automático, que não tem lote manual para virar.
        let vira = entrada
            .modelo
            .is_some_and(plots_windows::plots_print::fluxo::vira_para_ler)
            && lotes.iter().any(|item| !item.lote.duas_faces);
        assert!(!vira);
    }
}
