//! Trabalhos de uma fila, lidos do spooler. Os testes usam uma implementação em memória.

use std::fmt;

/// Um trabalho enquanto está na fila.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrabalhoNaFila {
    pub id: u32,
    pub documento: String,
    /// Bits `JOB_STATUS_*` sem tradução.
    pub status: u32,
    pub paginas_totais: u32,
    pub paginas_impressas: u32,
}

/// Falhas ao ler a fila.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErroSpooler {
    TextoInvalido,
    Sistema { chamada: &'static str, codigo: u32 },
    Grande { bytes: u32 },
    Resposta(&'static str),
    NaoSuportado,
}

impl fmt::Display for ErroSpooler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TextoInvalido => f.write_str("o nome da fila tem um caractere inválido"),
            Self::Sistema { chamada, codigo } => {
                write!(f, "o Windows recusou {chamada} (código {codigo:#010X})")
            }
            Self::Grande { bytes } => {
                write!(
                    f,
                    "a lista de trabalhos pede {bytes} bytes, acima do limite"
                )
            }
            Self::Resposta(detalhe) => write!(f, "resposta inesperada do spooler: {detalhe}"),
            Self::NaoSuportado => f.write_str("o spooler só existe no Windows"),
        }
    }
}

impl std::error::Error for ErroSpooler {}

/// O que a recepção precisa saber do spooler.
pub trait Spooler {
    fn trabalhos(&self, fila: &str) -> Result<Vec<TrabalhoNaFila>, ErroSpooler>;
}

#[cfg(windows)]
pub use sistema::SpoolerDoWindows;

#[cfg(windows)]
mod sistema {
    use super::*;
    use std::mem::size_of;
    use windows::core::{Error, PCWSTR};
    use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
    use windows::Win32::Graphics::Printing::{
        ClosePrinter, EnumJobsW, OpenPrinterW, JOB_INFO_1W, PRINTER_HANDLE,
    };

    const NIVEL_INFO_1: u32 = 1;
    /// Mais trabalhos do que isso na fila de um teste é sinal de problema.
    const MAXIMO_TRABALHOS: u32 = 1_000;
    /// Maior texto lido de um nome de documento, em caracteres.
    const MAXIMO_TEXTO: usize = 1_024;
    /// O tamanho pode crescer entre o pedido e a leitura.
    const TENTATIVAS: u32 = 5;

    /// O spooler de verdade. Só lê: não altera a fila nem os trabalhos.
    pub struct SpoolerDoWindows;

    struct Aberta(PRINTER_HANDLE);

    impl Drop for Aberta {
        fn drop(&mut self) {
            // Garantia: o identificador veio de `OpenPrinterW` e é fechado uma vez só.
            let _ = unsafe { ClosePrinter(self.0) };
        }
    }

    fn abrir(fila: &str) -> Result<Aberta, ErroSpooler> {
        let mut nome: Vec<u16> = fila.encode_utf16().collect();
        if nome.contains(&0) {
            return Err(ErroSpooler::TextoInvalido);
        }
        nome.push(0);
        let mut handle = PRINTER_HANDLE::default();
        // Garantia: `nome` termina em zero e vive durante a chamada; `handle` é um destino local.
        unsafe { OpenPrinterW(PCWSTR::from_raw(nome.as_ptr()), &mut handle, None) }.map_err(
            |erro| ErroSpooler::Sistema {
                chamada: "OpenPrinterW",
                codigo: erro.code().0 as u32,
            },
        )?;
        Ok(Aberta(handle))
    }

    fn ler_buffer(aberta: &Aberta) -> Result<(Vec<u8>, u32), ErroSpooler> {
        let mut buffer: Vec<u8> = Vec::new();
        for _ in 0..TENTATIVAS {
            let (mut pedido, mut contagem) = (0u32, 0u32);
            let destino = if buffer.is_empty() {
                None
            } else {
                Some(buffer.as_mut_slice())
            };
            // Garantia: `destino`, quando há, é vivo e exclusivo, e o tamanho dele vai junto; os
            // outros dois destinos são locais. `aberta` mantém o identificador válido.
            let resultado = unsafe {
                EnumJobsW(
                    aberta.0,
                    0,
                    MAXIMO_TRABALHOS,
                    NIVEL_INFO_1,
                    destino,
                    &mut pedido,
                    &mut contagem,
                )
            };
            match resultado {
                Ok(()) => return Ok((buffer, contagem)),
                Err(erro) if erro.code() == Error::from(ERROR_INSUFFICIENT_BUFFER).code() => {
                    if pedido > plots_windows::LIMITE_BUFFER_BYTES {
                        return Err(ErroSpooler::Grande { bytes: pedido });
                    }
                    let tamanho = usize::try_from(pedido)
                        .map_err(|_| ErroSpooler::Grande { bytes: pedido })?;
                    buffer.clear();
                    buffer
                        .try_reserve_exact(tamanho)
                        .map_err(|_| ErroSpooler::Grande { bytes: pedido })?;
                    buffer.resize(tamanho, 0);
                }
                Err(erro) => {
                    return Err(ErroSpooler::Sistema {
                        chamada: "EnumJobsW",
                        codigo: erro.code().0 as u32,
                    })
                }
            }
        }
        Err(ErroSpooler::Resposta(
            "a lista de trabalhos não parou de crescer",
        ))
    }

    /// Texto apontado de dentro do buffer. Ponteiro fora do buffer vira texto vazio; a leitura
    /// para no fim do buffer mesmo sem o zero final.
    fn texto_no_buffer(buffer: &[u8], ponteiro: *const u16) -> String {
        let base = buffer.as_ptr() as usize;
        let Some(inicio) = (ponteiro as usize).checked_sub(base) else {
            return String::new();
        };
        let Some(resto) = buffer.get(inicio..) else {
            return String::new();
        };
        let unidades = resto
            .chunks_exact(2)
            .map(|par| {
                u16::from_le_bytes([
                    par.first().copied().unwrap_or(0),
                    par.get(1).copied().unwrap_or(0),
                ])
            })
            .take(MAXIMO_TEXTO)
            .take_while(|unidade| *unidade != 0);
        char::decode_utf16(unidades)
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    }

    /// Lê os registros do buffer sem confiar na contagem: ela precisa caber nele.
    pub(super) fn decodificar(
        buffer: &[u8],
        contagem: u32,
    ) -> Result<Vec<TrabalhoNaFila>, ErroSpooler> {
        let contagem = usize::try_from(contagem)
            .map_err(|_| ErroSpooler::Resposta("contagem de trabalhos inválida"))?;
        let necessario = contagem
            .checked_mul(size_of::<JOB_INFO_1W>())
            .filter(|bytes| *bytes <= buffer.len())
            .ok_or(ErroSpooler::Resposta(
                "contagem de trabalhos maior que o buffer",
            ))?;
        let registros = buffer
            .get(..necessario)
            .ok_or(ErroSpooler::Resposta("buffer curto"))?;
        let mut trabalhos = Vec::with_capacity(contagem);
        for registro in registros.chunks_exact(size_of::<JOB_INFO_1W>()) {
            // Garantia: `registro` tem o tamanho exato da estrutura, e a leitura não exige
            // alinhamento. Os ponteiros dela não são seguidos: `texto_no_buffer` confere.
            let info: JOB_INFO_1W =
                unsafe { std::ptr::read_unaligned(registro.as_ptr().cast::<JOB_INFO_1W>()) };
            trabalhos.push(TrabalhoNaFila {
                id: info.JobId,
                documento: texto_no_buffer(buffer, info.pDocument.0.cast_const()),
                status: info.Status,
                paginas_totais: info.TotalPages,
                paginas_impressas: info.PagesPrinted,
            });
        }
        Ok(trabalhos)
    }

    impl Spooler for SpoolerDoWindows {
        fn trabalhos(&self, fila: &str) -> Result<Vec<TrabalhoNaFila>, ErroSpooler> {
            let aberta = abrir(fila)?;
            let (buffer, contagem) = ler_buffer(&aberta)?;
            decodificar(&buffer, contagem)
        }
    }

    #[cfg(test)]
    mod testes {
        use super::*;
        use windows::core::PWSTR;

        fn registro(id: u32, documento: PWSTR) -> JOB_INFO_1W {
            JOB_INFO_1W {
                JobId: id,
                pDocument: documento,
                Status: 0x10,
                TotalPages: 3,
                PagesPrinted: 1,
                ..Default::default()
            }
        }

        fn bytes_de(registro: &JOB_INFO_1W) -> Vec<u8> {
            // Garantia: lê exatamente os bytes da estrutura viva.
            unsafe {
                std::slice::from_raw_parts(
                    std::ptr::from_ref(registro).cast::<u8>(),
                    size_of::<JOB_INFO_1W>(),
                )
            }
            .to_vec()
        }

        #[test]
        fn le_registro_com_texto_dentro_do_buffer() {
            let tamanho = size_of::<JOB_INFO_1W>();
            let mut buffer = vec![0u8; tamanho + 8];
            for (par, unidade) in buffer[tamanho..]
                .chunks_exact_mut(2)
                .zip("Doc\0".encode_utf16())
            {
                par.copy_from_slice(&unidade.to_le_bytes());
            }
            let ponteiro = PWSTR(buffer[tamanho..].as_mut_ptr().cast());
            let bytes = bytes_de(&registro(7, ponteiro));
            buffer[..tamanho].copy_from_slice(&bytes);
            let lidos = decodificar(&buffer, 1).unwrap();
            assert_eq!(
                lidos,
                vec![TrabalhoNaFila {
                    id: 7,
                    documento: "Doc".to_string(),
                    status: 0x10,
                    paginas_totais: 3,
                    paginas_impressas: 1,
                }]
            );
        }

        #[test]
        fn contagem_que_nao_cabe_no_buffer_e_recusada() {
            let buffer = vec![0u8; size_of::<JOB_INFO_1W>()];
            assert!(decodificar(&buffer, 2).is_err());
            assert!(decodificar(&buffer, u32::MAX).is_err());
            assert_eq!(decodificar(&[], 0).unwrap(), Vec::new());
        }

        #[test]
        fn ponteiro_fora_do_buffer_nao_e_seguido() {
            let hostil = PWSTR(16usize as *mut u16);
            let buffer = bytes_de(&registro(1, hostil));
            assert_eq!(decodificar(&buffer, 1).unwrap()[0].documento, "");
            let alem = PWSTR(usize::MAX as *mut u16);
            let buffer = bytes_de(&registro(1, alem));
            assert_eq!(decodificar(&buffer, 1).unwrap()[0].documento, "");
        }

        #[test]
        fn texto_sem_zero_final_para_no_fim_do_buffer() {
            let mut buffer = vec![b'a'; 64];
            buffer.push(b'a');
            let ponteiro = buffer.as_ptr().cast::<u16>();
            let texto = texto_no_buffer(&buffer, ponteiro);
            assert_eq!(texto.encode_utf16().count(), 32);
        }

        #[test]
        fn fila_inexistente_vira_erro_do_sistema() {
            assert!(matches!(
                SpoolerDoWindows.trabalhos("PlotSim — esta fila nao existe"),
                Err(ErroSpooler::Sistema { .. })
            ));
            assert_eq!(
                SpoolerDoWindows.trabalhos("a\0b"),
                Err(ErroSpooler::TextoInvalido)
            );
        }
    }
}
