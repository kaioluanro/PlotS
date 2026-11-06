//! Chamadas ao spooler e à GDI usadas pelo envio. Cada recurso do sistema fica em um tipo que o
//! libera ao sair de escopo, mesmo em erro. Todas as chamadas podem bloquear enquanto o driver
//! trabalha: nunca devem rodar na thread de uma interface.

use crate::erro::ErroEnvio;
use crate::estado::Estado;
use crate::papel::{
    conferir_devmode, AjustePapel, CampoLido, FacesDoEnvio, LeituraDevmode, DMPAPER_USER,
    ESCALA_REAL, ORIENTACAO_RETRATO, UMA_COPIA, UMA_POR_FOLHA,
};
use crate::preferencias::PerguntaDuasFaces;
use crate::preferencias_driver::{montar, DriverDaFila};
use crate::raster::{bytes_do_bitmap, Dispositivo, PlanoPagina};
use crate::texto::{deslocamento_no_buffer, texto_utf16_le};
use crate::LIMITE_BUFFER_BYTES;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::{w, Error, HRESULT, PCWSTR};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_PARAMETER, HWND};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, CreateDCW, DeleteDC, DeleteObject, GetDeviceCaps,
    SelectObject, SetDIBits, SetStretchBltMode, StretchBlt, StretchDIBits, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DEVMODEW, DEVMODE_FIELD_FLAGS, DIB_RGB_COLORS, DMCOLLATE_FALSE,
    DMDUP_SIMPLEX, DMDUP_VERTICAL, DM_COLLATE, DM_COPIES, DM_DUPLEX, DM_FORMNAME, DM_IN_BUFFER,
    DM_IN_PROMPT, DM_NUP, DM_ORIENTATION, DM_OUT_BUFFER, DM_PAPERLENGTH, DM_PAPERSIZE,
    DM_PAPERWIDTH, DM_SCALE, HALFTONE, HBITMAP, HDC, HGDIOBJ, HORZRES, LOGPIXELSX, LOGPIXELSY,
    PHYSICALHEIGHT, PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH, RASTERCAPS, RC_STRETCHDIB,
    SRCCOPY, STRETCH_BLT_MODE, VERTRES,
};
use windows::Win32::Graphics::Printing::{
    ClosePrinter, DocumentPropertiesW, GetJobW, GetPrinterW, OpenPrinterW, SetJobW,
    JOB_CONTROL_DELETE, JOB_INFO_2W, PRINTER_HANDLE, PRINTER_INFO_2W,
};
use windows::Win32::Storage::Xps::{
    AbortDoc, DeviceCapabilitiesW, EndDoc, EndPage, StartDocW, StartPage, DC_DUPLEX, DOCINFOW,
};

/// Pergunta ao driver da própria fila, sem abrir nada na fila e sem mudar nada.
pub(crate) struct DriverDoSistema;

impl PerguntaDuasFaces for DriverDoSistema {
    fn dc_duplex(&self, fila: &str, porta: &str) -> i32 {
        let para_o_sistema =
            |texto: &str| -> Vec<u16> { texto.encode_utf16().chain(std::iter::once(0)).collect() };
        let nome = para_o_sistema(fila);
        // A porta pode vir em lista: o driver quer uma só.
        let porta = para_o_sistema(porta.split(',').next().unwrap_or_default().trim());
        // Garantia: os dois textos terminam em zero e vivem durante a chamada; sem buffer de saída
        // nem `DEVMODE`, a função só devolve o número.
        unsafe {
            DeviceCapabilitiesW(
                PCWSTR::from_raw(nome.as_ptr()),
                PCWSTR::from_raw(porta.as_ptr()),
                DC_DUPLEX,
                None,
                None,
            )
        }
    }
}

/// Nível de `GetJobW` que traz o estado e o texto de estado do trabalho.
const NIVEL_INFO_2: u32 = 2;
/// Quantas vezes pedir o trabalho: o texto de estado pode crescer entre o tamanho e a leitura.
const LEITURAS_DO_TRABALHO: u32 = 5;

pub(crate) fn sistema(chamada: &'static str) -> ErroEnvio {
    ErroEnvio::Sistema {
        chamada,
        codigo: Error::from_thread().code().0 as u32,
    }
}

fn hresult(codigo: u32) -> HRESULT {
    HRESULT::from_win32(codigo)
}

/// Texto terminado em zero. Um zero no meio cortaria o nome em silêncio, então é recusado.
fn utf16_com_zero(unidades: impl Iterator<Item = u16>) -> Result<Vec<u16>, ErroEnvio> {
    let mut texto: Vec<u16> = unidades.collect();
    if texto.contains(&0) {
        return Err(ErroEnvio::TextoInvalido);
    }
    texto.push(0);
    Ok(texto)
}

/// Fila aberta. `OpenPrinterW` com permissões padrão basta para ler e cancelar os próprios
/// trabalhos e não altera nada na fila.
pub(crate) struct Impressora {
    handle: PRINTER_HANDLE,
    nome: Vec<u16>,
}

impl Impressora {
    pub(crate) fn abrir(nome: &str) -> Result<Self, ErroEnvio> {
        let nome = utf16_com_zero(nome.encode_utf16())?;
        let mut handle = PRINTER_HANDLE::default();
        // Garantia: `nome` termina em zero e vive durante a chamada; `handle` é um destino local
        // válido. Sem `pdefault`, nada na fila é alterado.
        unsafe { OpenPrinterW(PCWSTR::from_raw(nome.as_ptr()), &mut handle, None) }.map_err(
            |erro| ErroEnvio::Sistema {
                chamada: "OpenPrinterW",
                codigo: erro.code().0 as u32,
            },
        )?;
        Ok(Self { handle, nome })
    }

    /// `DEVMODE` da fila com os campos que o envio precisa fixar, partindo do ajuste salvo quando
    /// há. O driver informa o tamanho, preenche os seus valores e depois valida o que foi mudado; o
    /// que ele não aplicar é recusado antes de qualquer envio.
    pub(crate) fn devmode(
        &self,
        ajuste: AjustePapel,
        faces: FacesDoEnvio,
        salvo: Option<&[u8]>,
    ) -> Result<BufferDevmode, ErroEnvio> {
        montar(self, ajuste, faces, salvo)
    }

    /// Estado atual do trabalho. `None` quando ele já não está na fila.
    pub(crate) fn ler_trabalho(&self, trabalho: u32) -> Result<Option<LeituraTrabalho>, ErroEnvio> {
        let buffer = ler_com_repeticao(|destino, pedido| {
            // Garantia: `destino`, quando há, é um slice vivo e exclusivo cujo tamanho vai junto
            // na chamada, e o sistema escreve no máximo esse número de bytes; `pedido` é uma
            // referência válida. `self.handle` é válido enquanto `self` existe.
            let feito = unsafe { GetJobW(self.handle, trabalho, NIVEL_INFO_2, destino, pedido) };
            if feito.as_bool() {
                Ok(())
            } else {
                Err(Error::from_thread().code())
            }
        })?;
        buffer.as_deref().map(decodificar_trabalho).transpose()
    }

    /// Estado atual da fila. `None` quando não deu para ler: quem não sabe o estado não dá alarme,
    /// e a consulta do trabalho continua valendo sozinha.
    pub(crate) fn ler_estado(&self) -> Option<Estado> {
        let buffer = ler_com_repeticao(|destino, pedido| {
            // Garantia: `destino`, quando há, é um slice vivo e exclusivo cujo tamanho vai junto
            // na chamada, e o sistema escreve no máximo esse número de bytes; `pedido` é uma
            // referência válida. `self.handle` é válido enquanto `self` existe.
            unsafe { GetPrinterW(self.handle, NIVEL_INFO_2, destino, pedido) }
                .map_err(|erro| erro.code())
        })
        .ok()??;
        decodificar_estado_da_fila(&buffer)
    }

    /// Quantos trabalhos há na fila. Só os testes de envio precisam disso, para conferir que
    /// nada ficou para trás.
    #[cfg(test)]
    pub(crate) fn trabalhos_na_fila(&self) -> Result<u32, ErroEnvio> {
        const NIVEL_INFO_1: u32 = 1;
        const MAXIMO: u32 = 1_000;
        let (mut pedido, mut contagem) = (0u32, 0u32);
        // Garantia: sem buffer, o sistema só escreve nos dois destinos locais.
        let primeira = unsafe {
            windows::Win32::Graphics::Printing::EnumJobsW(
                self.handle,
                0,
                MAXIMO,
                NIVEL_INFO_1,
                None,
                &mut pedido,
                &mut contagem,
            )
        };
        if primeira.is_ok() && pedido == 0 {
            return Ok(0);
        }
        let mut buffer = vec![0u8; usize::try_from(pedido).unwrap()];
        // Garantia: `buffer` é vivo e exclusivo, e o tamanho dele vai junto na chamada.
        unsafe {
            windows::Win32::Graphics::Printing::EnumJobsW(
                self.handle,
                0,
                MAXIMO,
                NIVEL_INFO_1,
                Some(buffer.as_mut_slice()),
                &mut pedido,
                &mut contagem,
            )
        }
        .map_err(|erro| ErroEnvio::Sistema {
            chamada: "EnumJobsW",
            codigo: erro.code().0 as u32,
        })?;
        Ok(contagem)
    }

    /// Tira o trabalho da fila. Se ele já saiu, não há o que cancelar.
    pub(crate) fn cancelar(&self, trabalho: u32) -> Result<(), ErroEnvio> {
        // Garantia: `self.handle` é válido; sem dados de nível zero, o sistema lê só o comando.
        let feito = unsafe { SetJobW(self.handle, trabalho, 0, None, JOB_CONTROL_DELETE) };
        if feito.as_bool() {
            return Ok(());
        }
        let erro = Error::from_thread().code();
        if erro == hresult(ERROR_INVALID_PARAMETER.0) {
            return Ok(());
        }
        Err(ErroEnvio::Sistema {
            chamada: "SetJobW",
            codigo: erro.0 as u32,
        })
    }
}

impl DriverDaFila for Impressora {
    fn tamanho(&self) -> i32 {
        // Garantia: `self.handle` e o nome são válidos. Com modo zero e sem buffers, a função só
        // devolve o tamanho.
        unsafe {
            DocumentPropertiesW(
                None,
                self.handle,
                PCWSTR::from_raw(self.nome.as_ptr()),
                None,
                None,
                0,
            )
        }
    }

    fn ler(&self, destino: &mut BufferDevmode) -> i32 {
        // Garantia: `destino` tem o tamanho pedido pelo driver, está alinhado para `DEVMODEW` e
        // vive durante a chamada.
        unsafe {
            DocumentPropertiesW(
                None,
                self.handle,
                PCWSTR::from_raw(self.nome.as_ptr()),
                Some(destino.ponteiro_mut()),
                None,
                DM_OUT_BUFFER.0,
            )
        }
    }

    fn mesclar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode) -> i32 {
        // Garantia: os dois buffers têm pelo menos o tamanho pedido pelo driver e são distintos.
        unsafe {
            DocumentPropertiesW(
                None,
                self.handle,
                PCWSTR::from_raw(self.nome.as_ptr()),
                Some(destino.ponteiro_mut()),
                Some(entrada.ponteiro()),
                (DM_IN_BUFFER | DM_OUT_BUFFER).0,
            )
        }
    }

    fn perguntar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode, dono: isize) -> i32 {
        // Garantia: como em `mesclar`. `dono` foi conferido por quem chamou; zero é sem dona.
        unsafe {
            DocumentPropertiesW(
                (dono != 0).then_some(HWND(dono as *mut core::ffi::c_void)),
                self.handle,
                PCWSTR::from_raw(self.nome.as_ptr()),
                Some(destino.ponteiro_mut()),
                Some(entrada.ponteiro()),
                (DM_IN_PROMPT | DM_IN_BUFFER | DM_OUT_BUFFER).0,
            )
        }
    }
}

impl Drop for Impressora {
    fn drop(&mut self) {
        // Garantia: o handle veio de `OpenPrinterW` e só é fechado aqui. Falhar ao fechar não
        // tem como ser tratado durante a liberação.
        let _ = unsafe { ClosePrinter(self.handle) };
    }
}

/// Lê um registro de tamanho variável: pergunta o tamanho e lê, repetindo quando o registro
/// cresce no meio do caminho. `None` quando o trabalho já não está na fila. O tamanho informado
/// é hostil e passa pelo limite do buffer.
fn ler_com_repeticao(
    mut chamada: impl FnMut(Option<&mut [u8]>, &mut u32) -> Result<(), HRESULT>,
) -> Result<Option<Vec<u8>>, ErroEnvio> {
    let mut buffer: Vec<u8> = Vec::new();
    for _ in 0..LEITURAS_DO_TRABALHO {
        let mut pedido: u32 = 0;
        let destino = (!buffer.is_empty()).then_some(buffer.as_mut_slice());
        let tem_destino = destino.is_some();
        match chamada(destino, &mut pedido) {
            Ok(()) if tem_destino => return Ok(Some(buffer)),
            Ok(()) => return Err(ErroEnvio::Resposta("trabalho lido sem buffer")),
            Err(erro) if erro == hresult(ERROR_INVALID_PARAMETER.0) => return Ok(None),
            Err(erro) if erro == hresult(ERROR_INSUFFICIENT_BUFFER.0) => {
                if pedido == 0 || pedido > LIMITE_BUFFER_BYTES {
                    return Err(ErroEnvio::Resposta("tamanho inválido para o trabalho"));
                }
                let tamanho = usize::try_from(pedido)
                    .map_err(|_| ErroEnvio::Resposta("tamanho inválido para o trabalho"))?;
                buffer = vec![0u8; tamanho];
            }
            Err(erro) => {
                return Err(ErroEnvio::Sistema {
                    chamada: "GetJobW",
                    codigo: erro.0 as u32,
                })
            }
        }
    }
    Err(ErroEnvio::Resposta(
        "o registro do trabalho mudou de tamanho a cada leitura",
    ))
}

/// O que o spooler diz de um trabalho.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LeituraTrabalho {
    pub status: u32,
    /// Texto livre que o driver pode informar; vazio quando não há.
    pub texto: String,
}

fn decodificar_trabalho(buffer: &[u8]) -> Result<LeituraTrabalho, ErroEnvio> {
    let tamanho_registro = std::mem::size_of::<JOB_INFO_2W>();
    let bytes = buffer
        .get(..tamanho_registro)
        .ok_or(ErroEnvio::Resposta("registro do trabalho fora do buffer"))?;
    // Garantia: `bytes` tem exatamente o tamanho do registro. `read_unaligned` não exige
    // alinhamento; os campos são inteiros ou ponteiros brutos e nenhum é desreferenciado aqui.
    let info: JOB_INFO_2W =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<JOB_INFO_2W>()) };
    let endereco = info.pStatus.0 as usize;
    let texto = if endereco == 0 {
        String::new()
    } else {
        deslocamento_no_buffer(buffer.as_ptr() as usize, buffer.len(), endereco)
            .and_then(|inicio| buffer.get(inicio..))
            .map(texto_utf16_le)
            .unwrap_or_default()
    };
    Ok(LeituraTrabalho {
        status: info.Status,
        texto,
    })
}

/// Lê só os dois campos inteiros do registro da fila. Buffer menor que o registro é `None`.
fn decodificar_estado_da_fila(buffer: &[u8]) -> Option<Estado> {
    let bytes = buffer.get(..std::mem::size_of::<PRINTER_INFO_2W>())?;
    // Garantia: `bytes` tem exatamente o tamanho do registro. `read_unaligned` não exige
    // alinhamento; os campos são inteiros ou ponteiros brutos e nenhum é desreferenciado aqui.
    let info: PRINTER_INFO_2W =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<PRINTER_INFO_2W>()) };
    Some(Estado::de_status_e_atributos(info.Status, info.Attributes))
}

/// Bloco do `DEVMODE` com os dados privados do driver depois da parte pública. Fica em `u64`
/// para garantir o alinhamento de `DEVMODEW`.
pub(crate) struct BufferDevmode(Vec<u64>);

impl BufferDevmode {
    pub(crate) fn com_tamanho(tamanho: i32) -> Result<Self, ErroEnvio> {
        let tamanho = u32::try_from(tamanho)
            .ok()
            .filter(|bytes| *bytes <= LIMITE_BUFFER_BYTES)
            .and_then(|bytes| usize::try_from(bytes).ok())
            .filter(|bytes| *bytes >= std::mem::size_of::<DEVMODEW>())
            .ok_or_else(|| sistema("DocumentPropertiesW"))?;
        Ok(Self(vec![0u64; tamanho.div_ceil(8)]))
    }

    pub(crate) fn ponteiro_mut(&mut self) -> *mut DEVMODEW {
        self.0.as_mut_ptr().cast()
    }

    pub(crate) fn ponteiro(&self) -> *const DEVMODEW {
        self.0.as_ptr().cast()
    }

    /// Copia `bytes` para o início do buffer. O que não couber fica de fora.
    pub(crate) fn preencher(&mut self, bytes: &[u8]) {
        for (palavra, pedaco) in self.0.iter_mut().zip(bytes.chunks(8)) {
            let mut oito = [0u8; 8];
            for (destino, byte) in oito.iter_mut().zip(pedaco) {
                *destino = *byte;
            }
            *palavra = u64::from_ne_bytes(oito);
        }
    }

    /// Os primeiros `total` bytes, se o buffer tiver tantos.
    pub(crate) fn copia_de(&self, total: usize) -> Option<Vec<u8>> {
        let mut bytes: Vec<u8> = self
            .0
            .iter()
            .flat_map(|palavra| palavra.to_ne_bytes())
            .collect();
        (total <= bytes.len()).then(|| {
            bytes.truncate(total);
            bytes
        })
    }

    /// O campo de páginas por folha vem depois da parte antiga do `DEVMODE`: um driver com
    /// `dmSize` menor não o tem, e escrever ali atingiria os dados privados dele.
    fn tem_paginas_por_folha(&self) -> bool {
        // Garantia: o buffer tem pelo menos `size_of::<DEVMODEW>()` bytes, conferidos em
        // `com_tamanho`, e está alinhado a 8 bytes.
        let modo = unsafe { &*self.ponteiro() };
        let fim_do_campo = std::mem::offset_of!(DEVMODEW, Anonymous2) + std::mem::size_of::<u32>();
        usize::from(modo.dmSize) >= fim_do_campo
    }

    /// Fixa uma cópia, escala real, retrato, uma página por folha, uma face, sem agrupar, e o
    /// papel. A bandeja continua a do driver. As preferências da fila não valem aqui: o
    /// alinhamento entre frente e verso depende desses valores, então eles não são preferência.
    pub(crate) fn aplicar(&mut self, ajuste: AjustePapel) {
        let tem_paginas_por_folha = self.tem_paginas_por_folha();
        // Garantia: o buffer tem pelo menos `size_of::<DEVMODEW>()` bytes, conferidos em
        // `com_tamanho`, está alinhado a 8 bytes e foi preenchido pelo driver. A referência
        // exclusiva vale só dentro desta função.
        let modo = unsafe { &mut *self.ponteiro_mut() };
        modo.dmFields |=
            DM_DUPLEX | DM_COLLATE | DM_PAPERSIZE | DM_COPIES | DM_SCALE | DM_ORIENTATION;
        modo.dmDuplex = DMDUP_SIMPLEX;
        modo.dmCollate = DMCOLLATE_FALSE;
        modo.Anonymous1.Anonymous1.dmCopies = UMA_COPIA;
        modo.Anonymous1.Anonymous1.dmScale = ESCALA_REAL;
        modo.Anonymous1.Anonymous1.dmOrientation = ORIENTACAO_RETRATO;
        if tem_paginas_por_folha {
            modo.dmFields |= DM_NUP;
            modo.Anonymous2.dmNup = UMA_POR_FOLHA;
        }
        // O nome do formulário venceria o papel pedido em alguns drivers.
        modo.dmFields = DEVMODE_FIELD_FLAGS(modo.dmFields.0 & !DM_FORMNAME.0);
        match ajuste {
            AjustePapel::Formato(formato) => {
                modo.dmFields =
                    DEVMODE_FIELD_FLAGS(modo.dmFields.0 & !(DM_PAPERLENGTH.0 | DM_PAPERWIDTH.0));
                modo.Anonymous1.Anonymous1.dmPaperSize = formato;
                modo.Anonymous1.Anonymous1.dmPaperWidth = 0;
                modo.Anonymous1.Anonymous1.dmPaperLength = 0;
            }
            AjustePapel::Medidas {
                largura,
                comprimento,
            } => {
                modo.Anonymous1.Anonymous1.dmPaperSize = DMPAPER_USER;
                modo.Anonymous1.Anonymous1.dmPaperWidth = largura;
                modo.Anonymous1.Anonymous1.dmPaperLength = comprimento;
                modo.dmFields |= DM_PAPERWIDTH | DM_PAPERLENGTH;
            }
        }
    }

    /// Pede as duas faces, com a virada pela borda longa. Vem depois de `aplicar`, que fixa o resto.
    pub(crate) fn pedir_duas_faces(&mut self) {
        // Garantia: mesmo tamanho e alinhamento de `aplicar`; a referência exclusiva vale só aqui.
        let modo = unsafe { &mut *self.ponteiro_mut() };
        modo.dmFields |= DM_DUPLEX;
        modo.dmDuplex = DMDUP_VERTICAL;
    }

    /// Em duas faces, o que vale é a resposta do driver: ele precisa ter ficado com o campo marcado
    /// e com a virada pedida. Qualquer outra resposta impede o envio.
    pub(crate) fn conferir_duas_faces(&self) -> Result<(), ErroEnvio> {
        // Garantia: mesmo tamanho e alinhamento de `aplicar`; o driver preencheu o buffer.
        let modo = unsafe { &*self.ponteiro() };
        if modo.dmFields.0 & DM_DUPLEX.0 != 0 && modo.dmDuplex.0 == DMDUP_VERTICAL.0 {
            Ok(())
        } else {
            Err(ErroEnvio::DuasFacesNaoAceitas)
        }
    }

    /// Um driver sem duplex pode deixar o campo sem efeito; só um valor de duas faces é recusado.
    pub(crate) fn conferir_simplex(&self) -> Result<(), ErroEnvio> {
        // Garantia: mesmo tamanho e alinhamento de `aplicar`; o driver preencheu o buffer.
        let modo = unsafe { &*self.ponteiro() };
        let duas_faces = modo.dmDuplex.0 != DMDUP_SIMPLEX.0 && modo.dmDuplex.0 != 0;
        if duas_faces {
            return Err(ErroEnvio::SimplexNaoAceito);
        }
        Ok(())
    }

    /// Lê os campos que `aplicar` fixou e recusa o que o driver não aplicou.
    pub(crate) fn conferir_aplicado(&self) -> Result<(), ErroEnvio> {
        let tem_paginas_por_folha = self.tem_paginas_por_folha();
        // Garantia: mesmo tamanho e alinhamento de `aplicar`; o driver preencheu o buffer. Os
        // campos dessas uniões lidos aqui são inteiros, válidos em qualquer padrão de bits.
        let (marcados, publico, paginas_por_folha) = unsafe {
            let modo = &*self.ponteiro();
            (
                modo.dmFields.0,
                modo.Anonymous1.Anonymous1,
                modo.Anonymous2.dmNup,
            )
        };
        let campo = |bit: DEVMODE_FIELD_FLAGS, valor: i64| CampoLido {
            marcado: marcados & bit.0 != 0,
            valor,
        };
        conferir_devmode(&LeituraDevmode {
            copias: campo(DM_COPIES, publico.dmCopies.into()),
            escala: campo(DM_SCALE, publico.dmScale.into()),
            orientacao: campo(DM_ORIENTATION, publico.dmOrientation.into()),
            paginas_por_folha: tem_paginas_por_folha
                .then(|| campo(DM_NUP, paginas_por_folha.into())),
        })
    }
}

/// Contexto GDI da fila, com o `DEVMODE` já aplicado.
pub(crate) struct ContextoFila {
    hdc: HDC,
}

impl ContextoFila {
    pub(crate) fn criar(nome_fila: &str, devmode: &BufferDevmode) -> Result<Self, ErroEnvio> {
        let nome = utf16_com_zero(nome_fila.encode_utf16())?;
        // Garantia: `nome` termina em zero e `devmode` foi validado pelo driver; ambos vivem
        // durante a chamada, e o sistema copia o que precisa.
        let hdc = unsafe {
            CreateDCW(
                w!("WINSPOOL"),
                PCWSTR::from_raw(nome.as_ptr()),
                PCWSTR::null(),
                Some(devmode.ponteiro()),
            )
        };
        if hdc.is_invalid() {
            return Err(sistema("CreateDCW"));
        }
        Ok(Self { hdc })
    }

    fn medida(&self, indice: windows::Win32::Graphics::Gdi::GET_DEVICE_CAPS_INDEX) -> i32 {
        // Garantia: `self.hdc` é um contexto válido enquanto `self` existe.
        unsafe { GetDeviceCaps(Some(self.hdc), indice) }
    }

    /// Lê resolução e tamanho do papel. Os valores são conferidos por quem os usa.
    pub(crate) fn dispositivo(&self) -> Dispositivo {
        Dispositivo {
            dpi_x: self.medida(LOGPIXELSX),
            dpi_y: self.medida(LOGPIXELSY),
            papel_largura: self.medida(PHYSICALWIDTH),
            papel_altura: self.medida(PHYSICALHEIGHT),
            margem_x: self.medida(PHYSICALOFFSETX),
            margem_y: self.medida(PHYSICALOFFSETY),
            area_largura: self.medida(HORZRES),
            area_altura: self.medida(VERTRES),
        }
    }

    /// O driver desenha DIB direto. Só o caminho de reserva precisa disso: o desenho por bitmap
    /// compatível tenta primeiro, sem consultar esta capacidade.
    pub(crate) fn aceita_bitmap(&self) -> bool {
        self.medida(RASTERCAPS) as u32 & RC_STRETCHDIB != 0
    }

    /// Abre o documento na fila. `saida`, quando há, é o arquivo que recebe o trabalho.
    pub(crate) fn iniciar_documento(
        &self,
        nome_documento: &str,
        saida: Option<&Path>,
    ) -> Result<DocumentoAberto<'_>, ErroEnvio> {
        let nome = utf16_com_zero(nome_documento.encode_utf16())?;
        let arquivo = saida
            .map(|caminho| utf16_com_zero(caminho.as_os_str().encode_wide()))
            .transpose()?;
        let info = DOCINFOW {
            cbSize: std::mem::size_of::<DOCINFOW>() as i32,
            lpszDocName: PCWSTR::from_raw(nome.as_ptr()),
            lpszOutput: arquivo
                .as_ref()
                .map_or(PCWSTR::null(), |texto| PCWSTR::from_raw(texto.as_ptr())),
            lpszDatatype: PCWSTR::null(),
            fwType: 0,
        };
        // Garantia: `info` e os textos para os quais aponta vivem durante a chamada, e os textos
        // terminam em zero. O sistema não guarda esses ponteiros.
        let trabalho = unsafe { StartDocW(self.hdc, &info) };
        let trabalho = u32::try_from(trabalho)
            .ok()
            .filter(|id| *id > 0)
            .ok_or_else(|| sistema("StartDocW"))?;
        Ok(DocumentoAberto {
            contexto: self,
            trabalho,
            terminado: false,
        })
    }
}

impl Drop for ContextoFila {
    fn drop(&mut self) {
        // Garantia: o contexto veio de `CreateDCW` e só é apagado aqui, depois de o documento
        // aberto, que o empresta, já ter sido liberado.
        let _ = unsafe { DeleteDC(self.hdc) };
    }
}

/// Bitmap compatível selecionado em um DC de memória. Os dois recursos são liberados ao sair de
/// escopo, inclusive quando um passo do desenho falha.
struct DesenhoEmMemoria {
    hdc: HDC,
    bitmap: HBITMAP,
    anterior: HGDIOBJ,
    selecionado: bool,
}

impl DesenhoEmMemoria {
    /// Cria o DC e o bitmap e copia os pixels antes de selecionar o bitmap: `SetDIBits` exige um
    /// bitmap que não esteja selecionado em nenhum DC.
    fn novo(
        hdc_destino: HDC,
        largura: i32,
        altura: i32,
        cabecalho: &BITMAPINFO,
        pixels: &[u8],
    ) -> Result<Self, ErroEnvio> {
        let altura_linhas = u32::try_from(altura).map_err(|_| ErroEnvio::TamanhoInvalido)?;
        // Garantia: `hdc_destino` é um DC vivo da fila.
        let hdc = unsafe { CreateCompatibleDC(Some(hdc_destino)) };
        if hdc.is_invalid() {
            return Err(sistema("CreateCompatibleDC"));
        }
        // Garantia: `hdc_destino` é válido e o bitmap nasce ligado à mesma configuração da fila.
        let bitmap = unsafe { CreateCompatibleBitmap(hdc_destino, largura, altura) };
        if bitmap.is_invalid() {
            let _ = unsafe { DeleteDC(hdc) };
            return Err(sistema("CreateCompatibleBitmap"));
        }
        // Garantia: o bitmap ainda não está selecionado; `pixels` foi conferido e tem exatamente
        // largura × altura × 4 bytes, o que o cabeçalho descreve. O `hdc` é ignorado com
        // `DIB_RGB_COLORS`; o sistema só lê `pixels` durante a chamada.
        let linhas = unsafe {
            SetDIBits(
                Some(hdc),
                bitmap,
                0,
                altura_linhas,
                pixels.as_ptr().cast(),
                cabecalho,
                DIB_RGB_COLORS,
            )
        };
        if linhas != i32::try_from(altura_linhas).unwrap_or(i32::MAX) {
            let _ = unsafe { DeleteObject(bitmap.into()) };
            let _ = unsafe { DeleteDC(hdc) };
            return Err(sistema("SetDIBits"));
        }
        let mut desenho = Self {
            hdc,
            bitmap,
            anterior: HGDIOBJ::default(),
            selecionado: false,
        };
        // Garantia: o DC de memória e o bitmap são válidos, e o bitmap ainda não está em outro DC.
        let anterior = unsafe { SelectObject(hdc, bitmap.into()) };
        if anterior.is_invalid() {
            return Err(sistema("SelectObject"));
        }
        desenho.anterior = anterior;
        desenho.selecionado = true;
        Ok(desenho)
    }
}

impl Drop for DesenhoEmMemoria {
    fn drop(&mut self) {
        // Garantia: `hdc` e `bitmap` vieram de `CreateCompatibleDC`/`CreateCompatibleBitmap` e só
        // são liberados aqui. O bitmap antigo volta ao DC antes de o novo ser apagado.
        unsafe {
            if self.selecionado {
                let _ = SelectObject(self.hdc, self.anterior);
            }
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.hdc);
        }
    }
}

/// Quantas páginas passaram pelo caminho de bitmap compatível nos testes.
#[cfg(test)]
pub(crate) static DESENHOS_PELA_MEMORIA: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Leva o bitmap do DC de memória ao DC da fila, em meio-tom, na posição e no tamanho do plano.
fn desenhar_pela_memoria(
    hdc: HDC,
    plano: &PlanoPagina,
    pixels: &[u8],
    cabecalho: &BITMAPINFO,
) -> Result<(), ErroEnvio> {
    let desenho =
        DesenhoEmMemoria::novo(hdc, plano.largura_px, plano.altura_px, cabecalho, pixels)?;
    // Garantia: `hdc` é o DC da fila com a página aberta; o modo anterior é um inteiro válido.
    let modo_anterior = unsafe { SetStretchBltMode(hdc, HALFTONE) };
    if modo_anterior == 0 {
        return Err(sistema("SetStretchBltMode"));
    }
    // Garantia: `desenho.hdc` tem o bitmap selecionado, e `hdc` é válido. O sistema lê a origem e
    // escreve no destino durante a chamada.
    let desenhado = unsafe {
        StretchBlt(
            hdc,
            plano.x,
            plano.y,
            plano.largura_px,
            plano.altura_px,
            Some(desenho.hdc),
            0,
            0,
            plano.largura_px,
            plano.altura_px,
            SRCCOPY,
        )
    };
    let _ = unsafe { SetStretchBltMode(hdc, STRETCH_BLT_MODE(modo_anterior)) };
    if !desenhado.as_bool() {
        return Err(sistema("StretchBlt"));
    }
    #[cfg(test)]
    DESENHOS_PELA_MEMORIA.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

/// Desenho direto do DIB de 32 bpp no DC da fila. Fica como reserva para quando o caminho de
/// memória não estiver disponível.
fn desenhar_com_dib(
    hdc: HDC,
    plano: &PlanoPagina,
    pixels: &[u8],
    cabecalho: &BITMAPINFO,
) -> Result<(), ErroEnvio> {
    // Garantia: `pixels` foi conferido e tem exatamente largura × altura × 4 bytes, o que o
    // cabeçalho descreve; o sistema só lê essa região durante a chamada.
    let linhas = unsafe {
        StretchDIBits(
            hdc,
            plano.x,
            plano.y,
            plano.largura_px,
            plano.altura_px,
            0,
            0,
            plano.largura_px,
            plano.altura_px,
            Some(pixels.as_ptr().cast()),
            cabecalho,
            DIB_RGB_COLORS,
            SRCCOPY,
        )
    };
    if linhas <= 0 {
        return Err(sistema("StretchDIBits"));
    }
    Ok(())
}

/// Documento em andamento. Sair de escopo sem terminar aborta o trabalho.
pub(crate) struct DocumentoAberto<'a> {
    contexto: &'a ContextoFila,
    trabalho: u32,
    terminado: bool,
}

impl DocumentoAberto<'_> {
    /// Desenha uma página inteira. `pixels` traz BGRA, de cima para baixo, sem folga entre linhas.
    pub(crate) fn desenhar_pagina(
        &mut self,
        plano: &PlanoPagina,
        pixels: &[u8],
    ) -> Result<(), ErroEnvio> {
        let esperado = u64::try_from(plano.largura_px)
            .ok()
            .zip(u64::try_from(plano.altura_px).ok())
            .ok_or(ErroEnvio::TamanhoInvalido)
            .and_then(|(largura, altura)| bytes_do_bitmap(largura, altura))?;
        if u64::try_from(pixels.len()).ok() != Some(esperado) {
            return Err(ErroEnvio::Rasterizacao(
                "o bitmap não tem o tamanho pedido".to_string(),
            ));
        }
        let hdc = self.contexto.hdc;
        // Garantia: `hdc` é válido e tem um documento aberto.
        if unsafe { StartPage(hdc) } <= 0 {
            return Err(sistema("StartPage"));
        }
        let cabecalho = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: plano.largura_px,
                // Altura negativa: as linhas vão de cima para baixo, como a biblioteca entrega.
                biHeight: plano.altura_px.saturating_neg(),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        // O caminho clássico do GDI: o bitmap compatível passa por um DC de memória e chega ao
        // DC da fila por `StretchBlt`. Se algum passo dele falhar, o DIB direto continua valendo.
        if let Err(erro_da_memoria) = desenhar_pela_memoria(hdc, plano, pixels, &cabecalho) {
            if !self.contexto.aceita_bitmap() {
                return Err(ErroEnvio::SemSuporteABitmap);
            }
            if let Err(erro_do_dib) = desenhar_com_dib(hdc, plano, pixels, &cabecalho) {
                return Err(match erro_do_dib {
                    ErroEnvio::Sistema { .. } => erro_do_dib,
                    _ => erro_da_memoria,
                });
            }
        }
        // Garantia: `hdc` é válido e tem uma página aberta.
        if unsafe { EndPage(hdc) } <= 0 {
            return Err(sistema("EndPage"));
        }
        Ok(())
    }

    /// Entrega o documento ao spooler.
    pub(crate) fn terminar(mut self) -> Result<u32, ErroEnvio> {
        // Garantia: `hdc` é válido e tem um documento aberto.
        if unsafe { EndDoc(self.contexto.hdc) } <= 0 {
            return Err(sistema("EndDoc"));
        }
        self.terminado = true;
        Ok(self.trabalho)
    }
}

impl Drop for DocumentoAberto<'_> {
    fn drop(&mut self) {
        if !self.terminado {
            // Garantia: `hdc` é válido e tem um documento aberto que não foi entregue.
            let _ = unsafe { AbortDoc(self.contexto.hdc) };
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::estado::Condicao;
    use windows::Win32::Foundation::ERROR_ACCESS_DENIED;
    use windows::Win32::Graphics::Gdi::{
        DEVMODEW_0_0, DEVMODE_DUPLEX, DMCOLLATE_TRUE, DMDUP_VERTICAL,
    };

    #[test]
    fn estado_da_fila_vem_do_registro_e_buffer_curto_nao_vira_alarme() {
        let info = PRINTER_INFO_2W {
            Status: 0x8,
            Attributes: 0x400,
            ..Default::default()
        };
        let tamanho = std::mem::size_of::<PRINTER_INFO_2W>();
        // Garantia: o registro é local e vive durante o uso; só seus bytes são lidos.
        let bytes =
            unsafe { std::slice::from_raw_parts(std::ptr::from_ref(&info).cast::<u8>(), tamanho) };
        let estado = decodificar_estado_da_fila(bytes).unwrap();
        assert_eq!(estado.condicoes(), &[Condicao::Offline, Condicao::Atolada]);
        assert_eq!(decodificar_estado_da_fila(&bytes[..tamanho - 1]), None);
        assert_eq!(decodificar_estado_da_fila(&[]), None);
    }

    /// Buffer como o driver devolveria: `dmSize` com a parte pública inteira e valores "errados"
    /// em tudo que o envio precisa forçar.
    fn devmode_do_driver() -> BufferDevmode {
        let mut buffer =
            BufferDevmode::com_tamanho(std::mem::size_of::<DEVMODEW>() as i32).unwrap();
        // Garantia: `com_tamanho` reservou o bloco com tamanho e alinhamento de `DEVMODEW`.
        let modo = unsafe { &mut *buffer.ponteiro_mut() };
        modo.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        modo.dmFields = DEVMODE_FIELD_FLAGS(DM_PAPERLENGTH.0 | DM_PAPERWIDTH.0 | DM_FORMNAME.0);
        modo.dmDuplex = DMDUP_VERTICAL;
        modo.dmCollate = DMCOLLATE_TRUE;
        modo.Anonymous1.Anonymous1 = DEVMODEW_0_0 {
            dmOrientation: 2,
            dmPaperSize: 1,
            dmPaperLength: 2794,
            dmPaperWidth: 2159,
            dmScale: 50,
            dmCopies: 3,
            dmDefaultSource: 7,
            dmPrintQuality: 600,
        };
        modo.Anonymous2.dmNup = 1;
        buffer
    }

    /// Campos do buffer, lidos como o driver os leria.
    fn campos(buffer: &BufferDevmode) -> (u32, DEVMODEW_0_0, u32) {
        // Garantia: o buffer tem o tamanho de `DEVMODEW` e os campos lidos são inteiros, válidos
        // em qualquer padrão de bits.
        unsafe {
            let modo = &*buffer.ponteiro();
            (
                modo.dmFields.0,
                modo.Anonymous1.Anonymous1,
                modo.Anonymous2.dmNup,
            )
        }
    }

    #[test]
    fn aplicar_forca_copias_escala_orientacao_e_uma_por_folha() {
        let mut buffer = devmode_do_driver();
        buffer.aplicar(AjustePapel::Formato(9));
        let (marcados, publico, nup) = campos(&buffer);
        assert_eq!(publico.dmCopies, 1);
        assert_eq!(publico.dmScale, 100);
        assert_eq!(publico.dmOrientation, 1);
        assert_eq!(nup, 2);
        for campo in [
            DM_COPIES,
            DM_SCALE,
            DM_ORIENTATION,
            DM_NUP,
            DM_DUPLEX,
            DM_COLLATE,
        ] {
            assert_ne!(marcados & campo.0, 0, "{:#x}", campo.0);
        }
        // Garantia: mesmo buffer, com tamanho de `DEVMODEW`.
        let modo = unsafe { &*buffer.ponteiro() };
        assert_eq!(modo.dmDuplex, DMDUP_SIMPLEX);
        assert_eq!(modo.dmCollate, DMCOLLATE_FALSE);
        // A bandeja e a qualidade continuam as do driver.
        assert_eq!(publico.dmDefaultSource, 7);
        assert_eq!(publico.dmPrintQuality, 600);
    }

    #[test]
    fn papel_por_formato_limpa_medidas_e_nome_do_formulario() {
        let mut buffer = devmode_do_driver();
        buffer.aplicar(AjustePapel::Formato(9));
        let (marcados, publico, _) = campos(&buffer);
        assert_eq!(publico.dmPaperSize, 9);
        assert_eq!(publico.dmPaperWidth, 0);
        assert_eq!(publico.dmPaperLength, 0);
        assert_ne!(marcados & DM_PAPERSIZE.0, 0);
        for campo in [DM_PAPERWIDTH, DM_PAPERLENGTH, DM_FORMNAME] {
            assert_eq!(marcados & campo.0, 0, "{:#x}", campo.0);
        }
    }

    #[test]
    fn papel_por_medidas_usa_as_medidas_e_limpa_o_nome_do_formulario() {
        let mut buffer = devmode_do_driver();
        buffer.aplicar(AjustePapel::Medidas {
            largura: 1000,
            comprimento: 1505,
        });
        let (marcados, publico, _) = campos(&buffer);
        assert_eq!(publico.dmPaperSize, DMPAPER_USER);
        assert_eq!(publico.dmPaperWidth, 1000);
        assert_eq!(publico.dmPaperLength, 1505);
        for campo in [DM_PAPERSIZE, DM_PAPERWIDTH, DM_PAPERLENGTH] {
            assert_ne!(marcados & campo.0, 0, "{:#x}", campo.0);
        }
        assert_eq!(marcados & DM_FORMNAME.0, 0);
    }

    #[test]
    fn conferir_simplex_recusa_duas_faces_e_aceita_uma_ou_campo_sem_efeito() {
        let mut buffer = devmode_do_driver();
        buffer.aplicar(AjustePapel::Formato(9));
        assert_eq!(buffer.conferir_simplex(), Ok(()));
        // Garantia: mesmo buffer, com tamanho de `DEVMODEW`, e o acesso é só uma escrita.
        unsafe { (*buffer.ponteiro_mut()).dmDuplex = DMDUP_VERTICAL };
        assert_eq!(buffer.conferir_simplex(), Err(ErroEnvio::SimplexNaoAceito));
        // Garantia: idem.
        unsafe { (*buffer.ponteiro_mut()).dmDuplex = DEVMODE_DUPLEX(0) };
        assert_eq!(buffer.conferir_simplex(), Ok(()));
    }

    #[test]
    fn duas_faces_com_o_bit_desligado_tambem_sao_recusadas() {
        let mut buffer = devmode_do_driver();
        // Garantia: mesmo buffer, com tamanho de `DEVMODEW`; o acesso é só uma escrita.
        unsafe { (*buffer.ponteiro_mut()).dmDuplex = DMDUP_VERTICAL };
        assert_eq!(buffer.conferir_simplex(), Err(ErroEnvio::SimplexNaoAceito));
    }

    #[test]
    fn devmode_curto_nao_recebe_o_campo_de_paginas_por_folha() {
        let mut buffer = devmode_do_driver();
        // Garantia: mesmo buffer; o acesso é só uma escrita em um inteiro.
        unsafe {
            (*buffer.ponteiro_mut()).dmSize = std::mem::offset_of!(DEVMODEW, Anonymous2) as u16;
        }
        buffer.aplicar(AjustePapel::Formato(9));
        let (marcados, _, nup) = campos(&buffer);
        assert_eq!(nup, 1, "o valor original fica como estava");
        assert_eq!(marcados & DM_NUP.0, 0);
        // O valor é ignorado na conferência, porque o campo não existe nesse driver.
        assert_eq!(buffer.conferir_aplicado(), Ok(()));
    }

    #[test]
    fn conferir_aplicado_passa_depois_de_aplicar_e_recusa_o_que_o_driver_mudou() {
        let mut buffer = devmode_do_driver();
        buffer.aplicar(AjustePapel::Formato(9));
        assert_eq!(buffer.conferir_aplicado(), Ok(()));
        // Garantia: mesmo buffer; a escrita é em um inteiro de uma união de inteiros.
        unsafe { (*buffer.ponteiro_mut()).Anonymous1.Anonymous1.dmCopies = 2 };
        assert_eq!(
            buffer.conferir_aplicado(),
            Err(ErroEnvio::ValorNaoAceito {
                campo: "o número de cópias",
                pedido: 1,
                obtido: 2,
            })
        );
    }

    #[test]
    fn leitura_repete_enquanto_o_registro_cresce() {
        let mut tamanhos = [100u32, 200].into_iter();
        let mut recebidos = Vec::new();
        let lido = ler_com_repeticao(|destino, pedido| {
            recebidos.push(destino.map(|b| b.len()));
            match tamanhos.next() {
                Some(tamanho) => {
                    *pedido = tamanho;
                    Err(hresult(ERROR_INSUFFICIENT_BUFFER.0))
                }
                None => Ok(()),
            }
        })
        .unwrap();
        assert_eq!(lido.unwrap().len(), 200);
        assert_eq!(recebidos, [None, Some(100), Some(200)]);
    }

    #[test]
    fn registro_que_nunca_para_de_crescer_vira_erro_em_poucas_tentativas() {
        let mut chamadas = 0;
        let erro = ler_com_repeticao(|_, pedido| {
            chamadas += 1;
            *pedido = 100 + chamadas;
            Err(hresult(ERROR_INSUFFICIENT_BUFFER.0))
        })
        .unwrap_err();
        assert!(matches!(erro, ErroEnvio::Resposta(_)), "{erro:?}");
        assert_eq!(chamadas, LEITURAS_DO_TRABALHO);
    }

    #[test]
    fn trabalho_que_saiu_da_fila_em_qualquer_leitura_e_nenhum() {
        assert_eq!(
            ler_com_repeticao(|_, _| Err(hresult(ERROR_INVALID_PARAMETER.0))),
            Ok(None)
        );
        let mut primeira = true;
        let lido = ler_com_repeticao(|_, pedido| {
            if primeira {
                primeira = false;
                *pedido = 100;
                Err(hresult(ERROR_INSUFFICIENT_BUFFER.0))
            } else {
                Err(hresult(ERROR_INVALID_PARAMETER.0))
            }
        });
        assert_eq!(lido, Ok(None));
    }

    #[test]
    fn tamanho_hostil_outro_erro_e_sucesso_sem_buffer_viram_erro() {
        for tamanho in [0, LIMITE_BUFFER_BYTES + 1, u32::MAX] {
            let erro = ler_com_repeticao(|_, pedido| {
                *pedido = tamanho;
                Err(hresult(ERROR_INSUFFICIENT_BUFFER.0))
            })
            .unwrap_err();
            assert!(matches!(erro, ErroEnvio::Resposta(_)), "{tamanho}");
        }
        let erro = ler_com_repeticao(|_, _| Err(hresult(ERROR_ACCESS_DENIED.0))).unwrap_err();
        assert!(matches!(erro, ErroEnvio::Sistema { .. }), "{erro:?}");
        let erro = ler_com_repeticao(|_, _| Ok(())).unwrap_err();
        assert!(matches!(erro, ErroEnvio::Resposta(_)), "{erro:?}");
    }

    #[test]
    fn nome_com_zero_no_meio_e_recusado() {
        let erro = utf16_com_zero("a\0b".encode_utf16()).unwrap_err();
        assert_eq!(erro, ErroEnvio::TextoInvalido);
        assert_eq!(
            utf16_com_zero("ab".encode_utf16()).unwrap(),
            vec![97, 98, 0]
        );
    }

    #[test]
    fn registro_de_trabalho_menor_que_o_esperado_vira_erro() {
        let erro = decodificar_trabalho(&[0u8; 4]).unwrap_err();
        assert!(matches!(erro, ErroEnvio::Resposta(_)), "{erro:?}");
    }

    #[test]
    fn texto_de_estado_fora_do_buffer_fica_vazio_e_nao_e_lido() {
        let mut buffer = vec![0u8; std::mem::size_of::<JOB_INFO_2W>()];
        // Garantia: o registro é zerado, um padrão válido para inteiros e ponteiros brutos, e
        // o ponteiro inventado nunca é desreferenciado.
        let mut info: JOB_INFO_2W = unsafe { std::mem::zeroed() };
        info.Status = 0x10;
        info.pStatus = windows::core::PWSTR(usize::MAX as *mut u16);
        // Garantia: copia os bytes do próprio registro para um buffer do mesmo tamanho.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                (&info as *const JOB_INFO_2W).cast::<u8>(),
                std::mem::size_of::<JOB_INFO_2W>(),
            )
        };
        buffer.copy_from_slice(bytes);
        let leitura = decodificar_trabalho(&buffer).unwrap();
        assert_eq!(leitura.status, 0x10);
        assert_eq!(leitura.texto, "");
    }
}
