//! Comandos que a interface usa para conduzir os trabalhos de impressao.

use crate::documento::{self, ErroDocumento};
use crate::trabalhos::{
    self, Comando, Emissor, Gerente, Identidade, Motivo, Pedido, ResumoDoEnvio,
};
use plots_core::ModeloImpressora;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Le o PDF para a confirmacao dizer paginas, lotes e papeis antes de qualquer envio.
#[tauri::command]
pub async fn resumo_do_envio(
    caminho: String,
    folhas: usize,
) -> Result<ResumoDoEnvio, ErroDocumento> {
    let falha = documento::falha_inesperada(&caminho);
    tauri::async_runtime::spawn_blocking(move || trabalhos::resumo_do_envio(&caminho, folhas))
        .await
        .unwrap_or(Err(falha))
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
) -> Vec<String> {
    let emitir: Emissor = Arc::new(move |cartao| {
        // Sem janela nao ha quem mostrar; o trabalho segue e termina sozinho.
        let _ = app.emit(trabalhos::EVENTO_DO_CARTAO, cartao);
    });
    impressoras
        .into_iter()
        .filter(|impressora| {
            iniciar_um(
                &gerente,
                Arc::clone(&emitir),
                aba,
                impressora,
                Pedido::novo(caminho.clone(), folhas),
            )
        })
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
