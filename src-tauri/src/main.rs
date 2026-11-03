#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod abertura;
mod comandos_trabalho;
mod documento;
mod fechamento;
mod grupos;
mod impressoras;
mod janela;
mod leitura_pdf;
mod lote;
mod preferencias;
mod trabalhos;

use abertura::{Andamento, Passagem};
use documento::{Documento, ErroDocumento};
use fechamento::{DecisaoDoPrazo, EsperaDeResposta};
use impressoras::{Aviso, ListaImpressoras};
use lote::LoteCaminhos;
use std::io::Write;
use std::process::ExitCode;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;
use tauri::{AppHandle, Manager, RunEvent, Runtime, State, Window, WindowEvent};

// Assincrono para que um caminho lento, como um compartilhamento de rede fora do ar, nao congele
// a janela enquanto o sistema responde.
#[tauri::command(async)]
fn abrir_documento(caminho: String) -> Result<Documento, ErroDocumento> {
    documento::abrir(&caminho)
}

/// Le o PDF inteiro. A leitura roda em uma thread propria, e o que a interromper vira recusa.
#[tauri::command]
async fn contar_paginas(caminho: String) -> Result<usize, ErroDocumento> {
    let falha = documento::falha_inesperada(&caminho);
    tauri::async_runtime::spawn_blocking(move || documento::contar_paginas(&caminho))
        .await
        .unwrap_or(Err(falha))
}

/// So le as filas e os perfis. O spooler pode demorar a responder, por isso a thread propria.
#[tauri::command]
async fn listar_impressoras() -> ListaImpressoras {
    tauri::async_runtime::spawn_blocking(impressoras::ler_do_sistema)
        .await
        .unwrap_or_else(|_| impressoras::sem_leitura())
}

/// Grava o perfil pronto do modelo. O disco pode demorar, por isso a thread propria.
#[tauri::command]
async fn definir_modelo(fila: String, modelo: String) -> Result<(), Aviso> {
    let interrompida = impressoras::falha_ao_gravar(&fila);
    tauri::async_runtime::spawn_blocking(move || impressoras::definir_modelo(&fila, &modelo))
        .await
        .unwrap_or(Err(interrompida))
}

/// Caminhos recebidos na linha de comando, como acontece no "Abrir com" do sistema.
#[tauri::command]
fn caminhos_iniciais() -> LoteCaminhos {
    lote::dos_argumentos(
        std::env::args_os()
            .skip(1)
            .map(|argumento| argumento.to_string_lossy().into_owned()),
    )
}

#[tauri::command]
fn limitar_lote(caminhos: Vec<String>) -> LoteCaminhos {
    lote::limitar(caminhos)
}

/// A interface avisa que recebeu o pedido de fechamento, antes de abrir qualquer dialogo.
#[tauri::command]
fn pedido_de_fechamento_recebido(espera: State<'_, Mutex<EsperaDeResposta>>) {
    espera
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .ao_receber_resposta();
}

/// Andamento do carregamento, lido pela tela de abertura.
#[tauri::command]
fn andamento_da_abertura(app: AppHandle) -> Andamento {
    abertura::andamento(&app)
}

/// A interface montou. Responde quando a janela principal ja foi mostrada, com os avisos.
#[tauri::command]
async fn interface_montou(app: AppHandle) -> Passagem {
    tauri::async_runtime::spawn_blocking(move || abertura::marcar_montou_e_esperar(&app))
        .await
        .unwrap_or(Passagem { avisos: Vec::new() })
}

fn com_espera<R: Runtime, T>(
    janela_atual: &Window<R>,
    acao: impl FnOnce(&mut EsperaDeResposta) -> T,
) -> Option<T> {
    let espera = janela_atual.try_state::<Mutex<EsperaDeResposta>>()?;
    let mut trava = espera.lock().unwrap_or_else(PoisonError::into_inner);
    Some(acao(&mut trava))
}

/// Vale para todo pedido. Sem ouvinte na interface a janela fecha na hora e a vigia morre com o
/// processo; com ouvinte, ela garante que uma interface travada nao prenda a janela.
fn vigiar_pedido_de_fechamento<R: Runtime>(janela_atual: &Window<R>) {
    let iniciar = com_espera(janela_atual, |espera| {
        espera.ao_pedir_fechamento(Instant::now())
    });
    if iniciar != Some(true) {
        return;
    }
    let janela_vigiada = janela_atual.clone();
    let iniciada = std::thread::Builder::new()
        .name("prazo-de-fechamento".into())
        .spawn(move || loop {
            let decisao = com_espera(&janela_vigiada, |espera| espera.decidir(Instant::now()));
            match decisao {
                Some(DecisaoDoPrazo::Esperar(restante)) => std::thread::sleep(restante),
                Some(DecisaoDoPrazo::Destruir) => {
                    preferencias::fechar_antes_de_destruir();
                    // Se o sistema recusar, o proximo pedido de fechamento tenta outra vez.
                    let _ = janela_vigiada.destroy();
                    return;
                }
                Some(DecisaoDoPrazo::Encerrar) | None => return,
            }
        });
    if iniciada.is_err() {
        com_espera(janela_atual, EsperaDeResposta::ao_falhar_vigia);
    }
}

fn main() -> ExitCode {
    let resultado = tauri::Builder::default()
        .manage(Mutex::new(EsperaDeResposta::default()))
        .manage(Mutex::new(abertura::Estado::default()))
        .manage(trabalhos::Gerente::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Some(principal) = app.get_webview_window("main") {
                janela::ajustar_minimo(&principal.as_ref().window());
            }
            preferencias::ligar_ao_envio(app.handle());
            abertura::iniciar(app.handle());
            Ok(())
        })
        .on_window_event(|janela_atual, evento| match evento {
            WindowEvent::ScaleFactorChanged { .. } if janela_atual.label() == "main" => {
                janela::ajustar_minimo(janela_atual)
            }
            // A abertura nao tem interface que responda ao pedido, e nao ocupa a vigia.
            WindowEvent::CloseRequested { api, .. } if janela_atual.label() != "abertura" => {
                if preferencias::adiar_fechamento(janela_atual) {
                    api.prevent_close();
                } else {
                    vigiar_pedido_de_fechamento(janela_atual)
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            abrir_documento,
            preferencias::abrir_preferencias,
            preferencias::estado_das_preferencias,
            preferencias::voltar_ao_padrao_da_fila,
            andamento_da_abertura,
            caminhos_iniciais,
            comandos_trabalho::comandar_trabalho,
            contar_paginas,
            definir_modelo,
            grupos::apagar_grupo,
            grupos::aplicar_grupo,
            grupos::listar_grupos,
            grupos::salvar_grupo,
            interface_montou,
            comandos_trabalho::iniciar_impressao,
            documento::ler_documento,
            limitar_lote,
            listar_impressoras,
            pedido_de_fechamento_recebido,
            comandos_trabalho::resumo_do_envio,
            comandos_trabalho::soltar_trabalhos
        ])
        .build(tauri::generate_context!());

    match resultado {
        Ok(app) => {
            app.run(|_app, evento| {
                if matches!(evento, RunEvent::Exit) {
                    preferencias::ao_encerrar();
                }
            });
            ExitCode::SUCCESS
        }
        Err(erro) => {
            let _ = writeln!(std::io::stderr(), "falha ao iniciar o PlotS: {erro}");
            ExitCode::FAILURE
        }
    }
}
