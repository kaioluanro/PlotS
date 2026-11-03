//! Dona da janela do driver e prova real da janela da fila de PDF, com uma dona de verdade.

use super::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use windows::core::w;
use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, PostThreadMessageW, MSG,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_QUIT,
};

#[derive(Clone, Default)]
struct DonaFalsa(Rc<RefCell<Vec<bool>>>);

impl JanelaDona for DonaFalsa {
    fn habilitar(&self, habilitada: bool) {
        self.0.borrow_mut().push(habilitada);
    }
}

#[test]
fn a_dona_fica_desabilitada_durante_o_dialogo_e_volta_ao_sair() {
    let contagem = Mutex::new(0);
    let dona = DonaFalsa::default();
    let guarda = DonaDesabilitada::nova(dona.clone(), &contagem);
    assert_eq!(*dona.0.borrow(), [false]);
    drop(guarda);
    assert_eq!(*dona.0.borrow(), [false, true]);
}

#[test]
fn a_dona_volta_mesmo_quando_o_caminho_sai_com_erro() {
    fn abre_e_falha(dona: DonaFalsa, contagem: &Mutex<u32>) -> Result<(), ()> {
        let _guarda = DonaDesabilitada::nova(dona, contagem);
        Err(())
    }
    let contagem = Mutex::new(0);
    let dona = DonaFalsa::default();
    assert_eq!(abre_e_falha(dona.clone(), &contagem), Err(()));
    assert_eq!(*dona.0.borrow(), [false, true]);
}

#[test]
fn com_dois_dialogos_a_dona_so_volta_no_ultimo() {
    let contagem = Mutex::new(0);
    let dona = DonaFalsa::default();
    let primeiro = DonaDesabilitada::nova(dona.clone(), &contagem);
    let segundo = DonaDesabilitada::nova(dona.clone(), &contagem);
    drop(primeiro);
    assert_eq!(*dona.0.borrow(), [false]);
    drop(segundo);
    assert_eq!(*dona.0.borrow(), [false, true]);
}

#[test]
fn dona_que_nao_existe_mais_nao_chega_ao_driver() {
    assert_eq!(dona_valida(0), 0);
    assert_eq!(dona_valida(0x7FFF_FF00), 0);
}

#[test]
fn sem_janela_aberta_fechar_termina_na_hora_e_vir_a_frente_nao_acha_nada() {
    let fila = "Fila sem janela nenhuma 91ab";
    assert!(fechar_janelas_do_driver(Duration::from_secs(5)));
    assert!(trazer_janela_do_driver_para_frente(fila).is_empty());
}

#[test]
fn thread_que_nao_existe_nao_tem_janelas() {
    assert!(janelas_da_thread(u32::MAX).is_empty());
}

/// Uma janela dona de verdade, invisível, em uma thread com fila de mensagens própria: o diálogo
/// fala com ela de outra thread, como faz com a janela principal.
struct DonaReal {
    janela: isize,
    thread: u32,
    laco: std::thread::JoinHandle<()>,
}

impl DonaReal {
    fn nova() -> Self {
        let (envia, recebe) = mpsc::channel();
        let laco = std::thread::spawn(move || {
            // Garantia: classe do sistema, sem dados de criação.
            let janela = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!("Dona"),
                    WINDOW_STYLE(0),
                    0,
                    0,
                    10,
                    10,
                    None,
                    None,
                    None,
                    None,
                )
            }
            .unwrap();
            // Garantia: sem pré-condições.
            let thread = unsafe { GetCurrentThreadId() };
            envia.send((janela.0 as isize, thread)).unwrap();
            let mut mensagem = MSG::default();
            // Garantia: `mensagem` vive durante o laço; a saída vem de `WM_QUIT` ou de um erro.
            while unsafe { GetMessageW(&mut mensagem, None, 0, 0) }.0 > 0 {
                unsafe { DispatchMessageW(&mensagem) };
            }
            // Garantia: a janela é desta thread.
            let _ = unsafe { DestroyWindow(janela) };
        });
        let (janela, thread) = recebe.recv_timeout(Duration::from_secs(10)).unwrap();
        Self {
            janela,
            thread,
            laco,
        }
    }

    fn habilitada(&self) -> bool {
        // Garantia: aceita qualquer identificador.
        unsafe { IsWindowEnabled(HWND(self.janela as *mut core::ffi::c_void)) }.as_bool()
    }

    fn encerrar(self) {
        // Garantia: mensagem sem dados para a thread criada acima.
        let _ = unsafe { PostThreadMessageW(self.thread, WM_QUIT, WPARAM(0), LPARAM(0)) };
        let _ = self.laco.join();
    }
}

#[test]
#[ignore = "abre a janela real da fila de PDF; fecha sozinha, sem captura e sem teclas"]
fn janela_real_com_dona_desabilita_fecha_e_devolve_a_dona() {
    const FILA: &str = "Microsoft Print to PDF";
    let dona = DonaReal::nova();
    let janela_dona = dona.janela;
    assert!(dona.habilitada());
    let (envia, recebe) = mpsc::channel();
    let dialogo = std::thread::Builder::new()
        .name("janela-do-driver-teste".into())
        .spawn(move || {
            let _ = envia.send(abrir_janela_do_driver(FILA, janela_dona, None));
        })
        .unwrap();
    let inicio = Instant::now();
    let mut titulos = Vec::new();
    while titulos.is_empty() && inicio.elapsed() < Duration::from_secs(30) {
        titulos = trazer_janela_do_driver_para_frente(FILA);
        std::thread::sleep(Duration::from_millis(100));
    }
    eprintln!("janela com dona, títulos: {titulos:?}");
    let a_dona_estava_desabilitada = !dona.habilitada();
    let fechou = fechar_janelas_do_driver(Duration::from_secs(10));
    let resposta = recebe.recv_timeout(Duration::from_secs(10));
    let terminou = fechou && dialogo.join().is_ok();
    let a_dona_voltou = dona.habilitada();
    dona.encerrar();
    assert!(!titulos.is_empty(), "a janela apareceu");
    assert!(a_dona_estava_desabilitada);
    assert!(terminou, "a thread do diálogo terminou");
    assert_eq!(resposta.unwrap(), Ok(RespostaDaJanela::Cancelada));
    assert!(a_dona_voltou);
}
