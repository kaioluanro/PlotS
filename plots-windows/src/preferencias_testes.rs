//! Lógica do ajuste do driver com um driver falso: nenhuma janela abre e nada vai para a fila.

use super::*;
use crate::papel::{FacesDoEnvio, UMA_POR_FOLHA};
use crate::spooler::BufferDevmode;
use std::cell::{Cell, RefCell};
use windows::Win32::Graphics::Gdi::{
    DEVMODE_DUPLEX, DEVMODE_FIELD_FLAGS, DMCOLOR_COLOR, DMCOLOR_MONOCHROME, DMDUP_SIMPLEX,
    DMDUP_VERTICAL, DM_COLLATE, DM_COLOR, DM_COPIES, DM_DEFAULTSOURCE, DM_DUPLEX, DM_NUP,
    DM_ORIENTATION, DM_PAPERSIZE, DM_PRINTQUALITY, DM_SCALE, DM_YRESOLUTION,
};

const EXTRA: u16 = 32;
const VERSAO_DO_DRIVER: u16 = 0x0300;
/// Qualidade que marca nos testes um ajuste feito pela pessoa.
const QUALIDADE_DA_PESSOA: i16 = 1200;
/// Qualidade que este driver falso não aceita mais.
const QUALIDADE_RECUSADA: i16 = -77;
const A4: i16 = 9;
const CARTA: i16 = 1;

fn total() -> usize {
    std::mem::size_of::<DEVMODEW>() + usize::from(EXTRA)
}

/// O `DEVMODE` que o driver falso devolve como padrão da fila.
fn padrao() -> BufferDevmode {
    let mut buffer = BufferDevmode::com_tamanho(total() as i32).unwrap();
    // Garantia: o buffer tem pelo menos o tamanho de `DEVMODEW`; as escritas são em inteiros.
    unsafe {
        let modo = &mut *buffer.ponteiro_mut();
        modo.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        modo.dmDriverExtra = EXTRA;
        modo.dmDriverVersion = VERSAO_DO_DRIVER;
        modo.dmSpecVersion = 0x0401;
        modo.dmDeviceName[0] = u16::from(b'F');
        modo.dmDuplex = DMDUP_SIMPLEX;
        modo.Anonymous1.Anonymous1.dmCopies = 1;
        modo.Anonymous1.Anonymous1.dmScale = 100;
        modo.Anonymous1.Anonymous1.dmOrientation = 1;
        modo.Anonymous1.Anonymous1.dmPaperSize = A4;
        modo.Anonymous2.dmNup = 1;
    }
    buffer
}

/// Ajuste de quem pediu tudo o que o envio não pode deixar passar.
fn salvo_de_duas_faces() -> Vec<u8> {
    let mut buffer = padrao();
    // Garantia: como em `padrao`.
    unsafe {
        let modo = &mut *buffer.ponteiro_mut();
        modo.dmFields |= DM_DUPLEX | DM_NUP | DM_COPIES | DM_PAPERSIZE | DM_SCALE | DM_ORIENTATION;
        modo.dmDuplex = DMDUP_VERTICAL;
        modo.Anonymous1.Anonymous1.dmPrintQuality = QUALIDADE_DA_PESSOA;
        modo.Anonymous1.Anonymous1.dmCopies = 3;
        modo.Anonymous1.Anonymous1.dmScale = 50;
        modo.Anonymous1.Anonymous1.dmOrientation = 2;
        modo.Anonymous1.Anonymous1.dmPaperSize = CARTA;
        modo.Anonymous2.dmNup = 4;
    }
    buffer.bytes().unwrap()
}

fn qualidade(buffer: &BufferDevmode) -> i16 {
    // Garantia: o buffer tem o tamanho de `DEVMODEW`, e os campos lidos são inteiros.
    unsafe { (*buffer.ponteiro()).Anonymous1.Anonymous1.dmPrintQuality }
}

struct Falso {
    resposta_da_janela: i32,
    recusa_tudo: Cell<bool>,
    recusa_mescla: Cell<bool>,
    ignora_bandeja: Cell<bool>,
    padrao_com_opcoes: Cell<bool>,
    entradas_da_janela: RefCell<Vec<Vec<u8>>>,
    donos: RefCell<Vec<isize>>,
    janelas: Cell<u32>,
}

impl Falso {
    fn novo() -> Self {
        Self {
            resposta_da_janela: IDCANCEL,
            recusa_tudo: Cell::new(false),
            recusa_mescla: Cell::new(false),
            ignora_bandeja: Cell::new(false),
            padrao_com_opcoes: Cell::new(false),
            entradas_da_janela: RefCell::new(Vec::new()),
            donos: RefCell::new(Vec::new()),
            janelas: Cell::new(0),
        }
    }
}

impl DriverDaFila for Falso {
    fn tamanho(&self) -> i32 {
        total() as i32
    }
    fn ler(&self, destino: &mut BufferDevmode) -> i32 {
        if self.recusa_tudo.get() {
            return -1;
        }
        destino.preencher(&padrao().bytes().unwrap());
        if self.padrao_com_opcoes.get() {
            // Garantia: o buffer tem o tamanho de `DEVMODEW` e os campos são inteiros.
            unsafe {
                let modo = &mut *destino.ponteiro_mut();
                modo.dmFields |= DM_DEFAULTSOURCE
                    | DM_PRINTQUALITY
                    | DM_YRESOLUTION
                    | DM_COLOR
                    | DM_DUPLEX
                    | DM_COLLATE
                    | DM_PAPERSIZE
                    | DM_COPIES
                    | DM_SCALE
                    | DM_ORIENTATION
                    | DM_NUP;
                modo.Anonymous1.Anonymous1.dmDefaultSource = 3;
                modo.Anonymous1.Anonymous1.dmPrintQuality = 300;
                modo.dmYResolution = 300;
                modo.dmColor = DMCOLOR_MONOCHROME;
            }
        }
        IDOK
    }
    fn mesclar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode) -> i32 {
        if self.recusa_mescla.get() || qualidade(entrada) == QUALIDADE_RECUSADA {
            return -1;
        }
        destino.preencher(&entrada.bytes().unwrap());
        if self.ignora_bandeja.get() {
            // Garantia: o buffer tem o tamanho de `DEVMODEW` e o campo é um inteiro.
            unsafe {
                let modo = &mut *destino.ponteiro_mut();
                modo.dmFields = DEVMODE_FIELD_FLAGS(modo.dmFields.0 & !DM_DEFAULTSOURCE.0);
            }
        }
        IDOK
    }
    fn perguntar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode, dono: isize) -> i32 {
        self.janelas.set(self.janelas.get() + 1);
        self.entradas_da_janela
            .borrow_mut()
            .push(entrada.bytes().unwrap());
        self.donos.borrow_mut().push(dono);
        destino.preencher(&entrada.bytes().unwrap());
        // Garantia: como em `padrao`.
        unsafe {
            (*destino.ponteiro_mut())
                .Anonymous1
                .Anonymous1
                .dmPrintQuality = QUALIDADE_DA_PESSOA
        };
        self.resposta_da_janela
    }
}

fn papel_a4() -> AjustePapel {
    AjustePapel::Formato(A4)
}

#[test]
fn cancelar_a_janela_nao_devolve_nada_para_gravar() {
    let driver = Falso::novo();
    assert_eq!(
        perguntar(&driver, None, 77),
        Ok(RespostaDaJanela::Cancelada)
    );
    assert_eq!(driver.janelas.get(), 1);
    assert_eq!(*driver.donos.borrow(), [77]);
}

#[test]
fn aceitar_a_janela_devolve_os_bytes_que_o_driver_preencheu() {
    let driver = Falso {
        resposta_da_janela: IDOK,
        ..Falso::novo()
    };
    let RespostaDaJanela::Aceita(bytes) = perguntar(&driver, None, 0).unwrap() else {
        panic!("a janela foi aceita");
    };
    assert_eq!(bytes.len(), total());
    assert_eq!(
        qualidade(&BufferDevmode::de_bytes(&bytes).unwrap()),
        QUALIDADE_DA_PESSOA
    );
}

#[test]
fn a_janela_abre_a_partir_do_ajuste_salvo() {
    let driver = Falso::novo();
    let salvo = salvo_de_duas_faces();
    perguntar(&driver, Some(&salvo), 0).unwrap();
    let entradas = driver.entradas_da_janela.borrow();
    let recebido = BufferDevmode::de_bytes(&entradas[0]).unwrap();
    assert_eq!(qualidade(&recebido), QUALIDADE_DA_PESSOA);
}

#[test]
fn ajuste_que_nao_serve_mais_abre_a_janela_no_padrao_da_fila() {
    let driver = Falso::novo();
    perguntar(&driver, Some(&[1, 2, 3]), 0).unwrap();
    let entradas = driver.entradas_da_janela.borrow();
    assert_eq!(entradas[0], padrao().bytes().unwrap());
}

#[test]
fn resposta_de_erro_da_janela_vira_erro_e_nao_grava() {
    let driver = Falso {
        resposta_da_janela: -1,
        ..Falso::novo()
    };
    assert!(matches!(
        perguntar(&driver, None, 0),
        Err(ErroEnvio::Sistema { .. })
    ));
}

#[test]
fn ajuste_salvo_chega_ao_envio() {
    let driver = Falso::novo();
    let salvo = salvo_de_duas_faces();
    let montado = montar(&driver, papel_a4(), FacesDoEnvio::Uma, Some(&salvo)).unwrap();
    assert_eq!(qualidade(&montado), QUALIDADE_DA_PESSOA);
}

#[test]
fn sem_ajuste_o_envio_parte_do_padrao_da_fila() {
    let driver = Falso::novo();
    let montado = montar(&driver, papel_a4(), FacesDoEnvio::Uma, None).unwrap();
    assert_eq!(qualidade(&montado), 0);
}

#[test]
fn ajuste_de_duas_faces_copias_paginas_por_folha_e_outro_papel_nao_vence_o_envio() {
    let driver = Falso::novo();
    let salvo = salvo_de_duas_faces();
    let montado = montar(&driver, papel_a4(), FacesDoEnvio::Uma, Some(&salvo)).unwrap();
    // Garantia: o buffer tem o tamanho de `DEVMODEW`, e os campos lidos são inteiros.
    let (duplex, publico, nup) = unsafe {
        let modo = &*montado.ponteiro();
        (
            modo.dmDuplex,
            modo.Anonymous1.Anonymous1,
            modo.Anonymous2.dmNup,
        )
    };
    assert_eq!(duplex, DMDUP_SIMPLEX);
    assert_eq!(publico.dmCopies, 1);
    assert_eq!(publico.dmScale, 100);
    assert_eq!(publico.dmOrientation, 1);
    assert_eq!(publico.dmPaperSize, A4);
    assert_eq!(nup, UMA_POR_FOLHA);
}

#[test]
fn driver_que_mantem_as_duas_faces_do_ajuste_recua_ao_padrao_com_aviso() {
    /// Mantém as duas faces do ajuste da pessoa, como um driver que as guarda nos dados privados.
    struct PrendeSoOAjuste(Falso);
    impl DriverDaFila for PrendeSoOAjuste {
        fn tamanho(&self) -> i32 {
            self.0.tamanho()
        }
        fn ler(&self, destino: &mut BufferDevmode) -> i32 {
            self.0.ler(destino)
        }
        fn mesclar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode) -> i32 {
            let resposta = self.0.mesclar(destino, entrada);
            if qualidade(entrada) == QUALIDADE_DA_PESSOA {
                // Garantia: como em `padrao`.
                unsafe { (*destino.ponteiro_mut()).dmDuplex = DMDUP_VERTICAL };
            }
            resposta
        }
        fn perguntar(&self, d: &mut BufferDevmode, e: &BufferDevmode, dono: isize) -> i32 {
            self.0.perguntar(d, e, dono)
        }
    }
    let driver = PrendeSoOAjuste(Falso::novo());
    let mut salvo = Some(salvo_de_duas_faces());
    let original = salvo.clone();
    let (montado, recusado) = com_recuo_ao_padrao(&mut salvo, |bytes| {
        montar(&driver, papel_a4(), FacesDoEnvio::Uma, bytes)
    })
    .unwrap();
    assert_eq!(recusado, original);
    assert_eq!(salvo, None);
    assert_eq!(qualidade(&montado), 0);
    assert_eq!(montado.conferir_simplex(), Ok(()));
}

#[test]
fn driver_que_recusa_o_ajuste_cai_no_padrao_e_entrega_os_bytes_recusados() {
    let driver = Falso::novo();
    let mut buffer = padrao();
    // Garantia: como em `padrao`.
    unsafe {
        (*buffer.ponteiro_mut())
            .Anonymous1
            .Anonymous1
            .dmPrintQuality = QUALIDADE_RECUSADA
    };
    let bytes = buffer.bytes().unwrap();
    let mut salvo = Some(bytes.clone());
    let (montado, recusado) = com_recuo_ao_padrao(&mut salvo, |ajuste| {
        montar(&driver, papel_a4(), FacesDoEnvio::Uma, ajuste)
    })
    .unwrap();
    assert_eq!(recusado, Some(bytes));
    assert_eq!(qualidade(&montado), 0);
    assert_eq!(salvo, None, "as próximas passadas não tentam de novo");
}

#[test]
fn se_o_padrao_tambem_falha_o_ajuste_fica_guardado_e_o_erro_aparece() {
    let driver = Falso::novo();
    driver.recusa_tudo.set(true);
    let original = salvo_de_duas_faces();
    let mut salvo = Some(original.clone());
    let resultado = com_recuo_ao_padrao(&mut salvo, |ajuste| {
        montar(&driver, papel_a4(), FacesDoEnvio::Uma, ajuste)
    });
    assert!(resultado.is_err());
    assert_eq!(salvo, Some(original));
}

#[test]
fn sem_ajuste_um_erro_nao_conta_como_recusa() {
    let mut salvo = None;
    let resultado: Result<((), _), _> =
        com_recuo_ao_padrao(&mut salvo, |_| Err(ErroEnvio::SemTrabalho));
    assert_eq!(resultado, Err(ErroEnvio::SemTrabalho));
    let (valor, recusado) = com_recuo_ao_padrao(&mut salvo, |ajuste| {
        assert_eq!(ajuste, None);
        Ok(5)
    })
    .unwrap();
    assert_eq!((valor, recusado), (5, None));
}

#[test]
fn o_ajuste_vale_em_cada_passada_ate_o_driver_recusar() {
    let mut salvo = Some(vec![1u8]);
    for _ in 0..2 {
        let (_, recusado) = com_recuo_ao_padrao(&mut salvo, |ajuste| {
            assert_eq!(ajuste, Some(&[1u8][..]));
            Ok(())
        })
        .unwrap();
        assert_eq!(recusado, None);
    }
    assert!(salvo.is_some());
}

/// Guarda o que chegou ao driver na fusão, para conferir o saneamento dos bytes salvos.
struct Espia {
    campos_do_padrao: DEVMODE_FIELD_FLAGS,
    formulario_do_padrao: [u16; 32],
    entradas: RefCell<Vec<(DEVMODE_FIELD_FLAGS, [u16; 32])>>,
}

impl Espia {
    fn novo() -> Self {
        let mut formulario = [0u16; 32];
        formulario[0] = u16::from(b'A');
        formulario[1] = u16::from(b'4');
        Self {
            campos_do_padrao: DM_DUPLEX | DM_COPIES | DM_PAPERSIZE,
            formulario_do_padrao: formulario,
            entradas: RefCell::new(Vec::new()),
        }
    }
}

impl DriverDaFila for Espia {
    fn tamanho(&self) -> i32 {
        total() as i32
    }
    fn ler(&self, destino: &mut BufferDevmode) -> i32 {
        destino.preencher(&padrao().bytes().unwrap());
        // Garantia: como em `padrao`.
        unsafe {
            let modo = &mut *destino.ponteiro_mut();
            modo.dmFields = self.campos_do_padrao;
            modo.dmFormName = self.formulario_do_padrao;
        }
        IDOK
    }
    fn mesclar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode) -> i32 {
        // Garantia: como em `padrao`.
        let modo = unsafe { &*entrada.ponteiro() };
        self.entradas
            .borrow_mut()
            .push((modo.dmFields, modo.dmFormName));
        destino.preencher(&entrada.bytes().unwrap());
        IDOK
    }
    fn perguntar(&self, _: &mut BufferDevmode, _: &BufferDevmode, _: isize) -> i32 {
        IDCANCEL
    }
}

/// Ajuste com um nome de formulário de 32 `u16` sem zero final e campos que o driver nunca informou.
fn salvo_com_formulario_sem_fim() -> Vec<u8> {
    let mut buffer = padrao();
    // Garantia: como em `padrao`.
    unsafe {
        let modo = &mut *buffer.ponteiro_mut();
        modo.dmFormName = [u16::from(b'X'); 32];
        modo.dmFields = DEVMODE_FIELD_FLAGS(u32::MAX);
    }
    buffer.bytes().unwrap()
}

#[test]
fn nome_de_formulario_sem_zero_final_nao_chega_ao_driver() {
    let driver = Espia::novo();
    montar(
        &driver,
        papel_a4(),
        FacesDoEnvio::Uma,
        Some(&salvo_com_formulario_sem_fim()),
    )
    .unwrap();
    let entradas = driver.entradas.borrow();
    let (_, formulario) = entradas.first().unwrap();
    assert_eq!(*formulario, driver.formulario_do_padrao);
}

#[test]
fn campos_que_o_padrao_nao_informou_nao_chegam_ao_driver() {
    let driver = Espia::novo();
    montar(
        &driver,
        papel_a4(),
        FacesDoEnvio::Uma,
        Some(&salvo_com_formulario_sem_fim()),
    )
    .unwrap();
    let entradas = driver.entradas.borrow();
    let (campos, _) = entradas.first().unwrap();
    assert_eq!(*campos, driver.campos_do_padrao);
}

#[test]
fn falha_de_sistema_ao_criar_o_contexto_nao_descarta_o_ajuste() {
    let original = vec![1u8, 2, 3];
    let mut salvo = Some(original.clone());
    let mut tentativas = Vec::new();
    let resultado: Result<((), _), _> = com_recuo_ao_padrao(&mut salvo, |ajuste| {
        tentativas.push(ajuste.map(<[u8]>::to_vec));
        if ajuste.is_some() {
            Err(ErroEnvio::Sistema {
                chamada: "CreateDCW",
                codigo: 5,
            })
        } else {
            Ok(())
        }
    });
    assert!(
        resultado.is_err(),
        "a falha aparece, sem envio com outro ajuste"
    );
    assert_eq!(salvo, Some(original.clone()), "o ajuste continua guardado");
    assert_eq!(tentativas, [Some(original)], "não tenta com o padrão");
}

#[test]
fn erro_do_driver_recua_ao_padrao_e_descarta_o_ajuste() {
    let casos = [
        ErroEnvio::Resposta("o driver recusou"),
        ErroEnvio::SimplexNaoAceito,
        ErroEnvio::PapelNaoAceito {
            pedido_mm: (210.0, 297.0),
            obtido_mm: (216.0, 279.0),
        },
        ErroEnvio::ValorNaoAceito {
            campo: "cópias",
            pedido: 1,
            obtido: 3,
        },
    ];
    for erro in casos {
        let mut salvo = Some(vec![1u8]);
        let (_, recusado) = com_recuo_ao_padrao(&mut salvo, |ajuste| {
            if ajuste.is_some() {
                Err(erro.clone())
            } else {
                Ok(())
            }
        })
        .unwrap();
        assert_eq!(recusado, Some(vec![1u8]), "{erro:?}");
    }
}

/// Limite conhecido: o que liga duas faces nos dados privados de um driver não é legível. Com o
/// campo público em uma face, este ajuste passa pelas conferências e vai ao envio como está.
#[test]
fn duas_faces_escondidas_nos_dados_privados_passam_como_limite_conhecido() {
    const LIGA_DUAS_FACES: u8 = 0xD2;
    let mut bytes = padrao().bytes().unwrap();
    let privados = std::mem::size_of::<DEVMODEW>();
    for byte in bytes.iter_mut().skip(privados) {
        *byte = LIGA_DUAS_FACES;
    }
    let driver = Falso::novo();
    let montado = montar(&driver, papel_a4(), FacesDoEnvio::Uma, Some(&bytes)).unwrap();
    assert_eq!(montado.conferir_simplex(), Ok(()));
    let saida = montado.bytes().unwrap();
    assert!(saida
        .iter()
        .skip(privados)
        .all(|byte| *byte == LIGA_DUAS_FACES));
}

#[test]
fn ajuste_do_driver_com_dmsize_curto_nao_tem_o_lugar_do_formulario_sobrescrito() {
    struct Curto;
    impl DriverDaFila for Curto {
        fn tamanho(&self) -> i32 {
            total() as i32
        }
        fn ler(&self, destino: &mut BufferDevmode) -> i32 {
            let mut bytes = padrao().bytes().unwrap();
            let curto = std::mem::offset_of!(DEVMODEW, dmFormName) as u16;
            let extra = bytes.len() as u16 - curto;
            let tamanho = std::mem::offset_of!(DEVMODEW, dmSize);
            bytes[tamanho..tamanho + 2].copy_from_slice(&curto.to_ne_bytes());
            let privado = std::mem::offset_of!(DEVMODEW, dmDriverExtra);
            bytes[privado..privado + 2].copy_from_slice(&extra.to_ne_bytes());
            destino.preencher(&bytes);
            IDOK
        }
        fn mesclar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode) -> i32 {
            destino.preencher(&entrada.bytes().unwrap());
            IDOK
        }
        fn perguntar(&self, _: &mut BufferDevmode, _: &BufferDevmode, _: isize) -> i32 {
            IDCANCEL
        }
    }
    let mut padrao_curto = BufferDevmode::com_tamanho(total() as i32).unwrap();
    Curto.ler(&mut padrao_curto);
    let mut salvo = padrao_curto.bytes().unwrap();
    let formulario = std::mem::offset_of!(DEVMODEW, dmFormName);
    salvo[formulario] = 0xAB;
    let validado = validar_salvo(&Curto, &padrao_curto, &salvo).unwrap();
    assert_eq!(validado.bytes().unwrap()[formulario], 0xAB);
}

#[test]
fn campo_publico_de_duas_faces_e_reconhecido() {
    assert!(ajuste_pede_duas_faces(&salvo_de_duas_faces()));
    assert!(!ajuste_pede_duas_faces(&padrao().bytes().unwrap()));
    assert!(!ajuste_pede_duas_faces(&[0xFF; 300]));
    assert!(!ajuste_pede_duas_faces(&[]));
}

/// Liga um `u16` no meio dos bytes, como faria um arquivo adulterado.
fn com_campo(mut bytes: Vec<u8>, deslocamento: usize, valor: u16) -> Vec<u8> {
    bytes[deslocamento..deslocamento + 2].copy_from_slice(&valor.to_ne_bytes());
    bytes
}

#[test]
fn bytes_curtos_ou_com_tamanhos_absurdos_nao_viram_devmode() {
    let bom = salvo_de_duas_faces();
    let tamanho = std::mem::offset_of!(DEVMODEW, dmSize);
    let extra = std::mem::offset_of!(DEVMODEW, dmDriverExtra);
    assert!(BufferDevmode::de_bytes(&bom).is_some());
    let casos: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![0; 3],
        vec![0; std::mem::size_of::<DEVMODEW>() - 1],
        bom[..bom.len() - 1].to_vec(),
        [bom.clone(), vec![0]].concat(),
        com_campo(bom.clone(), tamanho, u16::MAX),
        com_campo(bom.clone(), tamanho, 0),
        com_campo(bom.clone(), tamanho, 1),
        com_campo(bom.clone(), extra, u16::MAX),
        com_campo(bom.clone(), extra, 0),
        vec![0xFF; total()],
        vec![0; total()],
    ];
    for caso in casos {
        assert!(BufferDevmode::de_bytes(&caso).is_none(), "{}", caso.len());
    }
}

#[test]
fn bytes_hostis_nao_chegam_ao_driver_e_o_envio_usa_o_padrao() {
    let driver = Falso::novo();
    let mut salvo = Some(vec![0xFF; 5000]);
    let (montado, recusado) = com_recuo_ao_padrao(&mut salvo, |ajuste| {
        montar(&driver, papel_a4(), FacesDoEnvio::Uma, ajuste)
    })
    .unwrap();
    assert!(recusado.is_some());
    assert_eq!(qualidade(&montado), 0);
}

#[test]
fn ajuste_de_outra_versao_do_driver_e_recusado() {
    let driver = Falso::novo();
    let versao = std::mem::offset_of!(DEVMODEW, dmDriverVersion);
    let outro = com_campo(salvo_de_duas_faces(), versao, VERSAO_DO_DRIVER + 1);
    assert!(montar(&driver, papel_a4(), FacesDoEnvio::Uma, Some(&outro)).is_err());
}

#[test]
fn nome_da_fila_no_ajuste_nao_vale() {
    struct Olhador(Falso, RefCell<Vec<u16>>);
    impl DriverDaFila for Olhador {
        fn tamanho(&self) -> i32 {
            self.0.tamanho()
        }
        fn ler(&self, destino: &mut BufferDevmode) -> i32 {
            self.0.ler(destino)
        }
        fn mesclar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode) -> i32 {
            // Garantia: o buffer tem o tamanho de `DEVMODEW`, e o campo lido é um vetor de inteiros.
            let nome = unsafe { (*entrada.ponteiro()).dmDeviceName };
            self.1.borrow_mut().extend_from_slice(&nome[..2]);
            self.0.mesclar(destino, entrada)
        }
        fn perguntar(&self, d: &mut BufferDevmode, e: &BufferDevmode, dono: isize) -> i32 {
            self.0.perguntar(d, e, dono)
        }
    }
    let driver = Olhador(Falso::novo(), RefCell::new(Vec::new()));
    let nome = std::mem::offset_of!(DEVMODEW, dmDeviceName);
    let adulterado = com_campo(salvo_de_duas_faces(), nome, u16::from(b'X'));
    montar(&driver, papel_a4(), FacesDoEnvio::Uma, Some(&adulterado)).unwrap();
    assert_eq!(driver.1.borrow()[0], u16::from(b'F'));
}

#[test]
fn driver_que_informa_tamanho_fora_do_buffer_vira_erro_ao_devolver_os_bytes() {
    let mut buffer = padrao();
    // Garantia: como em `padrao`.
    unsafe { (*buffer.ponteiro_mut()).dmDriverExtra = u16::MAX };
    assert!(matches!(buffer.bytes(), Err(ErroEnvio::Resposta(_))));
}

#[test]
fn tamanho_absurdo_informado_pelo_driver_nao_reserva_memoria() {
    struct Absurdo(i32);
    impl DriverDaFila for Absurdo {
        fn tamanho(&self) -> i32 {
            self.0
        }
        fn ler(&self, _: &mut BufferDevmode) -> i32 {
            IDOK
        }
        fn mesclar(&self, _: &mut BufferDevmode, _: &BufferDevmode) -> i32 {
            IDOK
        }
        fn perguntar(&self, _: &mut BufferDevmode, _: &BufferDevmode, _: isize) -> i32 {
            IDOK
        }
    }
    for tamanho in [-1, 0, 10, i32::MAX] {
        assert!(montar(&Absurdo(tamanho), papel_a4(), FacesDoEnvio::Uma, None).is_err());
        assert!(perguntar(&Absurdo(tamanho), None, 0).is_err());
    }
}

const FILA_DE_PDF: &str = "Microsoft Print to PDF";

#[test]
#[ignore = "lê o driver da fila de PDF do Windows, sem janela e sem enviar"]
fn driver_real_aceita_o_proprio_ajuste_e_o_envio_continua_em_uma_face() {
    let bytes = ajuste_padrao_da_fila(FILA_DE_PDF).unwrap();
    assert!(BufferDevmode::de_bytes(&bytes).is_some());
    assert_eq!(driver_aceita_ajuste(FILA_DE_PDF, &bytes), Ok(()));

    let mut adulterado = BufferDevmode::de_bytes(&bytes).unwrap();
    // Garantia: como em `padrao`.
    unsafe {
        let modo = &mut *adulterado.ponteiro_mut();
        modo.dmFields |= DM_DUPLEX | DM_COPIES;
        modo.dmDuplex = DMDUP_VERTICAL;
        modo.Anonymous1.Anonymous1.dmCopies = 3;
    }
    let impressora = Impressora::abrir(FILA_DE_PDF).unwrap();
    let montado = impressora
        .devmode(
            papel_a4(),
            FacesDoEnvio::Uma,
            Some(&adulterado.bytes().unwrap()),
        )
        .unwrap();
    assert_eq!(montado.conferir_simplex(), Ok(()));
    assert_eq!(montado.conferir_aplicado(), Ok(()));
}

#[test]
#[ignore = "lê o driver da fila de PDF do Windows, sem janela e sem enviar"]
fn driver_real_nao_recebe_bytes_quebrados() {
    assert!(driver_aceita_ajuste(FILA_DE_PDF, &[0xFF; 300]).is_err());
}

#[test]
fn fila_que_sumiu_tem_erro_proprio() {
    assert_eq!(
        crate::preferencias_janela::abrir_janela_do_driver(
            "Fila que não existe neste computador 7f3a",
            0,
            None
        ),
        Err(ErroPreferencias::FilaInexistente)
    );
}

#[test]
fn nome_com_zero_no_meio_nao_chega_ao_sistema() {
    assert!(matches!(
        ajuste_padrao_da_fila("a\0b"),
        Err(ErroPreferencias::Envio(ErroEnvio::TextoInvalido))
    ));
}

/// Lê o que o `DEVMODE` montado diz sobre as faces.
fn faces_montadas(montado: &BufferDevmode) -> (DEVMODE_DUPLEX, bool) {
    // Garantia: o buffer tem o tamanho de `DEVMODEW`, e os campos lidos são inteiros.
    let modo = unsafe { &*montado.ponteiro() };
    (modo.dmDuplex, modo.dmFields.0 & DM_DUPLEX.0 != 0)
}

/// Driver que não fica com as duas faces, de cada jeito que um driver real pode recusar.
struct RecusaDuasFaces {
    driver: Falso,
    como: Recusa,
}

#[derive(Clone, Copy)]
enum Recusa {
    VoltaParaUmaFace,
    TiraOCampo,
    DiaRespostaDeErro,
}

impl DriverDaFila for RecusaDuasFaces {
    fn tamanho(&self) -> i32 {
        self.driver.tamanho()
    }
    fn ler(&self, destino: &mut BufferDevmode) -> i32 {
        self.driver.ler(destino)
    }
    fn mesclar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode) -> i32 {
        let resposta = self.driver.mesclar(destino, entrada);
        // Garantia: como em `padrao`.
        unsafe {
            let modo = &mut *destino.ponteiro_mut();
            match self.como {
                Recusa::VoltaParaUmaFace => modo.dmDuplex = DMDUP_SIMPLEX,
                Recusa::TiraOCampo => {
                    modo.dmFields = DEVMODE_FIELD_FLAGS(modo.dmFields.0 & !DM_DUPLEX.0);
                }
                Recusa::DiaRespostaDeErro => return -1,
            }
        }
        resposta
    }
    fn perguntar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode, dono: isize) -> i32 {
        self.driver.perguntar(destino, entrada, dono)
    }
}

#[test]
fn duas_faces_pedem_a_virada_pela_borda_longa_e_mantem_o_resto_do_envio() {
    let driver = Falso::novo();
    let montado = montar(&driver, papel_a4(), FacesDoEnvio::Duas, None).unwrap();
    assert_eq!(faces_montadas(&montado), (DMDUP_VERTICAL, true));
    // Garantia: como em `faces_montadas`.
    let publico = unsafe { (*montado.ponteiro()).Anonymous1.Anonymous1 };
    assert_eq!(publico.dmCopies, 1);
    assert_eq!(publico.dmScale, 100);
    assert_eq!(publico.dmOrientation, 1);
    assert_eq!(publico.dmPaperSize, A4);
}

#[test]
fn ajuste_salvo_continua_de_base_e_as_duas_faces_valem_sobre_ele() {
    let driver = Falso::novo();
    let salvo = salvo_de_duas_faces();
    let montado = montar(&driver, papel_a4(), FacesDoEnvio::Duas, Some(&salvo)).unwrap();
    assert_eq!(qualidade(&montado), QUALIDADE_DA_PESSOA);
    assert_eq!(faces_montadas(&montado), (DMDUP_VERTICAL, true));
}

#[test]
fn driver_que_nao_fica_com_as_duas_faces_e_recusado() {
    for como in [
        Recusa::VoltaParaUmaFace,
        Recusa::TiraOCampo,
        Recusa::DiaRespostaDeErro,
    ] {
        let driver = RecusaDuasFaces {
            driver: Falso::novo(),
            como,
        };
        assert_eq!(
            montar(&driver, papel_a4(), FacesDoEnvio::Duas, None).err(),
            Some(ErroEnvio::DuasFacesNaoAceitas)
        );
    }
}

#[test]
fn recusa_das_duas_faces_com_ajuste_tenta_o_padrao_e_nao_passa_em_silencio() {
    let driver = RecusaDuasFaces {
        driver: Falso::novo(),
        como: Recusa::VoltaParaUmaFace,
    };
    let mut salvo = Some(salvo_de_duas_faces());
    let resultado = com_recuo_ao_padrao(&mut salvo, |ajuste| {
        montar(&driver, papel_a4(), FacesDoEnvio::Duas, ajuste)
    });
    assert_eq!(resultado.err(), Some(ErroEnvio::DuasFacesNaoAceitas));
    assert!(salvo.is_some(), "o problema não era o ajuste: ele fica");
}

#[test]
fn fluxo_manual_continua_em_uma_face_mesmo_com_ajuste_de_duas_faces() {
    let driver = Falso::novo();
    let salvo = salvo_de_duas_faces();
    let montado = montar(&driver, papel_a4(), FacesDoEnvio::Uma, Some(&salvo)).unwrap();
    assert_eq!(faces_montadas(&montado).0, DMDUP_SIMPLEX);
}

const CAMPOS_DO_ENVIO: DEVMODE_FIELD_FLAGS = DEVMODE_FIELD_FLAGS(
    DM_DUPLEX.0 | DM_COPIES.0 | DM_SCALE.0 | DM_ORIENTATION.0 | DM_NUP.0 | DM_PAPERSIZE.0,
);

fn buffer_das_escolhas(bytes: &[u8]) -> BufferDevmode {
    BufferDevmode::de_bytes(bytes).expect("os bytes formam um DEVMODE")
}

#[test]
fn aplicar_escolhas_marca_bandeja_resolucao_e_cor() {
    let driver = Falso::novo();
    let escolhas = Escolhas {
        bandeja: Some(7),
        resolucao: Some((600, 300)),
        cor: Some(Cor::Monocromatica),
    };
    let bytes = aplicar_em(&driver, None, &escolhas).unwrap();
    let buffer = buffer_das_escolhas(&bytes);
    // Garantia: o buffer tem o tamanho de `DEVMODEW` e os campos lidos são inteiros.
    let modo = unsafe { &*buffer.ponteiro() };
    // Garantia: os campos da união são inteiros, válidos em qualquer padrão de bits.
    let (bandeja, qualidade, cor) = unsafe {
        (
            modo.Anonymous1.Anonymous1.dmDefaultSource,
            modo.Anonymous1.Anonymous1.dmPrintQuality,
            modo.dmColor,
        )
    };
    assert_ne!(modo.dmFields.0 & DM_DEFAULTSOURCE.0, 0);
    assert_eq!(bandeja, 7);
    assert_ne!(modo.dmFields.0 & DM_PRINTQUALITY.0, 0);
    assert_ne!(modo.dmFields.0 & DM_YRESOLUTION.0, 0);
    assert_eq!(qualidade, 600);
    assert_eq!(modo.dmYResolution, 300);
    assert_ne!(modo.dmFields.0 & DM_COLOR.0, 0);
    assert_eq!(cor, DMCOLOR_MONOCHROME);
}

#[test]
fn aplicar_escolhas_nao_marca_os_campos_que_o_envio_decide() {
    let driver = Falso::novo();
    let escolhas = Escolhas {
        bandeja: Some(3),
        ..Escolhas::default()
    };
    let bytes = aplicar_em(&driver, None, &escolhas).unwrap();
    let buffer = buffer_das_escolhas(&bytes);
    // Garantia: como em `aplicar_escolhas_marca_bandeja_resolucao_e_cor`.
    let modo = unsafe { &*buffer.ponteiro() };
    // Garantia: os campos da união são inteiros, válidos em qualquer padrão de bits.
    let (copias, escala) = unsafe {
        (
            modo.Anonymous1.Anonymous1.dmCopies,
            modo.Anonymous1.Anonymous1.dmScale,
        )
    };
    assert_eq!(modo.dmFields.0 & CAMPOS_DO_ENVIO.0, 0);
    assert_eq!(copias, 1);
    assert_eq!(escala, 100);
}

#[test]
fn campo_nao_escolhido_fica_como_estava_no_ajuste() {
    let driver = Falso::novo();
    let escolhas = Escolhas {
        resolucao: Some((1200, 1200)),
        ..Escolhas::default()
    };
    let bytes = aplicar_em(&driver, None, &escolhas).unwrap();
    let buffer = buffer_das_escolhas(&bytes);
    // Garantia: como em `aplicar_escolhas_marca_bandeja_resolucao_e_cor`.
    let modo = unsafe { &*buffer.ponteiro() };
    assert_eq!(modo.dmFields.0 & DM_DEFAULTSOURCE.0, 0);
    assert_eq!(modo.dmFields.0 & DM_COLOR.0, 0);
}

#[test]
fn campo_que_o_ajuste_ja_marcava_mantem_o_valor_quando_nao_foi_escolhido() {
    let mut salvo = salvo_de_duas_faces_buffer();
    salvo
        .aplicar_escolhas(&Escolhas {
            bandeja: Some(2),
            ..Escolhas::default()
        })
        .unwrap();
    // Garantia: como em `aplicar_escolhas_marca_bandeja_resolucao_e_cor`.
    let modo = unsafe { &*salvo.ponteiro() };
    // Garantia: os campos da união são inteiros, válidos em qualquer padrão de bits.
    let (copias, escala, papel) = unsafe {
        (
            modo.Anonymous1.Anonymous1.dmCopies,
            modo.Anonymous1.Anonymous1.dmScale,
            modo.Anonymous1.Anonymous1.dmPaperSize,
        )
    };
    assert_eq!(modo.dmFields.0 & CAMPOS_DO_ENVIO.0, CAMPOS_DO_ENVIO.0);
    assert_eq!(copias, 3);
    assert_eq!(escala, 50);
    assert_eq!(papel, CARTA);
    assert_eq!(modo.dmDuplex, DMDUP_VERTICAL);
}

fn salvo_de_duas_faces_buffer() -> BufferDevmode {
    BufferDevmode::de_bytes(&salvo_de_duas_faces()).unwrap()
}

/// Ajuste salvo com os três campos do painel em valores distintos do padrão do driver.
fn salvo_com_opcoes() -> Vec<u8> {
    let mut buffer = salvo_de_duas_faces_buffer();
    // Garantia: como em `padrao`.
    unsafe {
        let modo = &mut *buffer.ponteiro_mut();
        modo.dmFields |= DM_DEFAULTSOURCE | DM_PRINTQUALITY | DM_YRESOLUTION | DM_COLOR;
        modo.Anonymous1.Anonymous1.dmDefaultSource = 7;
        modo.Anonymous1.Anonymous1.dmPrintQuality = 1200;
        modo.dmYResolution = 1200;
        modo.dmColor = DMCOLOR_COLOR;
    }
    buffer.bytes().unwrap()
}

#[test]
fn padrao_da_impressora_repoe_os_tres_campos_e_preserva_os_do_envio() {
    let driver = Falso::novo();
    driver.padrao_com_opcoes.set(true);
    let salvo = salvo_com_opcoes();
    let bytes = aplicar_em(&driver, Some(&salvo), &Escolhas::default()).unwrap();
    let buffer = buffer_das_escolhas(&bytes);
    // Garantia: como em `aplicar_escolhas_marca_bandeja_resolucao_e_cor`.
    let modo = unsafe { &*buffer.ponteiro() };
    let (bandeja, qualidade, y, cor, copias, escala, papel, orientacao, nup) = unsafe {
        (
            modo.Anonymous1.Anonymous1.dmDefaultSource,
            modo.Anonymous1.Anonymous1.dmPrintQuality,
            modo.dmYResolution,
            modo.dmColor,
            modo.Anonymous1.Anonymous1.dmCopies,
            modo.Anonymous1.Anonymous1.dmScale,
            modo.Anonymous1.Anonymous1.dmPaperSize,
            modo.Anonymous1.Anonymous1.dmOrientation,
            modo.Anonymous2.dmNup,
        )
    };
    assert_ne!(modo.dmFields.0 & DM_DEFAULTSOURCE.0, 0);
    assert_eq!(bandeja, 3, "a bandeja volta ao padrão do driver");
    assert_ne!(modo.dmFields.0 & DM_PRINTQUALITY.0, 0);
    assert_ne!(modo.dmFields.0 & DM_YRESOLUTION.0, 0);
    assert_eq!(qualidade, 300, "a resolução volta ao padrão do driver");
    assert_eq!(y, 300);
    assert_ne!(modo.dmFields.0 & DM_COLOR.0, 0);
    assert_eq!(cor, DMCOLOR_MONOCHROME, "a cor volta ao padrão do driver");
    // O que não é opção do painel continua vindo do ajuste salvo.
    assert_eq!(modo.dmFields.0 & CAMPOS_DO_ENVIO.0, CAMPOS_DO_ENVIO.0);
    assert_eq!(modo.dmDuplex, DMDUP_VERTICAL);
    assert_eq!(copias, 3);
    assert_eq!(escala, 50);
    assert_eq!(orientacao, 2);
    assert_eq!(papel, CARTA);
    assert_eq!(nup, 4);
}

#[test]
fn escolhas_atuais_so_lem_os_campos_marcados_pelo_driver() {
    let padrao = padrao();
    assert_eq!(padrao.escolhas_atuais(), Escolhas::default());
    let mut marcado = padrao;
    // Garantia: como em `padrao`.
    unsafe {
        let modo = &mut *marcado.ponteiro_mut();
        modo.dmFields |= DM_DEFAULTSOURCE | DM_PRINTQUALITY | DM_YRESOLUTION | DM_COLOR;
        modo.Anonymous1.Anonymous1.dmDefaultSource = 4;
        modo.Anonymous1.Anonymous1.dmPrintQuality = 600;
        modo.dmYResolution = 600;
        modo.dmColor = DMCOLOR_COLOR;
    };
    assert_eq!(
        marcado.escolhas_atuais(),
        Escolhas {
            bandeja: Some(4),
            resolucao: Some((600, 600)),
            cor: Some(Cor::Colorida),
        }
    );
}

#[test]
fn driver_que_recusa_as_opcoes_nao_devolve_bytes() {
    let driver = Falso::novo();
    driver.recusa_mescla.set(true);
    let escolhas = Escolhas {
        cor: Some(Cor::Colorida),
        ..Escolhas::default()
    };
    assert_eq!(
        aplicar_em(&driver, None, &escolhas).err(),
        Some(ErroEnvio::Resposta("o driver não aceitou as opções"))
    );
}

#[test]
fn resolucao_que_nao_cabe_no_devmode_e_recusada_antes_do_driver() {
    let driver = Falso::novo();
    for (x, y) in [(40_000, 600), (600, 0), (-1, 600)] {
        let escolhas = Escolhas {
            resolucao: Some((x, y)),
            ..Escolhas::default()
        };
        assert!(aplicar_em(&driver, None, &escolhas).is_err(), "{x}x{y}");
    }
}

#[test]
fn driver_que_ignora_a_bandeja_escolhida_e_recusado() {
    let driver = Falso::novo();
    driver.ignora_bandeja.set(true);
    let escolhas = Escolhas {
        bandeja: Some(7),
        ..Escolhas::default()
    };
    assert_eq!(
        aplicar_em(&driver, None, &escolhas).err(),
        Some(ErroEnvio::Resposta(
            "o driver não manteve a bandeja escolhida"
        ))
    );
}

#[test]
fn escolhas_vazias_nao_marcam_nem_mudam_nada() {
    let driver = Falso::novo();
    let bytes = aplicar_em(&driver, None, &Escolhas::default()).unwrap();
    assert_eq!(bytes, padrao().bytes().unwrap());
}
