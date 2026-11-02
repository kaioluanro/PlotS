#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod documento;
mod fechamento;
mod janela;
mod lote;

use documento::{Documento, ErroDocumento};
use fechamento::{DecisaoDoPrazo, EsperaDeResposta};
use lote::LoteCaminhos;
use std::io::Write;
use std::process::ExitCode;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;
use tauri::{Manager, Runtime, State, Window, WindowEvent};

// Assincrono para que um caminho lento, como um compartilhamento de rede fora do ar, nao congele
// a janela enquanto o sistema responde.
#[tauri::command(async)]
fn abrir_documento(caminho: String) -> Result<Documento, ErroDocumento> {
    documento::abrir(&caminho)
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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Some(principal) = app.get_webview_window("main") {
                janela::ajustar_minimo(&principal.as_ref().window());
            }
            Ok(())
        })
        .on_window_event(|janela_atual, evento| match evento {
            WindowEvent::ScaleFactorChanged { .. } => janela::ajustar_minimo(janela_atual),
            WindowEvent::CloseRequested { .. } => vigiar_pedido_de_fechamento(janela_atual),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            abrir_documento,
            caminhos_iniciais,
            limitar_lote,
            pedido_de_fechamento_recebido
        ])
        .run(tauri::generate_context!());

    match resultado {
        Ok(()) => ExitCode::SUCCESS,
        Err(erro) => {
            let _ = writeln!(std::io::stderr(), "falha ao iniciar o PlotS: {erro}");
            ExitCode::FAILURE
        }
    }
}
