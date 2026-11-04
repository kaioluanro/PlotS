//! A vez de enviar, para o driver que não aguenta dois trabalhos ao mesmo tempo.
//!
//! O "Microsoft Print To PDF", nos testes locais, falhou no processador de impressão ao receber
//! dois trabalhos juntos, mesmo de filas diferentes: o evento 372 apareceu e os dois (ou um)
//! saíram sem páginas.
//! Para esse driver, o envio espera o trabalho anterior sair da fila. Os outros drivers seguem em
//! paralelo, porque a impressão simultânea é o motivo de marcar várias impressoras.
//!
//! A vez é um semáforo nomeado do Windows: vale entre as threads da janela e também entre
//! processos, como duas instâncias do PlotS ou do `plots imprimir`, e não pertence a uma thread.
//! Se quem a tem morrer sem devolvê-la, o prazo da espera destrava os outros.

use std::time::Duration;

/// O nome do driver que precisa de uma fila por vez.
const DRIVER_DE_UMA_VEZ: &str = "Microsoft Print To PDF";

/// Quanto esperar a vez antes de enviar mesmo assim: mais que isso seria travar o trabalho por um
/// anterior que não acaba.
pub(crate) const PRAZO_DA_VEZ: Duration = Duration::from_secs(90);

/// O semáforo da sessão do Windows; vale para todos os processos do usuário.
pub(crate) const NOME_DA_VEZ_DO_PDF: &str = "Local\\PlotS-vez-do-pdf";

/// Este driver pede uma fila por vez.
pub(crate) fn driver_pede_uma_fila_por_vez(driver: &str) -> bool {
    driver.trim().eq_ignore_ascii_case(DRIVER_DE_UMA_VEZ)
}

#[cfg(windows)]
pub(crate) use sistema::Posse;

#[cfg(windows)]
mod sistema {
    use std::time::Duration;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{
        CreateSemaphoreW, ReleaseSemaphore, WaitForSingleObject,
    };

    /// Quem a tem: ao sair de escopo, devolve a vez.
    pub(crate) struct Posse(HANDLE);

    // Garantia: a alça de um semáforo pode ser usada e fechada de qualquer thread.
    unsafe impl Send for Posse {}

    impl Posse {
        /// Espera a vez até o prazo. `None` quando o prazo acabou ou o sistema recusou o semáforo:
        /// quem chama segue sem ela, como antes.
        pub(crate) fn pegar(nome: &str, prazo: Duration) -> Option<Self> {
            let nome: Vec<u16> = nome.encode_utf16().chain(std::iter::once(0)).collect();
            // Garantia: o nome termina em zero e vive durante a chamada.
            let alca =
                unsafe { CreateSemaphoreW(None, 1, 1, PCWSTR::from_raw(nome.as_ptr())) }.ok()?;
            let ms = u32::try_from(prazo.as_millis()).unwrap_or(u32::MAX - 1);
            // Garantia: `alca` é uma alça de semáforo válida e aberta.
            if unsafe { WaitForSingleObject(alca, ms) } == WAIT_OBJECT_0 {
                Some(Self(alca))
            } else {
                // Garantia: a alça é nossa e não foi fechada.
                let _ = unsafe { CloseHandle(alca) };
                None
            }
        }
    }

    impl Drop for Posse {
        fn drop(&mut self) {
            // Garantia: a alça é válida, a vez foi tomada por esta posse e é devolvida uma vez só.
            unsafe {
                let _ = ReleaseSemaphore(self.0, 1, None);
                let _ = CloseHandle(self.0);
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn so_o_microsoft_print_to_pdf_pede_uma_fila_por_vez() {
        assert!(driver_pede_uma_fila_por_vez("Microsoft Print To PDF"));
        assert!(driver_pede_uma_fila_por_vez(" microsoft print to pdf "));
        assert!(!driver_pede_uma_fila_por_vez(
            "Microsoft XPS Document Writer"
        ));
        assert!(!driver_pede_uma_fila_por_vez("HP Universal Printing PCL 6"));
        assert!(!driver_pede_uma_fila_por_vez(""));
    }

    #[cfg(windows)]
    mod no_windows {
        use super::super::Posse;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        fn nome(sufixo: &str) -> String {
            format!("Local\\PlotS-teste-{}-{sufixo}", std::process::id())
        }

        #[test]
        fn duas_filas_nunca_enviam_juntas() {
            static DENTRO: AtomicUsize = AtomicUsize::new(0);
            static MAIOR: AtomicUsize = AtomicUsize::new(0);
            let nome = nome("juntas");
            let filas: Vec<_> = (0..4)
                .map(|_| {
                    let nome = nome.clone();
                    std::thread::spawn(move || {
                        let posse = Posse::pegar(&nome, Duration::from_secs(10)).expect("vez");
                        let agora = DENTRO.fetch_add(1, Ordering::SeqCst) + 1;
                        MAIOR.fetch_max(agora, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(30));
                        DENTRO.fetch_sub(1, Ordering::SeqCst);
                        drop(posse);
                    })
                })
                .collect();
            for fila in filas {
                fila.join().unwrap();
            }
            assert_eq!(MAIOR.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn passado_o_prazo_a_fila_segue_sem_a_vez_e_a_devolvida_libera_a_proxima() {
            let nome = nome("prazo");
            let primeira = Posse::pegar(&nome, Duration::from_secs(1)).expect("vez livre");
            assert!(Posse::pegar(&nome, Duration::from_millis(50)).is_none());
            drop(primeira);
            assert!(Posse::pegar(&nome, Duration::from_millis(50)).is_some());
        }

        #[test]
        fn a_vez_passa_de_uma_thread_para_outra() {
            // Quem pega e quem devolve nao precisam ser a mesma thread.
            let nome = nome("threads");
            let posse = Posse::pegar(&nome, Duration::from_secs(1)).expect("vez livre");
            std::thread::spawn(move || drop(posse)).join().unwrap();
            assert!(Posse::pegar(&nome, Duration::from_millis(50)).is_some());
        }
    }
}
