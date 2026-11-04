//! Janela do simulador de impressoras. A tela nunca espera o sistema: tudo que toca filas, spooler
//! ou PDF roda fora da linha da interface.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bandeja;
mod canal;
mod desenho;
mod erros;
mod estado;
mod historico;
#[path = "../../../src-tauri/src/janela.rs"]
mod janela;
mod marca;
mod ouvinte;
mod plataforma;
mod vista;

use bandeja::BandejaTela;
use erros::ErroTela;
use estado::{Estado, EstadoFila, FilaTela};
use plotsim_core::Pastas;
use plotsim_physics::{BordaQueEntra, Entrada, FaceNaEntrada, Gesto, Idioma};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

/// Nome do evento que manda a tela olhar as bandejas de novo.
const EVENTO_MUDOU: &str = "plotsim-mudou";
/// Sem resposta da interface, a janela aparece mesmo assim: nunca fica invisível.
const PRAZO_PARA_APARECER: Duration = Duration::from_secs(8);

type Resposta<T> = Result<T, ErroTela>;
type EstadoApp = Arc<Estado>;

/// Roda o trabalho fora da linha da interface. Nunca deixa a tela esperando.
async fn fora_da_tela<T: Send + 'static>(
    estado: &State<'_, EstadoApp>,
    trabalho: impl FnOnce(&Estado) -> Resposta<T> + Send + 'static,
) -> Resposta<T> {
    let estado = Arc::clone(estado.inner());
    tauri::async_runtime::spawn_blocking(move || trabalho(&estado))
        .await
        .unwrap_or_else(|_| Err(ErroTela::so("interno")))
}

fn idioma_de(idioma: &str) -> Idioma {
    if idioma.eq_ignore_ascii_case("en") {
        Idioma::En
    } else {
        Idioma::PtBr
    }
}

#[tauri::command]
async fn listar_filas(estado: State<'_, EstadoApp>) -> Resposta<Vec<FilaTela>> {
    fora_da_tela(&estado, Estado::listar).await
}

#[tauri::command]
async fn criar_fila(
    estado: State<'_, EstadoApp>,
    nome: String,
    jato: bool,
) -> Resposta<Vec<FilaTela>> {
    fora_da_tela(&estado, move |e| e.criar(&nome, jato)).await
}

#[tauri::command]
async fn remover_fila(
    estado: State<'_, EstadoApp>,
    nome: String,
    apagar_historico: bool,
) -> Resposta<Vec<FilaTela>> {
    fora_da_tela(&estado, move |e| e.remover(&nome, apagar_historico)).await
}

#[tauri::command]
async fn ver_bandeja(
    estado: State<'_, EstadoApp>,
    nome: String,
    idioma: String,
) -> Resposta<BandejaTela> {
    fora_da_tela(&estado, move |e| e.bandeja(&nome, idioma_de(&idioma))).await
}

#[tauri::command]
fn estados_das_filas(estado: State<'_, EstadoApp>) -> HashMap<String, EstadoFila> {
    estado.estados()
}

/// O gesto como a tela o manda. Os nomes são os da instrução.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EntradaPedida {
    Traseira,
    Gaveta,
    Manual,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FacePedida {
    ParaFrente,
    ParaTras,
    ParaCima,
    ParaBaixo,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BordaPedida {
    Topo,
    Base,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GestoPedido {
    entrada: EntradaPedida,
    face_impressa: FacePedida,
    borda_que_entra: BordaPedida,
    inverter_ordem: bool,
}

impl From<GestoPedido> for Gesto {
    fn from(pedido: GestoPedido) -> Self {
        Gesto {
            entrada: match pedido.entrada {
                EntradaPedida::Traseira => Entrada::Traseira,
                EntradaPedida::Gaveta => Entrada::Gaveta,
                EntradaPedida::Manual => Entrada::Manual,
            },
            face_impressa: match pedido.face_impressa {
                FacePedida::ParaFrente => FaceNaEntrada::ParaFrente,
                FacePedida::ParaTras => FaceNaEntrada::ParaTras,
                FacePedida::ParaCima => FaceNaEntrada::ParaCima,
                FacePedida::ParaBaixo => FaceNaEntrada::ParaBaixo,
            },
            borda_que_entra: match pedido.borda_que_entra {
                BordaPedida::Topo => BordaQueEntra::Topo,
                BordaPedida::Base => BordaQueEntra::Base,
            },
            inverter_ordem: pedido.inverter_ordem,
        }
    }
}

#[tauri::command]
async fn recolocar(estado: State<'_, EstadoApp>, nome: String, gesto: GestoPedido) -> Resposta<()> {
    fora_da_tela(&estado, move |e| {
        e.aplicar_gesto(&nome, gesto.into())
            .map_err(erros::do_gesto)
    })
    .await
}

#[tauri::command]
async fn limpar_bandeja(estado: State<'_, EstadoApp>, nome: String) -> Resposta<()> {
    fora_da_tela(&estado, move |e| e.limpar_bandeja(&nome)).await
}

/// Se o canal com o PlotS não abriu. A tela mostra o aviso enquanto for assim.
#[tauri::command]
fn canal_fechado(estado: State<'_, EstadoApp>) -> bool {
    estado.canal_fechado()
}

#[derive(Serialize)]
struct ResumoHistorico {
    arquivos: u64,
    bytes: u64,
}

#[tauri::command]
async fn resumo_do_historico(
    estado: State<'_, EstadoApp>,
    nome: String,
) -> Resposta<ResumoHistorico> {
    fora_da_tela(&estado, move |e| {
        e.resumo_do_historico(&nome)
            .map(|(arquivos, bytes)| ResumoHistorico { arquivos, bytes })
    })
    .await
}

#[tauri::command]
async fn limpar_historico(estado: State<'_, EstadoApp>, nome: String) -> Resposta<u64> {
    fora_da_tela(&estado, move |e| e.limpar_historico(&nome)).await
}

#[tauri::command]
async fn desenhar_pagina(
    estado: State<'_, EstadoApp>,
    nome: String,
    pagina: u32,
) -> Resposta<String> {
    fora_da_tela(&estado, move |e| {
        let (arquivo, indice) = e
            .origem_da_pagina(&nome, pagina)
            .ok_or_else(|| ErroTela::so("pagina_desconhecida"))?;
        desenho::desenhar(&arquivo, indice).map_err(|erro| erros::do_desenho(&erro))
    })
    .await
}

fn mostrar_janela(app: &AppHandle) {
    if let Some(janela) = app.get_webview_window("main") {
        // Se o sistema recusar, a reserva por prazo tenta de novo.
        let _ = janela.show();
    }
}

/// A interface montou: a janela aparece já desenhada, sem o clarão branco de uma página vazia.
#[tauri::command]
fn interface_montou(app: AppHandle) {
    mostrar_janela(&app);
}

fn main() -> ExitCode {
    let resultado = tauri::Builder::default()
        .setup(|app| {
            let manuseio = app.handle().clone();
            let avisar: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
                // Sem ninguém ouvindo, a consulta periódica da tela acaba vendo a mudança.
                let _ = manuseio.emit(EVENTO_MUDOU, ());
            });
            let estado: EstadoApp = Arc::new(Estado::novo(Pastas::do_sistema(), avisar));
            // Sem o canal (outro programa o tem, ou ele não abre), a janela segue só com a escolha da pessoa.
            // A tela consulta o resultado, para avisar que o gesto do modo de teste não chega.
            estado.guardar_canal(canal::iniciar(Arc::clone(&estado)).err());
            app.manage(estado);
            if let Some(principal) = app.get_webview_window("main") {
                janela::ajustar_minimo(&principal.as_ref().window());
            }
            let reserva = app.handle().clone();
            // Falha ao criar a linha só tira a reserva: a interface ainda mostra a janela.
            let _ = std::thread::Builder::new()
                .name("plotsim-mostrar-janela".to_string())
                .spawn(move || {
                    std::thread::sleep(PRAZO_PARA_APARECER);
                    mostrar_janela(&reserva);
                });
            Ok(())
        })
        .on_window_event(|janela_atual, evento| {
            if matches!(evento, WindowEvent::ScaleFactorChanged { .. }) {
                janela::ajustar_minimo(janela_atual);
            }
        })
        .invoke_handler(tauri::generate_handler![
            canal_fechado,
            criar_fila,
            desenhar_pagina,
            estados_das_filas,
            interface_montou,
            limpar_bandeja,
            limpar_historico,
            listar_filas,
            recolocar,
            remover_fila,
            resumo_do_historico,
            ver_bandeja
        ])
        .build(tauri::generate_context!());
    match resultado {
        Ok(app) => {
            app.run(|_, _| {});
            ExitCode::SUCCESS
        }
        Err(erro) => {
            let _ = writeln!(std::io::stderr(), "falha ao iniciar o PlotSim: {erro}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn idioma_desconhecido_cai_em_portugues() {
        assert_eq!(idioma_de("en"), Idioma::En);
        assert_eq!(idioma_de("EN"), Idioma::En);
        assert_eq!(idioma_de("pt-BR"), Idioma::PtBr);
        assert_eq!(idioma_de("zz"), Idioma::PtBr);
    }

    #[test]
    fn o_gesto_da_tela_chega_ao_modelo_com_os_mesmos_campos() {
        let gesto: Gesto = GestoPedido {
            entrada: EntradaPedida::Traseira,
            face_impressa: FacePedida::ParaTras,
            borda_que_entra: BordaPedida::Base,
            inverter_ordem: true,
        }
        .into();
        assert_eq!(gesto.entrada, Entrada::Traseira);
        assert_eq!(gesto.face_impressa, FaceNaEntrada::ParaTras);
        assert_eq!(gesto.borda_que_entra, BordaQueEntra::Base);
        assert!(gesto.inverter_ordem);
    }
}
