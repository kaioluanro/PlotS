//! A janela de preferências do driver vista de fora dela: em que thread está, quem é a dona e como
//! fechá-la. Ela roda em uma thread própria, com COM iniciado, e a janela principal fica desabilitada
//! enquanto isso, como em qualquer diálogo modal.

use crate::erro::ErroEnvio;
use crate::preferencias::{ErroPreferencias, RespostaDaJanela};
use crate::preferencias_driver::{abrir_fila, perguntar};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumThreadWindows, GetWindowTextW, IsIconic, IsWindow, IsWindowVisible, PostMessageW,
    SetForegroundWindow, ShowWindow, SW_RESTORE, WM_CLOSE,
};

/// Fila com a janela aberta e a thread em que ela roda.
static DIALOGOS: Mutex<BTreeMap<String, u32>> = Mutex::new(BTreeMap::new());
/// Quantos diálogos seguram a janela principal desabilitada.
static SEGURANDO_A_DONA: Mutex<u32> = Mutex::new(0);
/// O programa está terminando: nenhuma janela nova abre.
static ENCERRANDO: AtomicBool = AtomicBool::new(false);

/// De quanto em quanto tempo o fechamento repete o pedido: a janela pode ainda estar aparecendo.
const INTERVALO_DO_PEDIDO: Duration = Duration::from_millis(250);
const INTERVALO_DA_CONSULTA: Duration = Duration::from_millis(20);
/// Maior título lido, em unidades UTF-16.
const LIMITE_DO_TITULO: usize = 256;

/// Marca a thread desta fila enquanto a janela existe; sai em todo caminho de saída.
struct Registro {
    fila: String,
}

impl Registro {
    fn novo(fila: &str) -> Self {
        // Garantia: a função não tem pré-condições.
        let thread = unsafe { GetCurrentThreadId() };
        DIALOGOS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(fila.to_owned(), thread);
        Self {
            fila: fila.to_owned(),
        }
    }
}

impl Drop for Registro {
    fn drop(&mut self) {
        DIALOGOS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.fila);
    }
}

/// COM em apartamento desta thread: há driver que precisa dele para desenhar a janela.
struct ComDaThread {
    iniciado: bool,
}

impl ComDaThread {
    fn iniciar() -> Self {
        // Garantia: sem reservado, e cada início bem-sucedido tem o seu encerramento em `drop`.
        let resposta = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        Self {
            iniciado: resposta.is_ok(),
        }
    }
}

impl Drop for ComDaThread {
    fn drop(&mut self) {
        if self.iniciado {
            // Garantia: esta mesma thread iniciou COM, e a guarda não sai dela.
            unsafe { CoUninitialize() };
        }
    }
}

/// O que o diálogo faz com a janela dona. Nos testes, um falso responde no lugar dela.
pub(crate) trait JanelaDona {
    fn habilitar(&self, habilitada: bool);
}

struct JanelaDoSistema(isize);

impl JanelaDona for JanelaDoSistema {
    fn habilitar(&self, habilitada: bool) {
        let janela = HWND(self.0 as *mut core::ffi::c_void);
        // Garantia: a janela é conferida na hora, e as duas chamadas aceitam qualquer identificador.
        // A resposta de `EnableWindow` diz o estado anterior, não uma falha.
        unsafe {
            if IsWindow(Some(janela)).as_bool() {
                let _ = EnableWindow(janela, habilitada);
                if habilitada {
                    // O sistema pode negar: a janela continua usável, só não vem para a frente.
                    let _ = SetForegroundWindow(janela);
                }
            }
        }
    }
}

/// Desabilita a dona ao entrar e reabilita ao sair, em todo caminho de saída. Com dois diálogos ao
/// mesmo tempo, ela só volta quando o último fechar.
pub(crate) struct DonaDesabilitada<'a, J: JanelaDona> {
    dona: J,
    contagem: &'a Mutex<u32>,
}

impl<'a, J: JanelaDona> DonaDesabilitada<'a, J> {
    pub(crate) fn nova(dona: J, contagem: &'a Mutex<u32>) -> Self {
        let mut segurando = contagem.lock().unwrap_or_else(PoisonError::into_inner);
        if *segurando == 0 {
            dona.habilitar(false);
        }
        *segurando = segurando.saturating_add(1);
        drop(segurando);
        Self { dona, contagem }
    }
}

impl<J: JanelaDona> Drop for DonaDesabilitada<'_, J> {
    fn drop(&mut self) {
        let mut segurando = self.contagem.lock().unwrap_or_else(PoisonError::into_inner);
        *segurando = segurando.saturating_sub(1);
        if *segurando == 0 {
            self.dona.habilitar(true);
        }
    }
}

/// A dona só vale se ainda existir; senão, a janela do driver abre sem dona.
pub(crate) fn dona_valida(dono: isize) -> isize {
    if dono == 0 {
        return 0;
    }
    // Garantia: `IsWindow` aceita qualquer identificador.
    let existe = unsafe { IsWindow(Some(HWND(dono as *mut core::ffi::c_void))) };
    if existe.as_bool() {
        dono
    } else {
        0
    }
}

/// Abre a janela de preferências do driver e espera a pessoa. Bloqueia enquanto ela está aberta:
/// chame em uma thread dedicada, longe da interface. `dono` é a janela dona, ou zero. Nada muda no
/// padrão do Windows.
pub fn abrir_janela_do_driver(
    fila: &str,
    dono: isize,
    salvo: Option<&[u8]>,
) -> Result<RespostaDaJanela, ErroPreferencias> {
    if ENCERRANDO.load(Ordering::SeqCst) {
        return Err(ErroPreferencias::Envio(ErroEnvio::Resposta(
            "o programa está fechando",
        )));
    }
    let _registro = Registro::novo(fila);
    let _com = ComDaThread::iniciar();
    let dono = dona_valida(dono);
    let _dona =
        (dono != 0).then(|| DonaDesabilitada::nova(JanelaDoSistema(dono), &SEGURANDO_A_DONA));
    let impressora = abrir_fila(fila)?;
    perguntar(&impressora, salvo, dono).map_err(ErroPreferencias::Envio)
}

unsafe extern "system" fn juntar_visivel(janela: HWND, lista: LPARAM) -> windows::core::BOOL {
    // Garantia: `lista` é o `Vec` de `janelas_da_thread`, vivo e exclusivo durante a enumeração, e
    // este retorno de chamada roda na mesma thread que a chamou.
    let lista = unsafe { &mut *(lista.0 as *mut Vec<isize>) };
    // Garantia: o identificador veio do próprio sistema. A thread também tem janelas invisíveis, que
    // não são o diálogo e não devem receber pedido nenhum.
    if unsafe { IsWindowVisible(janela) }.as_bool() {
        lista.push(janela.0 as isize);
    }
    true.into()
}

fn janelas_da_thread(thread: u32) -> Vec<isize> {
    let mut lista: Vec<isize> = Vec::new();
    // Garantia: `lista` vive durante a chamada, que só volta depois do último retorno de chamada.
    // A resposta é falsa também quando a thread não tem janela, e a lista já diz isso.
    let _ = unsafe {
        EnumThreadWindows(
            thread,
            Some(juntar_visivel),
            LPARAM(std::ptr::from_mut(&mut lista) as isize),
        )
    };
    lista
}

fn titulo(janela: isize) -> String {
    let mut texto = [0u16; LIMITE_DO_TITULO];
    // Garantia: o sistema escreve no máximo o tamanho da fatia e aceita qualquer identificador.
    let lidos = unsafe { GetWindowTextW(HWND(janela as *mut core::ffi::c_void), &mut texto) };
    let lidos = usize::try_from(lidos).unwrap_or(0);
    String::from_utf16_lossy(texto.get(..lidos).unwrap_or(&[]))
}

fn threads_abertas() -> Vec<u32> {
    DIALOGOS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .values()
        .copied()
        .collect()
}

/// Há alguma janela de preferências aberta agora.
pub fn ha_janela_do_driver_aberta() -> bool {
    !threads_abertas().is_empty()
}

/// Traz para a frente a janela já aberta desta fila e devolve os títulos que ela mostra.
pub fn trazer_janela_do_driver_para_frente(fila: &str) -> Vec<String> {
    let thread = DIALOGOS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(fila)
        .copied();
    let Some(thread) = thread else {
        return Vec::new();
    };
    let janelas = janelas_da_thread(thread);
    for janela in &janelas {
        let janela = HWND(*janela as *mut core::ffi::c_void);
        // Garantia: as duas chamadas aceitam qualquer identificador. O sistema pode negar o pedido
        // de vir à frente: a janela continua aberta como estava.
        unsafe {
            if IsIconic(janela).as_bool() {
                let _ = ShowWindow(janela, SW_RESTORE);
            }
            let _ = SetForegroundWindow(janela);
        }
    }
    janelas.into_iter().map(titulo).collect()
}

/// Pede a cada janela aberta para fechar, como se a pessoa cancelasse, e espera até `prazo`. Diz se
/// todas fecharam. Nunca rode na thread da interface: o diálogo precisa dela para devolver a dona.
pub fn fechar_janelas_do_driver(prazo: Duration) -> bool {
    let inicio = Instant::now();
    let mut proximo_pedido = inicio;
    loop {
        let threads = threads_abertas();
        if threads.is_empty() {
            return true;
        }
        let agora = Instant::now();
        if agora >= proximo_pedido {
            proximo_pedido = agora + INTERVALO_DO_PEDIDO;
            for janela in threads.into_iter().flat_map(janelas_da_thread) {
                // Garantia: a fila de mensagens aceita qualquer identificador. Sem bloquear: se a
                // janela já não existir, a próxima volta vê a lista mais curta.
                let _ = unsafe {
                    PostMessageW(
                        Some(HWND(janela as *mut core::ffi::c_void)),
                        WM_CLOSE,
                        WPARAM(0),
                        LPARAM(0),
                    )
                };
            }
        }
        if inicio.elapsed() >= prazo {
            return false;
        }
        std::thread::sleep(INTERVALO_DA_CONSULTA);
    }
}

/// O programa está terminando: fecha o que estiver aberto e não deixa abrir mais nada.
pub fn encerrar_janelas_do_driver(prazo: Duration) -> bool {
    ENCERRANDO.store(true, Ordering::SeqCst);
    fechar_janelas_do_driver(prazo)
}

#[cfg(test)]
#[path = "preferencias_janela_testes.rs"]
mod testes;
