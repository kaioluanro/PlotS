//! Comandos que a interface usa para conduzir os trabalhos de impressao.

use crate::documento::{self, ErroDocumento};
use crate::historico::{self, CabecalhoDoEnvio, Diario, Diarios};
use crate::impressoras;
use crate::inversao;
use crate::tempo_de_impressao::RelogioDoSistema;
use crate::trabalhos::{
    self, Comando, Emissor, Gerente, Identidade, ModoDoEnvio, Motivo, ParteDoEnvio, ParteDoResumo,
    Pedido, ResumoDoEnvio,
};
use plots_core::distribuicao::{
    distribuir_lotes, filas_com_lote, ErroDistribuicao, ModoDistribuicao,
};
use plots_core::{IntervaloPaginas, ModeloImpressora};
use plots_perfis::{ErroPerfis, Perfis};
use plots_windows::plots_print::fluxo;
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
    folhas: usize,
    intervalo: Option<IntervaloRecebido>,
    modo: ModoDoEnvio,
    impressoras: Vec<String>,
    papel_de_ajuste: Option<String>,
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
        let mut resumo = trabalhos::resumo_do_envio_com_papel(
            &caminho,
            folhas,
            intervalo.map(Into::into),
            papel,
        )?;
        if modo == ModoDoEnvio::Divisao {
            let perfis = impressoras::ler_perfis_do_sistema();
            resumo.divisao = divisao_do_resumo(resumo.lotes, &impressoras, &perfis)
                .map_err(|_| documento::falha_inesperada(&caminho))?;
        }
        let quem = quem_imprime(&impressoras, &resumo.divisao);
        let perfis = impressoras::ler_perfis_do_sistema();
        resumo.comparacao = rota_comum(&quem, perfis.as_ref().ok(), &resumo.fluxos)
            .and_then(|automatico| historico::comparacao_do_envio(&quem, automatico));
        Ok(resumo)
    })
    .await
    .unwrap_or(Err(falha))
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

/// Quem de fato recebe lotes: na divisao, quem fica sem nenhum nao termina antes nem depois.
fn quem_imprime(impressoras: &[String], divisao: &[ParteDoResumo]) -> Vec<String> {
    if divisao.is_empty() {
        return impressoras.to_vec();
    }
    divisao
        .iter()
        .filter(|parte| !parte.lotes.is_empty())
        .map(|parte| parte.impressora.clone())
        .collect()
}

/// O que cabe a cada impressora, com os lotes numerados a partir de 1. Quem fica sem lote aparece
/// de lista vazia, para a confirmacao dizer isso antes de enviar.
fn divisao_do_resumo(
    lotes: usize,
    impressoras: &[String],
    perfis: &Result<Perfis, ErroPerfis>,
) -> Result<Vec<ParteDoResumo>, ErroDistribuicao> {
    let plano = distribuir_lotes(lotes, impressoras.len(), ModoDistribuicao::Divisao)?;
    Ok(plano
        .iter()
        .zip(impressoras)
        .map(|(atribuicao, impressora)| ParteDoResumo {
            impressora: impressora.clone(),
            lotes: atribuicao
                .indices_lotes()
                .map(|indice| indice.saturating_add(1))
                .collect(),
            vira_para_ler: vira_para_ler(impressora, perfis),
        })
        .collect())
}

/// So o modelo afirma como o maco sai; perfil personalizado ou nao lido nao diz nada.
fn vira_para_ler(impressora: &str, perfis: &Result<Perfis, ErroPerfis>) -> Option<bool> {
    let (simulada, nome) = identificar(impressora);
    if impressora == impressoras::ID_SIMULADA_AUTOMATICA {
        return None;
    }
    let modelo = match simulada {
        Some(modelo) => Some(modelo),
        None => {
            let entrada = perfis.as_ref().ok()?.obter(&nome)?;
            // Como o maco sai das duas faces nao se sabe: a pessoa confere.
            if entrada.automatico {
                return None;
            }
            entrada.modelo
        }
    };
    modelo.map(fluxo::vira_para_ler)
}

/// Quem recebe trabalho. Na divisao, fica de fora quem nao teria lote: nem trabalho, nem cartao.
fn destinos_do_envio(
    impressoras: Vec<String>,
    modo: ModoDoEnvio,
    lotes: usize,
    paginas: usize,
) -> Vec<(String, Option<ParteDoEnvio>)> {
    if modo == ModoDoEnvio::Espelho {
        return impressoras.into_iter().map(|nome| (nome, None)).collect();
    }
    let total = impressoras.len();
    impressoras
        .into_iter()
        .take(filas_com_lote(lotes, total))
        .enumerate()
        .map(|(indice, nome)| {
            let parte = ParteDoEnvio {
                indice,
                impressoras: total,
                lotes,
                paginas,
            };
            (nome, Some(parte))
        })
        .collect()
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

/// Inicia um trabalho por impressora. Cada um roda na propria thread e publica o proprio cartao.
#[tauri::command]
pub fn iniciar_impressao(
    app: AppHandle,
    gerente: State<'_, Gerente>,
    diarios: State<'_, Diarios>,
    aba: u64,
    caminho: String,
    folhas: usize,
    impressoras: Vec<String>,
    intervalo: Option<IntervaloRecebido>,
    modo: ModoDoEnvio,
    lotes: usize,
    paginas: usize,
    papel_de_ajuste: Option<String>,
    marca_controle: Option<MarcaRecebida>,
) -> Vec<String> {
    let emitir: Emissor = Arc::new(move |cartao| {
        // Sem janela nao ha quem mostrar; o trabalho segue e termina sozinho.
        let _ = app.emit(trabalhos::EVENTO_DO_CARTAO, cartao);
    });
    let destinos = destinos_do_envio(impressoras, modo, lotes, paginas);
    let diario = diarios.abrir(
        CabecalhoDoEnvio {
            arquivo: caminho.clone(),
            intervalo: intervalo.map(|trecho| (trecho.inicio, trecho.fim)),
            divisao: modo == ModoDoEnvio::Divisao,
            lotes_confirmados: lotes,
        },
        destinos.iter().map(|(nome, _)| nome.clone()).collect(),
    );
    destinos
        .into_iter()
        .filter(|(impressora, parte)| {
            let aceito = iniciar_um(
                &gerente,
                Arc::clone(&emitir),
                aba,
                impressora,
                Pedido::novo(caminho.clone(), folhas, intervalo.map(Into::into)).and_then(
                    |pedido| {
                        let papel = papel_de_ajuste
                            .as_deref()
                            .map(|codigo| {
                                plots_core::ajuste::PapelDeAjuste::de_codigo(codigo)
                                    .ok_or_else(|| Motivo::novo("pdf", "papel de destino inválido"))
                            })
                            .transpose()?;
                        Ok(pedido
                            .com_parte(*parte)
                            .com_papel_de_ajuste(papel)
                            .com_controle(marca_controle.map(Into::into)))
                    },
                ),
                &diario,
            );
            // A impressora ja tinha um trabalho: este nao comecou e nao entra no registro.
            if !aceito {
                diario.desistir(impressora);
            }
            aceito
        })
        .map(|(impressora, _)| impressora)
        .collect()
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
}

#[cfg(test)]
mod testes {
    use super::*;
    use plots_core::{DimensoesPagina, OrdemPassadas, PerfilImpressora};
    use plots_perfis::EntradaPerfil;
    use plots_windows::plots_print::fluxo::{Condutor, Espera, LoteDoFluxo, SimGuiado};
    use plots_windows::plots_print::Mecanica;
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    /// Duas folhas por lote: quatro paginas em cada um.
    const FOLHAS: usize = 2;
    const PAGINAS_POR_LOTE: usize = FOLHAS * 2;

    fn ids(nomes: &[&str]) -> Vec<String> {
        nomes.iter().map(|nome| (*nome).to_owned()).collect()
    }

    fn plano(paginas: usize, perfil: PerfilImpressora) -> Vec<LoteDoFluxo> {
        let dimensoes = vec![DimensoesPagina::new(210.0, 297.0); paginas];
        let lotes =
            plots_core::plano_lotes_por_papel(&dimensoes, FOLHAS, OrdemPassadas::default(), perfil)
                .unwrap();
        fluxo::lotes_do_plano(&lotes)
    }

    #[test]
    fn simulada_automatica_e_reconhecida_e_nao_afirma_como_o_maco_sai() {
        assert_eq!(
            identificar("simulada:automatica").0,
            Some(ModeloImpressora::JatoDeTinta)
        );
        let perfis = Ok(Perfis::new());
        assert_eq!(vira_para_ler("simulada:automatica", &perfis), None);
    }

    #[test]
    fn fila_automatica_nao_afirma_como_o_maco_sai_nem_com_gesto_de_laser() {
        let mut perfis = Perfis::new();
        perfis.definir_automatico("Sala", Some(ModeloImpressora::Laser));
        assert_eq!(vira_para_ler("fila:Sala", &Ok(perfis)), None);
    }

    /// Cada impressora tem o seu tipo de plano, mas o rodizio dos lotes e um so: nenhuma pagina se
    /// repete e nenhuma falta.
    #[test]
    fn divisao_entre_manual_e_automatica_nao_repete_nem_perde_paginas() {
        const PAGINAS: usize = 18;
        let dimensoes = vec![DimensoesPagina::new(210.0, 297.0); PAGINAS];
        let pedido = |indice| {
            Pedido::novo("x.pdf".to_owned(), FOLHAS, None)
                .unwrap()
                .com_parte(Some(ParteDoEnvio {
                    indice,
                    impressoras: 2,
                    lotes: 5,
                    paginas: PAGINAS,
                }))
        };
        let perfil = ModeloImpressora::JatoDeTinta.perfil();
        let manual = trabalhos::planejar_pedido(&dimensoes, &pedido(0), perfil, false).unwrap();
        let automatica = trabalhos::planejar_pedido(&dimensoes, &pedido(1), perfil, true).unwrap();
        assert_eq!(manual.len(), 3);
        assert_eq!(automatica.len(), 2);
        assert!(manual.iter().all(|item| !item.lote.duas_faces));
        assert!(automatica.iter().all(|item| item.lote.duas_faces));
        assert_eq!(
            automatica[0]
                .lote
                .passada1
                .iter()
                .filter_map(|pagina| pagina.numero)
                .collect::<Vec<_>>(),
            [5, 6, 7, 8],
            "em ordem de documento, sem inverter"
        );
        let mut vistas: Vec<usize> = manual
            .iter()
            .chain(&automatica)
            .flat_map(|item| item.lote.passada1.iter().chain(&item.lote.passada2))
            .filter_map(|pagina| pagina.numero)
            .collect();
        vistas.sort_unstable();
        assert_eq!(vistas, (1..=PAGINAS).collect::<Vec<_>>());
    }

    #[test]
    fn espelho_com_automatica_faz_um_trabalho_e_com_manual_respeita_as_folhas_por_lote() {
        let dimensoes = vec![DimensoesPagina::new(210.0, 297.0); 8];
        let pedido = Pedido::novo("x.pdf".to_owned(), 1, None).unwrap();
        let perfil = ModeloImpressora::JatoDeTinta.perfil();
        let automatica = trabalhos::planejar_pedido(&dimensoes, &pedido, perfil, true).unwrap();
        let manual = trabalhos::planejar_pedido(&dimensoes, &pedido, perfil, false).unwrap();
        assert_eq!(automatica.len(), 1);
        assert_eq!(manual.len(), 4);
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
    fn comparar_so_vale_entre_quem_de_fato_imprime() {
        let todas = ids(&["a", "b", "c"]);
        assert_eq!(
            quem_imprime(&todas, &[]),
            todas,
            "no espelho, todas imprimem"
        );
        let parte = |nome: &str, lotes: Vec<usize>| ParteDoResumo {
            impressora: nome.to_owned(),
            lotes,
            vira_para_ler: None,
        };
        let divisao = [
            parte("a", vec![1, 3]),
            parte("b", vec![2]),
            parte("c", Vec::new()),
        ];
        assert_eq!(quem_imprime(&todas, &divisao), ids(&["a", "b"]));
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
    fn espelho_envia_a_todas_sem_restringir_lotes() {
        let destinos = destinos_do_envio(ids(&["a", "b"]), ModoDoEnvio::Espelho, 0, 0);
        assert_eq!(destinos.len(), 2);
        assert!(destinos.iter().all(|(_, parte)| parte.is_none()));
    }

    #[test]
    fn mais_impressoras_que_lotes_as_de_sobra_nao_recebem_trabalho() {
        let destinos = destinos_do_envio(ids(&["a", "b", "c", "d"]), ModoDoEnvio::Divisao, 2, 8);
        let nomes: Vec<&str> = destinos.iter().map(|(nome, _)| nome.as_str()).collect();
        assert_eq!(nomes, ["a", "b"]);
        assert_eq!(
            destinos[1].1,
            Some(ParteDoEnvio {
                indice: 1,
                impressoras: 4,
                lotes: 2,
                paginas: 8
            })
        );
        assert!(destinos_do_envio(ids(&["a"]), ModoDoEnvio::Divisao, 0, 0).is_empty());
    }

    #[test]
    fn resumo_diz_os_lotes_de_cada_uma_e_quem_fica_sem_nada() {
        let mut modelo = Perfis::new();
        modelo.definir_modelo("Laser", ModeloImpressora::Laser);
        modelo.inserir("Medida", EntradaPerfil::default());
        let perfis = Ok(modelo);
        let partes =
            divisao_do_resumo(3, &ids(&["simulada:gaveta", "simulada:traseira"]), &perfis).unwrap();
        assert_eq!(partes[0].lotes, [1, 3]);
        assert_eq!(partes[1].lotes, [2]);
        assert_eq!(
            partes[0].vira_para_ler,
            Some(fluxo::vira_para_ler(ModeloImpressora::Laser))
        );
        assert_eq!(
            partes[1].vira_para_ler,
            Some(fluxo::vira_para_ler(ModeloImpressora::JatoDeTinta))
        );

        let de_sobra =
            divisao_do_resumo(1, &ids(&["fila:Laser", "fila:Medida", "fila:X"]), &perfis).unwrap();
        assert_eq!(de_sobra[0].lotes, [1]);
        assert!(de_sobra[1].lotes.is_empty());
        assert_eq!(
            de_sobra[0].vira_para_ler,
            Some(fluxo::vira_para_ler(ModeloImpressora::Laser))
        );
        assert_eq!(de_sobra[1].vira_para_ler, None, "perfil sem modelo");
        assert_eq!(de_sobra[2].vira_para_ler, None, "sem perfil");
        assert!(divisao_do_resumo(1, &[], &perfis).is_err());
    }

    #[test]
    fn divisao_com_perfis_diferentes_mantem_a_ordem_de_cada_um_sem_repetir_paginas() {
        const PAGINAS: usize = 18;
        let perfis = [
            ModeloImpressora::JatoDeTinta.perfil(),
            ModeloImpressora::Laser.perfil(),
        ];
        let inteiros = perfis.map(|perfil| plano(PAGINAS, perfil));
        assert_eq!(inteiros[0].len(), 5);
        assert_ne!(
            inteiros[0][0].lote.passada2, inteiros[1][0].lote.passada2,
            "os dois perfis pedem ordens diferentes para o mesmo lote"
        );
        let mut vistas = Vec::new();
        for (indice, inteiro) in inteiros.iter().enumerate() {
            let parte = ParteDoEnvio {
                indice,
                impressoras: 2,
                lotes: inteiro.len(),
                paginas: 0,
            };
            let dela = parte.separar(inteiro.clone()).unwrap();
            assert_eq!(dela.len(), if indice == 0 { 3 } else { 2 });
            for (posicao, lote) in dela.iter().enumerate() {
                assert_eq!(
                    Some(lote),
                    inteiro.get(indice + posicao * 2),
                    "o lote sai como o plano do proprio perfil o fez"
                );
            }
            vistas.extend(
                dela.iter()
                    .flat_map(|lote| lote.lote.passada1.iter().chain(&lote.lote.passada2))
                    .filter_map(|pagina| pagina.numero),
            );
        }
        vistas.sort_unstable();
        assert_eq!(vistas, (1..=PAGINAS).collect::<Vec<_>>());
    }

    #[test]
    fn documento_que_mudou_depois_da_confirmacao_nao_e_repartido() {
        let parte = ParteDoEnvio {
            indice: 0,
            impressoras: 2,
            lotes: 99,
            paginas: 0,
        };
        let recusa = parte
            .separar(plano(6, ModeloImpressora::Laser.perfil()))
            .unwrap_err();
        assert_eq!(recusa.codigo, trabalhos::MOTIVO_DOCUMENTO_MUDOU);
    }

    #[test]
    fn trecho_em_divisao_com_perfis_diferentes_sai_uma_vez_com_os_numeros_originais() {
        const TOTAL: usize = 20;
        let dimensoes = vec![DimensoesPagina::new(210.0, 297.0); TOTAL];
        let trecho = Some(IntervaloPaginas { inicio: 3, fim: 14 });
        let mut vistas = Vec::new();
        for (indice, modelo) in [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser]
            .into_iter()
            .enumerate()
        {
            let lotes = plots_core::plano_lotes_do_intervalo(
                &dimensoes,
                trecho,
                FOLHAS,
                OrdemPassadas::default(),
                modelo.perfil(),
            )
            .unwrap();
            assert_eq!(lotes.len(), 3);
            assert_eq!(
                trabalhos::faixas_dos_lotes(&lotes),
                [(1, 3, 6), (2, 7, 10), (3, 11, 14)]
                    .map(|(lote, inicio, fim)| trabalhos::FaixaDoLote { lote, inicio, fim })
            );
            let parte = ParteDoEnvio {
                indice,
                impressoras: 2,
                lotes: lotes.len(),
                paginas: TOTAL,
            };
            let dela = parte.separar(fluxo::lotes_do_plano(&lotes)).unwrap();
            vistas.extend(
                dela.iter()
                    .flat_map(|lote| lote.lote.passada1.iter().chain(&lote.lote.passada2))
                    .filter_map(|pagina| pagina.numero),
            );
        }
        vistas.sort_unstable();
        assert_eq!(vistas, (3..=14).collect::<Vec<_>>());
    }

    #[test]
    fn total_de_paginas_diferente_do_confirmado_e_recusado_mesmo_com_os_mesmos_lotes() {
        let dimensoes = vec![DimensoesPagina::new(210.0, 297.0); 8];
        let trecho = Some(IntervaloPaginas { inicio: 1, fim: 4 });
        let confirmada = |paginas| ParteDoEnvio {
            indice: 0,
            impressoras: 2,
            lotes: 1,
            paginas,
        };
        let pedido = |paginas| {
            Pedido::novo("x.pdf".to_owned(), FOLHAS, trecho)
                .unwrap()
                .com_parte(Some(confirmada(paginas)))
        };
        let perfil = ModeloImpressora::Laser.perfil();
        assert_eq!(
            trabalhos::planejar_pedido(&dimensoes, &pedido(9), perfil, false)
                .unwrap_err()
                .codigo,
            trabalhos::MOTIVO_DOCUMENTO_MUDOU
        );
        assert!(
            trabalhos::planejar_pedido(&dimensoes, &pedido(9), perfil, false)
                .unwrap_err()
                .texto
                .ends_with('.'),
            "a frase mostrada no cartao termina em ponto"
        );
        assert_eq!(
            trabalhos::planejar_pedido(&dimensoes, &pedido(8), perfil, false)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn documento_mudou_deixa_a_impressora_livre_na_hora() {
        let (gerente, emitir, cartoes) = cena();
        let trocado = ParteDoEnvio {
            indice: 0,
            impressoras: 2,
            lotes: 4,
            paginas: 0,
        };
        assert!(iniciar_simulada(
            &gerente,
            &emitir,
            "simulada:gaveta",
            ParteDoEnvio {
                lotes: 0,
                ..trocado
            }
        ));
        esperar_estado(&cartoes, "simulada:gaveta", |estado| {
            matches!(estado, trabalhos::EstadoCartao::Erro { .. })
        });
        gerente.soltar_aba(1);
        let identidade = Identidade {
            aba: 2,
            impressora: "simulada:traseira".to_owned(),
            nome: "simulada:traseira".to_owned(),
            simulada: true,
        };
        let aceito = gerente.iniciar(identidade, Arc::clone(&emitir), move || {
            trocado.separar(Vec::new())?;
            Err::<trabalhos::Preparo<SimGuiado>, _>(trabalhos::fila_indisponivel())
        });
        assert!(aceito);
        esperar_estado(
            &cartoes,
            "simulada:traseira",
            |estado| matches!(estado, trabalhos::EstadoCartao::Erro { motivo } if motivo.codigo == trabalhos::MOTIVO_DOCUMENTO_MUDOU),
        );
        let boa = ParteDoEnvio {
            indice: 0,
            impressoras: 2,
            lotes: 2,
            paginas: 0,
        };
        let prazo = Instant::now() + Duration::from_secs(5);
        while !iniciar_simulada(&gerente, &emitir, "simulada:traseira", boa) {
            assert!(Instant::now() < prazo, "a impressora continuou presa");
            std::thread::sleep(Duration::from_millis(2));
        }
        esperar_estado(&cartoes, "simulada:traseira", aguardando);
        gerente.soltar_aba(1);
    }

    #[test]
    fn posicao_fora_do_rodizio_e_recusada_sem_travar() {
        for (indice, impressoras) in [(5, 2), (0, 0), (usize::MAX, usize::MAX)] {
            let parte = ParteDoEnvio {
                indice,
                impressoras,
                lotes: 2,
                paginas: 0,
            };
            let lotes = plano(2 * PAGINAS_POR_LOTE, ModeloImpressora::Laser.perfil());
            assert!(parte.separar(lotes).is_err());
        }
    }

    const ESPERA_CURTA: Espera = Espera {
        intervalo: Duration::from_millis(1),
        consultas: 1_000,
    };

    type Cartoes = Arc<Mutex<BTreeMap<String, Vec<trabalhos::Cartao>>>>;

    fn esperar_estado(
        cartoes: &Cartoes,
        impressora: &str,
        aceita: impl Fn(&trabalhos::EstadoCartao) -> bool,
    ) -> trabalhos::Cartao {
        let prazo = Instant::now() + Duration::from_secs(20);
        loop {
            let achado = cartoes
                .lock()
                .unwrap()
                .get(impressora)
                .and_then(|lista| lista.last().cloned())
                .filter(|cartao| aceita(&cartao.estado));
            if let Some(cartao) = achado {
                return cartao;
            }
            assert!(Instant::now() < prazo, "{impressora} nao chegou ao estado");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Prepara a impressora simulada so com os lotes da parte, sem precisar de um PDF.
    fn iniciar_simulada(
        gerente: &Gerente,
        emitir: &Emissor,
        impressora: &str,
        parte: ParteDoEnvio,
    ) -> bool {
        let modelo = identificar(impressora).0.unwrap();
        let identidade = Identidade {
            aba: 1,
            impressora: impressora.to_owned(),
            nome: impressora.to_owned(),
            simulada: true,
        };
        gerente.iniciar(identidade, Arc::clone(emitir), move || {
            let lotes = parte.separar(plano(parte.lotes * PAGINAS_POR_LOTE, modelo.perfil()))?;
            let backend = SimGuiado::novo(Mecanica::do_modelo(modelo), modelo.gesto().into());
            let condutor = Condutor::novo(backend, lotes, ESPERA_CURTA)
                .map_err(|_| trabalhos::fila_indisponivel())?;
            Ok(trabalhos::Preparo {
                condutor,
                recolocar: trabalhos::Recolocar {
                    lado: None,
                    virada: None,
                    texto_do_perfil: None,
                },
                vira_para_ler: false,
            })
        })
    }

    fn cena() -> (Gerente, Emissor, Cartoes) {
        let cartoes: Cartoes = Arc::default();
        let copia = Arc::clone(&cartoes);
        let emitir: Emissor = Arc::new(move |cartao: trabalhos::Cartao| {
            copia
                .lock()
                .unwrap()
                .entry(cartao.impressora.clone())
                .or_default()
                .push(cartao);
        });
        (Gerente::default(), emitir, cartoes)
    }

    fn aguardando(estado: &trabalhos::EstadoCartao) -> bool {
        matches!(estado, trabalhos::EstadoCartao::Aguardando)
    }

    fn proximo(estado: &trabalhos::EstadoCartao) -> bool {
        matches!(estado, trabalhos::EstadoCartao::ProximoLote)
    }

    #[test]
    fn na_divisao_cada_impressora_avanca_no_proprio_ritmo_ate_o_fim() {
        let (gerente, emitir, cartoes) = cena();
        for (indice, impressora) in ["simulada:traseira", "simulada:gaveta"].iter().enumerate() {
            let parte = ParteDoEnvio {
                indice,
                impressoras: 2,
                lotes: 6,
                paginas: 0,
            };
            assert!(iniciar_simulada(&gerente, &emitir, impressora, parte));
        }
        // A gaveta termina os 3 lotes dela enquanto a traseira continua parada no primeiro.
        for lote in 1..=3 {
            let cartao = esperar_estado(&cartoes, "simulada:gaveta", aguardando);
            assert_eq!((cartao.lote, cartao.total), (lote, 3));
            gerente
                .comandar("simulada:gaveta", Comando::EnviarVerso)
                .unwrap();
            if lote < 3 {
                esperar_estado(&cartoes, "simulada:gaveta", proximo);
                gerente
                    .comandar("simulada:gaveta", Comando::EnviarFrente)
                    .unwrap();
            }
        }
        esperar_estado(&cartoes, "simulada:gaveta", |estado| {
            matches!(estado, trabalhos::EstadoCartao::Concluido)
        });
        let traseira = esperar_estado(&cartoes, "simulada:traseira", aguardando);
        assert_eq!((traseira.lote, traseira.total), (1, 3));
        gerente.soltar_aba(1);
    }

    #[test]
    fn erro_em_uma_impressora_nao_para_a_outra() {
        let (gerente, emitir, cartoes) = cena();
        // Posicao fora do rodizio: a preparacao desta falha, e o cartao dela mostra o erro.
        let falha = ParteDoEnvio {
            indice: 9,
            impressoras: 2,
            lotes: 4,
            paginas: 0,
        };
        assert!(iniciar_simulada(
            &gerente,
            &emitir,
            "simulada:gaveta",
            falha
        ));
        let boa = ParteDoEnvio {
            indice: 0,
            impressoras: 2,
            lotes: 4,
            paginas: 0,
        };
        assert!(iniciar_simulada(
            &gerente,
            &emitir,
            "simulada:traseira",
            boa
        ));
        esperar_estado(&cartoes, "simulada:gaveta", |estado| {
            matches!(estado, trabalhos::EstadoCartao::Erro { .. })
        });
        let cartao = esperar_estado(&cartoes, "simulada:traseira", aguardando);
        assert_eq!((cartao.lote, cartao.total), (1, 2));
        gerente
            .comandar("simulada:traseira", Comando::EnviarVerso)
            .unwrap();
        esperar_estado(&cartoes, "simulada:traseira", proximo);
        gerente.soltar_aba(1);
    }

    /// Reimprimir cancela e manda de novo a frente do lote em que o trabalho esta, nao outro.
    #[test]
    fn reimprimir_na_divisao_fica_no_lote_atual_daquela_impressora() {
        let parte = ParteDoEnvio {
            indice: 1,
            impressoras: 2,
            lotes: 4,
            paginas: 0,
        };
        let modelo = ModeloImpressora::Laser;
        let lotes = parte
            .separar(plano(4 * PAGINAS_POR_LOTE, modelo.perfil()))
            .unwrap();
        let esperado = lotes[0].clone();
        let backend = SimGuiado::novo(Mecanica::do_modelo(modelo), modelo.gesto().into());
        let mut condutor = Condutor::novo(backend, lotes, ESPERA_CURTA).unwrap();
        condutor.enviar_frente().unwrap();
        condutor.cancelar().unwrap();
        condutor.enviar_frente().unwrap();
        assert_eq!((condutor.numero(), condutor.total()), (1, 2));
        assert_eq!(condutor.lote(), Some(&esperado));
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
