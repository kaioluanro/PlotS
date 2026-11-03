//! Comandos que a interface usa para conduzir os trabalhos de impressao.

use crate::documento::{self, ErroDocumento};
use crate::impressoras;
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

/// Le o PDF para a confirmacao dizer paginas, lotes e papeis antes de qualquer envio.
#[tauri::command]
pub async fn resumo_do_envio(
    caminho: String,
    folhas: usize,
    intervalo: Option<IntervaloRecebido>,
    modo: ModoDoEnvio,
    impressoras: Vec<String>,
) -> Result<ResumoDoEnvio, ErroDocumento> {
    let falha = documento::falha_inesperada(&caminho);
    tauri::async_runtime::spawn_blocking(move || {
        let mut resumo = trabalhos::resumo_do_envio(&caminho, folhas, intervalo.map(Into::into))?;
        if modo == ModoDoEnvio::Divisao {
            let perfis = impressoras::ler_perfis_do_sistema();
            resumo.divisao = divisao_do_resumo(resumo.lotes, &impressoras, &perfis)
                .map_err(|_| documento::falha_inesperada(&caminho))?;
        }
        Ok(resumo)
    })
    .await
    .unwrap_or(Err(falha))
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
    let modelo = match simulada {
        Some(modelo) => Some(modelo),
        None => perfis.as_ref().ok()?.obter(&nome)?.modelo,
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

/// O identificador diz o tipo: so os dois nomes fixos sao simulados, e o resto e o nome exato da
/// fila. Um nome sem prefixo nunca cai em uma impressora simulada.
fn identificar(impressora: &str) -> (Option<ModeloImpressora>, String) {
    match impressora {
        "simulada:traseira" => (Some(ModeloImpressora::JatoDeTinta), impressora.to_owned()),
        "simulada:gaveta" => (Some(ModeloImpressora::Laser), impressora.to_owned()),
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
) -> bool {
    let (simulada, nome) = identificar(impressora);
    let identidade = Identidade {
        aba,
        impressora: impressora.to_owned(),
        nome: nome.clone(),
        simulada: simulada.is_some(),
    };
    if let Some(modelo) = simulada {
        return gerente.iniciar(identidade, emitir, move || {
            trabalhos::preparar_simulada(modelo, &pedido?)
        });
    }
    iniciar_fila(gerente, identidade, emitir, nome, pedido)
}

#[cfg(windows)]
fn iniciar_fila(
    gerente: &Gerente,
    identidade: Identidade,
    emitir: Emissor,
    nome: String,
    pedido: Result<Pedido, Motivo>,
) -> bool {
    gerente.iniciar(identidade, emitir, move || {
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
) -> bool {
    gerente.iniciar::<plots_windows::plots_print::fluxo::SimGuiado, _>(identidade, emitir, || {
        Err(trabalhos::fila_indisponivel())
    })
}

/// Inicia um trabalho por impressora. Cada um roda na propria thread e publica o proprio cartao.
#[tauri::command]
pub fn iniciar_impressao(
    app: AppHandle,
    gerente: State<'_, Gerente>,
    aba: u64,
    caminho: String,
    folhas: usize,
    impressoras: Vec<String>,
    intervalo: Option<IntervaloRecebido>,
    modo: ModoDoEnvio,
    lotes: usize,
    paginas: usize,
) -> Vec<String> {
    let emitir: Emissor = Arc::new(move |cartao| {
        // Sem janela nao ha quem mostrar; o trabalho segue e termina sozinho.
        let _ = app.emit(trabalhos::EVENTO_DO_CARTAO, cartao);
    });
    destinos_do_envio(impressoras, modo, lotes, paginas)
        .into_iter()
        .filter(|(impressora, parte)| {
            iniciar_um(
                &gerente,
                Arc::clone(&emitir),
                aba,
                impressora,
                Pedido::novo(caminho.clone(), folhas, intervalo.map(Into::into))
                    .map(|pedido| pedido.com_parte(*parte)),
            )
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
            trabalhos::planejar_pedido(&dimensoes, &pedido(9), perfil)
                .unwrap_err()
                .codigo,
            trabalhos::MOTIVO_DOCUMENTO_MUDOU
        );
        assert!(
            trabalhos::planejar_pedido(&dimensoes, &pedido(9), perfil)
                .unwrap_err()
                .texto
                .ends_with('.'),
            "a frase mostrada no cartao termina em ponto"
        );
        assert_eq!(
            trabalhos::planejar_pedido(&dimensoes, &pedido(8), perfil)
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
