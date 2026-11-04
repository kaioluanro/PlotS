//! Calibracao guiada pela janela: imprime as folhas de teste com o envio de sempre, le as respostas
//! e grava o perfil da fila. As simuladas passam pelo mesmo fluxo, sem fila e sem gravar nada.
//!
//! O teste e gerado na memoria e nada dele vai para o disco. O trabalho usa o mesmo `Gerente` dos
//! documentos, entao uma impressora ocupada recusa. Os cartoes saem em evento proprio e nao entram no
//! historico: a calibracao nao e um documento da pessoa.

use crate::impressoras::{self, Aviso, GRAVACAO_DE_PERFIS};
use crate::inversao;
use crate::tempo_de_impressao::RelogioDoSistema;
use crate::trabalhos::{
    self, Comando, Emissor, Gerente, Identidade, Motivo, Pedido, Preparo, Recolocar,
};
use plots_core::calibracao::{
    avaliar, conferir_automatica, face_impressa_entra_para_cima, ConferenciaAutomatica,
    LadoDoVerso, LeituraDoMaco, Mudanca, PerfilEmTeste, Respostas,
};
use plots_core::{
    BordaEntrada, DimensoesPagina, GestoRecolocar, ModeloImpressora, Pagina, PerfilImpressora,
    ViradaMaco,
};
use plots_pdf::PapelCalibracao;
use plots_perfis::{Calibracao, ErroPerfis};
use plots_windows::plots_print::fluxo::{Condutor, LoteDoFluxo, SimGuiado};
use plots_windows::plots_print::opcoes::{compensar_driver, DriverDeTeste};
use plots_windows::plots_print::{ErroSimulacao, Mecanica, Recolocacao, Simulador};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, PoisonError};
use tauri::{AppHandle, Emitter, State};

/// Evento que leva o cartao do teste ao dialogo. Os cartoes de documento continuam no outro.
const EVENTO: &str = "calibracao";
/// A calibracao nao pertence a nenhuma aba. O valor cabe sem perda em um numero da interface.
const ABA_DA_CALIBRACAO: u64 = 1 << 52;
/// O teste de 2 folhas sai inteiro em um lote.
const FOLHAS_DO_TESTE: usize = 2;
const PAGINAS_DO_TESTE: usize = plots_pdf::PAGINAS_CALIBRACAO;

const SIMULADA_TRASEIRA: &str = "simulada:traseira";
const SIMULADA_GAVETA: &str = "simulada:gaveta";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViradaRecebida {
    NaoVirar,
    Lateral,
    BordaDeCima,
}

impl From<ViradaRecebida> for ViradaMaco {
    fn from(virada: ViradaRecebida) -> Self {
        match virada {
            ViradaRecebida::NaoVirar => Self::NaoVirar,
            ViradaRecebida::Lateral => Self::PelaLateral,
            ViradaRecebida::BordaDeCima => Self::PelaBordaDeCima,
        }
    }
}

impl From<ViradaMaco> for ViradaRecebida {
    fn from(virada: ViradaMaco) -> Self {
        match virada {
            ViradaMaco::NaoVirar => Self::NaoVirar,
            ViradaMaco::PelaLateral => Self::Lateral,
            ViradaMaco::PelaBordaDeCima => Self::BordaDeCima,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BordaRecebida {
    Curta,
    Longa,
}

/// O perfil de uma rodada como a interface o guarda entre um comando e outro, sem interpretar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PerfilDaRodada {
    frente_inversa: bool,
    verso_inverso: bool,
    verso_girado: bool,
    virada: ViradaRecebida,
    borda: BordaRecebida,
    driver_inverte: bool,
}

impl From<PerfilDaRodada> for PerfilEmTeste {
    fn from(rodada: PerfilDaRodada) -> Self {
        Self {
            perfil: PerfilImpressora {
                passada1_inversa: rodada.frente_inversa,
                passada2_inversa: rodada.verso_inverso,
                verso_girar_180: rodada.verso_girado,
                gesto_recolocar: GestoRecolocar {
                    virada: rodada.virada.into(),
                    borda_primeiro: match rodada.borda {
                        BordaRecebida::Curta => BordaEntrada::Curta,
                        BordaRecebida::Longa => BordaEntrada::Longa,
                    },
                },
            },
            driver_inverte: rodada.driver_inverte,
        }
    }
}

impl From<PerfilEmTeste> for PerfilDaRodada {
    fn from(teste: PerfilEmTeste) -> Self {
        Self {
            frente_inversa: teste.perfil.passada1_inversa,
            verso_inverso: teste.perfil.passada2_inversa,
            verso_girado: teste.perfil.verso_girar_180,
            virada: teste.perfil.gesto_recolocar.virada.into(),
            borda: match teste.perfil.gesto_recolocar.borda_primeiro {
                BordaEntrada::Curta => BordaRecebida::Curta,
                BordaEntrada::Longa => BordaRecebida::Longa,
            },
            driver_inverte: teste.driver_inverte,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeituraRecebida {
    SemVirar,
    Virando,
    NaoFica,
}

impl From<LeituraRecebida> for LeituraDoMaco {
    fn from(leitura: LeituraRecebida) -> Self {
        match leitura {
            LeituraRecebida::SemVirar => Self::SemVirar,
            LeituraRecebida::Virando => Self::Virando,
            LeituraRecebida::NaoFica => Self::NaoFica,
        }
    }
}

/// As respostas de uma rodada. Com impressao por cima, as outras nem foram perguntadas e valem nada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RespostasRecebidas {
    por_cima: bool,
    par_certo: bool,
    setas_iguais: bool,
    leitura: LeituraRecebida,
    primeira_a_sair: Option<usize>,
}

/// O resultado de uma rodada; cada mudanca e um codigo que a interface poe em palavras.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvaliacaoDaRodada {
    proximo: PerfilDaRodada,
    mudancas: Vec<&'static str>,
    certo: bool,
}

fn codigo_da_mudanca(mudanca: Mudanca) -> &'static str {
    match mudanca {
        Mudanca::Virada(ViradaMaco::NaoVirar) => "virada_nao_virar",
        Mudanca::Virada(ViradaMaco::PelaLateral) => "virada_lateral",
        Mudanca::Virada(ViradaMaco::PelaBordaDeCima) => "virada_borda_de_cima",
        Mudanca::FrenteInversa(true) => "frente_inversa",
        Mudanca::FrenteInversa(false) => "frente_direta",
        Mudanca::VersoInverso(true) => "verso_inverso",
        Mudanca::VersoInverso(false) => "verso_direto",
        Mudanca::VersoGirado(true) => "verso_girado",
        Mudanca::VersoGirado(false) => "verso_sem_giro",
        Mudanca::CompensaDriver(true) => "driver_compensa",
        Mudanca::CompensaDriver(false) => "driver_nao_compensa",
    }
}

fn avaliar_rodada(perfil: PerfilDaRodada, respostas: RespostasRecebidas) -> AvaliacaoDaRodada {
    let avaliacao = avaliar(
        perfil.into(),
        Respostas {
            verso: if respostas.por_cima {
                LadoDoVerso::PorCima
            } else {
                LadoDoVerso::EmBranco
            },
            par_certo: respostas.par_certo,
            setas_iguais: respostas.setas_iguais,
            leitura: respostas.leitura.into(),
            primeira_a_sair: respostas.primeira_a_sair,
        },
    );
    AvaliacaoDaRodada {
        proximo: avaliacao.proximo.into(),
        mudancas: avaliacao
            .mudancas
            .into_iter()
            .map(codigo_da_mudanca)
            .collect(),
        certo: avaliacao.certo,
    }
}

/// O perfil do modelo escolhido, com o que a linha ja sabe do driver. Codigo fora da lista e recusado.
#[tauri::command]
pub fn calibracao_perfil_inicial(
    modelo: String,
    driver_inverte: bool,
) -> Result<PerfilDaRodada, ()> {
    let modelo = ModeloImpressora::de_codigo(&modelo).ok_or(())?;
    Ok(PerfilEmTeste::do_modelo(modelo, driver_inverte).into())
}

#[tauri::command]
pub fn calibracao_avaliar(
    perfil: PerfilDaRodada,
    respostas: RespostasRecebidas,
) -> AvaliacaoDaRodada {
    avaliar_rodada(perfil, respostas)
}

/// Um codigo para cada conclusao: a interface escolhe a frase.
#[tauri::command]
pub fn calibracao_conferir_automatica(
    par_certo: bool,
    setas_iguais: bool,
    leitura: LeituraRecebida,
) -> &'static str {
    match conferir_automatica(par_certo, setas_iguais, leitura.into()) {
        ConferenciaAutomatica::Certa => "certa",
        ConferenciaAutomatica::NaoViraPelaBordaLonga => "nao_vira_pela_borda_longa",
        ConferenciaAutomatica::OrdemNaoServe => "ordem_nao_serve",
    }
}

fn entra_para_cima(perfil: PerfilDaRodada, saiu_para_cima: bool) -> bool {
    face_impressa_entra_para_cima(perfil.virada.into(), saiu_para_cima)
}

/// A frase e a figura deste perfil. Quem diz o lado e como a folha saiu de verdade, que a pessoa viu.
#[tauri::command]
pub fn calibracao_recolocar(perfil: PerfilDaRodada, saiu_para_cima: bool) -> Recolocar {
    let teste = PerfilEmTeste::from(perfil);
    Recolocar::calibrado(teste.perfil, entra_para_cima(perfil, saiu_para_cima))
}

/// Grava o perfil na fila. Se falhar, o arquivo fica como estava.
fn gravar_no_arquivo(
    caminho: Result<PathBuf, ErroPerfis>,
    fila: &str,
    perfil: PerfilDaRodada,
    calibracao: Calibracao,
) -> Result<(), Aviso> {
    if fila.trim().is_empty() {
        return Err(impressoras::falha_ao_gravar(fila));
    }
    let _vez = GRAVACAO_DE_PERFIS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let falhou = |_: ErroPerfis| impressoras::falha_ao_gravar(fila);
    let caminho = caminho.map_err(falhou)?;
    let mut perfis = plots_perfis::carregar(&caminho).map_err(falhou)?;
    perfis.gravar_calibracao(fila, perfil.into(), calibracao);
    plots_perfis::salvar(&caminho, &perfis).map_err(falhou)
}

/// Fim da calibracao de uma fila. A simulada nao tem perfil em arquivo: o identificador e recusado.
#[tauri::command(async)]
pub fn calibracao_gravar(
    impressora: String,
    perfil: PerfilDaRodada,
    saiu_para_cima: bool,
    leitura: LeituraRecebida,
) -> Result<(), Aviso> {
    let Some(fila) = impressora.strip_prefix("fila:") else {
        return Err(impressoras::falha_ao_gravar(&impressora));
    };
    gravar_no_arquivo(
        impressoras::caminho_dos_perfis_do_sistema(),
        fila,
        perfil,
        Calibracao {
            face_impressa_entra_para_cima: entra_para_cima(perfil, saiu_para_cima),
            le_virando: leitura == LeituraRecebida::Virando,
        },
    )
}

fn papel_de_codigo(codigo: &str) -> Option<PapelCalibracao> {
    match codigo {
        "a4" => Some(PapelCalibracao::A4),
        "carta" => Some(PapelCalibracao::Carta),
        _ => None,
    }
}

fn dimensoes_do_papel(papel: PapelCalibracao) -> Vec<DimensoesPagina> {
    let (largura, altura) = papel.dimensoes_pt();
    let em_mm = |pontos: u32| f64::from(pontos) * 25.4 / 72.0;
    vec![DimensoesPagina::new(em_mm(largura), em_mm(altura)); PAGINAS_DO_TESTE]
}

/// As passadas do teste neste perfil, ja com a compensacao do driver.
fn lotes_do_teste(
    teste: PerfilEmTeste,
    dimensoes: &[DimensoesPagina],
    automatico: bool,
) -> Result<Vec<LoteDoFluxo>, Motivo> {
    let pedido = Pedido::novo(String::new(), None);
    let mut lotes = trabalhos::planejar_pedido(dimensoes, &pedido, teste.perfil, automatico)?;
    if teste.driver_inverte {
        compensar_driver(&mut lotes);
    }
    Ok(lotes)
}

fn mecanica_da_simulada(id: &str) -> Option<Mecanica> {
    match id {
        SIMULADA_TRASEIRA | impressoras::ID_SIMULADA_AUTOMATICA => Some(Mecanica::traseira()),
        SIMULADA_GAVETA => Some(Mecanica::gaveta()),
        _ => None,
    }
}

/// A simulada faz exatamente o gesto do perfil em teste: um perfil errado sai errado.
fn preparar_simulada(
    id: &str,
    perfil: PerfilDaRodada,
    papel: PapelCalibracao,
) -> Result<Preparo<DriverDeTeste<SimGuiado>>, Motivo> {
    let mecanica = mecanica_da_simulada(id).ok_or_else(trabalhos::fila_indisponivel)?;
    let teste = PerfilEmTeste::from(perfil);
    let automatica = id == impressoras::ID_SIMULADA_AUTOMATICA;
    let dimensoes = dimensoes_do_papel(papel);
    let lotes = lotes_do_teste(teste, &dimensoes, automatica)?;
    let gesto: Recolocacao = teste.perfil.gesto_recolocar.into();
    let guiado = if automatica {
        SimGuiado::automatica(mecanica, gesto, dimensoes)
    } else {
        SimGuiado::novo(mecanica, gesto)
    };
    let backend = DriverDeTeste::novo(guiado, inversao::modo_de_teste() && !automatica);
    let condutor = Condutor::novo(backend, lotes, trabalhos::ESPERA_SIMULADA)
        .map_err(|erro| trabalhos::motivo_do_job(&erro))?;
    Ok(Preparo {
        condutor,
        recolocar: Recolocar::calibrado(teste.perfil, false),
        vira_para_ler: false,
    })
}

/// Cada envio a uma fila de arquivo tem a sua pasta: o teste repetido nao esbarra no anterior, e os
/// arquivos ficam para abrir e conferir.
#[cfg(windows)]
fn pasta_da_rodada(pasta: PathBuf) -> Result<PathBuf, Motivo> {
    let agora = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |decorrido| decorrido.as_millis());
    let destino = pasta.join(format!("calibracao-{agora}"));
    std::fs::create_dir_all(&destino).map_err(|erro| {
        Motivo::novo(
            "arquivo_saida",
            format!("não foi possível criar a pasta de saída: {erro}"),
        )
    })?;
    Ok(destino)
}

/// Fila real: as mesmas conferencias do envio de um documento, com o PDF de teste na memoria.
#[cfg(windows)]
fn preparar_fila(
    nome: &str,
    perfil: PerfilDaRodada,
    papel: PapelCalibracao,
) -> Result<Preparo<plots_windows::BackendWindows>, Motivo> {
    use crate::trabalhos::ParaMotivo;
    let perfis = impressoras::ler_perfis_do_sistema()
        .map_err(|erro| Motivo::novo("perfil", erro.to_string()))?;
    let automatico = perfis.obter(nome).is_some_and(|entrada| entrada.automatico);
    let teste = PerfilEmTeste::from(perfil);
    let dimensoes = dimensoes_do_papel(papel);
    let lotes = lotes_do_teste(teste, &dimensoes, automatico)?;
    let bytes =
        plots_pdf::pdf_calibracao(papel).map_err(|erro| Motivo::novo("pdf", erro.to_string()))?;
    let origem = plots_pdf::PdfOrigem::carregar(&bytes)
        .map_err(|erro| Motivo::novo("pdf", erro.to_string()))?;
    let pasta = impressoras::pasta_de_saida_do_sistema();
    let filas =
        plots_windows::listar_filas().map_err(|erro| Motivo::novo("sistema", erro.to_string()))?;
    let fila = plots_windows::decidir_fila(&filas, nome, pasta.is_some())
        .map_err(|recusa| trabalhos::motivo_da_recusa(nome, recusa))?;
    if impressoras::fila_com_problema(fila) {
        return Err(Motivo::novo(
            "fila",
            format!("a fila '{nome}' não pode receber agora; veja o estado dela na lista"),
        ));
    }
    let pasta = pasta.map(pasta_da_rodada).transpose()?;
    let backend =
        plots_windows::BackendWindows::novo(fila, origem, dimensoes.clone(), pasta.as_deref())
            .map_err(|erro| erro.para_motivo())?;
    let condutor = Condutor::novo(
        backend,
        lotes,
        plots_windows::plots_print::fluxo::ESPERA_REAL,
    )
    .map_err(|erro| trabalhos::motivo_do_job(&erro))?;
    Ok(Preparo {
        condutor,
        recolocar: Recolocar::calibrado(teste.perfil, false),
        vira_para_ler: false,
    })
}

fn iniciar(
    gerente: &Gerente,
    emitir: Emissor,
    impressora: String,
    perfil: PerfilDaRodada,
    papel: PapelCalibracao,
) -> bool {
    let simulada = mecanica_da_simulada(&impressora).is_some();
    let nome = impressora
        .strip_prefix("fila:")
        .unwrap_or(&impressora)
        .to_owned();
    let identidade = Identidade {
        aba: ABA_DA_CALIBRACAO,
        impressora: impressora.clone(),
        nome: nome.clone(),
        simulada,
    };
    let relogio = Arc::new(RelogioDoSistema::default());
    if simulada {
        return gerente.iniciar_medido(identidade, emitir, relogio, Box::new(|_| {}), move || {
            preparar_simulada(&impressora, perfil, papel)
        });
    }
    iniciar_na_fila(gerente, emitir, identidade, relogio, nome, perfil, papel)
}

#[cfg(windows)]
fn iniciar_na_fila(
    gerente: &Gerente,
    emitir: Emissor,
    identidade: Identidade,
    relogio: Arc<RelogioDoSistema>,
    nome: String,
    perfil: PerfilDaRodada,
    papel: PapelCalibracao,
) -> bool {
    gerente.iniciar_medido(identidade, emitir, relogio, Box::new(|_| {}), move || {
        preparar_fila(&nome, perfil, papel)
    })
}

#[cfg(not(windows))]
fn iniciar_na_fila(
    gerente: &Gerente,
    emitir: Emissor,
    identidade: Identidade,
    relogio: Arc<RelogioDoSistema>,
    _nome: String,
    _perfil: PerfilDaRodada,
    _papel: PapelCalibracao,
) -> bool {
    gerente.iniciar_medido::<SimGuiado, _>(identidade, emitir, relogio, Box::new(|_| {}), || {
        Err(trabalhos::fila_indisponivel())
    })
}

/// Comeca uma rodada. `false` quer dizer que a impressora ja tem um trabalho; nada foi enviado. O
/// verso e pedido depois, pelo mesmo comando dos documentos.
#[tauri::command]
pub fn calibracao_enviar(
    app: AppHandle,
    gerente: State<'_, Gerente>,
    impressora: String,
    perfil: PerfilDaRodada,
    papel: String,
) -> Result<bool, ()> {
    let papel = papel_de_codigo(&papel).ok_or(())?;
    let emitir: Emissor = Arc::new(move |cartao| {
        let _ = app.emit(EVENTO, cartao);
    });
    Ok(iniciar(&gerente, emitir, impressora, perfil, papel))
}

/// A pessoa saiu. Com uma passada em andamento, ela pede para tirar da fila; sem isso, nada e apagado.
#[tauri::command]
pub fn calibracao_encerrar(gerente: State<'_, Gerente>, impressora: String, cancelar: bool) {
    if cancelar {
        let _ = gerente.comandar(&impressora, Comando::Cancelar);
    }
    gerente.soltar_aba(ABA_DA_CALIBRACAO);
}

/// Uma folha da simulada, de cima para baixo, como quem a pega na mao.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolhaVista {
    para_cima: Vec<usize>,
    para_baixo: Vec<usize>,
    setas_iguais: bool,
}

/// O que saiu de uma simulada: e o unico jeito de responder as perguntas sem papel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaidaSimulada {
    saiu_para_cima: bool,
    primeira_a_sair: Option<usize>,
    folhas: Vec<FolhaVista>,
}

fn com_driver_da_simulada(mut paginas: Vec<Pagina>, compensa: bool, inverte: bool) -> Vec<Pagina> {
    if compensa != inverte {
        paginas.reverse();
    }
    paginas
}

fn ler_folha(folha: &plots_windows::plots_print::Folha) -> FolhaVista {
    let numeros = |face: Option<plots_windows::plots_print::Marca>, a: bool| {
        let mut lista: Vec<usize> = face.and_then(|marca| marca.pagina).into_iter().collect();
        if let Some((sobre_a, marca)) = folha.sobreimpressao {
            if sobre_a == a {
                lista.extend(marca.pagina);
            }
        }
        lista
    };
    let a = numeros(folha.face_a, true);
    let b = numeros(folha.face_b, false);
    let setas_iguais = match (folha.face_a, folha.face_b) {
        (Some(ma), Some(mb)) => ma.girada == mb.girada,
        _ => true,
    };
    let (para_cima, para_baixo) = if folha.face_a_para_cima {
        (a, b)
    } else {
        (b, a)
    };
    FolhaVista {
        para_cima,
        para_baixo,
        setas_iguais,
    }
}

fn saida_da_simulada(
    mecanica: Mecanica,
    automatica: bool,
    inverte: bool,
    perfil: PerfilDaRodada,
) -> Result<SaidaSimulada, ErroSimulacao> {
    let teste = PerfilEmTeste::from(perfil);
    let mut simulador = Simulador::novo(mecanica);
    let mut primeira_a_sair = None;
    if automatica {
        let paginas: Vec<Pagina> = (1..=PAGINAS_DO_TESTE)
            .map(|numero| Pagina {
                numero: Some(numero),
                girar_180: false,
                copia: 0,
            })
            .collect();
        simulador.imprimir_duas_faces(&paginas)?;
    } else {
        let lote = plots_core::plano_documento(
            PAGINAS_DO_TESTE,
            FOLHAS_DO_TESTE,
            plots_core::OrdemPassadas::default(),
            teste.perfil,
        )
        .ok()
        .and_then(|lotes| lotes.into_iter().next())
        .unwrap_or(plots_core::Lote {
            paginas: Vec::new(),
            passada1: Vec::new(),
            passada2: Vec::new(),
            duas_faces: false,
        });
        let frente = com_driver_da_simulada(lote.passada1, teste.driver_inverte, inverte);
        let verso = com_driver_da_simulada(lote.passada2, teste.driver_inverte, inverte);
        primeira_a_sair = verso.first().and_then(|pagina| pagina.numero);
        simulador.imprimir_frente(&frente)?;
        simulador.recolocar(teste.perfil.gesto_recolocar.into())?;
        simulador.imprimir_verso(&verso)?;
    }
    Ok(SaidaSimulada {
        saiu_para_cima: mecanica.saida_face_impressa_para_cima,
        primeira_a_sair,
        folhas: simulador.pilha().iter().rev().map(ler_folha).collect(),
    })
}

/// So a simulada responde: de uma fila real, quem sabe o que saiu e a pessoa.
#[tauri::command]
pub fn calibracao_saida_simulada(
    impressora: String,
    perfil: PerfilDaRodada,
) -> Result<SaidaSimulada, ()> {
    let mecanica = mecanica_da_simulada(&impressora).ok_or(())?;
    let automatica = impressora == impressoras::ID_SIMULADA_AUTOMATICA;
    let inverte = inversao::modo_de_teste() && !automatica;
    saida_da_simulada(mecanica, automatica, inverte, perfil).map_err(|_| ())
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::trabalhos::{Cartao, EstadoCartao};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc::{channel, Receiver};
    use std::time::Duration;

    fn de_modelo(modelo: ModeloImpressora) -> PerfilDaRodada {
        PerfilEmTeste::do_modelo(modelo, false).into()
    }

    const CERTAS: RespostasRecebidas = RespostasRecebidas {
        por_cima: false,
        par_certo: true,
        setas_iguais: true,
        leitura: LeituraRecebida::SemVirar,
        primeira_a_sair: None,
    };

    static PROXIMO: AtomicU64 = AtomicU64::new(0);

    struct Pasta(PathBuf);

    impl Pasta {
        fn nova() -> Self {
            let numero = PROXIMO.fetch_add(1, Ordering::Relaxed);
            let pasta = std::env::temp_dir()
                .join(format!("plots-calibracao-{}-{numero}", std::process::id()));
            std::fs::create_dir_all(&pasta).unwrap();
            Self(pasta)
        }
        fn perfis(&self) -> PathBuf {
            self.0.join("perfis.toml")
        }
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn o_perfil_inicial_e_o_do_modelo_e_codigo_estranho_e_recusado() {
        let jato = calibracao_perfil_inicial("jato".into(), true).unwrap();
        let teste = PerfilEmTeste::from(jato);
        assert_eq!(teste.perfil, ModeloImpressora::JatoDeTinta.perfil());
        assert!(teste.driver_inverte);
        assert_eq!(calibracao_perfil_inicial("marca".into(), false), Err(()));
    }

    #[test]
    fn perfil_vai_e_volta_sem_perder_nada() {
        for modelo in [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser] {
            let teste = PerfilEmTeste::do_modelo(modelo, true);
            assert_eq!(PerfilEmTeste::from(PerfilDaRodada::from(teste)), teste);
        }
    }

    #[test]
    fn a_avaliacao_diz_cada_mudanca_em_um_codigo() {
        let avaliacao = avaliar_rodada(
            de_modelo(ModeloImpressora::JatoDeTinta),
            RespostasRecebidas {
                setas_iguais: false,
                par_certo: false,
                leitura: LeituraRecebida::Virando,
                ..CERTAS
            },
        );
        assert!(!avaliacao.certo);
        assert_eq!(avaliacao.mudancas, ["frente_direta", "verso_sem_giro"]);
        let por_cima = avaliar_rodada(
            de_modelo(ModeloImpressora::JatoDeTinta),
            RespostasRecebidas {
                por_cima: true,
                ..CERTAS
            },
        );
        assert_eq!(por_cima.mudancas, ["virada_nao_virar"]);
    }

    #[test]
    fn o_lado_da_frase_vem_de_como_a_folha_saiu_e_da_virada() {
        let jato = calibracao_recolocar(de_modelo(ModeloImpressora::JatoDeTinta), true);
        assert_eq!(jato.lado, Some("baixo"));
        assert_eq!(jato.virada, Some("borda_de_cima"));
        let sem_virar = calibracao_recolocar(de_modelo(ModeloImpressora::Laser), false);
        assert_eq!(sem_virar.lado, Some("baixo"));
        assert_eq!(sem_virar.virada, Some("nao_virar"));
    }

    #[test]
    fn as_passadas_saem_do_perfil_em_teste_e_nao_de_um_salvo() {
        let dimensoes = dimensoes_do_papel(PapelCalibracao::A4);
        let teste = PerfilEmTeste::do_modelo(ModeloImpressora::JatoDeTinta, false);
        let lotes = lotes_do_teste(teste, &dimensoes, false).unwrap();
        assert_eq!(lotes.len(), 1);
        let numeros = |paginas: &[Pagina]| {
            paginas
                .iter()
                .map(|pagina| pagina.numero.unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(numeros(&lotes[0].lote.passada1), [4, 2]);
        assert_eq!(numeros(&lotes[0].lote.passada2), [3, 1]);
        assert!(lotes[0].lote.passada2.iter().all(|p| p.girar_180));

        let compensado = PerfilEmTeste::do_modelo(ModeloImpressora::JatoDeTinta, true);
        let lotes = lotes_do_teste(compensado, &dimensoes, false).unwrap();
        assert_eq!(numeros(&lotes[0].lote.passada1), [2, 4]);
        assert_eq!(numeros(&lotes[0].lote.passada2), [1, 3]);
    }

    #[test]
    fn no_automatico_as_quatro_paginas_saem_em_um_trabalho_na_ordem_do_documento() {
        let dimensoes = dimensoes_do_papel(PapelCalibracao::Carta);
        let teste = PerfilEmTeste::do_modelo(ModeloImpressora::JatoDeTinta, true);
        let lotes = lotes_do_teste(teste, &dimensoes, true).unwrap();
        assert_eq!(lotes.len(), 1);
        assert!(lotes[0].lote.duas_faces);
        assert!(lotes[0].lote.passada2.is_empty());
        let ordem: Vec<_> = lotes[0]
            .lote
            .passada1
            .iter()
            .map(|p| p.numero.unwrap())
            .collect();
        assert_eq!(ordem, [1, 2, 3, 4]);
    }

    #[test]
    fn so_os_papeis_oferecidos_sao_aceitos() {
        assert_eq!(papel_de_codigo("a4"), Some(PapelCalibracao::A4));
        assert_eq!(papel_de_codigo("carta"), Some(PapelCalibracao::Carta));
        assert_eq!(papel_de_codigo("A4"), None);
        assert_eq!(papel_de_codigo(""), None);
    }

    #[test]
    fn perfil_de_jato_na_simulada_de_gaveta_imprime_por_cima_e_a_saida_mostra() {
        let saida = saida_da_simulada(
            Mecanica::gaveta(),
            false,
            false,
            de_modelo(ModeloImpressora::JatoDeTinta),
        )
        .unwrap();
        assert!(!saida.saiu_para_cima);
        assert!(saida
            .folhas
            .iter()
            .any(|folha| folha.para_cima.len() > 1 || folha.para_baixo.len() > 1));
    }

    #[test]
    fn o_perfil_certo_da_simulada_traseira_deixa_o_1_por_cima() {
        let saida = saida_da_simulada(
            Mecanica::traseira(),
            false,
            false,
            de_modelo(ModeloImpressora::JatoDeTinta),
        )
        .unwrap();
        assert!(saida.saiu_para_cima);
        assert_eq!(saida.primeira_a_sair, Some(3));
        assert_eq!(saida.folhas[0].para_cima, [1]);
        assert_eq!(saida.folhas[0].para_baixo, [2]);
        assert_eq!(saida.folhas[1].para_cima, [3]);
    }

    #[test]
    fn driver_que_inverte_na_simulada_troca_a_primeira_a_sair() {
        let saida = saida_da_simulada(
            Mecanica::traseira(),
            false,
            true,
            de_modelo(ModeloImpressora::JatoDeTinta),
        )
        .unwrap();
        assert_eq!(saida.primeira_a_sair, Some(1));
    }

    #[test]
    fn a_simulada_automatica_imprime_o_par_nas_duas_faces() {
        let saida = saida_da_simulada(
            Mecanica::traseira(),
            true,
            false,
            de_modelo(ModeloImpressora::JatoDeTinta),
        )
        .unwrap();
        assert_eq!(saida.primeira_a_sair, None);
        assert!(saida.folhas.iter().all(|folha| folha.setas_iguais));
        let com_o_1 = saida
            .folhas
            .iter()
            .find(|folha| folha.para_cima.contains(&1) || folha.para_baixo.contains(&1))
            .unwrap();
        assert!(com_o_1.para_cima.contains(&2) || com_o_1.para_baixo.contains(&2));
    }

    #[test]
    fn so_o_identificador_das_simuladas_tem_saida_simulada() {
        let perfil = de_modelo(ModeloImpressora::Laser);
        assert!(calibracao_saida_simulada("simulada:gaveta".into(), perfil).is_ok());
        assert_eq!(
            calibracao_saida_simulada("fila:Sala".into(), perfil),
            Err(())
        );
        assert_eq!(calibracao_saida_simulada("Sala".into(), perfil), Err(()));
    }

    const LIDA: Calibracao = Calibracao {
        face_impressa_entra_para_cima: false,
        le_virando: true,
    };

    #[test]
    fn gravar_deixa_a_fila_calibrada_e_a_lista_mostra_e_escolher_modelo_apaga() {
        let pasta = Pasta::nova();
        let perfil = de_modelo(ModeloImpressora::Laser);
        gravar_no_arquivo(Ok(pasta.perfis()), "Sala", perfil, LIDA).unwrap();
        let lido = plots_perfis::carregar(&pasta.perfis()).unwrap();
        assert_eq!(lido.calibracao("Sala"), Some(LIDA));
        assert_eq!(lido.obter("Sala").unwrap().modelo, None);

        impressoras::gravar_modelo(Ok(pasta.perfis()), "Sala", "jato").unwrap();
        let lido = plots_perfis::carregar(&pasta.perfis()).unwrap();
        assert_eq!(lido.calibracao("Sala"), None);
    }

    #[test]
    fn arquivo_danificado_nao_e_sobrescrito_e_nome_vazio_nao_grava() {
        let pasta = Pasta::nova();
        std::fs::write(pasta.perfis(), "isto nao e toml [").unwrap();
        let perfil = de_modelo(ModeloImpressora::Laser);
        assert!(gravar_no_arquivo(Ok(pasta.perfis()), "Sala", perfil, LIDA).is_err());
        assert_eq!(
            std::fs::read_to_string(pasta.perfis()).unwrap(),
            "isto nao e toml ["
        );
        let vazio = Pasta::nova();
        assert!(gravar_no_arquivo(Ok(vazio.perfis()), " ", perfil, LIDA).is_err());
        assert!(!vazio.perfis().exists());
    }

    #[test]
    fn simulada_nao_grava_em_arquivo_nenhum() {
        let perfil = de_modelo(ModeloImpressora::Laser);
        let recusa = calibracao_gravar(
            "simulada:gaveta".into(),
            perfil,
            false,
            LeituraRecebida::Virando,
        );
        assert!(recusa.is_err());
    }

    fn cena(id: &str, perfil: PerfilDaRodada) -> (Gerente, Receiver<Cartao>) {
        let (envia, recebe) = channel();
        let envia = std::sync::Mutex::new(envia);
        let emitir: Emissor = Arc::new(move |cartao| {
            let _ = envia
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .send(cartao);
        });
        let gerente = Gerente::default();
        assert!(iniciar(
            &gerente,
            emitir,
            id.to_owned(),
            perfil,
            PapelCalibracao::A4
        ));
        (gerente, recebe)
    }

    fn esperar(cartoes: &Receiver<Cartao>, parar: impl Fn(&EstadoCartao) -> bool) -> Cartao {
        loop {
            let cartao = cartoes
                .recv_timeout(Duration::from_secs(20))
                .expect("o cartao chega");
            assert!(
                !matches!(cartao.estado, EstadoCartao::Erro { .. }),
                "{:?}",
                cartao.estado
            );
            if parar(&cartao.estado) {
                return cartao;
            }
        }
    }

    #[test]
    fn a_rodada_na_simulada_para_na_recolocacao_e_o_verso_leva_ao_fim_e_libera() {
        let perfil = de_modelo(ModeloImpressora::Laser);
        let (gerente, cartoes) = cena(SIMULADA_GAVETA, perfil);
        let aguardando = esperar(&cartoes, |estado| {
            matches!(estado, EstadoCartao::Aguardando)
        });
        assert_eq!(aguardando.aba, ABA_DA_CALIBRACAO);
        gerente
            .comandar(SIMULADA_GAVETA, Comando::EnviarVerso)
            .unwrap();
        esperar(&cartoes, |estado| matches!(estado, EstadoCartao::Concluido));
        // A thread acabou: a proxima rodada encontra a impressora livre.
        let mut livre = false;
        for _ in 0..200 {
            let (envia, _recebe) = channel::<Cartao>();
            let envia = std::sync::Mutex::new(envia);
            let emitir: Emissor = Arc::new(move |cartao| {
                let _ = envia
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .send(cartao);
            });
            if iniciar(
                &gerente,
                emitir,
                SIMULADA_GAVETA.to_owned(),
                perfil,
                PapelCalibracao::A4,
            ) {
                livre = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(livre);
        gerente.soltar_aba(ABA_DA_CALIBRACAO);
    }

    #[test]
    fn a_automatica_termina_sem_parar_na_recolocacao() {
        let (_gerente, cartoes) = cena(
            impressoras::ID_SIMULADA_AUTOMATICA,
            de_modelo(ModeloImpressora::JatoDeTinta),
        );
        let fim = esperar(&cartoes, |estado| {
            matches!(estado, EstadoCartao::Concluido | EstadoCartao::Aguardando)
        });
        assert!(matches!(fim.estado, EstadoCartao::Concluido));
    }

    #[test]
    fn impressora_ocupada_recusa_e_encerrar_solta() {
        let perfil = de_modelo(ModeloImpressora::JatoDeTinta);
        let (gerente, cartoes) = cena(SIMULADA_TRASEIRA, perfil);
        esperar(&cartoes, |estado| {
            matches!(estado, EstadoCartao::Aguardando)
        });
        let emitir: Emissor = Arc::new(|_| {});
        assert!(!iniciar(
            &gerente,
            emitir,
            SIMULADA_TRASEIRA.to_owned(),
            perfil,
            PapelCalibracao::A4
        ));
        gerente.soltar_aba(ABA_DA_CALIBRACAO);
    }

    #[test]
    fn a_lista_mostra_calibrada_so_na_fila_que_passou_pela_calibracao() {
        let fila = |nome: &str| plots_windows::Fila {
            nome: nome.to_owned(),
            driver: String::new(),
            porta: "USB001".to_owned(),
            padrao: false,
            estado: plots_windows::Estado::de_bits(0),
        };
        let mut perfis = plots_perfis::Perfis::new();
        perfis.gravar_calibracao(
            "Sala",
            PerfilEmTeste::do_modelo(ModeloImpressora::Laser, false),
            LIDA,
        );
        perfis.definir_modelo("Corredor", ModeloImpressora::Laser);
        let lista = impressoras::montar(Ok(vec![fila("Sala"), fila("Corredor")]), Ok(perfis));
        let modo = |nome: &str| {
            lista
                .impressoras
                .iter()
                .find(|impressora| impressora.nome == nome)
                .map(|impressora| impressora.modo)
        };
        assert_eq!(modo("Sala"), Some(impressoras::Modo::Calibrada));
        assert_eq!(modo("Corredor"), Some(impressoras::Modo::Laser));
    }

    #[test]
    fn o_recolocar_calibrado_diz_o_lado_e_a_virada_gravados() {
        let perfil = ModeloImpressora::JatoDeTinta.perfil();
        let recolocar = Recolocar::calibrado(perfil, true);
        assert_eq!(recolocar.lado, Some("cima"));
        assert_eq!(recolocar.virada, Some("borda_de_cima"));
        assert_eq!(recolocar.texto_do_perfil, None);
    }
}
